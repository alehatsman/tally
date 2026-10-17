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

/// The tmux socket tally runs its own server on (`docs/24-the-tally-server.md`).
///
/// **A separate server, not a separate session.** A session is not an
/// isolation boundary: `list-clients` returns every client on the server,
/// and `jump` switches all of them, so a press used to yank the terminal
/// you were reading a diff in. A named socket cannot see your clients and
/// therefore cannot move them — measured, `docs/24`.
pub const SOCKET: &str = "tally";

/// The session the terminal window lands on when no agent is in front of
/// you.
///
/// Not a row, because a row session may not exist yet and `attach` to an
/// empty server exits with `no sessions`. It is one window running your
/// shell, and it is the one stray shell this module is allowed to make:
/// it is what the window shows when there is nothing to show.
pub const HOME_SESSION: &str = "tally";

/// The **pane** option carrying a slot address, `page:row:col`.
///
/// Read by the pane border and the pane title, and by the placement rule
/// that keeps columns in order. Identity still lives in [`PANE_OPTION`] —
/// this one is for the eye and for geometry, and a pane that lost it is
/// still a findable view.
pub const SLOT_OPTION: &str = "@tally_slot";

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
    /// what a view this creates is labelled with. `where_` is the same
    /// address as geometry, and says which row session and which column
    /// the pane belongs in (`docs/24-the-tally-server.md`). Both are
    /// passed rather than derived because this module knows about tmux and
    /// nothing else.
    ///
    /// # Errors
    /// Whatever stopped it.
    fn jump(&self, session: &str, at: &str, where_: Place) -> Result<(), Error>;

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

/// Where on the grid a view belongs: the **absolute** row (1-64, the
/// stacked row space `Slot::absolute_row` produces) and the column.
///
/// Passed in rather than derived because this module knows about tmux and
/// nothing else — a page *name* is config, and config does not get to
/// rename sixty-four tmux sessions when it changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place {
    /// 1-64, the absolute row. Names the session.
    pub row: u8,
    /// 1-8, 1 at the left. Orders the pane within it.
    pub col: u8,
}

/// The session name for an absolute row: `r07`.
///
/// Zero-padded to two digits because the only place you read these is a
/// `list-sessions` dump, and an unsorted one is a list you search instead
/// of scan.
fn row_session(row: u8) -> String {
    format!("r{row:02}")
}

/// Args creating the row session *with* its first view already in it.
///
/// `new-session -d -s r07` on its own leaves a stray shell window that
/// nothing asked for and nothing cleans up. Creating the session around
/// the first view means one is never made. (The one stray shell that *is*
/// wanted is [`HOME_SESSION`], and it is made deliberately.)
fn new_row_session_args(place: Place, session: &str, anchor_bin: &str) -> Vec<String> {
    let name = row_session(place.row);
    vec![
        "new-session".into(),
        "-d".into(),
        "-s".into(),
        name.clone(),
        "-P".into(),
        "-F".into(),
        "#{pane_id}".into(),
        "-n".into(),
        name,
        format!("{anchor_bin} attach {session}"),
    ]
}

/// Args opening a view as a new **window** in an existing row.
///
/// The window index *is* the column, so a row reads left to right in
/// tmux's own status line with no insertion order to maintain: tab 3 is
/// column 3 whether or not 1 and 2 exist yet. That is the whole reason a
/// column is a window and not a pane — eight agents side by side are
/// eight unreadable 23-column slivers, and the thing you actually do with
/// a row is look at one of them at a time (`docs/24`).
///
/// `-d` because creating a view must not move anybody: the switch is a
/// separate, deliberate step.
fn new_window_args(place: Place, session: &str, anchor_bin: &str) -> Vec<String> {
    vec![
        "new-window".into(),
        "-d".into(),
        "-t".into(),
        format!("{}:{}", row_session(place.row), place.col),
        "-P".into(),
        "-F".into(),
        "#{pane_id}".into(),
        format!("{anchor_bin} attach {session}"),
    ]
}

/// Args moving a freshly-made row session's only window to its column.
///
/// `new-session` puts the first window at the server's `base-index`,
/// which is whatever the user's config says and is not the column. One
/// `move-window` afterwards is what makes the index an address again.
fn move_window_args(pane: &str, place: Place) -> Vec<String> {
    vec![
        "move-window".into(),
        "-s".into(),
        pane.into(),
        "-t".into(),
        format!("{}:{}", row_session(place.row), place.col),
    ]
}

/// Args stopping tmux renaming a view's window after whatever is running
/// in it. The name is an address; addresses are not opinions.
///
/// Targeted by **pane**, not by row: a row is many windows now, and
/// `-t r07:` would set this on whichever one happened to be current.
fn automatic_rename_off_args(pane: &str) -> Vec<String> {
    vec![
        "set-option".into(),
        "-w".into(),
        "-t".into(),
        pane.into(),
        "automatic-rename".into(),
        "off".into(),
    ]
}

/// Args naming a view's window, which is what a tab reads as.
fn rename_window_args(pane: &str, place: Place, at: &str, session: &str) -> Vec<String> {
    vec![
        "rename-window".into(),
        "-t".into(),
        pane.into(),
        pane_title(place, at, session),
    ]
}

/// Args creating the home session, detached, running the login shell.
fn new_home_session_args() -> Vec<String> {
    vec![
        "new-session".into(),
        "-d".into(),
        "-s".into(),
        HOME_SESSION.into(),
        "-n".into(),
        HOME_SESSION.into(),
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

/// Args tagging `pane` with the address it occupies.
///
/// Separate from [`set_option_args`] because the two answer different
/// questions and go stale at different times: the session id is identity
/// and changes when the pad is rebound; the address is geometry and
/// changes never.
fn set_slot_args(pane: &str, at: &str) -> Vec<String> {
    vec![
        "set-option".into(),
        "-p".into(),
        "-t".into(),
        pane.into(),
        SLOT_OPTION.into(),
        at.into(),
    ]
}

/// Args giving `pane` a readable title: the column, the address, and a
/// short tail of the session id.
///
/// The tail stays for the reason the old window name kept it: a slot
/// outlives the sessions in it, so a dead pane and its replacement can
/// both claim one address, and two panes labelled `work:7:3` is a row you
/// cannot read.
fn pane_title_args(pane: &str, place: Place, at: &str, session: &str) -> Vec<String> {
    vec![
        "select-pane".into(),
        "-t".into(),
        pane.into(),
        "-T".into(),
        pane_title(place, at, session),
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

/// What a dead view says about itself.
///
/// tmux draws a line on the last row of a pane whose command has exited;
/// since 3.2 its text is the `remain-on-exit-format` pane option. The
/// default — `Pane is dead (status 0, …)` — is the last thing a view ever
/// tells you, and it does not name the session it was
/// (`docs/19-the-dead-pane.md`).
///
/// **The id is read from the tag, not baked in here.** A literal would go
/// stale the moment the pane is rebound to another session; `#{@…}`
/// expands at draw time, so the line is right by construction.
///
/// The id goes early because the line is truncated at pane width — a
/// narrow split loses the tail, never the payload. `attach exit` is named
/// for what it is: the pane runs `anchor attach`, so the status is the
/// attach client's, not the agent's.
fn dead_pane_format() -> String {
    format!("tally · anchor attach #{{{PANE_OPTION}}} · attach exit #{{pane_dead_status}}")
}

/// Args giving `pane` a dead-line that names its session.
///
/// Pane-scoped for the same reason as [`remain_on_exit_args`]: an adopted
/// view sits among panes that are yours.
///
/// **Set long before any death.** Setting this on an already-dead pane
/// does not repaint it — measured, tmux 3.6b — so it belongs with the
/// other options in [`Tmux::tag`], not on the kill path.
fn dead_pane_format_args(pane: &str) -> Vec<String> {
    vec![
        "set-option".into(),
        "-p".into(),
        "-t".into(),
        pane.into(),
        "remain-on-exit-format".into(),
        dead_pane_format(),
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

/// A title you can read: the column, the pad it belongs to, then a short
/// tail of the session id.
///
/// `c3 · work:7:3 · Z8YN`. The address is the part you actually know — it
/// is where your finger goes, and it is what the `tl.slot` label says —
/// whereas four characters of a ULID identify the session to nobody, and
/// the column is what tells you which of the panes in front of you this
/// is without counting from the left.
fn pane_title(place: Place, at: &str, session: &str) -> String {
    let tail: String = session
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let col = place.col;
    if at.is_empty() {
        return format!("c{col} · {tail}");
    }
    format!("c{col} · {at} · {tail}")
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
    /// Where each client was standing before it started this trip.
    ///
    /// **One deep, per client, first jump wins.** A jump switches every
    /// attached client on purpose, so "where I was" is per client and
    /// never global; and a jump made while a trip is already open does not
    /// overwrite it, so `next → next → next → back` lands where you
    /// started rather than three agents deep.
    ///
    /// ⚠️ **It used to refuse to record a view, and that was wrong.**
    /// `docs/16` assumed the place you jump *from* is nvim — a pane that
    /// is not a view. It is not: every pane this user works in is an
    /// anchor session, so every candidate origin was a view and `◄` never
    /// had anywhere to go. Measured within an hour of shipping, from the
    /// `kind:"nav"` lines written for exactly this purpose: two
    /// `{"via":"back","ok":false}` in a row.
    ///
    /// What that rule was protecting — walking back through agents one at
    /// a time — is held by *first jump wins* instead, which does not care
    /// what kind of pane you started in.
    ///
    /// In memory, so a `tallyd` restart loses it — and the degradation is
    /// that `◄` goes dark until you jump again, which is honest.
    origin: Mutex<BTreeMap<String, String>>,
    /// The socket its server listens on — [`SOCKET`] unless
    /// `TALLY_TMUX_SOCKET` says otherwise.
    ///
    /// Held rather than read per command so that one process cannot drive
    /// two servers because the environment changed under it.
    socket: String,
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
            socket: std::env::var("TALLY_TMUX_SOCKET")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| SOCKET.to_owned()),
        }
    }

    /// Is a tmux server reachable?
    ///
    /// Checked rather than inferred, and **not** fatal: a missing server is
    /// the normal state at login, and the grid is still truthful without
    /// one. It is the jump that has no target, and the jump starts a server
    /// when it has to (`docs/06-jump.md`).
    pub fn server_running(&self) -> bool {
        self.tmux()
            .arg("has-session")
            .output()
            .is_ok_and(|o| o.status.success())
    }

    /// A `tmux` invocation pointed at **tally's** server and no other.
    ///
    /// Every command in this module goes through here. A call that forgot
    /// the socket would land on your personal server, which is the bug
    /// this whole document set out to fix (`docs/24-the-tally-server.md`).
    fn tmux(&self) -> Command {
        let mut cmd = Command::new("tmux");
        let _socket = cmd.arg("-L").arg(&self.socket);
        cmd
    }

    fn run(&self, what: &'static str, args: &[String]) -> Result<Output, Error> {
        let out = self
            .tmux()
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

    fn stdout(&self, what: &'static str, args: &[String]) -> Result<String, Error> {
        Ok(String::from_utf8_lossy(&self.run(what, args)?.stdout).into_owned())
    }

    /// Does a session by this name exist on tally's server?
    fn session_exists(&self, name: &str) -> bool {
        self.tmux()
            .args(["has-session", "-t", name])
            .output()
            .is_ok_and(|o| o.status.success())
    }

    fn panes(&self) -> String {
        self.stdout("list-panes", &list_panes_args())
            .unwrap_or_default()
    }

    fn client_panes(&self) -> Vec<(String, String)> {
        let listing = self
            .stdout(
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
    /// created before any of these rules existed.
    fn tag(&self, pane: &str, session: &str, place: Place, at: &str) -> Result<(), Error> {
        // The tag first: the dead-line format reads `@tally_session`, and
        // a pane that has not got one yet would die saying nothing.
        let _opt = self.run("set-option", &set_option_args(pane, session))?;
        let _roe = self.run("set-option", &remain_on_exit_args(pane))?;
        let _fmt = self.run("set-option", &dead_pane_format_args(pane))?;
        // The address and the title are for the eye. A failure here is a
        // view that reads badly, not a view that cannot be found, so it
        // does not take the jump down with it.
        let _slot = self.run("set-option", &set_slot_args(pane, at));
        let _title = self.run("select-pane", &pane_title_args(pane, place, at, session));
        // Per window, because a row is many windows now. Both are for the
        // eye, so both are best-effort for the same reason the title is.
        let _auto = self.run("set-option", &automatic_rename_off_args(pane));
        let _name = self.run(
            "rename-window",
            &rename_window_args(pane, place, at, session),
        );
        Ok(())
    }

    /// The pane viewing `session`, creating or reviving it if it is gone.
    ///
    /// Three cases (`docs/15-views.md`): alive, use it; dead, respawn
    /// **in place**; absent, make one in the row session this slot
    /// belongs to (`docs/24-the-tally-server.md`).
    fn view_pane(&self, session: &str, place: Place, at: &str) -> Result<String, Error> {
        let listing = self.panes();
        if let Some(found) = view_for(&listing, session) {
            if found.dead {
                let args = respawn_pane_args(found.pane, session, &self.anchor_bin);
                let _revived = self.run("respawn-pane", &args)?;
            }
            self.tag(found.pane, session, place, at)?;
            // After the pane owns the tag, never before: an inherited tag
            // is the only thing identifying a legacy view, and clearing it
            // first would lose the session mid-sequence.
            let _legacy = self.run("set-option", &clear_window_option_args(found.window));
            return Ok(found.pane.to_owned());
        }
        let pane = self.make_view(session, place)?;
        self.tag(&pane, session, place, at)?;
        Ok(pane)
    }

    /// Put a new view window in its row, creating the row if it is not
    /// there.
    ///
    /// The column is the **window index**, so there is nothing to order:
    /// `new-window -t r07:3` lands at 3 whatever else the row holds.
    /// Creating the session *around* the first view is what keeps a stray
    /// shell from appearing; `new-session -d` on a dead server also
    /// starts one, which is why a missing tally server is not a startup
    /// failure.
    fn make_view(&self, session: &str, place: Place) -> Result<String, Error> {
        if !self.session_exists(&row_session(place.row)) {
            let args = new_row_session_args(place, session, &self.anchor_bin);
            let pane = self.stdout("new-session", &args)?.trim().to_owned();
            // `new-session` puts it at `base-index`; the column is where
            // it belongs. Best-effort: a view at the wrong index is read
            // in the wrong order, not lost.
            let _moved = self.run("move-window", &move_window_args(&pane, place));
            return Ok(pane);
        }
        let args = new_window_args(place, session, &self.anchor_bin);
        Ok(self.stdout("new-window", &args)?.trim().to_owned())
    }

    /// Make sure there is something for a terminal to attach to, and that
    /// a terminal is attached to it.
    ///
    /// A jump that lands on a server nobody is looking at succeeds
    /// invisibly, which is the one failure the grid cannot show you. So
    /// zero clients means open the window (`docs/24-the-tally-server.md`).
    fn ensure_terminal(&self) {
        if !self.client_panes().is_empty() {
            return;
        }
        if !self.session_exists(HOME_SESSION) {
            let _home = self.run("new-session", &new_home_session_args());
        }
        open_terminal(&self.socket);
    }

    /// Remember where each client is standing, if a trip is not already
    /// open for it.
    fn record_origins(&self, clients: &[(String, String)], destination: &str) {
        let Ok(mut origin) = self.origin.lock() else {
            return;
        };
        for (client, pane) in clients {
            // A jump to where you already are starts no trip and must not
            // record an origin that would send you nowhere.
            if pane == destination {
                continue;
            }
            // `or_insert`, never `insert`: the first jump of a trip is the
            // one you want to come back from.
            let _kept = origin.entry(client.clone()).or_insert_with(|| pane.clone());
        }
    }
}

/// Tell the pads where the view for `session` goes, without looking at it.
///
/// Used by `tally spawn`: the session has just been created from a shell,
/// and the agent should appear in the tally window straight away rather
/// than on the first press. Your shell stays exactly where it is — a pane
/// on your personal server cannot be a view on tally's
/// (`docs/24-the-tally-server.md#what-this-costs`).
///
/// # Errors
/// Whatever tmux said. Not fatal to the caller: the session exists either
/// way, and the next press makes the view.
pub fn place(session: &str, at: &str, where_: Place) -> Result<(), Error> {
    let tmux = Tmux::default();
    let _pane = tmux.view_pane(session, where_, at)?;
    Ok(())
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

/// The command that opens the window looking at tally's server.
///
/// Split out from the running of it for the reason every other command in
/// this module is: the arguments are the part worth testing.
///
/// `-na` forces a **new** window even though the app is already running,
/// which is the difference between this and [`raise`]: that one focuses
/// what exists, this one exists because nothing does.
///
/// One string, overridable as `TALLY_TERMINAL_CMD`, for the same reason
/// [`terminal_app`] is: it is machine-specific and the rest is not. The
/// override is split on spaces, which is enough for a path and some flags
/// and deliberately not a shell.
fn terminal_command(socket: &str, app: &str, override_cmd: Option<&str>) -> (String, Vec<String>) {
    let attach = [
        "tmux".to_owned(),
        "-L".to_owned(),
        socket.to_owned(),
        "attach".to_owned(),
        "-t".to_owned(),
        HOME_SESSION.to_owned(),
    ];
    let Some(cmd) = override_cmd.map(str::trim).filter(|c| !c.is_empty()) else {
        let mut args = vec![
            "-na".to_owned(),
            app.to_owned(),
            "--args".to_owned(),
            "--title".to_owned(),
            HOME_SESSION.to_owned(),
            "-e".to_owned(),
        ];
        args.extend(attach);
        return ("open".to_owned(), args);
    };
    let mut parts = cmd.split(' ').filter(|p| !p.is_empty());
    let bin = parts.next().unwrap_or("true").to_owned();
    let mut args: Vec<String> = parts.map(ToOwned::to_owned).collect();
    args.extend(attach);
    (bin, args)
}

/// Open the window that looks at tally's server. Best-effort, like
/// [`raise`]: the view exists whether or not anything shows it.
fn open_terminal(socket: &str) {
    let app = terminal_app().unwrap_or_else(|| "Alacritty".to_owned());
    let over = std::env::var("TALLY_TERMINAL_CMD").ok();
    let (bin, args) = terminal_command(socket, &app, over.as_deref());
    let _opened = Command::new(bin).args(args).output();
}

impl Jump for Tmux {
    fn jump(&self, session: &str, at: &str, where_: Place) -> Result<(), Error> {
        let pane = self.view_pane(session, where_, at)?;
        // Before the clients are listed, not after: a jump with nothing
        // attached used to succeed invisibly, and the window it opens is
        // a client by the time the switch runs.
        self.ensure_terminal();
        let clients = self.client_panes();
        self.record_origins(&clients, &pane);

        // Switch **every** client — but every client is now one of
        // tally's, because they are the only ones on this socket. Your
        // personal server cannot be listed from here and so cannot be
        // moved (`docs/24-the-tally-server.md`).
        //
        // Zero clients is not an error: the view exists and is correct,
        // and it is there the next time you attach.
        for (client, _) in &clients {
            self.run("switch-client", &switch_args(client, &pane))?;
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
        let live = self.panes();
        let mut moved = false;
        for (client, pane) in taken {
            if !live
                .lines()
                .any(|l| l.split_whitespace().next() == Some(&*pane))
            {
                continue; // the origin closed while you were away
            }
            self.run("switch-client", &switch_args(&client, &pane))?;
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
        let panes = self.panes();
        self.client_panes()
            .into_iter()
            .filter_map(|(_, pane)| session_of(&panes, &pane))
            .collect()
    }

    fn close(&self, session: &str) -> Result<(), Error> {
        let listing = self.panes();
        let Some(found) = view_for(&listing, session) else {
            return Ok(());
        };
        let _closed = self.run("kill-pane", &kill_pane_args(found.pane))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SESSION: &str = "01M52GEKED8ZEJPMT2KCJ35ZWT";
    /// The slot the pad lives on, as `page:row:col`.
    const AT: &str = "1:4:4";
    /// The same slot as geometry: absolute row 4, column 4.
    const PLACE: Place = Place { row: 4, col: 4 };

    /// `#{pane_id} #{window_id} #{pane_dead} #{@tally_session}`
    const LISTING: &str = "\
%1 @1 0 \n\
%2 @2 0 other-session\n\
%3 @2 0 01M52GEKED8ZEJPMT2KCJ35ZWT\n\
%4 @3 1 01M52MXB4WQTA79K5FTHMXZ8YN\n\
%5 @3 0 \n";

    #[test]
    fn a_view_is_created_in_the_row_session_for_its_row() {
        let args = new_row_session_args(PLACE, SESSION, "anchor");
        assert!(
            args.contains(&"-d".to_owned()),
            "detached: do not steal focus to create it"
        );
        let at = args.iter().position(|a| a == "-s").expect("-s");
        assert_eq!(args.get(at + 1).map(String::as_str), Some("r04"));
        assert!(
            args.last()
                .is_some_and(|c| c == &format!("anchor attach {SESSION}")),
            "the row session's first pane IS a view, not a stray shell"
        );
    }

    /// The row names the session and nothing else does: a page rename
    /// must not orphan the panes in it.
    #[test]
    fn a_row_session_is_named_by_the_absolute_row_zero_padded() {
        assert_eq!(row_session(1), "r01");
        assert_eq!(row_session(7), "r07");
        assert_eq!(row_session(64), "r64");
    }

    /// Creating a view must not move anybody: the switch is a separate,
    /// deliberate step.
    #[test]
    fn opening_a_view_in_a_row_does_not_steal_focus() {
        let args = new_window_args(PLACE, SESSION, "anchor");
        assert!(args.contains(&"-d".to_owned()));
        assert!(
            args.last()
                .is_some_and(|c| c == &format!("anchor attach {SESSION}")),
            "the window runs the attach"
        );
    }

    /// The column *is* the window index, so a row reads left to right
    /// whatever order its tabs arrived in — and with nothing to order,
    /// there is no insertion rule left to get wrong
    /// (`docs/24-the-tally-server.md`).
    #[test]
    fn a_column_is_the_window_index() {
        let args = new_window_args(Place { row: 7, col: 3 }, SESSION, "anchor");
        let t = args.iter().position(|a| a == "-t").expect("-t");
        assert_eq!(args.get(t + 1).map(String::as_str), Some("r07:3"));

        // Column 8 and column 1 address the same row, never each other.
        let eight = new_window_args(Place { row: 7, col: 8 }, SESSION, "anchor");
        let t = eight.iter().position(|a| a == "-t").expect("-t");
        assert_eq!(eight.get(t + 1).map(String::as_str), Some("r07:8"));
    }

    /// `new-session` puts the first window at `base-index`, which is the
    /// user's config and not an address. The move is what makes the index
    /// mean the column again.
    #[test]
    fn a_rows_first_view_is_moved_to_its_column() {
        let args = move_window_args("%3", Place { row: 7, col: 4 });
        let t = args.iter().position(|a| a == "-t").expect("-t");
        assert_eq!(args.get(t + 1).map(String::as_str), Some("r07:4"));
        let src = args.iter().position(|a| a == "-s").expect("-s");
        assert_eq!(args.get(src + 1).map(String::as_str), Some("%3"));
    }

    /// A row is many windows now, so anything window-scoped has to be
    /// aimed at the pane that identifies one. `-t r07:` would land on
    /// whichever tab happened to be current.
    #[test]
    fn window_options_are_aimed_at_a_pane_not_a_row() {
        let auto = automatic_rename_off_args("%7");
        assert!(auto.contains(&"%7".to_owned()));
        assert!(auto.iter().all(|a| !a.ends_with(':')), "not a row target");
        let name = rename_window_args("%7", PLACE, "work:7:3", SESSION);
        assert!(name.contains(&"%7".to_owned()));
    }

    /// A freshly created session has exactly one pane, so the id it
    /// prints is the view. Printing a window id instead would hand the
    /// rest of this module the wrong kind of target.
    #[test]
    fn creating_a_view_prints_a_pane_not_a_window() {
        for args in [
            new_row_session_args(PLACE, SESSION, "anchor"),
            new_window_args(PLACE, SESSION, "anchor"),
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
        assert!(new_window_args(PLACE, SESSION, "anchor").contains(&"-t".to_owned()));
        assert!(new_row_session_args(PLACE, SESSION, "anchor").contains(&"-s".to_owned()));
        for args in [
            set_option_args("%7", SESSION),
            remain_on_exit_args("%7"),
            dead_pane_format_args("%7"),
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

    /// The dead-line must read the tag, not a literal: a pane rebound to
    /// another session would otherwise die naming the previous one.
    #[test]
    fn the_dead_line_names_the_session_through_the_tag() {
        let args = dead_pane_format_args("%7");
        assert!(args.contains(&"-p".to_owned()));
        let fmt = args.last().expect("a format").clone();
        assert!(
            fmt.contains(&format!("#{{{PANE_OPTION}}}")),
            "{fmt} expands the tag at draw time"
        );
        assert!(
            !fmt.contains(SESSION),
            "a baked-in id would be stale on a rebind"
        );
    }

    /// `attach exit` is the attach client's status, not the agent's. The
    /// agent's exit is on the pad; calling this one `exit` would be a lie
    /// on every kill (`docs/19-the-dead-pane.md`).
    #[test]
    fn the_dead_line_does_not_claim_the_agents_exit() {
        let fmt = dead_pane_format();
        assert!(fmt.contains("attach exit #{pane_dead_status}"), "{fmt}");
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

    /// `viewing()` rests on this: a pane maps back to the session it
    /// shows, and an untagged one maps to nothing.
    #[test]
    fn a_pane_maps_back_to_the_session_it_shows() {
        assert_eq!(session_of(LISTING, "%3").as_deref(), Some(SESSION));
        assert_eq!(
            session_of(LISTING, "%4").as_deref(),
            Some("01M52MXB4WQTA79K5FTHMXZ8YN"),
            "a dead view still shows a session"
        );
        assert_eq!(session_of(LISTING, "%1"), None, "untagged");
        assert_eq!(session_of(LISTING, "%404"), None, "not on the server");
    }

    /// The column leads, because it is what tells you which of the
    /// panes in front of you this is without counting from the left; the
    /// address follows, because that is where your finger goes.
    #[test]
    fn a_pane_is_titled_after_the_pad_it_belongs_to() {
        assert_eq!(pane_title(PLACE, AT, SESSION), "c4 · 1:4:4 · 5ZWT");
        assert_eq!(
            pane_title(Place { row: 19, col: 5 }, "work:3:5", SESSION),
            "c5 · work:3:5 · 5ZWT"
        );
    }

    /// A slot outlives the sessions in it. Run something on the same pad
    /// twice while the old view is open and two panes share an address.
    #[test]
    fn two_sessions_on_one_pad_get_different_titles() {
        let first = pane_title(PLACE, AT, "01M52GEKED8ZEJPMT2KCJ35ZWT");
        let second = pane_title(PLACE, AT, "01M52MXB4WQTA79K5FTHMXZ8YN");
        assert_ne!(first, second);
        assert!(first.contains(AT) && second.contains(AT));
    }

    #[test]
    fn a_nameless_pad_still_produces_a_usable_title() {
        assert_eq!(pane_title(PLACE, "", "ab"), "c4 · ab");
        assert_eq!(pane_title(PLACE, "", ""), "c4 · ");
        assert_eq!(pane_title(PLACE, AT, ""), "c4 · 1:4:4 · ");
    }

    /// The home session is the one stray shell that is wanted: it is
    /// what the window shows when nothing is in front of you, and what
    /// `attach` needs to exist at all.
    #[test]
    fn the_home_session_is_a_shell_and_the_rows_are_not() {
        let home = new_home_session_args();
        assert!(home.contains(&"-d".to_owned()));
        let at = home.iter().position(|a| a == "-s").expect("-s");
        assert_eq!(home.get(at + 1).map(String::as_str), Some(HOME_SESSION));
        assert!(
            !home.iter().any(|a| a.starts_with("anchor ")),
            "the home session runs your shell, not an attach"
        );
    }

    #[test]
    fn the_option_name_is_the_one_docs_promise() {
        assert_eq!(PANE_OPTION, "@tally_session");
        assert_eq!(SLOT_OPTION, "@tally_slot");
        assert_eq!(HOME_SESSION, "tally");
        assert_eq!(SOCKET, "tally");
    }

    /// The window has to be told which server to look at: an `attach`
    /// with no socket lands on your personal one, which is the whole
    /// thing this is separating.
    #[test]
    fn the_terminal_is_opened_against_tallys_socket() {
        let (bin, args) = terminal_command("tally", "Alacritty", None);
        assert_eq!(bin, "open");
        assert!(
            args.contains(&"-na".to_owned()),
            "a new window, not a focus"
        );
        let l = args.iter().position(|a| a == "-L").expect("-L");
        assert_eq!(args.get(l + 1).map(String::as_str), Some("tally"));
        assert!(args.ends_with(&["-t".to_owned(), HOME_SESSION.to_owned()]));
    }

    /// The override replaces how the window is opened, never what it
    /// attaches to.
    #[test]
    fn the_terminal_override_still_attaches_to_tallys_server() {
        let (bin, args) = terminal_command("other", "Alacritty", Some("wezterm start --"));
        assert_eq!(bin, "wezterm");
        assert_eq!(
            args,
            [
                "start",
                "--",
                "tmux",
                "-L",
                "other",
                "attach",
                "-t",
                HOME_SESSION
            ]
        );
        // An override that is only whitespace is not an override.
        assert_eq!(terminal_command("tally", "Alacritty", Some("  ")).0, "open");
    }

    // ── origin: the half `◄` is built on ───────────────────────────────

    #[test]
    fn where_you_were_standing_is_remembered_per_client() {
        let t = Tmux::default();
        assert!(!t.can_go_back(), "nothing has jumped yet");
        t.record_origins(
            &[
                ("/dev/ttys000".to_owned(), "%1".to_owned()),
                ("/dev/ttys003".to_owned(), "%5".to_owned()),
            ],
            "%3",
        );
        let o = t.origin.lock().expect("lock");
        assert_eq!(o.get("/dev/ttys000").map(String::as_str), Some("%1"));
        assert_eq!(o.get("/dev/ttys003").map(String::as_str), Some("%5"));
    }

    /// The bug the journal caught an hour after shipping. `docs/16`
    /// assumed you jump *from* nvim; every pane this grid is used from is
    /// an anchor session, so every candidate origin was a view and `◄`
    /// never had anywhere to go.
    #[test]
    fn standing_in_a_view_still_counts_as_somewhere_to_come_back_to() {
        let t = Tmux::default();
        // %3 is a view, and it is where the client is standing.
        t.record_origins(&[("c".to_owned(), "%3".to_owned())], "%4");
        assert!(t.can_go_back(), "a view is a place you were");
        let o = t.origin.lock().expect("lock");
        assert_eq!(o.get("c").map(String::as_str), Some("%3"));
    }

    /// `next → next → next → back` must land where you started, not three
    /// agents deep. Held by *first jump wins* rather than by refusing to
    /// record views, which is what made `◄` useless.
    #[test]
    fn a_chained_jump_does_not_overwrite_where_you_started() {
        let t = Tmux::default();
        t.record_origins(&[("c".to_owned(), "%1".to_owned())], "%3");
        t.record_origins(&[("c".to_owned(), "%3".to_owned())], "%4");
        t.record_origins(&[("c".to_owned(), "%4".to_owned())], "%5");
        let o = t.origin.lock().expect("lock");
        assert_eq!(
            o.get("c").map(String::as_str),
            Some("%1"),
            "home, not the last agent"
        );
    }

    /// Pressing a pad you are already looking at starts no trip, so it
    /// must not record an origin that would send you nowhere.
    #[test]
    fn jumping_to_where_you_already_are_records_nothing() {
        let t = Tmux::default();
        t.record_origins(&[("c".to_owned(), "%3".to_owned())], "%3");
        assert!(!t.can_go_back());
    }

    /// `◄` is dark exactly when there is nowhere to go — one predicate,
    /// read by the renderer and by the press.
    #[test]
    fn back_is_offered_only_when_there_is_somewhere_to_go() {
        let t = Tmux::default();
        assert!(!t.can_go_back());
        t.record_origins(&[("c".to_owned(), "%1".to_owned())], "%3");
        assert!(t.can_go_back());
    }
}
