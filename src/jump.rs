//! The press that puts the real terminal in front of you.
//!
//! An anchor session is a PTY in a daemon, not a window. Something has to
//! be attached for there to be anywhere to jump *to*, and that something
//! has to be findable and focusable — so spawning a slot creates two
//! things: an anchor session, and a tmux window running `anchor attach`
//! in it (`docs/06-jump.md`).
//!
//! The tmux window is a **view**. Closing it does not touch the agent,
//! because anchor owns the PTY; reopening replays the scrollback from
//! offset 0. That is anchor's whole pitch, used as intended.
//!
//! Everything here is behind [`Jump`] with one implementation, which is
//! not speculative generality — it is the seam that keeps a hard platform
//! dependency out of the state layer.

use std::fmt;
use std::process::{Command, Output};

/// The tmux session that holds view windows.
///
/// A dedicated one, because `tallyd` runs under launchd with no `$TMUX`
/// and possibly **no attached client at all** — so every tmux default that
/// means "the current one" resolves to something arbitrary. View windows
/// must not land in the middle of whatever you were working in.
pub const VIEW_SESSION: &str = "tally";

/// The window option carrying an anchor session id.
///
/// An option rather than a name convention: window names get renamed by
/// the programs running in them, options do not, and a jump that finds the
/// wrong window is worse than one that creates a new one.
pub const WINDOW_OPTION: &str = "@tally_session";

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

/// Put a session in front of the user.
pub trait Jump {
    /// Focus the view for `session`, creating it if it is gone.
    ///
    /// `at` is the slot address the pad carries — `page:row:col` — and is
    /// used only to name the window. It is passed rather than derived
    /// because this module knows about tmux and nothing else; a slot is
    /// `crate::slot`'s idea and a session id is anchor's.
    ///
    /// # Errors
    /// Whatever stopped it.
    fn jump(&self, session: &str, at: &str) -> Result<(), Error>;
}

// ── command construction, separated so the arguments are testable ──────────

/// Args listing every window's id alongside its [`WINDOW_OPTION`].
fn list_windows_args() -> Vec<String> {
    vec![
        "list-windows".into(),
        "-a".into(),
        "-F".into(),
        format!("#{{window_id}} #{{{WINDOW_OPTION}}}"),
    ]
}

/// Args creating the view window for `session`, detached, in [`VIEW_SESSION`].
fn new_window_args(session: &str, at: &str, anchor_bin: &str) -> Vec<String> {
    vec![
        "new-window".into(),
        "-d".into(),
        "-P".into(),
        "-F".into(),
        "#{window_id}".into(),
        "-t".into(),
        format!("{VIEW_SESSION}:"),
        "-n".into(),
        window_name(session, at),
        format!("{anchor_bin} attach {session}"),
    ]
}

/// Args tagging `window` with the session it views.
///
/// **`-w` is load-bearing.** Without it `set-option` sets a *session*
/// option, which every window in the session inherits — so every view would
/// report the first session id ever tagged, and a jump would confidently
/// land on the wrong window. A jump that finds the wrong window is worse
/// than one that creates a new one (`docs/06-jump.md`), and this is exactly
/// how you get one. Observed, not theorised.
fn set_option_args(window: &str, session: &str) -> Vec<String> {
    vec![
        "set-option".into(),
        "-w".into(),
        "-t".into(),
        window.into(),
        WINDOW_OPTION.into(),
        session.into(),
    ]
}

/// Args creating the view session *with* its first view already in it.
///
/// `new-session -d -s tally` on its own leaves a stray shell window that
/// nothing asked for and nothing cleans up. Creating the session around the
/// first view means one is never made.
fn new_session_args(session: &str, at: &str, anchor_bin: &str) -> Vec<String> {
    vec![
        "new-session".into(),
        "-d".into(),
        "-s".into(),
        VIEW_SESSION.into(),
        "-P".into(),
        "-F".into(),
        "#{window_id}".into(),
        "-n".into(),
        window_name(session, at),
        format!("{anchor_bin} attach {session}"),
    ]
}

/// Args switching one client to `window`.
fn switch_args(client: &str, window: &str) -> Vec<String> {
    vec![
        "switch-client".into(),
        "-c".into(),
        client.into(),
        "-t".into(),
        window.into(),
    ]
}

/// A window name you can read: the pad it belongs to, then a short tail
/// of the session id.
///
/// `1:4:4-Z8YN` rather than `tl-Z8YN`. The address is the part you
/// actually know — it is where your finger goes, and it is what the
/// `tl.slot` label says — whereas four characters of a ULID identify the
/// session to nobody. The tail stays because a slot outlives the sessions
/// in it: run something on `1:4:4` twice and the old window may still be
/// open, and two windows called `1:4:4` is a list you cannot read.
///
/// **The colon is safe.** tmux splits a target on its *first* colon only,
/// so `tally:1:4:4-Z8YN` resolves to the window of that name — verified
/// against a real server. Nothing here depends on it either way:
/// [`WINDOW_OPTION`] is the identity and this is cosmetic, which is why a
/// program renaming the window breaks nothing.
fn window_name(session: &str, at: &str) -> String {
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

/// Find the window viewing `session` in `list-windows` output.
///
/// An **unset** option prints as an empty field, so a window with no tag
/// must never match — otherwise an empty session id would claim the first
/// untagged window on the server, which is every window you work in.
fn window_for<'a>(listing: &'a str, session: &str) -> Option<&'a str> {
    if session.is_empty() {
        return None;
    }
    listing.lines().find_map(|line| {
        let (window, tagged) = line.split_once(' ')?;
        (tagged.trim() == session).then_some(window)
    })
}

/// Every attached client, from `list-clients` output.
fn clients(listing: &str) -> Vec<&str> {
    listing
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect()
}

// ── the tmux implementation ────────────────────────────────────────────────

/// Drives tmux by running it.
#[derive(Debug, Clone)]
pub struct Tmux {
    /// Path to the `anchor` binary the view window runs.
    anchor_bin: String,
}

impl Default for Tmux {
    fn default() -> Self {
        Self {
            anchor_bin: "anchor".to_owned(),
        }
    }
}

impl Tmux {
    /// Use a specific `anchor` binary in view windows.
    pub fn with_anchor(anchor_bin: impl Into<String>) -> Self {
        Self {
            anchor_bin: anchor_bin.into(),
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

    /// The window viewing `session`, creating it if it is gone.
    ///
    /// A view can disappear — closed, server restarted, a reboot — while
    /// the session is fine. Pressing the pad then **recreates it**, rather
    /// than reporting an error: the pad means "put this in front of me",
    /// and whether that needs a new window is an implementation detail.
    fn view_window(&self, session: &str, at: &str) -> Result<String, Error> {
        let listing = Self::stdout("list-windows", &list_windows_args()).unwrap_or_default();
        if let Some(window) = window_for(&listing, session) {
            return Ok(window.to_owned());
        }
        // Creating the session *around* its first view, rather than making
        // an empty one and adding to it, is what keeps a stray shell window
        // from appearing. `new-session -d` on a dead server also starts one,
        // which is why a missing tmux is not a startup failure.
        let (what, args) = if Self::view_session_exists() {
            ("new-window", new_window_args(session, at, &self.anchor_bin))
        } else {
            ("new-session", new_session_args(session, at, &self.anchor_bin))
        };
        let window = Self::stdout(what, &args)?.trim().to_owned();
        Self::run("set-option", &set_option_args(&window, session))?;
        Ok(window)
    }
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

impl Jump for Tmux {
    fn jump(&self, session: &str, at: &str) -> Result<(), Error> {
        let window = self.view_window(session, at)?;
        let listing = Tmux::stdout(
            "list-clients",
            &["list-clients", "-F", "#{client_name}"].map(str::to_owned),
        )
        .unwrap_or_default();

        // Switch **every** client. A jump means "put this in front of me",
        // and which terminal "me" is sitting at is not knowable from here.
        // Zero clients is not an error: the window exists and is correct,
        // and it is there the next time you attach.
        for client in clients(&listing) {
            Tmux::run("switch-client", &switch_args(client, &window))?;
        }

        // Raising is best-effort and deliberately not an error: tmux has
        // already been switched, so the jump has happened even if the
        // window manager will not cooperate.
        if let Some(app) = terminal_app() {
            let _raised = Command::new("open").args(["-a", &app]).output();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SESSION: &str = "01M52GEKED8ZEJPMT2KCJ35ZWT";
    /// The slot the pad lives on, as `page:row:col`.
    const AT: &str = "1:4:4";

    #[test]
    fn view_windows_go_to_a_dedicated_session() {
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
            "the window runs the attach"
        );
    }

    /// Under launchd there is no `$TMUX`, so every tmux default meaning
    /// "the current one" resolves to something arbitrary. Nothing here may
    /// rely on one.
    #[test]
    fn no_command_relies_on_a_current_session_or_client() {
        let all = [
            list_windows_args(),
            new_window_args(SESSION, AT, "anchor"),
            set_option_args("@7", SESSION),
            switch_args("/dev/ttys000", "@7"),
        ];
        for args in all {
            let joined = args.join(" ");
            assert!(!joined.is_empty());
        }
        // switch-client names its client; new-window names its session.
        assert!(switch_args("/dev/ttys000", "@7").contains(&"-c".to_owned()));
        assert!(new_window_args(SESSION, AT, "anchor").contains(&"-t".to_owned()));
    }

    #[test]
    fn the_window_is_found_by_option_not_by_name() {
        let listing = "@1 \n@2 other-session\n@3 01M52GEKED8ZEJPMT2KCJ35ZWT\n@4 \n";
        assert_eq!(window_for(listing, SESSION), Some("@3"));
        assert_eq!(window_for(listing, "nope"), None);
        // An untagged window must never match, however it is named.
        assert_eq!(
            window_for("@1 \n", ""),
            None,
            "an empty tag is not a match for nothing"
        );
    }

    #[test]
    fn a_window_with_no_option_is_not_a_candidate() {
        // tmux prints an empty field for an unset option, so the line has a
        // trailing space and nothing after it.
        assert_eq!(window_for("@9 ", SESSION), None);
    }

    #[test]
    fn every_client_is_switched_and_none_is_required() {
        assert_eq!(
            clients("/dev/ttys000\n/dev/ttys003\n"),
            ["/dev/ttys000", "/dev/ttys003"]
        );
        assert!(
            clients("").is_empty(),
            "zero clients is a valid state, not an error"
        );
        assert!(clients("\n\n").is_empty());
    }

    /// The address leads, because that is the part you know: it is where
    /// your finger goes and what the `tl.slot` label says. Four characters
    /// of a ULID identify the session to nobody.
    #[test]
    fn a_window_is_named_after_the_pad_it_belongs_to() {
        assert_eq!(window_name(SESSION, AT), "1:4:4-5ZWT");
        assert_eq!(window_name(SESSION, "work:3:5"), "work:3:5-5ZWT");
    }

    /// A slot outlives the sessions in it. Run something on the same pad
    /// twice while the old window is open and two windows share a name,
    /// which is a list you cannot read — so the tail stays.
    #[test]
    fn two_sessions_on_one_pad_get_different_names() {
        let first = window_name("01M52GEKED8ZEJPMT2KCJ35ZWT", AT);
        let second = window_name("01M52MXB4WQTA79K5FTHMXZ8YN", AT);
        assert_ne!(first, second);
        assert!(first.starts_with(AT) && second.starts_with(AT));
    }

    /// Never panics on a short or empty id, and an address-less jump
    /// falls back to the old form rather than to a name starting with `-`.
    #[test]
    fn a_nameless_pad_still_produces_a_usable_name() {
        assert_eq!(window_name("ab", ""), "tl-ab");
        assert_eq!(window_name("", ""), "tl-");
        assert_eq!(window_name("", AT), "1:4:4-");
    }

    /// The bug this caught: without `-w`, `set-option` tags the whole
    /// session and every window in it claims the same anchor session.
    #[test]
    fn the_tag_is_a_window_option_not_a_session_one() {
        let args = set_option_args("@7", SESSION);
        assert!(
            args.contains(&"-w".to_owned()),
            "without -w this tags the session and every window inherits it"
        );
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
            "the session's first window IS a view, not a stray shell"
        );
    }

    #[test]
    fn the_option_name_is_the_one_docs_promise() {
        assert_eq!(WINDOW_OPTION, "@tally_session");
        assert_eq!(VIEW_SESSION, "tally");
    }
}
