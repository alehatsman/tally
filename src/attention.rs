//! Finding the anchor session a hook is running inside.
//!
//! `docs/04-signals.md` wrote the hook as
//! `anchor label $ANCHOR_SESSION tl.attention=1`. That variable is set by
//! nothing — it appears nowhere in anchor's source and is not in a live
//! session's environment — so the recipe expanded to an empty argument and
//! never worked (`docs/04-signals.md`).
//!
//! The session has to be *found*, and the only key that survives two agents
//! running in one repo — the normal case on this grid — is the process
//! tree. anchord records the agent's pid; a hook is a descendant of it.

use std::collections::BTreeMap;

use crate::anchor::Session;
use crate::signal::SessionState;

/// How far up to walk before giving up.
///
/// A hook is two or three levels under the agent. This is not a tuning
/// knob, it is a guard: `ps` output is parsed, and a cycle in a parsed map
/// would otherwise be an infinite loop in something that runs on every turn
/// of every agent on the machine.
const MAX_DEPTH: usize = 64;

/// pid -> parent pid, as one snapshot of the process table.
#[derive(Debug, Default, Clone)]
pub struct Ancestry(BTreeMap<i32, i32>);

impl Ancestry {
    /// Read the whole table in one `ps`.
    ///
    /// One spawn rather than one per level: a hook already costs a process,
    /// and walking with `ps -o ppid= -p` would cost one per ancestor for a
    /// tree that is three deep and might be eight.
    ///
    /// An unreadable or unparsable table yields an **empty** ancestry, not
    /// an error. Every caller treats "could not find the session" the same
    /// way — silence — so a failure here is already the right answer.
    #[must_use]
    pub fn read() -> Self {
        let Ok(out) = std::process::Command::new("ps")
            .args(["-A", "-o", "pid=,ppid="])
            .output()
        else {
            return Self::default();
        };
        let Ok(text) = String::from_utf8(out.stdout) else {
            return Self::default();
        };
        Self::parse(&text)
    }

    /// Parse `pid ppid` pairs, one per line, ignoring anything else.
    ///
    /// Lenient on purpose: this is a best-effort lookup whose failure mode
    /// is a pad that does not light, and a strict parser would turn one odd
    /// line into no attention at all.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut map = BTreeMap::new();
        for line in text.lines() {
            let mut fields = line.split_whitespace();
            let (Some(pid), Some(ppid)) = (fields.next(), fields.next()) else {
                continue;
            };
            let (Ok(pid), Ok(ppid)) = (pid.parse::<i32>(), ppid.parse::<i32>()) else {
                continue;
            };
            let _previous = map.insert(pid, ppid);
        }
        Self(map)
    }

    /// Build one directly. For tests, and for anything that already has the
    /// table.
    #[must_use]
    pub fn from_pairs(pairs: impl IntoIterator<Item = (i32, i32)>) -> Self {
        Self(pairs.into_iter().collect())
    }
}

/// The session `start` is running inside, if any.
///
/// Walks `start` and then its ancestors, returning the first that some
/// session claims as its pid. **Nearest wins**: agents nest — an agent that
/// spawned an agent is two sessions deep — and the one whose turn just
/// ended is the inner one.
///
/// `None` means *not under anchor*, which is the common case and is not an
/// error: this runs on every turn of every agent on the machine, and almost
/// none of them are on the grid.
#[must_use]
pub fn owning_session<'a>(
    sessions: &'a [Session],
    start: i32,
    ancestry: &Ancestry,
) -> Option<&'a Session> {
    let mut pid = start;
    for _ in 0..MAX_DEPTH {
        // Terminal rows keep their pid in anchor's history, and a pid is
        // reused by the OS eventually. Only a live session can be the one
        // this hook is running inside.
        //
        // Through [`Session::lifecycle`] rather than against the raw
        // strings: this used to match `"running" | "closing"` here, so a
        // state anchord renamed would have been decided twice, once by
        // each spelling. `lifecycle` owns the unknown-is-`Lost` rule.
        if let Some(s) = sessions.iter().find(|s| {
            s.pid == Some(pid)
                && matches!(s.lifecycle(), SessionState::Running | SessionState::Closing)
        }) {
            return Some(s);
        }
        // pid 1 is launchd/init and 0 is the kernel: either is the top.
        if pid <= 1 {
            return None;
        }
        match ancestry.0.get(&pid) {
            Some(&parent) if parent != pid => pid = parent,
            // Unknown pid, or a self-parent in a table we did not write.
            _ => return None,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: &str, pid: i32, state: &str) -> Session {
        Session {
            id: id.to_owned(),
            state: state.to_owned(),
            exit_code: None,
            lost_reason: None,
            labels: BTreeMap::new(),
            output_bytes: 0,
            pid: Some(pid),
            cwd: None,
            command: None,
            login_shell: false,
        }
    }

    /// The whole point: the hook is three levels under the agent anchord
    /// recorded, and it still finds it.
    #[test]
    fn a_hook_finds_the_agent_it_runs_under() {
        let sessions = [session("A", 100, "running")];
        let tree = Ancestry::from_pairs([(400, 300), (300, 200), (200, 100), (100, 1)]);
        let found = owning_session(&sessions, 400, &tree).map(|s| s.id.clone());
        assert_eq!(found, Some("A".to_owned()));
    }

    /// Not under anchor is the common case, not an error.
    #[test]
    fn a_shell_outside_anchor_finds_nothing() {
        let sessions = [session("A", 100, "running")];
        let tree = Ancestry::from_pairs([(900, 800), (800, 1)]);
        assert!(owning_session(&sessions, 900, &tree).is_none());
    }

    /// The case cwd cannot answer, and the reason this is a pid walk:
    /// two agents in one repo are two sessions and must get two pads.
    #[test]
    fn two_agents_in_one_repo_are_told_apart() {
        let sessions = [session("A", 100, "running"), session("B", 500, "running")];
        let tree = Ancestry::from_pairs([(101, 100), (501, 500), (100, 1), (500, 1)]);
        let a = owning_session(&sessions, 101, &tree).map(|s| s.id.clone());
        let b = owning_session(&sessions, 501, &tree).map(|s| s.id.clone());
        assert_eq!((a, b), (Some("A".to_owned()), Some("B".to_owned())));
    }

    /// Agents nest. The turn that just ended belongs to the innermost one,
    /// so the walk must stop at the first match rather than the outermost.
    #[test]
    fn the_nearest_session_wins_when_agents_nest() {
        let sessions = [
            session("outer", 100, "running"),
            session("inner", 300, "running"),
        ];
        let tree = Ancestry::from_pairs([(400, 300), (300, 200), (200, 100), (100, 1)]);
        let found = owning_session(&sessions, 400, &tree).map(|s| s.id.clone());
        assert_eq!(found, Some("inner".to_owned()));
    }

    /// A pid is reused once the process is gone, so an exited row claiming
    /// it would label a session that ended hours ago.
    #[test]
    fn an_exited_row_does_not_claim_a_reused_pid() {
        let sessions = [session("old", 100, "exited")];
        let tree = Ancestry::from_pairs([(200, 100), (100, 1)]);
        assert!(owning_session(&sessions, 200, &tree).is_none());
    }

    /// A parsed table is not one we wrote. A cycle must terminate.
    #[test]
    fn a_cycle_in_the_table_terminates() {
        let sessions = [session("A", 100, "running")];
        let tree = Ancestry::from_pairs([(10, 11), (11, 12), (12, 10)]);
        assert!(owning_session(&sessions, 10, &tree).is_none());
    }

    #[test]
    fn ps_output_parses_and_junk_lines_are_skipped() {
        let tree = Ancestry::parse("  400   300\nnot a row\n  300     1\n\n  bad x\n");
        assert_eq!(tree.0.get(&400), Some(&300));
        assert_eq!(tree.0.get(&300), Some(&1));
        assert_eq!(tree.0.len(), 2);
    }
}
