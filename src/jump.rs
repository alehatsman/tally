//! The press that puts the real terminal in front of you.
//!
//! An anchor session is a PTY in a daemon, not a window. Something has to
//! be attached for there to be anywhere to jump *to*, and that something
//! has to be findable and focusable (`docs/06-jump.md`).
//!
//! **That something is a pane** (`docs/15-views.md`). It used to be a
//! window, which was right while `tallyd` under launchd was the only thing
//! creating views and wrong as a definition — a pane is the unit you can
//! adopt, the unit that can outlive its command, and the unit a return
//! gesture can name.
//!
//! The pane is a **view**. Closing it does not touch the agent, because
//! anchor owns the PTY; reopening replays the scrollback from offset 0.
//! That is anchor's whole pitch, used as intended.
//!
//! Everything here is behind [`Jump`] with one implementation, which is
//! not speculative generality — it is the seam that keeps a hard platform
//! dependency out of the state layer.

use std::collections::BTreeMap;
use std::fmt;
use std::process::{Command, Output};
use std::sync::Mutex;

/// The tmux session that holds view panes tally creates for itself.
///
/// A dedicated one, because `tallyd` runs under launchd with no `$TMUX`
/// and possibly **no attached client at all** — so every tmux default that
/// means "the current one" resolves to something arbitrary. Views tally
/// opens must not land in the middle of whatever you were working in.
///
/// A view you hand it — `tally spawn` adopting the pane you ran it from —
/// lives wherever you are instead, which is the point of adopting it.
pub const VIEW_SESSION: &str = "tally";

/// The **pane** option carrying an anchor session id.
///
/// An option rather than a name convention: names get renamed by the
/// programs running in them, options do not, and a jump that finds the
/// wrong view is worse than one that creates a new one.
///
/// **Pane-scoped, and that is load-bearing.** A window option is inherited
/// by every pane in the window — measured, `docs/15-views.md` — so a split
/// of a view window produces two panes both claiming the same session. A
/// pane option is not inherited by siblings.
pub const PANE_OPTION: &str = "@tally_session";

/// Why a jump did not happen.
#[derive(Debug)]
pub enum Error {
    /// `tmux` is not installed, or would not run.
    NoTmux(String),
    /// A tmux command failed.
    Tmux {
        /// What was asked for.
        what: &'static str,
        /// What tmux said.
        stderr: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoTmux(e) => write!(f, "tmux is not available: {e}"),
            Self::Tmux { what, stderr } => write!(f, "tmux {what}: {}", stderr.trim()),
        }
    }
}

impl std::error::Error for Error {}

/// Put a session in front of the user, and take them back again.
pub trait Jump {
    /// Focus the view for `session`, creating or reviving it if it is gone.
    ///
    /// `at` is the slot address the pad carries — `page:row:col` — and is
    /// used only to name a view this creates. It is passed rather than
    /// derived because this module knows about tmux and nothing else.
    ///
    /// # Errors
    /// Whatever stopped it.
    fn jump(&self, session: &str, at: &str) -> Result<(), Error>;

    /// Put each client back where it was standing before it jumped.
    ///
    /// `Ok(false)` when there was nowhere to go. That is not an error: a
    /// client that has not jumped, or whose origin pane has since been
    /// closed, has no honest destination, and landing somewhere arbitrary
    /// is worse than not moving (`docs/16-the-queue.md`).
    ///
    /// # Errors
    /// Whatever stopped it.
    fn back(&self) -> Result<bool, Error>;

    /// Is there anywhere for [`Jump::back`] to go, for any client?
    ///
    /// Read by the renderer to decide whether `◄` is lit, and by the press
    /// to decide whether it does anything — one predicate, two readers,
    /// which is the rule `src/act.rs` already holds for the chords.
    fn can_go_back(&self) -> bool;

    /// Close the view pane for `session`, if there is one.
    ///
    /// # Errors
    /// Whatever stopped it.
    fn close(&self, session: &str) -> Result<(), Error>;

    /// The sessions currently in front of an attached client.
    ///
    /// The queue excludes these: `►` must not offer you the thing already
    /// filling your terminal (`docs/16-the-queue.md`). Returns empty when
    /// tmux cannot be asked, which degrades to offering you everything —
    /// the safe direction, since the worst case is a press that lands you
    /// where you already are.
    fn viewing(&self) -> Vec<String>;
}

// ── command construction, separated so the arguments are testable ──────────

/// Args listing every pane, its window, whether it is dead, and its tag.
///
/// **The possibly-empty field goes last.** An unset option prints as an
/// empty string, so putting it anywhere else would make the fields after
/// it shift left and the parse silently wrong.
fn list_panes_args() -> Vec<String> {
    vec![
        "list-panes".into(),
        "-a".into(),
        "-F".into(),
        format!("#{{pane_id}} #{{window_id}} #{{pane_dead}} #{{{PANE_OPTION}}}"),
    ]
}

/// Args creating a view pane for `session`, detached, in [`VIEW_SESSION`].
fn new_window_args(session: &str, at: &str, anchor_bin: &str) -> Vec<String> {
    vec![
        "new-window".into(),
        "-d".into(),
        "-P".into(),
        "-F".into(),
        "#{pane_id}".into(),
        "-t".into(),
        format!("{VIEW_SESSION}:"),
        "-n".into(),
        view_name(session, at),
        format!("{anchor_bin} attach {session}"),
    ]
}

/// Args creating the view session *with* its first view already in it.
///
/// `new-session -d -s tally` on its own leaves a stray shell window that
/// nothing asked for and nothing cleans up. Creating the session around
/// the first view means one is never made.
fn new_session_args(session: &str, at: &str, anchor_bin: &str) -> Vec<String> {
    vec![
        "new-session".into(),
        "-d".into(),
        "-s".into(),
        VIEW_SESSION.into(),
        "-P".into(),
        "-F".into(),
        "#{pane_id}".into(),
        "-n".into(),
        view_name(session, at),
        format!("{anchor_bin} attach {session}"),
    ]
}

/// Args tagging `pane` with the session it views.
///
/// **`-p` is load-bearing.** With `-w` this sets a *window* option, which
/// every pane in the window inherits — so splitting a view would give two
/// panes claiming one session, and a jump would confidently land on either.
/// Measured on tmux 3.6b, not theorised (`docs/15-views.md`).
fn set_option_args(pane: &str, session: &str) -> Vec<String> {
    vec![
        "set-option".into(),
        "-p".into(),
        "-t".into(),
        pane.into(),
        PANE_OPTION.into(),
        session.into(),
    ]
}

/// Args making `pane` outlive the command running in it.
///
/// Without this, `anchor attach` exiting takes the pane, then the window,
/// then the session, then your client — which is the detach in
/// [#2](https://github.com/alehatsman/tally/issues/2). Pane-scoped because
/// an adopted view sits in a window full of panes that are **yours**, and
/// tally does not get to change how those behave.
fn remain_on_exit_args(pane: &str) -> Vec<String> {
    vec![
        "set-option".into(),
        "-p".into(),
        "-t".into(),
        pane.into(),
        "remain-on-exit".into(),
        "on".into(),
    ]
}

/// Args clearing a legacy window-scoped tag.
///
/// Views created before the identity moved to the pane carry the option on
/// their window, where every pane inherits it. They resolve correctly today
/// because such a window has one pane — so this is not a repair, it is
/// removing an ambiguity before someone splits one.
fn clear_window_option_args(window: &str) -> Vec<String> {
    vec![
        "set-option".into(),
        "-uw".into(),
        "-t".into(),
        window.into(),
        PANE_OPTION.into(),
    ]
}

/// Args bringing a dead view back **in the pane it already occupies**.
///
/// The pane keeps its id across a respawn — measured — so it keeps its
/// tag, and the pad goes on meaning the place you put it. Without this an
/// adopted view that died would be abandoned for a window under `tally`,
/// and the pad would quietly stop pointing at your layout.
fn respawn_pane_args(pane: &str, session: &str, anchor_bin: &str) -> Vec<String> {
    vec![
        "respawn-pane".into(),
        "-t".into(),
        pane.into(),
        format!("{anchor_bin} attach {session}"),
    ]
}

/// Args switching one client to `pane`.
///
/// tmux(1): *"As a special case, -t may refer to a pane … to change
/// session, window and pane."* One command, not three.
fn switch_args(client: &str, pane: &str) -> Vec<String> {
    vec![
        "switch-client".into(),
        "-c".into(),
        client.into(),
        "-t".into(),
        pane.into(),
    ]
}

/// Args closing one pane.
fn kill_pane_args(pane: &str) -> Vec<String> {
    vec!["kill-pane".into(), "-t".into(), pane.into()]
}

/// A name you can read: the pad it belongs to, then a short tail of the
/// session id.
///
/// `1:4:4-Z8YN` rather than `tl-Z8YN`. The address is the part you
/// actually know — it is where your finger goes, and it is what the
/// `tl.slot` label says — whereas four characters of a ULID identify the
/// session to nobody. The tail stays because a slot outlives the sessions
/// in it: run something on `1:4:4` twice and the old view may still be
/// open, and two windows called `1:4:4` is a list you cannot read.
///
/// **Only views tally creates get named.** An adopted pane lives in a
/// window that is yours and probably holds other things; renaming it would
/// be tally redecorating a room it was invited into.
fn view_name(session: &str, at: &str) -> String {
    let tail: String = session
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if at.is_empty() {
        return format!("tl-{tail}");
    }
    format!("{at}-{tail}")
}

/// A view found in a `list-panes` listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct View<'a> {
    /// The pane holding it.
    pane: &'a str,
    /// The window that pane is in, for clearing a legacy tag.
    window: &'a str,
    /// Is its command gone? **Not** the same as the agent being gone:
    /// detaching from `anchor attach` also ends the attach, and the pad is
    /// what answers which (`docs/15-views.md`).
    dead: bool,
}

/// Find the view for `session` in `list-panes` output.
///
/// An **unset** option prints as an empty field, so an untagged pane must
/// never match — otherwise an empty session id would claim the first
/// untagged pane on the server, which is every pane you work in.
fn view_for<'a>(listing: &'a str, session: &str) -> Option<View<'a>> {
    if session.is_empty() {
        return None;
    }
    listing.lines().find_map(|line| {
        let mut f = line.split_whitespace();
        let pane = f.next()?;
        let window = f.next()?;
        let dead = f.next()?;
        (f.next()? == session).then_some(View {
            pane,
            window,
            dead: dead == "1",
        })
    })
}

/// Every attached client and the pane it is looking at.
///
/// ⚠️ **`display-message -p -c <client>` is the wrong call**, and it looks
/// more correct than this one. `-c` chooses where a message would be
/// *displayed*, not what the target resolves against — it was measured
/// returning a pane in an unrelated session (`docs/15-views.md`).
fn clients(listing: &str) -> Vec<(&str, &str)> {
    listing
        .lines()
        .filter_map(|l| {
            let (client, pane) = l.trim().split_once(' ')?;
            (!client.is_empty() && !pane.is_empty()).then_some((client, pane))
        })
        .collect()
}

// ── the tmux implementation ────────────────────────────────────────────────

/// Drives tmux by running it.
#[derive(Debug)]
pub struct Tmux {
    /// Path to the `anchor` binary a view runs.
    anchor_bin: String,
    /// Where each client was standing before it jumped.
    ///
    /// **One deep, per client, non-views only.** A jump switches every
    /// attached client on purpose, so "where I was" is per client and never
    /// global; and a view is never recorded as an origin, so
    /// `next → next → next → back` lands where you started rather than
    /// three agents deep (`docs/16-the-queue.md`).
    ///
    /// In memory, so a `tallyd` restart loses it — and the degradation is
    /// that `◄` goes dark until you jump again, which is honest.
    origin: Mutex<BTreeMap<String, String>>,
}

impl Default for Tmux {
    fn default() -> Self {
        Self::with_anchor("anchor")
    }
}

impl Tmux {
    /// Use a specific `anchor` binary in views.
    pub fn with_anchor(anchor_bin: impl Into<String>) -> Self {
        Self {
            anchor_bin: anchor_bin.into(),
            origin: Mutex::new(BTreeMap::new()),
        }
    }

    /// Is a tmux server reachable?
    ///
    /// Checked rather than inferred, and **not** fatal: a missing server is
    /// the normal state at login, and the grid is still truthful without
    /// one. It is the jump that has no target, and the jump starts a server
    /// when it has to (`docs/06-jump.md`).
    pub fn server_running(&self) -> bool {
        Command::new("tmux")
            .arg("has-session")
            .output()
            .is_ok_and(|o| o.status.success())
    }

    fn run(what: &'static str, args: &[String]) -> Result<Output, Error> {
        let out = Command::new("tmux")
            .args(args)
            .output()
            .map_err(|e| Error::NoTmux(e.to_string()))?;
        if out.status.success() {
            return Ok(out);
        }
        Err(Error::Tmux {
            what,
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }

    fn stdout(what: &'static str, args: &[String]) -> Result<String, Error> {
        Ok(String::from_utf8_lossy(&Self::run(what, args)?.stdout).into_owned())
    }

    /// Does [`VIEW_SESSION`] exist?
    fn view_session_exists() -> bool {
        Command::new("tmux")
            .args(["has-session", "-t", VIEW_SESSION])
            .output()
            .is_ok_and(|o| o.status.success())
    }

    fn panes() -> String {
        Self::stdout("list-panes", &list_panes_args()).unwrap_or_default()
    }

    fn client_panes() -> Vec<(String, String)> {
        let listing = Self::stdout(
            "list-clients",
            &["list-clients", "-F", "#{client_name} #{pane_id}"].map(str::to_owned),
        )
        .unwrap_or_default();
        clients(&listing)
            .into_iter()
            .map(|(c, p)| (c.to_owned(), p.to_owned()))
            .collect()
    }

    /// Mark `pane` as the view for `session`, and make it outlive its
    /// command. Idempotent, so it runs on every resolve and repairs views
    /// created before either rule existed.
    fn tag(pane: &str, session: &str) -> Result<(), Error> {
        let _opt = Self::run("set-option", &set_option_args(pane, session))?;
        let _roe = Self::run("set-option", &remain_on_exit_args(pane))?;
        Ok(())
    }

    /// The pane viewing `session`, creating or reviving it if it is gone.
    ///
    /// Three cases, where there used to be two (`docs/15-views.md`):
    /// alive, switch to it; dead, respawn **in place** and switch to it;
    /// absent, create one under [`VIEW_SESSION`].
    fn view_pane(&self, session: &str, at: &str) -> Result<String, Error> {
        let listing = Self::panes();
        if let Some(found) = view_for(&listing, session) {
            if found.dead {
                let args = respawn_pane_args(found.pane, session, &self.anchor_bin);
                let _revived = Self::run("respawn-pane", &args)?;
            }
            Self::tag(found.pane, session)?;
            // After the pane owns the tag, never before: an inherited tag
            // is the only thing identifying a legacy view, and clearing it
            // first would lose the session mid-sequence.
            let _legacy = Self::run("set-option", &clear_window_option_args(found.window));
            return Ok(found.pane.to_owned());
        }
        // Creating the session *around* its first view, rather than making
        // an empty one and adding to it, is what keeps a stray shell window
        // from appearing. `new-session -d` on a dead server also starts one,
        // which is why a missing tmux is not a startup failure.
        let (what, args) = if Self::view_session_exists() {
            ("new-window", new_window_args(session, at, &self.anchor_bin))
        } else {
            (
                "new-session",
                new_session_args(session, at, &self.anchor_bin),
            )
        };
        let pane = Self::stdout(what, &args)?.trim().to_owned();
        Self::tag(&pane, session)?;
        Ok(pane)
    }

    /// Remember where each client is standing, unless it is standing in a
    /// view.
    fn record_origins(&self, views: &str, clients: &[(String, String)]) {
        let Ok(mut origin) = self.origin.lock() else {
            return;
        };
        for (client, pane) in clients {
            if is_view(views, pane) {
                continue;
            }
            let _prev = origin.insert(client.clone(), pane.clone());
        }
    }
}

/// Claim `pane` as the view for `session`: tag it, and make it outlive
/// the command about to replace this process.
///
/// Used by `tally spawn` to adopt the pane it was run in. It is a free
/// function rather than a [`Jump`] method because the caller has no grid,
/// no origin table and nothing to jump *to* — it is already standing
/// where the view is going (`docs/15-views.md`).
///
/// # Errors
/// Whatever tmux said. The caller must treat a failure as fatal **before**
/// it execs: a pane that runs the attach without carrying the tag is a
/// view no pad can ever find.
pub fn adopt(pane: &str, session: &str) -> Result<(), Error> {
    Tmux::tag(pane, session)
}

/// The session a pane views, if it is a view at all.
fn session_of(listing: &str, pane: &str) -> Option<String> {
    listing.lines().find_map(|line| {
        let mut f = line.split_whitespace();
        (f.next() == Some(pane))
            .then(|| f.nth(2))
            .flatten()
            .map(ToOwned::to_owned)
    })
}

/// Is `pane` a view — that is, does it carry a non-empty tag?
fn is_view(listing: &str, pane: &str) -> bool {
    listing.lines().any(|line| {
        let mut f = line.split_whitespace();
        f.next() == Some(pane) && f.nth(2).is_some_and(|tag| !tag.is_empty())
    })
}

/// The macOS app to bring forward after switching.
///
/// `switch-client` moves tmux's idea of where you are; it does nothing
/// about a terminal sitting behind a browser. `open -a` focuses the
/// existing app rather than starting a second one.
///
/// Overridable because this is the one genuinely machine-specific string
/// in the file — the rest of tmux is the same everywhere.
fn terminal_app() -> Option<String> {
    crate::env_path("TALLY_TERMINAL")
        .map(|p| p.to_string_lossy().into_owned())
        .or_else(|| Some("Alacritty".to_owned()))
}

/// Bring the terminal emulator forward. Best-effort and deliberately not
/// an error: tmux has already been switched, so the move has happened even
/// if the window manager will not cooperate.
fn raise() {
    if let Some(app) = terminal_app() {
        let _raised = Command::new("open").args(["-a", &app]).output();
    }
}

impl Jump for Tmux {
    fn jump(&self, session: &str, at: &str) -> Result<(), Error> {
        let pane = self.view_pane(session, at)?;
        let clients = Tmux::client_panes();
        self.record_origins(&Tmux::panes(), &clients);

        // Switch **every** client. A jump means "put this in front of me",
        // and which terminal "me" is sitting at is not knowable from here.
        // Zero clients is not an error: the view exists and is correct,
        // and it is there the next time you attach.
        for (client, _) in &clients {
            Tmux::run("switch-client", &switch_args(client, &pane))?;
        }
        raise();
        Ok(())
    }

    fn back(&self) -> Result<bool, Error> {
        let taken: Vec<(String, String)> = {
            let Ok(mut origin) = self.origin.lock() else {
                return Ok(false);
            };
            // Taken, not read: a return is spent. Pressing `◄` twice must
            // not bounce you between two places, and an origin that is
            // gone should go dark rather than stay on offer.
            std::mem::take(&mut *origin).into_iter().collect()
        };
        if taken.is_empty() {
            return Ok(false);
        }
        let live = Tmux::panes();
        let mut moved = false;
        for (client, pane) in taken {
            if !live
                .lines()
                .any(|l| l.split_whitespace().next() == Some(&*pane))
            {
                continue; // the origin closed while you were away
            }
            Tmux::run("switch-client", &switch_args(&client, &pane))?;
            moved = true;
        }
        if moved {
            raise();
        }
        Ok(moved)
    }

    fn can_go_back(&self) -> bool {
        self.origin.lock().is_ok_and(|o| !o.is_empty())
    }

    fn viewing(&self) -> Vec<String> {
        let panes = Tmux::panes();
        Tmux::client_panes()
            .into_iter()
            .filter_map(|(_, pane)| session_of(&panes, &pane))
            .collect()
    }

    fn close(&self, session: &str) -> Result<(), Error> {
        let listing = Tmux::panes();
        let Some(found) = view_for(&listing, session) else {
            return Ok(());
        };
        let _closed = Tmux::run("kill-pane", &kill_pane_args(found.pane))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SESSION: &str = "01M52GEKED8ZEJPMT2KCJ35ZWT";
    /// The slot the pad lives on, as `page:row:col`.
    const AT: &str = "1:4:4";

    /// `#{pane_id} #{window_id} #{pane_dead} #{@tally_session}`
    const LISTING: &str = "\
%1 @1 0 \n\
%2 @2 0 other-session\n\
%3 @2 0 01M52GEKED8ZEJPMT2KCJ35ZWT\n\
%4 @3 1 01M52MXB4WQTA79K5FTHMXZ8YN\n\
%5 @3 0 \n";

    #[test]
    fn views_tally_creates_go_to_a_dedicated_session() {
        let args = new_window_args(SESSION, AT, "anchor");
        assert!(
            args.contains(&"-d".to_owned()),
            "detached: do not steal focus to create it"
        );
        let at = args.iter().position(|a| a == "-t").expect("-t");
        assert_eq!(args.get(at + 1).map(String::as_str), Some("tally:"));
        assert!(
            args.last()
                .is_some_and(|c| c == &format!("anchor attach {SESSION}")),
            "the view runs the attach"
        );
    }

    /// A freshly created window has exactly one pane, so the id it prints
    /// is the view. Printing a window id instead would hand the rest of
    /// this module the wrong kind of target.
    #[test]
    fn creating_a_view_prints_a_pane_not_a_window() {
        for args in [
            new_window_args(SESSION, AT, "anchor"),
            new_session_args(SESSION, AT, "anchor"),
        ] {
            let f = args.iter().position(|a| a == "-F").expect("-F");
            assert_eq!(args.get(f + 1).map(String::as_str), Some("#{pane_id}"));
        }
    }

    /// Under launchd there is no `$TMUX`, so every tmux default meaning
    /// "the current one" resolves to something arbitrary. Nothing here may
    /// rely on one.
    #[test]
    fn no_command_relies_on_a_current_session_or_client() {
        assert!(switch_args("/dev/ttys000", "%7").contains(&"-c".to_owned()));
        assert!(new_window_args(SESSION, AT, "anchor").contains(&"-t".to_owned()));
        for args in [
            set_option_args("%7", SESSION),
            remain_on_exit_args("%7"),
            respawn_pane_args("%7", SESSION, "anchor"),
            kill_pane_args("%7"),
            clear_window_option_args("@7"),
        ] {
            assert!(args.contains(&"-t".to_owned()), "{args:?} names its target");
        }
    }

    #[test]
    fn the_view_is_found_by_option_not_by_name() {
        let found = view_for(LISTING, SESSION).expect("tagged pane");
        assert_eq!(found.pane, "%3");
        assert_eq!(found.window, "@2");
        assert!(!found.dead);
        assert_eq!(view_for(LISTING, "nope"), None);
    }

    /// An unset option prints as an empty field. An untagged pane must
    /// never match, or an empty session id claims the first pane on the
    /// server — which is every pane you work in.
    #[test]
    fn an_untagged_pane_is_not_a_candidate() {
        assert_eq!(view_for("%9 @9 0 \n", SESSION), None);
        assert_eq!(view_for(LISTING, ""), None, "an empty tag matches nothing");
    }

    /// The field that can be empty goes last, so nothing after it shifts
    /// left when it is. This is the parse the whole module rests on.
    #[test]
    fn a_dead_view_is_distinguished_from_a_live_one() {
        let dead = view_for(LISTING, "01M52MXB4WQTA79K5FTHMXZ8YN").expect("dead pane");
        assert!(dead.dead, "pane_dead=1 means nothing is attached here");
        assert_eq!(dead.pane, "%4");
    }

    /// The bug a window option would reintroduce: every pane in the window
    /// inherits it, so a split of a view claims the same session.
    #[test]
    fn the_tag_is_a_pane_option_not_a_window_one() {
        let args = set_option_args("%7", SESSION);
        assert!(
            args.contains(&"-p".to_owned()),
            "with -w a split of a view would claim the same session"
        );
        assert!(!args.contains(&"-w".to_owned()));
    }

    /// `remain-on-exit` is pane-scoped too, and for a second reason: an
    /// adopted view sits among panes that are yours.
    #[test]
    fn remain_on_exit_is_set_on_the_pane_alone() {
        let args = remain_on_exit_args("%7");
        assert!(args.contains(&"-p".to_owned()));
        assert_eq!(args.last().map(String::as_str), Some("on"));
    }

    #[test]
    fn a_dead_view_is_revived_in_the_pane_it_already_occupies() {
        let args = respawn_pane_args("%4", SESSION, "anchor");
        let at = args.iter().position(|a| a == "-t").expect("-t");
        assert_eq!(args.get(at + 1).map(String::as_str), Some("%4"));
        assert!(
            args.last()
                .is_some_and(|c| c == &format!("anchor attach {SESSION}")),
            "revival runs the attach again"
        );
    }

    /// Legacy tags are cleared at window scope, which is the only scope
    /// they were ever set at.
    #[test]
    fn a_legacy_window_tag_is_unset_not_overwritten() {
        let args = clear_window_option_args("@2");
        assert!(args.contains(&"-uw".to_owned()));
        assert_eq!(args.last().map(String::as_str), Some(PANE_OPTION));
    }

    #[test]
    fn every_client_is_read_with_the_pane_it_is_on() {
        assert_eq!(
            clients("/dev/ttys000 %99\n/dev/ttys003 %12\n"),
            [("/dev/ttys000", "%99"), ("/dev/ttys003", "%12")]
        );
        assert!(
            clients("").is_empty(),
            "zero clients is a valid state, not an error"
        );
        assert!(clients("\n\n").is_empty());
        assert!(clients("/dev/ttys000\n").is_empty(), "a pane is required");
    }

    #[test]
    fn a_pane_is_a_view_only_when_it_carries_a_tag() {
        assert!(is_view(LISTING, "%3"));
        assert!(is_view(LISTING, "%4"), "a dead view is still a view");
        assert!(!is_view(LISTING, "%1"), "untagged");
        assert!(!is_view(LISTING, "%5"), "untagged");
        assert!(!is_view(LISTING, "%404"), "not on the server at all");
    }

    /// The address leads, because that is the part you know: it is where
    /// your finger goes and what the `tl.slot` label says.
    #[test]
    fn a_view_is_named_after_the_pad_it_belongs_to() {
        assert_eq!(view_name(SESSION, AT), "1:4:4-5ZWT");
        assert_eq!(view_name(SESSION, "work:3:5"), "work:3:5-5ZWT");
    }

    /// A slot outlives the sessions in it. Run something on the same pad
    /// twice while the old view is open and two of them share a name.
    #[test]
    fn two_sessions_on_one_pad_get_different_names() {
        let first = view_name("01M52GEKED8ZEJPMT2KCJ35ZWT", AT);
        let second = view_name("01M52MXB4WQTA79K5FTHMXZ8YN", AT);
        assert_ne!(first, second);
        assert!(first.starts_with(AT) && second.starts_with(AT));
    }

    #[test]
    fn a_nameless_pad_still_produces_a_usable_name() {
        assert_eq!(view_name("ab", ""), "tl-ab");
        assert_eq!(view_name("", ""), "tl-");
        assert_eq!(view_name("", AT), "1:4:4-");
    }

    /// Creating the view session must not leave a shell window behind.
    #[test]
    fn the_view_session_is_created_around_its_first_view() {
        let args = new_session_args(SESSION, AT, "anchor");
        assert!(args.contains(&"-d".to_owned()));
        let at = args.iter().position(|a| a == "-s").expect("-s");
        assert_eq!(args.get(at + 1).map(String::as_str), Some(VIEW_SESSION));
        assert!(
            args.last()
                .is_some_and(|c| c == &format!("anchor attach {SESSION}")),
            "the session's first pane IS a view, not a stray shell"
        );
    }

    #[test]
    fn the_option_name_is_the_one_docs_promise() {
        assert_eq!(PANE_OPTION, "@tally_session");
        assert_eq!(VIEW_SESSION, "tally");
    }

    // ── origin: the half `◄` is built on ───────────────────────────────

    #[test]
    fn where_you_were_standing_is_remembered_per_client() {
        let t = Tmux::default();
        assert!(!t.can_go_back(), "nothing has jumped yet");
        t.record_origins(
            LISTING,
            &[
                ("/dev/ttys000".to_owned(), "%1".to_owned()),
                ("/dev/ttys003".to_owned(), "%5".to_owned()),
            ],
        );
        let o = t.origin.lock().expect("lock");
        assert_eq!(o.get("/dev/ttys000").map(String::as_str), Some("%1"));
        assert_eq!(o.get("/dev/ttys003").map(String::as_str), Some("%5"));
    }

    /// `next → next → next → back` must land where you started, not three
    /// agents deep. A view is never an origin, so a chained jump cannot
    /// overwrite one.
    #[test]
    fn a_chained_jump_does_not_overwrite_where_you_started() {
        let t = Tmux::default();
        t.record_origins(LISTING, &[("c".to_owned(), "%1".to_owned())]);
        // now standing in a view, and jumping again
        t.record_origins(LISTING, &[("c".to_owned(), "%3".to_owned())]);
        t.record_origins(LISTING, &[("c".to_owned(), "%4".to_owned())]);
        let o = t.origin.lock().expect("lock");
        assert_eq!(
            o.get("c").map(String::as_str),
            Some("%1"),
            "home, not the last agent"
        );
    }

    #[test]
    fn a_client_standing_in_a_view_records_no_origin() {
        let t = Tmux::default();
        t.record_origins(LISTING, &[("c".to_owned(), "%3".to_owned())]);
        assert!(!t.can_go_back());
    }

    /// `◄` is dark exactly when there is nowhere to go — one predicate,
    /// read by the renderer and by the press.
    #[test]
    fn back_is_offered_only_when_there_is_somewhere_to_go() {
        let t = Tmux::default();
        assert!(!t.can_go_back());
        t.record_origins(LISTING, &[("c".to_owned(), "%1".to_owned())]);
        assert!(t.can_go_back());
    }
}
