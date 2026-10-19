//! Sessions in, frame out. The only place that knows both halves.
//!
//! This is a **pure function**: `anchor list` plus derived signals plus
//! which page you are looking at, producing 81 lit cells. Nothing is
//! cached, nothing is incremental. That is what makes `tallyd` disposable —
//! it holds no state that cannot be rebuilt from a `list` and a config file
//! (`docs/08-daemon.md`).

use std::collections::{BTreeMap, BTreeSet};

use crate::anchor::Session;
use crate::chord::Action;
use crate::effect::{effect, Pad};
use crate::render;
use crate::signal::{slot_state, Derived, SessionState, SlotState};
use crate::slot::{self, Pages, Slot};
use crate::surface::{Button, Frame};

/// Everything the renderer needs that is not a session.
#[derive(Debug, Clone)]
pub struct View<'a> {
    /// Declared page names, in top-row order.
    pub pages: &'a Pages,
    /// How far the eight-row window is scrolled down the stacked row
    /// space, `0..=`[`slot::MAX_OFFSET`].
    ///
    /// **Rows, not pages.** `▲▼` move this by one, so the grid slides
    /// rather than cutting — a page boundary is not a thing you can feel
    /// and a row is. Pages still name addresses; they stopped being a
    /// thing you jump between (`docs/05-interaction.md#the-queue-and-the-two-buttons-that-read-it`).
    pub offset: u8,
    /// Did the last `list` succeed? An unreachable anchord lights the logo
    /// and **leaves the grid alone**, because those sessions were there a
    /// moment ago and probably still are.
    pub anchord_reachable: bool,
    /// Milliseconds since the epoch.
    pub now: i64,
    /// Is there anywhere for `◄` to go? Read from
    /// [`crate::jump::Jump::can_go_back`] — the same predicate the press
    /// asks, so the button never lights up and then does nothing.
    pub can_go_back: bool,
    /// How many slots `►` would offer — [`crate::queue::depth`]. Zero is
    /// what makes `►` dark, and it is the same count the journal records
    /// as `queued`.
    pub queued: usize,
    /// Which grid rows `>` would spawn into, 1-7. One predicate, two
    /// readers: this lights the button and the press uses the same
    /// [`row_launch`] that filled it.
    pub launchable: &'a BTreeSet<u8>,
    /// The modifier being held, if exactly one is
    /// (`crate::chord::Chords::modifier`). The grid recolours to show
    /// what it would do.
    pub held: Option<Action>,
    /// The pad picked up under a held [`Action::Move`], if any.
    ///
    /// Only ever `Some` while [`Self::held`] is [`Action::Move`]: letting
    /// go empties the hand ([`crate::act::Done::Picked`]).
    pub hand: Option<Slot>,
    /// Every action button physically down — which is **not** the same as
    /// [`Self::held`]: two buttons down lights both white and previews
    /// neither, so a fumble is visible rather than merely inert
    /// (`crate::chord::Chords::modifier`).
    pub pressed: &'a BTreeSet<Action>,
    /// Is an art playing? It lights `Drums` white, and nothing else here
    /// knows about it: the daemon paints the 64 pads itself while this is
    /// true (`docs/05-interaction.md`).
    pub art: bool,
    /// Which verb `CC19` is carrying (`crate::chord::Chords::mode`).
    /// Read here rather than assumed, because the corner's colour is the
    /// only thing that says it ([`render::action`]).
    pub mode: Action,
}

/// What a session resolved to, once.
#[derive(Debug, Clone)]
pub struct Resolved {
    /// Which pad it holds.
    pub slot: Slot,
    /// The session in it.
    pub session: String,
    /// What that session last called itself, if it said. Absent is normal.
    pub title: Option<String>,
    /// What that pad renders.
    pub state: SlotState,
    /// Carries `tl.mute`. Not part of [`SlotState`] on purpose: a muted
    /// slot renders the state *underneath* its attention, so muting is
    /// invisible until you hold the mute button and ask.
    pub muted: bool,
    /// The conversation it is running, from `tl.claude`.
    ///
    /// Carried here so the view can be tagged with it on a jump, which is
    /// what puts it on the dead line
    /// (`docs/06-views.md`). Nothing on the grid renders
    /// it: a pad is an address, and a UUID is not something you can see
    /// in a colour.
    pub claude: Option<String>,
    /// The directory it is running in, from anchord's row.
    ///
    /// Carried for [`Resolved::claude`]'s reason and no other: on a jump
    /// the view's tmux pane has to be *put* somewhere, and the command it
    /// runs — `anchor attach` — tells tmux nothing about where the agent
    /// is (`docs/06-views.md#the-pane-starts-where-the-agent-does`).
    /// Nothing on the grid renders it.
    pub cwd: Option<String>,
}

/// Resolve every session to a slot, newest wins, invalid ignored.
///
/// anchor lists newest-first, so "first one seen owns the slot" *is*
/// newest-wins, and it costs nothing. Two sessions on one pad is a pad you
/// cannot read; the older one is simply not rendered.
pub fn resolve(
    sessions: &[Session],
    derived: &BTreeMap<String, Derived>,
    pages: &Pages,
    now: i64,
) -> BTreeMap<Slot, Resolved> {
    let mut taken: BTreeMap<Slot, Resolved> = BTreeMap::new();
    for session in sessions {
        let Some(label) = session.slot_label() else {
            continue;
        };
        let Some(slot) = slot::parse(label, pages) else {
            continue;
        };
        if taken.contains_key(&slot) {
            continue; // an older session for the same pad
        }
        let view = crate::signal::Session {
            lost_to_restart: session.lost_to_restart(),
            state: session.lifecycle(),
            exit_code: session.exit_code,
            muted: session.muted(),
            declared_attention: session.declared_attention(),
            derived: derived.get(&session.id),
        };
        let state = slot_state(Some(view), now);
        // A kill you asked for is not a failure. anchord records a
        // terminate as an ordinary non-zero exit, so without the label
        // the pad you just killed goes red -- and red has one job, which
        // is *go and look*. Checked here rather than in `slot_state`
        // because this is tally's opinion about its own label, not
        // anything the signal layer can derive from bytes.
        let asked_for = session.killed() && matches!(state, SlotState::Failed { .. });
        if state == SlotState::Empty || asked_for {
            // A clean exit holds its slot in anchor but not on the grid.
            // Leaving it in `taken` would shadow a fresh session spawned
            // onto the same pad, which is exactly what pressing it does.
            continue;
        }
        let title = derived
            .get(&session.id)
            .and_then(|d| d.title().map(str::to_owned));
        let _previous = taken.insert(
            slot,
            Resolved {
                slot,
                session: session.id.clone(),
                title,
                state,
                muted: session.muted(),
                claude: session.claude().map(ToOwned::to_owned),
                cwd: session.cwd.clone(),
            },
        );
    }
    taken
}

/// Every session anchord would agree to forget, by the pad that claims it.
///
/// **Not the same set `resolve` draws.** A clean exit keeps its `tl.slot`
/// and its log but renders as nothing, so a pad can be dark and still have
/// a dozen rows behind it. Those are exactly what the purge chord is for,
/// and the only way to see them is to hold it
/// (`docs/02-model.md#exited-rows-accumulate-and-purge-is-the-answer`).
///
/// Newest first, because that is the order anchord lists in and the order
/// you would delete them by hand.
pub fn purgeable(sessions: &[Session], pages: &Pages) -> BTreeMap<Slot, Vec<String>> {
    let mut by_slot: BTreeMap<Slot, Vec<String>> = BTreeMap::new();
    for session in sessions {
        // Running and closing are not purgeable; anchord refuses them
        // with `already_exited`, which reads backwards and is its name to
        // choose (`anchor/src/session/manager.rs:416-422`).
        if !matches!(
            session.lifecycle(),
            SessionState::Exited | SessionState::Lost
        ) {
            continue;
        }
        let Some(slot) = session.slot_label().and_then(|l| slot::parse(l, pages)) else {
            continue;
        };
        by_slot.entry(slot).or_default().push(session.id.clone());
    }
    by_slot
}

/// What `>` beside a grid row would spawn.
///
/// The row already knows: anchord keeps `cwd` and `command` on every row
/// it has, dead ones included, so a row that has ever held a session can
/// hold another one without anything being declared in advance. That is
/// what `slots.toml` was trying to be and could not
/// (`docs/05-interaction.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowLaunch {
    /// The leftmost pad in that row with nothing drawn on it.
    ///
    /// Stricter than [`SlotState::frees_the_pad`], which `tally spawn`
    /// uses: a failed pad is drawn, so `>` steps over it. Failures stay
    /// visible because they are the ones you have to go and look at
    /// (`docs/05-interaction.md`), and a tap with no confirmation must not
    /// be what buries one. `tally spawn` can reclaim it because it names
    /// the pad.
    pub at: Slot,
    /// `argv[0]`, from the row's newest session.
    pub command: String,
    /// The directory that session was started in.
    pub cwd: String,
    /// Whether to wrap it in a login shell, as that session was.
    pub login_shell: bool,
}

/// Resolve `>` for the slots on absolute row `abs`.
///
/// `None` — and therefore a dark button — when the row has never held a
/// session, when nothing in it recorded a directory, or when every column
/// is taken. A lit button that would do nothing is one you press twice.
///
/// **Arguments are deliberately dropped.** A row running
/// `claude --resume <id>` describes one conversation; copying that would
/// put a second pad on the same conversation, which is the one thing a new
/// agent must not be.
#[must_use]
pub fn row_launch(
    sessions: &[Session],
    pages: &Pages,
    drawn: &BTreeMap<Slot, Resolved>,
    abs: u8,
) -> Option<RowLaunch> {
    // Newest first, which is the order anchord lists in and the order
    // `resolve` already depends on.
    let newest = sessions.iter().find(|s| {
        s.slot_label()
            .and_then(|l| slot::parse(l, pages))
            .is_some_and(|at| at.absolute_row() == abs)
    })?;
    let cwd = newest.cwd.clone()?;
    let command = newest.command.clone()?;
    let at = (1..=slot::COLS).find_map(|col| {
        let at = Slot::at_absolute(abs, col)?;
        (!drawn.contains_key(&at)).then_some(at)
    })?;
    Some(RowLaunch {
        at,
        command,
        cwd,
        login_shell: newest.login_shell,
    })
}

/// What a pad can be forked from.
///
/// Three fields and no address: `row_launch` picks the destination
/// because `>` is a one-press gesture with nowhere else to put it, and
/// `tally fork` is told both ends by the person typing it
/// (`docs/07-cli.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inherit {
    /// `argv[0]`, from the newest session that pad has ever held.
    pub command: String,
    /// The directory that session was started in.
    pub cwd: String,
    /// Whether to wrap it in a login shell, as that session was.
    pub login_shell: bool,
}

/// What `at` would hand to a session forked from it.
///
/// **The newest session that pad has ever held, dead rows included** —
/// the same rule `row_launch` runs on, and what lets a fork work on a pad
/// whose agent has already exited. `None` when the pad has never held
/// anything, or when the row anchord has for it recorded no directory.
///
/// **Arguments are not here**, for [`row_launch`]'s reason.
#[must_use]
pub fn inherit(sessions: &[Session], pages: &Pages, at: Slot) -> Option<Inherit> {
    // Newest first ([`row_launch`]).
    let newest = sessions
        .iter()
        .find(|s| s.slot_label().and_then(|l| slot::parse(l, pages)) == Some(at))?;
    Some(Inherit {
        command: newest.command.clone()?,
        cwd: newest.cwd.clone()?,
        login_shell: newest.login_shell,
    })
}

/// The grid and the rows behind it — everything it takes to answer
/// "what is this pad" for any of the 512 addresses.
///
/// **The one place a [`Pad`] is built.** The preview asks it 64 times a
/// frame and [`crate::act::Context`] asks it once per press, and because
/// it is the same function and not two copies of the same four lines,
/// they cannot answer differently. They used to, and
/// `the_preview_and_the_press_agree_about_a_dead_pad` is what that cost.
#[derive(Debug, Clone, Copy)]
pub struct Pads<'a> {
    grid: &'a BTreeMap<Slot, Resolved>,
    purgeable: &'a BTreeMap<Slot, Vec<String>>,
    /// The pad picked up under a held [`Action::Move`], if any.
    ///
    /// Here rather than passed to [`effect`] by each caller, for the
    /// reason the whole struct exists: one function handed two
    /// differently-built [`Pad`]s tells the same lie as two functions.
    hand: Option<Slot>,
}

impl<'a> Pads<'a> {
    /// Borrow a resolved grid and the rows anchord would forget
    /// ([`purgeable`]).
    /// `hand` is the pad picked up under a held [`Action::Move`]. It is a
    /// parameter rather than a setter because a [`Pads`] built without it
    /// answers [`effect`] wrongly for every pad on the grid, and a
    /// builder you can forget to call is how the preview and the press
    /// drifted apart the first time.
    ///
    /// **A hand whose session is no longer alive is an empty hand.** The
    /// pick-up refuses a dead pad, so the session died *after* it was
    /// picked up, with the modifier still down — and a failed or lost row
    /// stays in the grid, so only the pad's state can say so. Resolved
    /// here, with everything else a pad means, so the preview and the
    /// press both see the hand empty and neither offers a drop that would
    /// rewrite the address of a session that has ended
    /// ([`crate::effect::effect`]). The hand the surface keeps is left
    /// alone; it clears on release, as it always has.
    pub fn new(
        grid: &'a BTreeMap<Slot, Resolved>,
        purgeable: &'a BTreeMap<Slot, Vec<String>>,
        hand: Option<Slot>,
    ) -> Self {
        let hand = hand.filter(|h| {
            grid.get(h).is_some_and(|r| {
                Pad {
                    state: Some(r.state),
                    ..Pad::default()
                }
                .alive()
            })
        });
        Self {
            grid,
            purgeable,
            hand,
        }
    }

    /// The pad in your hand, if one is.
    pub fn hand(&self) -> Option<Slot> {
        self.hand
    }

    /// What is drawn, by pad. The caller that already resolved gets it
    /// back rather than resolving again.
    pub fn grid(&self) -> &'a BTreeMap<Slot, Resolved> {
        self.grid
    }

    /// Every session anchord would forget on this pad, newest first.
    /// Empty for a pad with nothing behind it.
    pub fn rows(&self, at: Slot) -> &'a [String] {
        self.purgeable.get(&at).map_or(&[], Vec::as_slice)
    }

    /// One pad, as [`effect`] sees it.
    pub fn at(&self, at: Slot) -> Pad {
        let r = self.grid.get(&at);
        Pad {
            state: r.map(|r| r.state),
            muted: r.is_some_and(|r| r.muted),
            purgeable: !self.rows(at).is_empty(),
            in_hand: self.hand == Some(at),
        }
    }
}

impl View<'_> {
    /// Which grid row shows `at`, or `None` if it is scrolled off.
    ///
    /// The renderer's way to turn the window into a position; the inverse
    /// of [`Self::slot_at`], which both the preview and the press go
    /// through.
    #[must_use]
    pub fn display_row(&self, at: Slot) -> Option<u8> {
        at.absolute_row()
            .checked_sub(self.offset)
            .filter(|row| (1..=slot::WINDOW).contains(row))
    }

    /// The slot under grid row `row`, column `col` of a window scrolled
    /// to `offset`, or `None` past the end of the address space.
    ///
    /// The inverse of [`Self::display_row`]. **Takes the offset rather
    /// than `&self` so the press can reach it** — `tallyd` has an `offset`
    /// and no `View` at the moment a pad goes down, and so open-coded this
    /// arithmetic with a `saturating_add` where this used `checked_add`.
    /// The two agreed only because `MAX_OFFSET + WINDOW == ROWS` makes
    /// every overflow fail the bounds check anyway; they were one
    /// constant away from disagreeing, which is the split [`Pads`] exists
    /// to stop one module over. Same shape as [`Self::offset_showing`].
    #[must_use]
    pub fn slot_at(offset: u8, row: u8, col: u8) -> Option<Slot> {
        Slot::at_absolute(offset.checked_add(row)?, col)
    }

    /// The window scrolled by the smallest amount that brings `at` into
    /// view.
    ///
    /// Minimal, not centred: `►` already moves your terminal, and moving
    /// the grid further than it has to is a second thing changing under
    /// you for no reason.
    #[must_use]
    pub fn offset_showing(at: Slot, offset: u8) -> u8 {
        let abs = at.absolute_row();
        if abs <= offset {
            return abs.saturating_sub(1);
        }
        if abs > offset + slot::WINDOW {
            return abs.saturating_sub(slot::WINDOW);
        }
        offset
    }
}

/// Build the frame, resolving and counting purgeable rows on the way.
///
/// The convenience entry point, not the one the daemon takes: `tallyd`
/// and [`crate::queue`] call [`resolve`], [`purgeable`] and [`Pads::new`]
/// themselves so the press path can hold on to the grid it just drew, and
/// then call [`frame_from`]. Everything that drives this today is a test.
pub fn frame(sessions: &[Session], derived: &BTreeMap<String, Derived>, view: &View<'_>) -> Frame {
    let resolved = resolve(sessions, derived, view.pages, view.now);
    let rows = purgeable(sessions, view.pages);
    frame_from(&Pads::new(&resolved, &rows, view.hand), view)
}

/// Build the frame from pads already assembled, so a caller that needs
/// the states as well as the picture does not resolve twice.
pub fn frame_from(pads: &Pads<'_>, view: &View<'_>) -> Frame {
    let resolved = pads.grid();
    let mut frame = Frame::dark();

    for r in resolved.values() {
        // Off the window. `►` is what reports what you cannot see
        // (`view.queued`); nothing here needs to know.
        let Some(row) = view.display_row(r.slot) else {
            continue;
        };
        if view.held.is_none() {
            frame.set(
                Button::Grid {
                    row,
                    col: r.slot.col,
                },
                render::slot(r.state),
            );
        }
    }

    // A held modifier replaces the grid with the answer to "what happens
    // if I press this". Every pad in the window is asked, not just the lit
    // ones: a dark pad can be a spawn or a pile of clean exits waiting to
    // be forgotten, and both are things the preview exists to reveal.
    if let Some(action) = view.held {
        for row in 1..=slot::WINDOW {
            for col in 1..=slot::COLS {
                let Some(at) = View::slot_at(view.offset, row, col) else {
                    continue;
                };
                let pad = pads.at(at);
                frame.set(
                    Button::Grid { row, col },
                    render::preview(effect(action, pad, pads.hand())),
                );
            }
        }
    }

    // The top row: `▲ ▼ ◄ ► · · · ·` (`docs/05-interaction.md`). Every
    // button here is self-describing ([`render::arrow`]).
    frame.set(Button::Top(1), render::arrow(view.offset > 0));
    frame.set(
        Button::Top(2),
        render::arrow(view.offset < slot::MAX_OFFSET),
    );
    frame.set(Button::Top(3), render::back(view.can_go_back));
    frame.set(Button::Top(4), render::next(view.queued > 0));
    frame.set(Button::Top(6), render::arts(view.art));
    // `Session` held the overview up until #41 ([`crate::chord::top`]).
    // Dark rather than carrying a new verb: an unassigned button is
    // better than a strained one.
    for index in [5].into_iter().chain(7..=crate::surface::TOP_BUTTONS) {
        frame.set(Button::Top(index), crate::surface::Cell::Off);
    }

    // The right column: seven `>` launchers and the kill modifier at the
    // bottom (`docs/05-interaction.md`).
    for row in 1..=7u8 {
        frame.set(
            Button::Action(row),
            render::launch(view.launchable.contains(&row)),
        );
    }
    frame.set(
        Button::Action(8),
        // Held is asked of the **button**, not of the verb the mode names
        // ([`crate::chord::Chords::feed`]). `pressed` holds whichever
        // verb was live when it went down.
        render::action(Some(view.mode), !view.pressed.is_empty()),
    );

    // Health, and nothing else. It does not move when the window does
    // (D26 in `docs/10-decisions.md`).
    frame.set(Button::Logo, render::logo(view.anchord_reachable));

    frame
}

/// Sessions worth attaching a watcher to.
///
/// Only slots get watchers: a session with no `tl.slot` is not on the grid,
/// so scanning its bytes buys nothing. Only live ones, because an exited
/// session has nothing left to read.
pub fn needs_watcher<'a>(
    sessions: &'a [Session],
    pages: &'a Pages,
) -> impl Iterator<Item = &'a Session> {
    sessions.iter().filter(move |s| {
        s.slot_label()
            .is_some_and(|l| slot::parse(l, pages).is_some())
            && matches!(s.lifecycle(), SessionState::Running | SessionState::Closing)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::palette::{AMBER, DARK_GREY, DARK_RED, GREEN, RED, WHITE};
    use crate::surface::Cell;

    const NOW: i64 = 1_760_000_000_000;

    fn session(id: &str, state: &str, slot: Option<&str>) -> Session {
        let mut labels = BTreeMap::new();
        if let Some(s) = slot {
            let _previous = labels.insert(crate::anchor::LABEL_SLOT.to_owned(), s.to_owned());
        }
        Session::fixture(id, state)
            .with_labels(labels)
            .with_output(10)
    }

    /// The resting view: nothing held, nothing pressed. Tests that care
    /// about those build on top of it.
    static LAUNCHABLE: std::sync::LazyLock<BTreeSet<u8>> = std::sync::LazyLock::new(BTreeSet::new);

    fn view(pages: &Pages) -> View<'_> {
        View {
            launchable: &LAUNCHABLE,
            pages,
            offset: 0,
            anchord_reachable: true,
            now: NOW,
            can_go_back: false,
            queued: 0,
            held: None,
            hand: None,
            pressed: &NO_BUTTONS,
            art: false,
            mode: Action::Stop,
        }
    }

    static NO_BUTTONS: std::sync::LazyLock<BTreeSet<Action>> =
        std::sync::LazyLock::new(BTreeSet::new);

    #[test]
    fn a_labelled_session_lights_its_pad() {
        let pages = Pages::default();
        let sessions = [session("a", "running", Some("1:3:5"))];
        let f = frame(&sessions, &BTreeMap::new(), &view(&pages));
        assert_eq!(f.get(Button::Grid { row: 3, col: 5 }), Cell::Static(GREEN));
    }

    /// teleport's sessions are on the same anchord and must be invisible.
    #[test]
    fn a_session_with_no_slot_label_is_not_on_the_grid() {
        let pages = Pages::default();
        let sessions = [session("teleport-owned", "running", None)];
        let f = frame(&sessions, &BTreeMap::new(), &view(&pages));
        assert_eq!(f, frame(&[], &BTreeMap::new(), &view(&pages)));
    }

    /// Reclaimable changes what a press does, never what the eye sees.
    /// The daemon still cannot account for what that session did.
    #[test]
    fn a_reclaimable_lost_pad_looks_exactly_like_any_other_lost_pad() {
        assert_eq!(
            render::slot(SlotState::Lost { reclaimable: true }),
            render::slot(SlotState::Lost { reclaimable: false })
        );
    }

    /// anchord records a terminate as an ordinary non-zero exit — there
    /// is no field saying who asked for it, and there should not be.
    /// Without `tl.killed` the pad you just killed on purpose renders
    /// identically to an agent that crashed, and red stops meaning *go
    /// and look*.
    #[test]
    fn a_kill_you_asked_for_is_not_a_failure() {
        let pages = Pages::default();
        let mut killed = session("k", "exited", Some("1:1:1"));
        killed.exit_code = Some(1);
        let _l = killed
            .labels
            .insert(crate::anchor::LABEL_KILLED.to_owned(), "1".to_owned());
        assert!(
            resolve(&[killed], &BTreeMap::new(), &pages, NOW).is_empty(),
            "a deliberate kill leaves no mark"
        );

        // The same exit without the label is still red: an agent that
        // died on its own is exactly what you have to go and look at.
        let mut crashed = session("c", "exited", Some("1:1:1"));
        crashed.exit_code = Some(1);
        let drawn = resolve(&[crashed], &BTreeMap::new(), &pages, NOW);
        assert_eq!(
            drawn
                .get(&Slot {
                    page: 1,
                    row: 1,
                    col: 1
                })
                .map(|r| r.state),
            Some(SlotState::Failed { code: 1 })
        );
    }

    /// The label only speaks about endings. A session we labelled and
    /// then failed to terminate is still running, and must still be green.
    #[test]
    fn the_killed_label_does_not_hide_a_live_session() {
        let pages = Pages::default();
        let mut live = session("k", "running", Some("1:1:1"));
        let _l = live
            .labels
            .insert(crate::anchor::LABEL_KILLED.to_owned(), "1".to_owned());
        let drawn = resolve(&[live], &BTreeMap::new(), &pages, NOW);
        assert_eq!(
            drawn
                .get(&Slot {
                    page: 1,
                    row: 1,
                    col: 1
                })
                .map(|r| r.state),
            Some(SlotState::Quiet)
        );
    }

    /// Dark does not mean forgotten: the row and its scrollback are still
    /// there, and the purge chord is still how you clear them.
    #[test]
    fn a_killed_row_is_still_purgeable() {
        let pages = Pages::default();
        let mut killed = session("k", "exited", Some("1:1:1"));
        killed.exit_code = Some(1);
        let _l = killed
            .labels
            .insert(crate::anchor::LABEL_KILLED.to_owned(), "1".to_owned());
        let rows = purgeable(&[killed], &pages);
        assert_eq!(
            rows.get(&Slot {
                page: 1,
                row: 1,
                col: 1
            })
            .map(Vec::len),
            Some(1)
        );
    }

    /// Holding a modifier replaces the grid. The pad that was green is
    /// now the colour of what would happen to it.
    #[test]
    fn a_held_modifier_recolours_the_grid() {
        let pages = Pages::default();
        let sessions = [session("a", "running", Some("1:3:5"))];
        let resting = frame(&sessions, &BTreeMap::new(), &view(&pages));
        assert_eq!(
            resting.get(Button::Grid { row: 3, col: 5 }),
            Cell::Static(GREEN)
        );

        let held = View {
            held: Some(Action::Stop),
            ..view(&pages)
        };
        let f = frame(&sessions, &BTreeMap::new(), &held);
        assert_eq!(
            f.get(Button::Grid { row: 3, col: 5 }),
            Cell::Static(RED),
            "the pad that would be killed is red"
        );
        assert_eq!(
            f.get(Button::Grid { row: 1, col: 1 }),
            Cell::Off,
            "an empty pad stays dark"
        );
    }

    /// `purgeable` is deliberately not the set `resolve` draws.
    #[test]
    fn purgeable_includes_rows_that_nothing_draws() {
        let pages = Pages::default();
        let mut clean = session("done", "exited", Some("1:2:2"));
        clean.exit_code = Some(0);
        let sessions = [clean, session("live", "running", Some("1:1:1"))];

        let drawn = resolve(&sessions, &BTreeMap::new(), &pages, NOW);
        let at = Slot {
            page: 1,
            row: 2,
            col: 2,
        };
        assert!(!drawn.contains_key(&at), "a clean exit is not drawn");

        let rows = purgeable(&sessions, &pages);
        assert_eq!(rows.get(&at).map(Vec::len), Some(1), "but it is purgeable");
        assert!(
            !rows.contains_key(&Slot {
                page: 1,
                row: 1,
                col: 1
            }),
            "a running session is not"
        );
    }

    /// A session with a directory and a command, the way anchord keeps
    /// them — on dead rows too.
    fn ran(id: &str, state: &str, slot: &str, command: &str, cwd: &str) -> Session {
        let mut s = session(id, state, Some(slot));
        s.command = Some(command.to_owned());
        s.cwd = Some(cwd.to_owned());
        s.login_shell = true;
        s
    }

    /// The whole point of the column: the row already knows the folder,
    /// so `>` needs nothing declared in advance.
    #[test]
    fn a_launch_inherits_the_rows_directory_and_command() {
        let pages = Pages::default();
        let sessions = [ran("a", "running", "1:2:1", "isayes", "/repo/bright")];
        let drawn = resolve(&sessions, &BTreeMap::new(), &pages, 0);
        let l = row_launch(&sessions, &pages, &drawn, 2).expect("launchable");
        assert_eq!(l.cwd, "/repo/bright");
        assert_eq!(l.command, "isayes");
        assert!(l.login_shell, "as that session was");
        assert_eq!(
            l.at,
            Slot {
                page: 1,
                row: 2,
                col: 2
            },
            "leftmost free"
        );
    }

    /// Copying `--resume <id>` would put a second pad on the same
    /// conversation, which is the one thing a new agent must not be.
    #[test]
    fn a_launch_drops_the_arguments() {
        let pages = Pages::default();
        let sessions = [ran("a", "running", "1:1:1", "claude", "/repo")];
        let drawn = resolve(&sessions, &BTreeMap::new(), &pages, 0);
        let l = row_launch(&sessions, &pages, &drawn, 1).expect("launchable");
        // `RowLaunch` has no args field at all -- there is nowhere for
        // them to be carried, which is stronger than remembering not to.
        assert_eq!(l.command, "claude");
    }

    /// A dead row still knows where it was.
    #[test]
    fn a_row_of_corpses_can_still_be_launched_into() {
        let pages = Pages::default();
        // Exit code 0: a *clean* exit, which frees its pad. An exited row
        // with no code at all is anchor saying it cannot account for the
        // ending, which is Lost, and Lost still holds the pad.
        let mut gone = ran("a", "exited", "1:3:1", "claude", "/repo/tally");
        gone.exit_code = Some(0);
        let sessions = [gone];
        let drawn = resolve(&sessions, &BTreeMap::new(), &pages, 3);
        let l = row_launch(&sessions, &pages, &drawn, 3).expect("launchable");
        assert_eq!(l.cwd, "/repo/tally");
        assert_eq!(
            l.at,
            Slot {
                page: 1,
                row: 3,
                col: 1
            },
            "a clean exit frees its own pad"
        );
    }

    /// Dark when it would do nothing: nothing to inherit, or nowhere to
    /// put it.
    #[test]
    fn an_empty_row_and_a_full_row_both_launch_nothing() {
        let pages = Pages::default();
        let drawn = BTreeMap::new();
        assert_eq!(row_launch(&[], &pages, &drawn, 1), None, "never used");

        let full: Vec<Session> = (1..=8)
            .map(|col| ran("s", "running", &format!("1:4:{col}"), "claude", "/repo"))
            .collect();
        let drawn = resolve(&full, &BTreeMap::new(), &pages, 0);
        assert_eq!(row_launch(&full, &pages, &drawn, 4), None, "no free column");
    }

    /// `tally spawn` reclaims a failed pad; `>` does not. The disagreement
    /// is deliberate: a red pad is telling you to go and look, and a tap
    /// must not be what silently reuses it.
    #[test]
    fn a_launch_steps_over_a_failed_pad_that_spawn_would_reclaim() {
        let pages = Pages::default();
        let mut bad = ran("a", "exited", "1:3:1", "claude", "/repo/tally");
        bad.exit_code = Some(1);
        let sessions = [bad];
        let drawn = resolve(&sessions, &BTreeMap::new(), &pages, 0);
        let held = drawn
            .get(&Slot {
                page: 1,
                row: 3,
                col: 1,
            })
            .expect("drawn");
        assert!(held.state.frees_the_pad(), "spawn may reclaim it");
        let l = row_launch(&sessions, &pages, &drawn, 3).expect("launchable");
        assert_eq!(
            l.at,
            Slot {
                page: 1,
                row: 3,
                col: 2
            },
            "`>` does not"
        );
    }

    /// anchord lists newest first, and the newest is the one whose
    /// directory you meant.
    #[test]
    fn a_launch_inherits_from_the_newest_session_in_the_row() {
        let pages = Pages::default();
        let sessions = [
            ran("new", "running", "1:5:1", "claude", "/repo/new"),
            ran("old", "exited", "1:5:2", "isayes", "/repo/old"),
        ];
        let drawn = resolve(&sessions, &BTreeMap::new(), &pages, 0);
        let l = row_launch(&sessions, &pages, &drawn, 5).expect("launchable");
        assert_eq!(l.cwd, "/repo/new");
        assert_eq!(l.command, "claude");
    }

    /// `fork` keys on the pad, not the row — which is the whole
    /// difference between it and `>`.
    #[test]
    fn a_fork_inherits_from_the_pad_it_names() {
        let pages = Pages::default();
        let sessions = [
            ran("a", "running", "1:5:1", "claude", "/repo/left"),
            ran("b", "running", "1:5:2", "codex", "/repo/right"),
        ];
        let left = Slot {
            page: 1,
            row: 5,
            col: 1,
        };
        let right = Slot { col: 2, ..left };
        let i = inherit(&sessions, &pages, left).expect("left");
        assert_eq!(i.cwd, "/repo/left");
        assert_eq!(i.command, "claude");
        assert_eq!(
            inherit(&sessions, &pages, right).expect("right").cwd,
            "/repo/right",
            "the row is not the unit here"
        );
    }

    /// The directory outlives the agent: anchord keeps `cwd` on rows that
    /// have exited, which is what lets you fork a finished one.
    #[test]
    fn a_fork_inherits_from_a_pad_whose_agent_has_ended() {
        let pages = Pages::default();
        let mut gone = ran("a", "exited", "1:7:1", "claude", "/repo/done");
        gone.exit_code = Some(0);
        let at = Slot {
            page: 1,
            row: 7,
            col: 1,
        };
        assert_eq!(
            inherit(&[gone], &pages, at).expect("ended").cwd,
            "/repo/done"
        );
    }

    /// Nothing to inherit is `None`, not a guess. A pad that has never
    /// held anything has no directory, and neither does one whose rows
    /// recorded none.
    #[test]
    fn a_fork_from_an_unused_pad_inherits_nothing() {
        let pages = Pages::default();
        let at = Slot {
            page: 1,
            row: 8,
            col: 1,
        };
        assert_eq!(inherit(&[], &pages, at), None, "never used");
        let no_cwd = [session("a", "running", Some("1:8:1"))];
        assert_eq!(inherit(&no_cwd, &pages, at), None, "no directory");
    }

    /// A row with sessions but no recorded directory has nothing to
    /// inherit, and a lit button would be a promise it cannot keep.
    #[test]
    fn a_row_with_no_directory_launches_nothing() {
        let pages = Pages::default();
        let sessions = [session("a", "running", Some("1:6:1"))];
        let drawn = resolve(&sessions, &BTreeMap::new(), &pages, 0);
        assert_eq!(row_launch(&sessions, &pages, &drawn, 6), None);
    }

    /// `>` is beside a *grid* row, so it follows the window.
    #[test]
    fn the_launch_column_follows_the_scroll() {
        let pages = Pages::default();
        // Page 2 row 1 is absolute row 9.
        let sessions = [ran("a", "running", "2:1:1", "claude", "/repo/far")];
        let drawn = resolve(&sessions, &BTreeMap::new(), &pages, 0);
        assert_eq!(row_launch(&sessions, &pages, &drawn, 1), None, "not row 1");
        let l = row_launch(&sessions, &pages, &drawn, 9).expect("row 9");
        assert_eq!(
            l.at,
            Slot {
                page: 2,
                row: 1,
                col: 2
            }
        );
    }

    #[test]
    fn a_malformed_slot_is_ignored_not_clamped() {
        let pages = Pages::default();
        for bad in ["1:9:1", "nope:1:1", "garbage"] {
            let sessions = [session("x", "running", Some(bad))];
            let f = frame(&sessions, &BTreeMap::new(), &view(&pages));
            // Only the controls. Named rather than counted, so a button
            // that starts lighting up says which one it is instead of
            // moving a number: `▼` (there is somewhere to scroll),
            // `Drums` (always lit, there is always an art to start),
            // `CC19` (kill, at rest) and the logo (green: the stack is
            // up, and it is lit whenever tallyd is painting at all).
            // Every `>` is dark because an unparseable label puts nothing
            // in any row, and the rest of the top row has nowhere to go
            // and nothing to offer.
            let lit: Vec<Button> = Button::all().filter(|&b| f.get(b) != Cell::Off).collect();
            assert_eq!(
                lit,
                vec![
                    Button::Action(8),
                    Button::Top(2),
                    Button::Top(6),
                    Button::Logo
                ],
                "{bad:?} lit a pad"
            );
        }
    }

    /// The corner wears the mode's colour, on the frame and not merely
    /// in [`crate::render::action`].
    ///
    /// It is a regression test: `frame_from` passed [`Action::Stop`] as
    /// a literal, so the renderer knew about turquoise and the surface
    /// never showed it — the latched mode was invisible, which is the
    /// one property `docs/05-interaction.md#changing-the-mode` says
    /// it must never have.
    #[test]
    fn the_corner_wears_the_mode() {
        let pages = Pages::default();
        let sessions: [Session; 0] = [];
        for mode in Action::all() {
            let mut v = view(&pages);
            v.mode = mode;
            let f = frame(&sessions, &BTreeMap::new(), &v);
            assert_eq!(
                f.get(Button::Action(8)),
                render::action(Some(mode), false),
                "{mode:?} at rest"
            );
        }
    }

    /// Held is held, and it is asked of the **button**. A double-tap
    /// completes on the release, so the verb in `pressed` can be the one
    /// that was live when the press went down rather than the one the
    /// mode names now — matching by verb is what stranded a hold and
    /// froze the grid once (`docs/05-interaction.md`).
    #[test]
    fn the_corner_is_white_while_held_in_either_mode() {
        let pages = Pages::default();
        let sessions: [Session; 0] = [];
        for mode in Action::all() {
            for down in Action::all() {
                let held: BTreeSet<Action> = [down].into_iter().collect();
                let mut v = view(&pages);
                v.mode = mode;
                v.pressed = &held;
                let f = frame(&sessions, &BTreeMap::new(), &v);
                assert_eq!(
                    f.get(Button::Action(8)),
                    Cell::Static(WHITE),
                    "{mode:?} with {down:?} down"
                );
            }
        }
    }

    /// anchor lists newest-first, so the first one seen owns the pad.
    ///
    /// **Both orderings, because the list decides and not the state.**
    /// With the corpse listed first it is the corpse that is drawn, and
    /// the live session is on no pad at all — which is not a bug here
    /// but the rule, working. Only one gesture can produce that list:
    /// a drop (`Context::put_down`) is the only path that gives an
    /// address to a session anchord already had, so it is the only one
    /// that can arrive older than what is already claiming the pad, and
    /// it clears the pad rather than asking this function to pick
    /// differently.
    #[test]
    fn two_sessions_on_one_pad_render_the_newest() {
        let pages = Pages::default();
        let mut newest = session("new", "running", Some("1:1:1"));
        newest.state = "running".to_owned();
        let mut older = session("old", "exited", Some("1:1:1"));
        older.exit_code = Some(1);
        let f = frame(&[newest, older], &BTreeMap::new(), &view(&pages));
        assert_eq!(
            f.get(Button::Grid { row: 1, col: 1 }),
            Cell::Static(GREEN),
            "newest should win"
        );

        let mut failed = session("dead", "exited", Some("1:1:1"));
        failed.exit_code = Some(1);
        let mut running = session("live", "running", Some("1:1:1"));
        running.state = "running".to_owned();
        let f = frame(&[failed, running], &BTreeMap::new(), &view(&pages));
        assert_eq!(
            f.get(Button::Grid { row: 1, col: 1 }),
            Cell::Static(RED),
            "newest still wins when the newest is the dead one"
        );
    }

    /// A clean exit must not shadow a pad you are about to press.
    #[test]
    fn a_cleanly_exited_session_frees_its_pad() {
        let pages = Pages::default();
        let mut done = session("done", "exited", Some("1:2:2"));
        done.exit_code = Some(0);
        let f = frame(&[done], &BTreeMap::new(), &view(&pages));
        assert_eq!(f.get(Button::Grid { row: 2, col: 2 }), Cell::Off);
    }

    #[test]
    fn a_failure_stays_visible() {
        let pages = Pages::default();
        let mut failed = session("bad", "exited", Some("1:2:2"));
        failed.exit_code = Some(1);
        let f = frame(&[failed], &BTreeMap::new(), &view(&pages));
        assert_ne!(f.get(Button::Grid { row: 2, col: 2 }), Cell::Off);
    }

    #[test]
    fn only_the_current_page_is_on_the_grid() {
        let pages = Pages::default();
        let sessions = [session("a", "running", Some("2:3:5"))];
        let f = frame(&sessions, &BTreeMap::new(), &view(&pages));
        assert_eq!(
            f.get(Button::Grid { row: 3, col: 5 }),
            Cell::Off,
            "page 2 is not showing"
        );
        // The top row no longer says which page you are on, or which
        // page holds work. `▲` is dark because page 1 is the first, `▼`
        // is dim because there is somewhere to go (`docs/05-interaction.md`).
        assert_eq!(f.get(Button::Top(1)), Cell::Off, "no page above page 1");
        assert_eq!(
            f.get(Button::Top(2)),
            Cell::Static(DARK_GREY),
            "and somewhere to go below it"
        );
    }

    /// The grid is a window, and it slides. Page 2 row 3 is absolute row
    /// 11, so it appears on the grid once the window has been scrolled
    /// three rows — at grid row 8, then 7, then 6.
    #[test]
    fn a_slot_slides_into_the_window_a_row_at_a_time() {
        let pages = Pages::default();
        let sessions = [session("a", "running", Some("2:3:5"))];
        for (offset, expected) in [
            (0, None),
            (2, None),
            (3, Some(8)),
            (4, Some(7)),
            (5, Some(6)),
        ] {
            let v = View {
                offset,
                ..view(&pages)
            };
            let f = frame(&sessions, &BTreeMap::new(), &v);
            let lit: Option<u8> =
                (1..=8).find(|&row| f.get(Button::Grid { row, col: 5 }) != Cell::Off);
            assert_eq!(lit, expected, "offset {offset}");
        }
    }

    /// Nothing special happens at a page boundary, because nothing special
    /// is there: page 1 row 8 and page 2 row 1 are adjacent rows.
    #[test]
    fn scrolling_crosses_a_page_boundary_without_noticing() {
        let pages = Pages::default();
        let sessions = [
            session("a", "running", Some("1:8:1")),
            session("b", "running", Some("2:1:1")),
        ];
        let v = View {
            offset: 7,
            ..view(&pages)
        };
        let f = frame(&sessions, &BTreeMap::new(), &v);
        assert_ne!(f.get(Button::Grid { row: 1, col: 1 }), Cell::Off, "1:8:1");
        assert_ne!(f.get(Button::Grid { row: 2, col: 1 }), Cell::Off, "2:1:1");
    }

    /// The renderer and the press ask the same two functions, so they
    /// cannot disagree about which pad is which.
    #[test]
    fn a_pad_and_an_address_round_trip_through_the_window() {
        let pages = Pages::default();
        for offset in [0, 1, 7, 8, slot::MAX_OFFSET] {
            let v = View {
                offset,
                ..view(&pages)
            };
            for row in 1..=8 {
                let at = View::slot_at(v.offset, row, 4).expect("inside the address space");
                assert_eq!(v.display_row(at), Some(row), "offset {offset} row {row}");
            }
        }
    }

    /// Past the end there is no pad, and nothing happens. Clamping would
    /// invent an address the finger did not land on.
    #[test]
    fn the_window_does_not_wrap_or_clamp_past_the_end() {
        assert_eq!(Slot::at_absolute(slot::ROWS + 1, 1), None);
        assert_eq!(Slot::at_absolute(0, 1), None);
        assert_eq!(Slot::at_absolute(1, 9), None);
    }

    /// `►` already moves your terminal. Moving the grid further than it
    /// has to is a second thing changing under you for no reason.
    #[test]
    fn next_scrolls_the_least_it_can_to_show_where_it_landed() {
        let at = |page, row| Slot { page, row, col: 1 };
        // Already visible: do not move at all.
        assert_eq!(View::offset_showing(at(1, 3), 0), 0);
        assert_eq!(View::offset_showing(at(2, 1), 1), 1);
        // Below the window: bring it to the bottom row, not the middle.
        assert_eq!(View::offset_showing(at(2, 2), 0), 2);
        // Above the window: bring it to the top row.
        assert_eq!(View::offset_showing(at(1, 1), 10), 0);
    }

    /// Attention scrolled off the window leaves the health light alone.
    /// It used to pulse it, and that read from the desk as the scroll
    /// button leaking into the logo (D26 in `docs/10-decisions.md`). `►` is
    /// what reports what you cannot see, and it does not say *which*
    /// page: `►` crosses pages, so you do not navigate to attention any
    /// more (`docs/05-interaction.md`).
    #[test]
    fn attention_scrolled_off_the_window_leaves_the_logo_alone() {
        let pages = Pages::default();
        let mut derived = BTreeMap::new();
        let mut d = Derived::starting_at(NOW);
        d.feed(b"ready?\x07", NOW);
        let _previous = derived.insert("a".to_owned(), d);
        let sessions = [session("a", "running", Some("4:1:1"))];
        let v = View {
            now: NOW + 5_000,
            offset: 0,
            ..view(&pages)
        };
        let f = frame(&sessions, &derived, &v);
        assert_eq!(
            f.get(Button::Logo),
            Cell::Static(GREEN),
            "health is all the logo says"
        );
    }

    /// Dark does nothing: at the top there is nothing above, at the bottom
    /// nothing below.
    #[test]
    fn the_arrows_go_dark_at_the_ends_of_the_row_space() {
        let pages = Pages::default();
        let top = frame(
            &[],
            &BTreeMap::new(),
            &View {
                offset: 0,
                ..view(&pages)
            },
        );
        assert_eq!(top.get(Button::Top(1)), Cell::Off, "nothing above row 1");
        assert_ne!(top.get(Button::Top(2)), Cell::Off, "somewhere below");

        let bottom = frame(
            &[],
            &BTreeMap::new(),
            &View {
                offset: slot::MAX_OFFSET,
                ..view(&pages)
            },
        );
        assert_ne!(bottom.get(Button::Top(1)), Cell::Off, "somewhere above");
        assert_eq!(bottom.get(Button::Top(2)), Cell::Off, "nothing below");
    }

    /// The two buttons that carry nothing stay dark, in every state. A
    /// lit button that does nothing is one you press twice.
    #[test]
    fn the_unassigned_end_of_the_top_row_is_always_dark() {
        let pages = Pages::default();
        let sessions = [session("a", "running", Some("1:3:5"))];
        let v = View {
            can_go_back: true,
            queued: 3,
            ..view(&pages)
        };
        let f = frame(&sessions, &BTreeMap::new(), &v);
        for index in 7..=crate::surface::TOP_BUTTONS {
            assert_eq!(f.get(Button::Top(index)), Cell::Off, "Top({index})");
        }
        assert_eq!(f.get(Button::Top(3)), Cell::Static(WHITE), "back is live");
        assert_eq!(f.get(Button::Top(4)), Cell::Pulse(AMBER), "next is waiting");
    }

    /// The other half of the same rule: on-window attention never
    /// touched the logo, and now neither half does.
    #[test]
    fn attention_on_the_page_you_are_looking_at_does_not_light_the_logo() {
        let pages = Pages::default();
        let mut derived = BTreeMap::new();
        let mut d = Derived::starting_at(NOW);
        d.feed(b"ready?\x07", NOW);
        let _previous = derived.insert("a".to_owned(), d);

        let sessions = [session("a", "running", Some("1:1:1"))];
        let v = View {
            now: NOW + 5_000,
            ..view(&pages)
        };
        let f = frame(&sessions, &derived, &v);
        assert_eq!(
            f.get(Button::Logo),
            Cell::Static(GREEN),
            "the stack is up; that is the whole message"
        );
    }

    /// The row that matters: an unreachable anchord lights the logo and
    /// does not clear the grid.
    #[test]
    fn an_unreachable_anchord_lights_the_logo() {
        let pages = Pages::default();
        let sessions = [session("a", "running", Some("1:3:5"))];
        let v = View {
            anchord_reachable: false,
            ..view(&pages)
        };
        let f = frame(&sessions, &BTreeMap::new(), &v);
        assert_eq!(f.get(Button::Logo), Cell::Static(DARK_RED));
        assert_eq!(
            f.get(Button::Grid { row: 3, col: 5 }),
            Cell::Static(GREEN),
            "grid is not cleared"
        );
    }

    #[test]
    fn watchers_go_to_live_slots_only() {
        let pages = Pages::default();
        let sessions = [
            session("live", "running", Some("1:1:1")),
            session("no-slot", "running", None),
            session("dead", "exited", Some("1:2:2")),
            session("bad-slot", "running", Some("nope")),
            session("closing", "closing", Some("1:3:3")),
        ];
        let ids: Vec<&str> = needs_watcher(&sessions, &pages)
            .map(|s| s.id.as_str())
            .collect();
        assert_eq!(ids, ["live", "closing"]);
    }

    /// The frame says an art is playing, and says it on the button you
    /// pressed to start one (`docs/05-interaction.md`).
    #[test]
    fn an_art_lights_the_button_that_started_it() {
        let pages = Pages::default();
        let sessions = Vec::new();
        let playing = View {
            art: true,
            ..view(&pages)
        };
        let f = frame(&sessions, &BTreeMap::new(), &playing);
        assert_eq!(f.get(Button::Top(6)), render::arts(true));
        let resting = frame(&sessions, &BTreeMap::new(), &view(&pages));
        assert_eq!(resting.get(Button::Top(6)), render::arts(false));
        assert_ne!(f.get(Button::Top(6)), resting.get(Button::Top(6)));
    }
    /// The press and the preview must read a pad the same way.
    ///
    /// `tallyd`'s `Gesture::Tap | Gesture::Chord` arm used to open-code
    /// `Slot::at_absolute(offset.saturating_add(row), col)` while the
    /// preview called `slot_at`, which used `checked_add`. They agreed
    /// only because `MAX_OFFSET + WINDOW == ROWS` makes a saturated `255`
    /// fail the bounds check anyway — one constant away from a press
    /// landing on a different pad than the one that lit up.
    ///
    /// Now there is one function. This pins that it answers for the whole
    /// window at every legal offset, including the overflow edge.
    #[test]
    fn the_press_and_the_preview_read_a_pad_the_same_way() {
        for offset in 0..=slot::MAX_OFFSET {
            for row in 1..=slot::WINDOW {
                for col in 1..=slot::COLS {
                    let at = View::slot_at(offset, row, col).expect("inside the window");
                    assert_eq!(
                        at.absolute_row(),
                        offset + row,
                        "offset {offset}, row {row}"
                    );
                }
            }
        }
        // Past the address space, and the saturating edge the old press
        // relied on: both must be None, not a clamped pad.
        assert_eq!(View::slot_at(slot::MAX_OFFSET, slot::WINDOW + 1, 1), None);
        assert_eq!(View::slot_at(u8::MAX, 1, 1), None);
        assert_eq!(View::slot_at(0, 1, slot::COLS + 1), None);
    }
}
