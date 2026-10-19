//! The press that puts the real terminal in front of you.
//!
//! An anchor session is a PTY in a daemon, not a window. Something has to
//! be attached for there to be anywhere to jump *to*, and that something
//! has to be findable and focusable (`docs/06-views.md`).
//!
//! **That something is a pane** (`docs/06-views.md`). It used to be a
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

/// The tmux socket tally runs its own server on (`docs/06-views.md`).
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
/// by every pane in the window — measured, `docs/06-views.md` — so a split
/// of a view window produces two panes both claiming the same session. A
/// pane option is not inherited by siblings.
pub const PANE_OPTION: &str = "@tally_session";

/// The **pane** option carrying the Claude Code session id of the
/// conversation the viewed session is running.
///
/// Separate from [`PANE_OPTION`] because it answers a different question
/// and is often not answerable at all: a view of a shell has no
/// conversation, and a view of an agent has none until its first turn
/// (`docs/06-views.md`).
///
/// Pane-scoped for the same reason [`PANE_OPTION`] is: a window option is
/// inherited by every pane in the window.
pub const CLAUDE_OPTION: &str = "@tally_claude";

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
    /// `claude` is the conversation that session is running, when tally
    /// knows one. `None` means *not known*, never *none any more*: the
    /// tag is left alone rather than cleared, so a view created by
    /// `tally spawn` — before the agent has had a turn — keeps whatever
    /// it had rather than having its dead line blanked.
    ///
    /// `at` is the slot address the pad carries — `page:row:col` — and is
    /// what a view this creates is labelled with. `where_` is the same
    /// address as geometry, and says which row session and which column
    /// the pane belongs in (`docs/06-views.md`). Both are
    /// passed rather than derived because this module knows about tmux and
    /// nothing else.
    ///
    /// # Errors
    /// Whatever stopped it.
    fn jump(
        &self,
        session: &str,
        claude: Option<&str>,
        at: &str,
        where_: Place,
    ) -> Result<(), Error>;

    /// Which of these views are not in the window their address names.
    ///
    /// `want` is every session that holds a pad, with the [`Place`] its
    /// view belongs in and the address string to report it by — both
    /// passed rather than derived, because turning an address into a
    /// `Place` needs the page table and this module does not have one
    /// (`docs/06-views.md`).
    ///
    /// **Reports; moves nothing.** A session with no view is not drift,
    /// and is not reported.
    fn audit(&self, want: &[(String, String, Place)]) -> Vec<Drift>;

    /// Move the view for `session`, if it has one, to the window `where_`
    /// names, and re-label it `at`. **Nothing is switched.**
    ///
    /// `Ok(())` when there is no view: a session that was never pressed
    /// has no pane to move, and that is not an error
    /// (`docs/06-views.md`).
    ///
    /// # Errors
    /// Whatever stopped it.
    fn relocate(&self, session: &str, at: &str, where_: Place) -> Result<(), Error>;

    /// Put each client back where it was standing before it jumped.
    ///
    /// `Ok(false)` when there was nowhere to go. That is not an error: a
    /// client that has not jumped, or whose origin pane has since been
    /// closed, has no honest destination, and landing somewhere arbitrary
    /// is worse than not moving (`docs/05-interaction.md`).
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
    /// filling your terminal (`docs/05-interaction.md`). Returns empty when
    /// tmux cannot be asked, which degrades to offering you everything —
    /// the safe direction, since the worst case is a press that lands you
    /// where you already are.
    fn viewing(&self) -> Vec<String>;
}

// ── command construction, separated so the arguments are testable ──────────

/// The field separator in the pane listing.
///
/// **Not a space.** An unset option prints as an empty string, and
/// whitespace-splitting swallows it — which was survivable while only the
/// *last* field could be empty, and is not now that the address can be
/// too (`docs/06-views.md`).
///
/// Position holds because nothing tally writes into a line can contain
/// this: a ULID is alphanumeric, and a `page:row:col` address is fenced
/// at the source — `src/config.rs::resolve_pages` refuses a page name
/// containing `|` for exactly this reason. That fence was missing until
/// review w7 F1, and a page named `a|b` shifted every field after the
/// slot tag silently, because the session id is read *before* the
/// damage. The one field tally does not own is `session_name`, which is
/// why [`row`] refuses a line with an eighth field rather than reading a
/// shifted one.
const FIELD: char = '|';

/// Args listing every pane, its window, whether it is dead, the two
/// things tally writes on it — the session it views and the address it
/// claims — and where it **is**: tmux session and window index
/// (`docs/06-views.md`). The claim and the place are different
/// questions, and a moved view is exactly where they disagree.
///
/// The last two are never empty, so [`FIELD`]'s position rule is as it
/// was.
fn list_panes_args() -> Vec<String> {
    vec![
        "list-panes".into(),
        "-a".into(),
        "-F".into(),
        format!(
            "#{{pane_id}}{FIELD}#{{window_id}}{FIELD}#{{pane_dead}}{FIELD}#{{{PANE_OPTION}}}{FIELD}#{{{SLOT_OPTION}}}{FIELD}#{{session_name}}{FIELD}#{{window_index}}"
        ),
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

/// The first window index a squatter is parked at
/// (`docs/06-views.md`).
///
/// **Above the addressable column range**, which is 1-8, so no pad can
/// ever name it. A view parked here cannot wedge anything, is still
/// reachable by id, and is brought home by the lazy path the next time
/// its own pad is pressed.
const PARK: u8 = 90;

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
fn new_window_args(place: Place, session: &str, anchor_bin: &str, replace: bool) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "new-window".into(),
        "-d".into(),
        "-t".into(),
        format!("{}:{}", row_session(place.row), place.col),
        "-P".into(),
        "-F".into(),
        "#{pane_id}".into(),
        format!("{anchor_bin} attach {session}"),
    ];
    if replace {
        // A corpse holds the index. `-k` takes it over in one step, the
        // same reason `relocate_window_args` passes it
        // (`docs/06-views.md`).
        args.insert(1, "-k".into());
    }
    args
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

/// [`move_window_args`], replacing whatever holds the index when asked.
fn relocate_window_args(pane: &str, place: Place, replace: bool) -> Vec<String> {
    let mut args = move_window_args(pane, place);
    if replace {
        args.insert(1, "-k".into());
    }
    args
}

/// Args trading two live views' windows (`docs/06-views.md`).
///
/// Aimed at the **pane**, like every other address this module writes:
/// tmux resolves a pane id to the window holding it, which is what keeps
/// an index out of the source half of the command.
///
/// `-d` because a relocation must not move anybody: the active window is
/// unchanged, and the switch is a separate, deliberate step.
///
/// Measured, tmux 3.6b: a pane id as `-s` works, the two windows may be
/// in different sessions, and a destination index holding **no** window
/// is refused (`can't find window: N`) — which is the one branch this is
/// never the plan for.
fn swap_window_args(pane: &str, place: Place) -> Vec<String> {
    vec![
        "swap-window".into(),
        "-d".into(),
        "-s".into(),
        pane.into(),
        "-t".into(),
        format!("{}:{}", row_session(place.row), place.col),
    ]
}

/// Args making tmux leave window indexes alone when a window closes.
///
/// A window index is an address (`docs/06-views.md`), and tmux's
/// `renumber-windows` rewrites addresses: closing `r01:2` slides `r01:3`
/// down into its place. A named socket is a separate *server* and not a
/// separate *config*, so every server tally starts reads the user's
/// `~/.tmux.conf` and inherits the option if they set it (`docs/33`).
///
/// `-g` and not `-t`: the global session options are what every session
/// without its own falls back to, so it holds for rows that already exist
/// and for ones made later, and for rows tally did not create.
fn hold_indexes_args() -> Vec<String> {
    vec![
        "set-option".into(),
        "-g".into(),
        "renumber-windows".into(),
        "off".into(),
    ]
}

/// Args keeping a client in `session` when it stops existing.
///
/// Moving a row's last window out destroys the row session, and with
/// tmux's default a client in it is **detached** — so `jump` would list
/// no clients and open a second terminal
/// (`docs/06-views.md`). Per session, not `-g`: only the one
/// about to be left.
fn keep_clients_args(session: &str) -> Vec<String> {
    vec![
        "set-option".into(),
        "-t".into(),
        session.into(),
        "detach-on-destroy".into(),
        "off".into(),
    ]
}

/// Args making the row session a moved view is going to, printing its
/// first window's id.
///
/// `move-window` needs the target session to exist and tmux has no verb
/// that adopts a pane into one it creates, so this is the stray shell
/// [`new_row_session_args`] exists to avoid — for the length of one move.
fn placeholder_args(place: Place) -> Vec<String> {
    vec![
        "new-session".into(),
        "-d".into(),
        "-s".into(),
        row_session(place.row),
        "-P".into(),
        "-F".into(),
        "#{window_id}".into(),
    ]
}

/// Args closing a window by id.
fn kill_window_args(window: &str) -> Vec<String> {
    vec!["kill-window".into(), "-t".into(), window.into()]
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
/// Measured on tmux 3.6b, not theorised (`docs/06-views.md`).
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

/// Args tagging `pane` with the conversation it is running.
///
/// Only ever called with an id in hand: there is no "clear it" path,
/// because *not knowing* is the ordinary state — `tally spawn` makes a
/// view before the agent has had a turn — and a jump that blanked the
/// tag would take the resume command off the dead line of a session that
/// still has one (`docs/06-views.md`).
fn set_claude_args(pane: &str, claude: &str) -> Vec<String> {
    vec![
        "set-option".into(),
        "-p".into(),
        "-t".into(),
        pane.into(),
        CLAUDE_OPTION.into(),
        claude.into(),
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
/// (`docs/06-views.md`).
///
/// **The id is read from the tag, not baked in here.** A literal would go
/// stale the moment the pane is rebound to another session; `#{@…}`
/// expands at draw time, so the line is right by construction.
///
/// The id goes early because the line is truncated at pane width — a
/// narrow split loses the tail, never the payload. `attach exit` is named
/// for what it is: the pane runs `anchor attach`, so the status is the
/// attach client's, not the agent's.
///
/// **The conversation goes first, ahead of the anchor id**
/// (`docs/06-views.md`). The rule is *payload early, because
/// the tail is what truncation eats*. Measured at 80 columns with the
/// conversation second,
/// the cut landed mid-UUID — `claude --resume c1567a67-d7e0` — and half
/// an id resumes nothing, while half an anchor id costs nothing: it names
/// a PTY that is gone by the time you are reading this line.
fn dead_pane_format() -> String {
    // The conversation is conditional on itself: an **unset** user option
    // takes the false branch, so a view of a plain shell keeps exactly
    // the line it had before any of this existed. Measured, tmux 3.6b.
    let conversation =
        format!("#{{?#{{{CLAUDE_OPTION}}}, · claude --resume #{{{CLAUDE_OPTION}}},}}");
    format!(
        "tally{conversation} · anchor attach #{{{PANE_OPTION}}} · attach exit #{{pane_dead_status}}"
    )
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

/// The pane a view is in, as one `list-panes` line describes it.
///
/// **Was `View`**, which collided with [`crate::model::View`] — the 8-row
/// window onto the address space — in a crate where both senses of the
/// word are load-bearing (`docs/08-daemon.md`). A view is the
/// concept ([15](../docs/06-views.md)); this is the pane holding one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pane<'a> {
    /// tmux's id for it, `%N`.
    id: &'a str,
    /// The window that pane is in, for clearing a legacy tag.
    window: &'a str,
    /// Is its command gone? **Not** the same as the agent being gone:
    /// detaching from `anchor attach` also ends the attach, and the pad is
    /// what answers which (`docs/06-views.md`).
    dead: bool,
    /// The tmux session it is in now: `r03`.
    tmux_session: &'a str,
    /// The window index it is at now. The column, when nothing has moved.
    index: &'a str,
}

/// One line of the pane listing: a pane, and the two tags on it.
#[derive(Debug, Clone, Copy)]
struct Row<'a> {
    pane: Pane<'a>,
    /// `@tally_session`, empty when the pane is not tally's.
    session: &'a str,
    /// `@tally_slot`, empty likewise. The address, not the identity.
    slot: &'a str,
}

/// Parse one listing line. Positional, which is what [`FIELD`] buys.
fn row(line: &str) -> Option<Row<'_>> {
    let mut f = line.split(FIELD);
    let pane = f.next()?;
    let window = f.next()?;
    let dead = f.next()?;
    let session = f.next()?;
    let slot = f.next()?;
    let tmux_session = f.next()?;
    let index = f.next()?;
    // An eighth field means a [`FIELD`] got into a value, so the seven
    // above are not the seven they are named as. Tally's own fields are
    // fenced (see [`FIELD`]); a tmux session somebody named `a|b` is not,
    // and a line that cannot be trusted positionally is dropped rather
    // than misread.
    if f.next().is_some() {
        return None;
    }
    Some(Row {
        pane: Pane {
            id: pane,
            window,
            dead: dead == "1",
            tmux_session,
            index,
        },
        session,
        slot,
    })
}

/// Find the view for `session` in `list-panes` output.
///
/// An **unset** option prints as an empty field, so an untagged pane must
/// never match — otherwise an empty session id would claim the first
/// untagged pane on the server, which is every pane you work in.
fn view_for<'a>(listing: &'a str, session: &str) -> Option<Pane<'a>> {
    if session.is_empty() {
        return None;
    }
    listing
        .lines()
        .filter_map(row)
        .find(|r| r.session == session)
        .map(|r| r.pane)
}

/// Find a **dead** pane holding the window that address `at` names.
///
/// The second lookup, and the one that keeps a corpse from locking a pad:
/// a column is a window *index*, so while the previous session's dead
/// view sits there, no new window can be made at that address
/// (`docs/06-views.md`).
///
/// Dead only, by decision: a live `anchor attach` at this address is a
/// view somebody is reading, and this is not the gesture that closes it.
/// Empty address never matches, for `view_for`'s reason.
fn dead_view_at<'a>(listing: &'a str, at: &str) -> Option<Pane<'a>> {
    if at.is_empty() {
        return None;
    }
    listing
        .lines()
        .filter_map(row)
        .find(|r| r.slot == at && r.pane.dead)
        .map(|r| r.pane)
}

/// What it takes to put a view's pane in the window `place` names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Relocation {
    /// Take the window over: it is empty, holds a corpse, or does not
    /// exist yet.
    Move {
        /// Pass `-k`: whatever is at the destination is a corpse, or is
        /// the placeholder [`placeholder_args`] made.
        replace: bool,
        /// The row session is not there to move into.
        create_row: bool,
    },
    /// The destination holds a **live** view, so the two trade places
    /// (`docs/06-views.md`).
    ///
    /// Nothing is created and nothing is destroyed, which is what keeps
    /// [27](../docs/06-views.md)'s rule — a live
    /// `anchor attach` is something somebody may be reading — intact
    /// while still freeing the address.
    Swap,
}

/// The pane sitting in the window `place` names, if any.
///
/// Dead or alive — unlike [`dead_view_at`], which asks *"is there a
/// corpse claiming this address"* and so cannot see a corpse claiming a
/// different one (`docs/06-views.md`).
fn occupant_at(listing: &str, place: Place) -> Option<Row<'_>> {
    let want_session = row_session(place.row);
    let want_index = place.col.to_string();
    listing
        .lines()
        .filter_map(row)
        .find(|r| r.pane.tmux_session == want_session && r.pane.index == want_index)
}

/// The first window index at or above [`PARK`] that `place`'s row is not
/// using.
///
/// A pure function of the listing, so where a squatter lands is testable
/// without a server. `u16` because parking is the one place an index is
/// deliberately not a column and has no reason to stop at 8.
fn park_index(listing: &str, place: Place) -> u16 {
    let want_session = row_session(place.row);
    let taken: Vec<u16> = listing
        .lines()
        .filter_map(row)
        .filter(|r| r.pane.tmux_session == want_session)
        .filter_map(|r| r.pane.index.parse().ok())
        .collect();
    let mut index = u16::from(PARK);
    while taken.contains(&index) {
        index = index.saturating_add(1);
    }
    index
}

/// Args parking a squatter's window out of the address space.
fn park_args(pane: &str, place: Place, index: u16) -> Vec<String> {
    vec![
        "move-window".into(),
        "-s".into(),
        pane.into(),
        "-t".into(),
        format!("{}:{index}", row_session(place.row)),
    ]
}

/// A view that is not in the window its address names
/// (`docs/06-views.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drift {
    /// The anchor session whose view it is.
    pub session: String,
    /// The address it claims, as the caller gave it.
    pub claims: String,
    /// Where it actually sits, as tmux names it: `r01:2`.
    pub sits_at: String,
}

/// Whether a pane is known to be in the window the address names.
///
/// **An address is a claim, and a claim that is not true is the bug**
/// `docs/31` spent itself on: `dead_view_at` finds a view by its
/// `@tally_slot`, so a pane wearing an address its window does not hold
/// is a pane the next spawn at that address will not find and cannot
/// replace (`docs/06-views.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Holds {
    /// It is where the address says. Write the tag.
    TheAddress,
    /// A relocation was refused or failed. Leave whatever it last held.
    SomewhereElse,
}

/// How to get `pane` to `place`, or `None` when it should stay.
///
/// Stays only when it is **already there**.
///
/// ⚠️ A live pane at the destination used to mean *stay* too, and that
/// was the hole `docs/31` shipped knowingly: the caller then tagged the
/// pane where it stood, so the address and the window disagreed and the
/// pad nobody could press stayed unpressable
/// ([#19](https://github.com/alehatsman/tally/issues/19)). It is a
/// [`Relocation::Swap`] now (`docs/06-views.md`). Nothing is closed,
/// which is the part of `27`'s decision that was always the good one.
///
/// `-k` is only ever asked for after the occupant has been seen dead.
///
/// A pure function of the listing, so the whole table is testable
/// without a server.
fn relocation(listing: &str, pane: Pane<'_>, place: Place) -> Option<Relocation> {
    let want_session = row_session(place.row);
    let want_index = place.col.to_string();
    if pane.tmux_session == want_session && pane.index == want_index {
        return None;
    }
    let mut row_exists = false;
    let mut occupant = None;
    for r in listing.lines().filter_map(row) {
        if r.pane.tmux_session != want_session {
            continue;
        }
        row_exists = true;
        if r.pane.index == want_index {
            occupant = Some(r.pane);
        }
    }
    match occupant {
        Some(held) if !held.dead => Some(Relocation::Swap),
        Some(_) => Some(Relocation::Move {
            replace: true,
            create_row: false,
        }),
        None => Some(Relocation::Move {
            replace: !row_exists,
            create_row: !row_exists,
        }),
    }
}

/// Every attached client and the pane it is looking at.
///
/// ⚠️ **`display-message -p -c <client>` is the wrong call**, and it looks
/// more correct than this one. `-c` chooses where a message would be
/// *displayed*, not what the target resolves against — it was measured
/// returning a pane in an unrelated session (`docs/06-views.md`).
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
    /// when it has to (`docs/06-views.md`).
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
    /// this whole document set out to fix (`docs/06-views.md`).
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

    /// Pin the one server option that would rewrite addresses under us.
    ///
    /// Idempotent and **repeated** rather than done once, because the
    /// server outlives and is outlived by everything that could do it: a
    /// restart is a fresh read of the user's config, and the daemon can
    /// start before the server exists. Best-effort for the same reason:
    /// with no server there is nothing to pin and nothing to renumber, and
    /// every call that *starts* one pins it straight after.
    ///
    /// `move-window` never renumbers (measured, tmux 3.6b, same row and
    /// across rows); a close does. So this is called before anything of
    /// ours that closes a window, and after anything that may have made
    /// the server — a tab the user closes between presses is covered by
    /// the latter, which is the case that started this (`docs/33`).
    pub fn hold_indexes(&self) {
        let _held = self.run("set-option", &hold_indexes_args());
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
    fn tag(
        &self,
        pane: &str,
        session: &str,
        claude: Option<&str>,
        place: Place,
        at: &str,
        holds: Holds,
    ) -> Result<(), Error> {
        // The tag first: the dead-line format reads `@tally_session`, and
        // a pane that has not got one yet would die saying nothing.
        let _opt = self.run("set-option", &set_option_args(pane, session))?;
        // The conversation, on the same argument and before the format:
        // both ids are read by the line, and both must be there before it
        // can be. Skipped when unknown rather than cleared — see
        // `set_claude_args`.
        if let Some(claude) = claude {
            let _conversation = self.run("set-option", &set_claude_args(pane, claude))?;
        }
        let _roe = self.run("set-option", &remain_on_exit_args(pane))?;
        let _fmt = self.run("set-option", &dead_pane_format_args(pane))?;
        // The address, only when the pane is actually there. A pane that
        // could not be moved keeps whatever it last legitimately held, so
        // the drift stays visible to `list-panes` and to anything that
        // later looks for it, rather than being papered over
        // (`docs/06-views.md`). It is still reachable either way:
        // `view_for` matches on the session id, which is always written.
        match holds {
            Holds::TheAddress => {
                // A failure here is a view that reads badly, not a view
                // that cannot be found, so it does not take the jump down
                // with it.
                let _slot = self.run("set-option", &set_slot_args(pane, at));
            }
            Holds::SomewhereElse => {
                eprintln!("tally: {pane} is not at {at}; slot tag left alone");
            }
        }
        // The title is for the eye and says where it belongs, which is
        // worth saying even when it is not there yet.
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

    /// Put `pane` in the window `place` names, if it is not already there.
    ///
    /// Best-effort for the caller to decide: this reports, it does not
    /// retry (`docs/06-views.md`).
    fn move_view(&self, listing: &str, pane: Pane<'_>, place: Place) -> Result<(), Error> {
        let Some(plan) = relocation(listing, pane, place) else {
            return Ok(());
        };
        let Relocation::Move {
            replace,
            create_row,
        } = plan
        else {
            // A swap destroys no session, so there is nothing to tell not
            // to detach, and nothing to create or kill afterwards. One
            // call (`docs/06-views.md`).
            return self
                .run("swap-window", &swap_window_args(pane.id, place))
                .map(|_| ());
        };
        // Before the move, while the session still exists to be told.
        let _kept = self.run("set-option", &keep_clients_args(pane.tmux_session));
        let placeholder = if create_row {
            let window = self
                .stdout("new-session", &placeholder_args(place))?
                .trim()
                .to_owned();
            self.hold_indexes();
            Some(window)
        } else {
            None
        };
        let moved = self.run(
            "move-window",
            &relocate_window_args(pane.id, place, replace),
        );
        // `-k` may already have replaced it, so there may be nothing to kill.
        if let Some(window) = placeholder {
            let _gone = self.run("kill-window", &kill_window_args(&window));
        }
        moved.map(|_| ())
    }

    /// The pane viewing `session`, creating or reviving it if it is gone.
    ///
    /// Four cases: by **id** — alive, use it; dead, respawn **in place**
    /// (`docs/06-views.md`) — and in the window its address names, which
    /// a Move may have changed (`docs/06-views.md`). Then by **address** — a dead view of some
    /// other session holding this slot's window is reclaimed
    /// (`docs/06-views.md`). Otherwise make one in the row
    /// session this slot belongs to (`docs/06-views.md`).
    fn view_pane(
        &self,
        session: &str,
        claude: Option<&str>,
        place: Place,
        at: &str,
    ) -> Result<String, Error> {
        self.hold_indexes();
        let listing = self.panes();
        if let Some(found) = view_for(&listing, session) {
            if found.dead {
                let args = respawn_pane_args(found.id, session, &self.anchor_bin);
                let _revived = self.run("respawn-pane", &args)?;
            }
            // Where the pad *is*, not where the pane was left. For the eye
            // and for the pad it vacated, so best-effort like the tag below
            // it: the view is reachable by id either way.
            //
            // The result is **read**, not discarded: a relocation that
            // failed must not be followed by a tag claiming it worked
            // (`docs/06-views.md`).
            let holds = match self.move_view(&listing, found, place) {
                Ok(()) => Holds::TheAddress,
                Err(_) => Holds::SomewhereElse,
            };
            self.tag(found.id, session, claude, place, at, holds)?;
            // After the pane owns the tag, never before: an inherited tag
            // is the only thing identifying a legacy view, and clearing it
            // first would lose the session mid-sequence.
            let _legacy = self.run("set-option", &clear_window_option_args(found.window));
            return Ok(found.id.to_owned());
        }
        // Not found by id, so this is a new session for the slot — and the
        // slot's window may still be held by the previous one's corpse.
        // The column is a window index, so there is nothing to create
        // while that is true; the corpse is also exactly the pane we
        // want, keeping its id, its tab and its place in the row across a
        // respawn (`docs/06-views.md`).
        if let Some(stale) = dead_view_at(&listing, at) {
            let args = respawn_pane_args(stale.id, session, &self.anchor_bin);
            let _reclaimed = self.run("respawn-pane", &args)?;
            // Found *by* the address, so it is at it by construction.
            self.tag(stale.id, session, claude, place, at, Holds::TheAddress)?;
            let _legacy = self.run("set-option", &clear_window_option_args(stale.window));
            return Ok(stale.id.to_owned());
        }
        let pane = self.make_view(session, place)?;
        // Made at the address, or `make_view` would have failed.
        self.tag(&pane, session, claude, place, at, Holds::TheAddress)?;
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
            self.hold_indexes();
            // `new-session` puts it at `base-index`; the column is where
            // it belongs. Best-effort: a view at the wrong index is read
            // in the wrong order, not lost.
            let _moved = self.run("move-window", &move_window_args(&pane, place));
            return Ok(pane);
        }
        // Nothing may hold this index that is not entitled to it
        // (`docs/06-views.md`). By here the caller has already failed
        // to find a view by id and a corpse claiming *this* address, so
        // anything still sitting at the column is in the way.
        let replace = self.clear_the_index(place);
        let args = new_window_args(place, session, &self.anchor_bin, replace);
        Ok(self.stdout("new-window", &args)?.trim().to_owned())
    }

    /// Make the window `place` names available, and say whether the
    /// creation still has to replace what is there.
    ///
    /// Returns `true` when the index holds a **corpse** — `new-window -k`
    /// takes it over in one step, which is `27`'s reclaim generalised to a
    /// corpse claiming some *other* address, the case `dead_view_at`
    /// cannot see.
    ///
    /// A **live** view is parked instead: it is somebody's to close, not
    /// ours (`27`), and it does not have to go home to stop being in the
    /// way (`docs/06-views.md`). An untagged pane is left alone — it
    /// is not a view and this is not the gesture that moves it; the
    /// creation then fails as it always did.
    ///
    /// Best-effort throughout: every outcome here is either the window
    /// being free or `new-window` reporting `index N in use`, which is
    /// what it did before any of this.
    fn clear_the_index(&self, place: Place) -> bool {
        let listing = self.panes();
        let Some(held) = occupant_at(&listing, place) else {
            return false;
        };
        if held.pane.dead {
            return true;
        }
        if held.session.is_empty() {
            // Not a view. Nothing here knows what it is for.
            return false;
        }
        let index = park_index(&listing, place);
        let parked = self.run("move-window", &park_args(held.pane.id, place, index));
        if parked.is_err() {
            eprintln!(
                "tally: {} squats on {}:{} and would not move",
                held.pane.id,
                row_session(place.row),
                place.col
            );
        }
        false
    }

    /// Make sure there is something for a terminal to attach to, and that
    /// a terminal is attached to it.
    ///
    /// A jump that lands on a server nobody is looking at succeeds
    /// invisibly, which is the one failure the grid cannot show you. So
    /// zero clients means open the window (`docs/06-views.md`).
    fn ensure_terminal(&self) {
        if !self.client_panes().is_empty() {
            return;
        }
        if !self.session_exists(HOME_SESSION) {
            let _home = self.run("new-session", &new_home_session_args());
            self.hold_indexes();
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
/// (`docs/06-views.md#when-the-destination-is-live`).
///
/// # Errors
/// Whatever tmux said. Not fatal to the caller: the session exists either
/// way, and the next press makes the view.
pub fn place(session: &str, at: &str, where_: Place) -> Result<(), Error> {
    let tmux = Tmux::default();
    // No conversation: the agent has not had a turn, so no hook has
    // reported one. The first press tags it
    // (`docs/06-views.md`).
    let _pane = tmux.view_pane(session, None, where_, at)?;
    Ok(())
}

/// The session a pane views, if it is a view at all.
///
/// An empty tag is not a session: a pane of yours would otherwise be
/// reported as a view of nothing rather than as not a view.
fn session_of(listing: &str, pane: &str) -> Option<String> {
    listing
        .lines()
        .filter_map(row)
        .find(|r| r.pane.id == pane)
        .map(|r| r.session)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
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

/// The command that brings the terminal forward.
///
/// Split out from the running of it for the reason every other command in
/// this module is: the arguments are the part worth testing.
///
/// Overridable as `TALLY_RAISE_CMD`, the twin of `TALLY_TERMINAL_CMD`,
/// and the suite sets it. It has to: leaving `TALLY_TERMINAL` unset does
/// **not** leave [`raise`] with nothing to raise — [`terminal_app`] falls
/// back to Alacritty — so without an override every headless jump in the
/// tests takes the screen of whoever is running them. The override is
/// split on spaces, which is enough for a path and some flags and
/// deliberately not a shell.
fn raise_command(app: &str, override_cmd: Option<&str>) -> (String, Vec<String>) {
    let Some(cmd) = override_cmd.map(str::trim).filter(|c| !c.is_empty()) else {
        return ("open".to_owned(), vec!["-a".to_owned(), app.to_owned()]);
    };
    let mut parts = cmd.split(' ').filter(|p| !p.is_empty());
    let bin = parts.next().unwrap_or("true").to_owned();
    (bin, parts.map(ToOwned::to_owned).collect())
}

/// Bring the terminal emulator forward. Best-effort and deliberately not
/// an error: tmux has already been switched, so the move has happened even
/// if the window manager will not cooperate.
fn raise() {
    let Some(app) = terminal_app() else {
        return;
    };
    let over = std::env::var("TALLY_RAISE_CMD").ok();
    let (bin, args) = raise_command(&app, over.as_deref());
    let _raised = Command::new(bin).args(args).output();
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
    fn jump(
        &self,
        session: &str,
        claude: Option<&str>,
        at: &str,
        where_: Place,
    ) -> Result<(), Error> {
        let pane = self.view_pane(session, claude, where_, at)?;
        // Before the clients are listed, not after: a jump with nothing
        // attached used to succeed invisibly, and the window it opens is
        // a client by the time the switch runs.
        self.ensure_terminal();
        let clients = self.client_panes();
        self.record_origins(&clients, &pane);

        // Switch **every** client — but every client is now one of
        // tally's, because they are the only ones on this socket. Your
        // personal server cannot be listed from here and so cannot be
        // moved (`docs/06-views.md`).
        //
        // Zero clients is not an error: the view exists and is correct,
        // and it is there the next time you attach.
        for (client, _) in &clients {
            self.run("switch-client", &switch_args(client, &pane))?;
        }
        raise();
        Ok(())
    }

    fn audit(&self, want: &[(String, String, Place)]) -> Vec<Drift> {
        let listing = self.panes();
        want.iter()
            .filter_map(|(session, at, place)| {
                let pane = view_for(&listing, session)?;
                // The same pure function the move path plans with, so the
                // sweep and the repair can never disagree about what out
                // of place means.
                relocation(&listing, pane, *place)?;
                Some(Drift {
                    session: session.clone(),
                    claims: at.clone(),
                    sits_at: format!("{}:{}", pane.tmux_session, pane.index),
                })
            })
            .collect()
    }

    fn relocate(&self, session: &str, at: &str, where_: Place) -> Result<(), Error> {
        let listing = self.panes();
        let Some(found) = view_for(&listing, session) else {
            return Ok(());
        };
        // `?`, so a refusal or a failure never reaches the tag at all:
        // the only way past this line is a pane that is where `where_`
        // says (`docs/06-views.md`).
        self.move_view(&listing, found, where_)?;
        // `None`: no hook has spoken since the move, so what the tag holds
        // is left alone.
        self.tag(found.id, session, None, where_, at, Holds::TheAddress)
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
            // Through `row`, like every other read of this listing: the
            // fields are `|`-separated and a hand-rolled split here went
            // stale the moment that changed
            // (`docs/06-views.md`).
            if !live.lines().filter_map(row).any(|r| r.pane.id == pane) {
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
        // The close is the thing that renumbers, and `purge` can run on a
        // grid whose daemon has never been pressed.
        self.hold_indexes();
        let _closed = self.run("kill-pane", &kill_pane_args(found.id))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Held by any test that touches the **process** environment, which
    /// the rest of this binary's tests are running in parallel with.
    static ENV: Mutex<()> = Mutex::new(());

    const SESSION: &str = "01M52GEKED8ZEJPMT2KCJ35ZWT";
    /// The slot the pad lives on, as `page:row:col`.
    // A **non-numeric** page name on purpose: every fixture here was
    // `1:row:col` until review w7 F1, which is why nothing noticed that a
    // page name reaches the listing unfenced.
    const AT: &str = "work:4:4";
    /// The same slot as geometry: absolute row 4, column 4.
    const PLACE: Place = Place { row: 4, col: 4 };

    /// `#{pane_id}|#{window_id}|#{pane_dead}|#{@tally_session}|#{@tally_slot}|#{session_name}|#{window_index}`
    ///
    /// `%4` is the corpse this file is about: dead, holding slot `AT`,
    /// tagged with a session that is not [`SESSION`]. `%5` is a live pane
    /// at another address, and `%1` is one of yours — neither tag set.
    const LISTING: &str = "\
%1|@1|0|||tally|1\n\
%2|@2|0|other-session|work:4:1|r04|1\n\
%3|@2|0|01M52GEKED8ZEJPMT2KCJ35ZWT|work:4:2|r04|2\n\
%4|@3|1|01M52MXB4WQTA79K5FTHMXZ8YN|work:4:4|r04|4\n\
%5|@3|0|||r04|4\n";

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
        let args = new_window_args(PLACE, SESSION, "anchor", false);
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
    /// (`docs/06-views.md`).
    #[test]
    fn a_column_is_the_window_index() {
        let args = new_window_args(Place { row: 7, col: 3 }, SESSION, "anchor", false);
        let t = args.iter().position(|a| a == "-t").expect("-t");
        assert_eq!(args.get(t + 1).map(String::as_str), Some("r07:3"));

        // Column 8 and column 1 address the same row, never each other.
        let eight = new_window_args(Place { row: 7, col: 8 }, SESSION, "anchor", false);
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

    /// The option is server-wide: a row the user made, or one made before
    /// this ran, falls back to it. `-t r07` would pin only the rows we
    /// happened to name.
    #[test]
    fn indexes_are_held_for_the_whole_server_and_not_one_row() {
        let args = hold_indexes_args();
        assert_eq!(args.first().map(String::as_str), Some("set-option"));
        assert!(args.contains(&"-g".to_owned()));
        assert!(!args.contains(&"-t".to_owned()));
        assert_eq!(
            args.iter()
                .rev()
                .take(2)
                .rev()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["renumber-windows", "off"]
        );
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
            new_window_args(PLACE, SESSION, "anchor", false),
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
        assert!(new_window_args(PLACE, SESSION, "anchor", false).contains(&"-t".to_owned()));
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
        assert_eq!(found.id, "%3");
        assert_eq!(found.window, "@2");
        assert!(!found.dead);
        assert_eq!(view_for(LISTING, "nope"), None);
    }

    /// An unset option prints as an empty field. An untagged pane must
    /// never match, or an empty session id claims the first pane on the
    /// server — which is every pane you work in.
    #[test]
    fn an_untagged_pane_is_not_a_candidate() {
        assert_eq!(view_for("%9|@9|0|||tally|1\n", SESSION), None);
        assert_eq!(view_for(LISTING, ""), None, "an empty tag matches nothing");
    }

    /// A page named `a|b` used to shift every field after the slot tag,
    /// and the jump still *looked* right because the session id is field
    /// four, ahead of the damage (review w7 F1).
    /// `src/config.rs::resolve_pages` now refuses such a name; this pins
    /// the other half, that a line carrying one is unparseable rather
    /// than quietly misparsed.
    #[test]
    fn a_pipe_in_the_address_does_not_shift_the_fields() {
        assert_eq!(
            row("%4|@3|1|01M52MXB4WQTA79K5FTHMXZ8YN|a|b:4:4|r04|4").map(|r| r.pane.id),
            None,
            "eight fields: the seven read would be the wrong seven"
        );
    }

    /// Two fields can be empty now, so neither may be positional-by-luck:
    /// an untagged pane must still parse as itself rather than shifting
    /// the fields after it (`docs/06-views.md`).
    #[test]
    fn an_empty_tag_does_not_shift_the_fields() {
        let r = row("%9|@9|1|||tally|1").expect("parses");
        assert_eq!(r.pane.id, "%9");
        assert_eq!(r.pane.window, "@9");
        assert!(r.pane.dead, "the field after two empties is still read");
        assert_eq!(r.session, "");
        assert_eq!(r.slot, "");
    }

    /// The listing has to carry the address, or the second lookup has
    /// nothing to look in.
    #[test]
    fn the_listing_carries_the_address() {
        let fmt = list_panes_args().last().expect("format").clone();
        assert!(fmt.contains(SLOT_OPTION), "{fmt}");
        assert!(fmt.contains(PANE_OPTION), "{fmt}");
        assert!(
            fmt.contains(FIELD),
            "fields are separated by {FIELD}: {fmt}"
        );
        assert!(
            !fmt.contains(' '),
            "a space would swallow an empty tag: {fmt}"
        );
    }

    /// The bug: a corpse from the previous session holds the slot's
    /// window, so nothing can be created at that index until it is
    /// reused (`docs/06-views.md`).
    #[test]
    fn a_corpse_holding_the_slot_is_found_by_address() {
        let stale = dead_view_at(LISTING, AT).expect("the dead pane at this address");
        assert_eq!(stale.id, "%4");
        assert_eq!(stale.window, "@3");
        assert_ne!(
            view_for(LISTING, SESSION).map(|v| v.id),
            Some(stale.id),
            "found by address precisely because the id does not match"
        );
    }

    /// Decided in 27: a live `anchor attach` at this address is a view
    /// somebody is reading, and a spawn is not the gesture that closes it.
    #[test]
    fn a_live_view_at_the_address_is_left_alone() {
        let live = "%7|@7|0|other-session|work:4:4|r04|4\n";
        assert_eq!(dead_view_at(live, AT), None);
    }

    /// An unset address is not an address, for the reason an unset
    /// session id is not a session id.
    #[test]
    fn an_untagged_pane_holds_no_address() {
        assert_eq!(dead_view_at("%9|@9|1|||tally|1\n", ""), None);
        assert_eq!(dead_view_at(LISTING, ""), None);
        assert_eq!(
            dead_view_at(LISTING, "nowhere:9:9"),
            None,
            "nothing at that slot"
        );
    }

    /// The view for [`SESSION`] after a Move to row 2, column 1: tagged
    /// with its new address, **still in `r03:1`** — the state the owner's
    /// log is made of (`docs/06-views.md`).
    const SQUATTER: &str = "%7|@7|0|01M52GEKED8ZEJPMT2KCJ35ZWT|1:2:1|r03|1\n";
    const MOVED_TO: Place = Place { row: 2, col: 1 };

    fn squatter(listing: &str) -> Pane<'_> {
        view_for(listing, SESSION).expect("the moved view")
    }

    /// The claim and the place are different questions. A listing that
    /// carried only the claim could not see a moved view at all.
    #[test]
    fn the_listing_says_where_a_pane_is() {
        let fmt = list_panes_args().last().expect("format").clone();
        assert!(fmt.contains("#{session_name}"), "{fmt}");
        assert!(fmt.contains("#{window_index}"), "{fmt}");
        let p = squatter(SQUATTER);
        assert_eq!((p.tmux_session, p.index), ("r03", "1"));
    }

    /// Where it already is, there is nothing to do — and tmux would
    /// refuse it (`same index`) if asked.
    #[test]
    fn a_view_at_its_address_stays() {
        let here = "%7|@7|0|01M52GEKED8ZEJPMT2KCJ35ZWT|1:3:1|r03|1\n";
        assert_eq!(
            relocation(here, squatter(here), Place { row: 3, col: 1 }),
            None
        );
    }

    /// The bug, with an empty destination in a row that exists.
    #[test]
    fn a_moved_view_goes_to_the_window_its_address_names() {
        let listing = format!("{SQUATTER}%8|@8|0|other|1:2:2|r02|2\n");
        assert_eq!(
            relocation(&listing, squatter(&listing), MOVED_TO),
            Some(Relocation::Move {
                replace: false,
                create_row: false
            })
        );
    }

    /// The owner's log, line 19183: the pad it moved to was Stopped a
    /// moment before, so its corpse holds the index. A relocation that
    /// failed on that would turn a working jump into a broken one.
    #[test]
    fn a_corpse_at_the_destination_is_replaced() {
        let listing = format!("{SQUATTER}%8|@8|1|01DEAD|1:2:1|r02|1\n");
        assert_eq!(
            relocation(&listing, squatter(&listing), MOVED_TO),
            Some(Relocation::Move {
                replace: true,
                create_row: false
            })
        );
    }

    /// Decided in 27 and kept: a live view is somebody's to close. Never
    /// `-k` over it — it is a **swap** instead, so the address is freed
    /// without anything being closed (`docs/06-views.md`).
    ///
    /// ⚠️ This returned `None` until #19. A refusal here was followed by
    /// a tag written anyway, which is how a pane came to wear an address
    /// its window did not hold.
    #[test]
    fn a_live_view_at_the_destination_is_swapped_with_never_replaced() {
        let listing = format!("{SQUATTER}%8|@8|0|01LIVE|1:9:9|r02|1\n");
        let plan = relocation(&listing, squatter(&listing), MOVED_TO);
        assert_eq!(plan, Some(Relocation::Swap));
        assert!(
            !matches!(plan, Some(Relocation::Move { replace: true, .. })),
            "a live view is never killed to make room"
        );
    }

    /// `move-window` cannot create the session it is moving into.
    #[test]
    fn a_missing_row_is_made_and_its_placeholder_may_be_replaced() {
        assert_eq!(
            relocation(SQUATTER, squatter(SQUATTER), MOVED_TO),
            Some(Relocation::Move {
                replace: true,
                create_row: true
            })
        );
    }

    /// Within a row the index is all that differs.
    #[test]
    fn a_move_within_a_row_is_a_move_of_the_index() {
        let listing = "%7|@7|0|01M52GEKED8ZEJPMT2KCJ35ZWT|1:3:4|r03|1\n";
        assert_eq!(
            relocation(listing, squatter(listing), Place { row: 3, col: 4 }),
            Some(Relocation::Move {
                replace: false,
                create_row: false
            })
        );
    }

    /// Aimed at the pane, never at a bare `rNN:`, and `-d` so that a
    /// relocation moves nobody. Measured against tmux 3.6b in
    /// `docs/06-views.md`; this pins the arguments that measurement
    /// was made with.
    #[test]
    fn a_swap_names_the_pane_and_does_not_follow_it() {
        let args = swap_window_args("%7", MOVED_TO);
        assert_eq!(args[0], "swap-window");
        assert!(args.contains(&"-d".to_owned()), "{args:?}");
        let source = args.iter().position(|a| a == "-s").expect("-s");
        assert_eq!(args[source + 1], "%7", "the source is the pane: {args:?}");
        let target = args.iter().position(|a| a == "-t").expect("-t");
        assert_eq!(args[target + 1], "r02:1", "{args:?}");
    }

    /// `dead_view_at` asks *"is there a corpse claiming this address"*,
    /// so a corpse claiming a **different** one is invisible to it and
    /// wedged the pad forever (`docs/06-views.md`). This one asks
    /// about the window instead, which is the question that was missing.
    #[test]
    fn the_occupant_is_found_by_its_window_whatever_it_claims() {
        let listing = "%8|@8|1|01DEAD|1:9:9|r02|1\n";
        let held = occupant_at(listing, MOVED_TO).expect("occupied");
        assert_eq!(held.pane.id, "%8");
        assert!(held.pane.dead);
        assert_eq!(held.slot, "1:9:9", "it claims somewhere else entirely");
        assert!(
            dead_view_at(listing, "1:2:1").is_none(),
            "and the address lookup cannot see it, which was the bug"
        );
    }

    /// Nothing there is not an occupant.
    #[test]
    fn an_empty_index_has_no_occupant() {
        let listing = "%8|@8|0|01LIVE|1:2:2|r02|2\n";
        assert!(occupant_at(listing, MOVED_TO).is_none());
    }

    /// Parking is above the column range on purpose: no pad can name
    /// index 90, so a parked view cannot wedge anything.
    #[test]
    fn a_squatter_is_parked_out_of_the_address_space() {
        let listing = "%8|@8|0|01LIVE|1:9:9|r02|1\n";
        let index = park_index(listing, MOVED_TO);
        assert!(index >= u16::from(PARK), "{index} must be unaddressable");
        assert!(index > 8, "a column is 1-8 and this must not be one");
    }

    /// Two squatters in one row do not land on each other.
    #[test]
    fn parking_skips_a_space_already_taken() {
        let listing = format!("%8|@8|0|01LIVE|1:9:9|r02|1\n%9|@9|0|01OTHER|1:9:8|r02|{PARK}\n");
        assert_eq!(park_index(&listing, MOVED_TO), u16::from(PARK) + 1);
    }

    /// `-k` on a creation is the corpse case and nothing else, for the
    /// same reason it is on a move: it is a flag somebody can pass by
    /// accident.
    #[test]
    fn a_new_window_only_replaces_when_it_was_asked_to() {
        let plain = new_window_args(PLACE, SESSION, "anchor", false);
        assert!(!plain.contains(&"-k".to_owned()), "{plain:?}");
        let replacing = new_window_args(PLACE, SESSION, "anchor", true);
        assert!(replacing.contains(&"-k".to_owned()), "{replacing:?}");
        assert_eq!(replacing[0], "new-window");
    }

    /// `-k` is a flag somebody can pass by accident, so it is only there
    /// when the plan asked, and always before the pane it is aimed with.
    #[test]
    fn replacing_is_asked_for_and_never_assumed() {
        let plain = relocate_window_args("%7", MOVED_TO, false);
        assert!(!plain.contains(&"-k".to_owned()));
        assert_eq!(plain, move_window_args("%7", MOVED_TO));
        let hard = relocate_window_args("%7", MOVED_TO, true);
        assert_eq!(hard.get(1).map(String::as_str), Some("-k"));
        let s = hard.iter().position(|a| a == "-s").expect("-s");
        assert_eq!(hard.get(s + 1).map(String::as_str), Some("%7"));
        let t = hard.iter().position(|a| a == "-t").expect("-t");
        assert_eq!(hard.get(t + 1).map(String::as_str), Some("r02:1"));
    }

    /// Server-wide would change how every row behaves for a move that
    /// concerns one.
    #[test]
    fn a_client_is_kept_by_the_session_not_the_server() {
        let args = keep_clients_args("r03");
        assert!(!args.contains(&"-g".to_owned()));
        assert_eq!(args.get(2).map(String::as_str), Some("r03"));
        assert_eq!(args.last().map(String::as_str), Some("off"));
    }

    /// The placeholder is killed by the id it printed, because a window
    /// index is not a thing it can be sure of.
    #[test]
    fn the_placeholder_is_named_by_window_id() {
        let args = placeholder_args(MOVED_TO);
        let f = args.iter().position(|a| a == "-F").expect("-F");
        assert_eq!(args.get(f + 1).map(String::as_str), Some("#{window_id}"));
        let n = args.iter().position(|a| a == "-s").expect("-s");
        assert_eq!(args.get(n + 1).map(String::as_str), Some("r02"));
        assert_eq!(
            kill_window_args("@4").last().map(String::as_str),
            Some("@4")
        );
    }

    /// The field that can be empty goes last, so nothing after it shifts
    /// left when it is. This is the parse the whole module rests on.
    #[test]
    fn a_dead_view_is_distinguished_from_a_live_one() {
        let dead = view_for(LISTING, "01M52MXB4WQTA79K5FTHMXZ8YN").expect("dead pane");
        assert!(dead.dead, "pane_dead=1 means nothing is attached here");
        assert_eq!(dead.id, "%4");
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

    /// The conversation is the id you can actually use after a kill:
    /// anchor's names a PTY that is gone (`docs/06-views.md`).
    #[test]
    fn the_dead_line_offers_the_conversation_as_a_resume_command() {
        let fmt = dead_pane_format();
        assert!(
            fmt.contains(&format!("claude --resume #{{{CLAUDE_OPTION}}}")),
            "{fmt} resumes the conversation by name"
        );
        assert!(
            fmt.find(CLAUDE_OPTION) < fmt.find(PANE_OPTION),
            "{fmt}: the id that resumes something survives the truncation"
        );
    }

    /// A view of a shell has no conversation, and must read exactly as it
    /// did before any of this existed. The false branch of the
    /// conditional is empty, so an unset option costs the line nothing.
    #[test]
    fn a_view_with_no_conversation_says_nothing_about_one() {
        let fmt = dead_pane_format();
        assert!(
            fmt.contains(&format!("#{{?#{{{CLAUDE_OPTION}}},")),
            "{fmt} is conditional on the option being set"
        );
        assert!(
            fmt.contains(&format!("#{{{CLAUDE_OPTION}}},}}")),
            "{fmt} has an empty false branch"
        );
    }

    /// Both ids are read at draw time, for the reason the session id is:
    /// a literal would be stale the moment the pane is rebound.
    /// Not an assertion — the string itself, so the end-to-end probe in
    /// `docs/26` is run against what the binary actually produces rather
    /// than a retyped copy of it. `cargo test -- --nocapture the_dead_line_as_shipped`.
    #[test]
    fn the_dead_line_as_shipped() {
        println!("{}", dead_pane_format());
    }

    #[test]
    fn the_conversation_tag_is_a_pane_option() {
        let args = set_claude_args("%7", "c1567a67");
        assert!(args.contains(&"-p".to_owned()), "{args:?} is pane-scoped");
        assert!(args.contains(&CLAUDE_OPTION.to_owned()), "{args:?}");
        assert!(args.last().is_some_and(|v| v == "c1567a67"), "{args:?}");
    }

    /// `attach exit` is the attach client's status, not the agent's. The
    /// agent's exit is on the pad; calling this one `exit` would be a lie
    /// on every kill (`docs/06-views.md`).
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
        let blank = clients("\n\n");
        assert!(blank.is_empty(), "{blank:?}");
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
        assert_eq!(pane_title(PLACE, AT, SESSION), "c4 · work:4:4 · 5ZWT");
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
        assert_eq!(pane_title(PLACE, AT, ""), "c4 · work:4:4 · ");
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

    /// With no override this really does take the screen, which is the
    /// point of it on a desk and the bug off one.
    #[test]
    fn a_raise_with_no_override_focuses_the_app() {
        assert_eq!(
            raise_command("Alacritty", None),
            (
                "open".to_owned(),
                vec!["-a".to_owned(), "Alacritty".to_owned()]
            )
        );
        // Whitespace is not an override, exactly as for the window.
        assert_eq!(raise_command("Alacritty", Some(" ")).0, "open");
    }

    /// The suite's escape hatch, and the reason this function exists:
    /// `TALLY_RAISE_CMD=true` has to leave nothing that can reach a
    /// window manager.
    #[test]
    fn a_raise_override_replaces_the_whole_command() {
        assert_eq!(
            raise_command("Alacritty", Some("true")),
            ("true".to_owned(), vec![])
        );
        // The app name is gone, not appended: an override that still
        // carried `-a Alacritty` would raise the window anyway.
        let (bin, args) = raise_command("Alacritty", Some("echo raised"));
        assert_eq!(bin, "echo");
        assert_eq!(args, ["raised"]);
    }

    /// The trap this whole fix is about, pinned: **unset is not off.**
    /// `tests/harness` believed otherwise and raised a window on the
    /// running machine for every jump in the suite.
    #[test]
    fn an_unset_terminal_still_names_an_app() {
        let _gate = ENV
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let restore = std::env::var_os("TALLY_TERMINAL");
        std::env::remove_var("TALLY_TERMINAL");
        let app = terminal_app();
        if let Some(v) = restore {
            std::env::set_var("TALLY_TERMINAL", v);
        }
        assert_eq!(
            app.as_deref(),
            Some("Alacritty"),
            "unsetting the app does not disable the raise -- TALLY_RAISE_CMD does"
        );
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
