//! Sessions in, frame out. The only place that knows both halves.
//!
//! This is a **pure function**: `anchor list` plus derived signals plus
//! which page you are looking at, producing 81 lit cells. Nothing is
//! cached, nothing is incremental. That is what makes `tallyd` disposable —
//! it holds no state that cannot be rebuilt from a `list` and a config file
//! (`docs/08-lifecycle.md`).

use std::collections::{BTreeMap, BTreeSet};

use crate::anchor::Session;
use crate::chord::Action;
use crate::config::{Config, Launch};
use crate::render::{self, Health};
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
    /// thing you jump between (`docs/16-the-queue.md#the-top-row`).
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
    /// go empties the hand, because a pick-up you did not put down must
    /// leave the board exactly as it was
    /// (`docs/22-the-mode-button.md#hold-tap-the-source-tap-the-destination`).
    pub hand: Option<Slot>,
    /// `Session` is held, so the grid is the whole 64-row address space
    /// rather than the eight-row window onto it
    /// (`docs/20-the-overview.md`). Outranks [`Self::held`], which is
    /// `None` for the same press.
    pub overview: bool,
    /// Every action button physically down — which is **not** the same as
    /// [`Self::held`]: two buttons down lights both white and previews
    /// neither, so a fumble is visible rather than merely inert
    /// (`crate::chord::Chords::modifier`).
    pub pressed: &'a BTreeSet<Action>,
    /// Is an art playing? It lights `Drums` white, and nothing else here
    /// knows about it: the daemon paints the 64 pads itself while this is
    /// true (`docs/21-the-arts.md`).
    pub art: bool,
    /// Which verb `CC19` is carrying
    /// (`crate::chord::Chords::mode`). **The colour of that corner is
    /// the only thing that says which mode you are in**, which is what
    /// makes a latched mode honest — so it is read here rather than
    /// assumed (`docs/22-the-mode-button.md#changing-the-mode`).
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
    /// The spec name from `tl.spec`, if it was spawned with one.
    pub spec: Option<String>,
    /// The conversation it is running, from `tl.claude`.
    ///
    /// Carried here so the view can be tagged with it on a jump, which is
    /// what puts it on the dead line
    /// (`docs/26-the-claude-session-id.md`). Nothing on the grid renders
    /// it: a pad is an address, and a UUID is not something you can see
    /// in a colour.
    pub claude: Option<String>,
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
                spec: session.labels.get(crate::anchor::LABEL_SPEC).cloned(),
                claude: session.claude().map(ToOwned::to_owned),
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
/// (`docs/03-model.md#exited-rows-accumulate-and-purge-is-the-answer`).
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

/// What a held gesture would do to one pad.
///
/// **This is read twice**: once to colour the pad while the modifier is
/// held, once to decide what the tap actually does. One function, so the
/// preview cannot promise something the press does not deliver — which is
/// the whole claim `docs/05` makes for putting a destructive action on a
/// button with no label.
///
/// One function is only half of it, and the half that is easy to see.
/// [`Pads`] is the other half: one function handed two differently-built
/// [`Pad`]s tells the same lie as two functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// It will act on this pad, and this is what happens.
    ///
    /// The consequence rides here rather than being re-derived from the
    /// [`Action`] by whoever is asking, because the colour of a pad is
    /// the colour of *what happens to it*
    /// (`docs/05-interaction.md#the-held-preview`). One button with two
    /// consequences is the case that makes the difference load-bearing
    /// (`docs/22-the-mode-button.md`).
    Acts(Consequence),
    /// The pad already in your hand. Lit, and a tap on it does nothing —
    /// putting a pad back where it came from is not a move, and the
    /// write that would express it is the one anchord already has.
    Holds,
    /// A live session is here and the pad in your hand cannot land on
    /// it. A tap does nothing, exactly as [`Skips`](Effect::Skips) — but
    /// it is a separate variant because it is the only pad under a hold
    /// that is *dim by consequence and bright by content*. With a hand
    /// full, the question the grid answers flips from "what can I
    /// touch" to "where is there room", and occupancy is the half of
    /// that answer that has to stay readable
    /// (`docs/22-the-mode-button.md#the-colours`).
    Occupied,
    /// Something is here and this gesture does not apply to it. Dim.
    Skips,
    /// Nothing here for this gesture. Dark.
    None,
}

/// What acting on a pad does to it.
///
/// Two, and they can never apply to the same pad: `Pad::alive` is
/// working, quiet or asking, and `Pad::purgeable` is exited or lost.
/// That disjointness is what lets one button carry both — every pad
/// under the hold has exactly one outcome and exactly one colour
/// (`docs/22-the-mode-button.md#stop-does-kill-and-purge`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consequence {
    /// The session drawn here is terminated.
    Ends,
    /// Every ended row behind this pad is forgotten — including the ones
    /// nothing draws.
    Forgotten,
    /// This session comes into your hand. Nothing is written yet — a
    /// pick-up that is never put down must leave anchord untouched.
    PickedUp,
    /// The session in your hand lands here: one `tl.slot` write, and the
    /// agent never learns it happened.
    Dropped,
}

/// One pad, as a gesture sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pad {
    /// What is rendered there, or `None` for a dark pad.
    pub state: Option<SlotState>,
    /// It carries `tl.mute`.
    pub muted: bool,
    /// Something can be spawned here, and it **resolves**: a `[[slot]]` in
    /// `slots.toml`, or a `tl.spec` label naming a `[[spec]]` that is
    /// still in the file. A label alone is not enough — rename the
    /// `[[spec]]` and the name on the dead session points at nothing, so
    /// a pad lit on the strength of the label would promise a respawn
    /// that [`Pads::launch`] cannot perform.
    pub spec: bool,
    /// This is the pad currently **in your hand** under a held
    /// [`Action::Move`]: picked up, not yet put down.
    ///
    /// Positional rather than a property of the session, which is why it
    /// is resolved by [`Pads`] alongside everything else a pad means. A
    /// gesture that spans two taps needs somewhere to keep the first one,
    /// and the only honest place is the same struct the second tap will
    /// be judged against (`docs/22-the-mode-button.md#hold-tap-the-source-tap-the-destination`).
    pub in_hand: bool,
    /// anchord holds at least one row for this pad that it would forget.
    /// Includes rows that are **not drawn**: a clean exit keeps its
    /// `tl.slot` and its log, and its pad is dark
    /// (`docs/03-model.md#exited-rows-accumulate-and-purge-is-the-answer`).
    pub purgeable: bool,
}

impl Pad {
    /// Alive: working, quiet, or asking. The three states a kill ends and
    /// a mute is about.
    fn alive(self) -> bool {
        matches!(
            self.state,
            Some(SlotState::Working | SlotState::Quiet | SlotState::NeedsAnswer { .. })
        )
    }
}

/// What `>` beside a grid row would spawn.
///
/// The row already knows: anchord keeps `cwd` and `command` on every row
/// it has, dead ones included, so a row that has ever held a session can
/// hold another one without anything being declared in advance. That is
/// what `slots.toml` was trying to be and could not
/// (`docs/18-the-launch-column.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowLaunch {
    /// The leftmost free pad in that row.
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
    let at = (1..=8).find_map(|col| {
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
/// (`docs/11-cli.md`).
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
/// **Arguments are not here**, for the reason `row_launch` gives: a pad
/// running `claude --resume <id>` describes one conversation, and copying
/// its argv would put a second agent on it.
#[must_use]
pub fn inherit(sessions: &[Session], pages: &Pages, at: Slot) -> Option<Inherit> {
    // Newest first, which is the order anchord lists in and the order
    // `resolve` already depends on.
    let newest = sessions
        .iter()
        .find(|s| s.slot_label().and_then(|l| slot::parse(l, pages)) == Some(at))?;
    Some(Inherit {
        command: newest.command.clone()?,
        cwd: newest.cwd.clone()?,
        login_shell: newest.login_shell,
    })
}

/// What `action` would do to `pad`.
///
/// One action now (`docs/18-the-launch-column.md`), and the function stays
/// because the *preview* is the point: a destructive button with no label
/// has to answer "what happens if I press this" before you press it, and
/// it has to answer with the same predicate the press uses.
pub fn effect(action: Action, pad: Pad, hand: Option<Slot>) -> Effect {
    match action {
        // Alive wins over purgeable, and the order is the rule: a pad can
        // be running *and* have a dozen ended rows behind it, and the one
        // you mean is the one you can see. The rows are still there
        // afterwards.
        Action::Stop if pad.alive() => Effect::Acts(Consequence::Ends),
        Action::Stop if pad.purgeable => Effect::Acts(Consequence::Forgotten),
        // Drawn, not alive, and nothing anchord would forget. Defensive:
        // every state `resolve` draws dead is also one `purgeable`
        // collects, and this arm is what keeps that an observation
        // rather than an assumption.
        Action::Stop if pad.state.is_some() => Effect::Skips,
        Action::Stop => Effect::None,

        // Move is two questions, and which one is being asked depends on
        // whether your hand is full. Nothing else about the pad changes
        // between them.
        Action::Move if pad.in_hand => Effect::Holds,
        // Hand empty: what can be picked up. A session that has ended is
        // not a thing you carry — its address is the only record of where
        // it ran, and moving that would be rewriting history rather than
        // arranging work.
        Action::Move if hand.is_none() => {
            if pad.alive() {
                Effect::Acts(Consequence::PickedUp)
            } else if pad.state.is_some() {
                Effect::Skips
            } else {
                Effect::None
            }
        }
        // Hand full: what it can land on. A pad with something alive on
        // it is not a swap — two label writes are two calls, and a
        // failure between them leaves two sessions claiming one address
        // (`docs/22-the-mode-button.md#what-is-deliberately-not-here`).
        // It stays occupancy-coloured rather than dim, because that is
        // the shape the empty space is read against.
        Action::Move if pad.alive() => Effect::Occupied,
        Action::Move => Effect::Acts(Consequence::Dropped),
    }
}

/// The grid, a config and the rows behind it — everything it takes to
/// answer "what is this pad" for any of the 512 addresses.
///
/// **The one place a [`Pad`] is built.** The preview asks it 64 times a
/// frame and [`crate::act::Context`] asks it once per press, and because
/// it is the same function and not two copies of the same four lines,
/// they cannot answer differently. They used to: the renderer counted any
/// `tl.spec` label, the press counted only one that resolved to a
/// declared `[[spec]]`, and a pad whose spec had been renamed went green
/// under a held respawn and then did nothing.
#[derive(Debug, Clone, Copy)]
pub struct Pads<'a> {
    grid: &'a BTreeMap<Slot, Resolved>,
    config: &'a Config,
    purgeable: &'a BTreeMap<Slot, Vec<String>>,
    /// The pad picked up under a held [`Action::Move`], if any.
    ///
    /// Here rather than passed to [`effect`] by each caller, for the
    /// reason the whole struct exists: one function handed two
    /// differently-built [`Pad`]s tells the same lie as two functions.
    hand: Option<Slot>,
}

impl<'a> Pads<'a> {
    /// Borrow a resolved grid, the config behind it, and the rows anchord
    /// would forget ([`purgeable`]).
    /// `hand` is the pad picked up under a held [`Action::Move`]. It is a
    /// parameter rather than a setter because a [`Pads`] built without it
    /// answers [`effect`] wrongly for every pad on the grid, and a
    /// builder you can forget to call is how the preview and the press
    /// drifted apart the first time.
    pub fn new(
        grid: &'a BTreeMap<Slot, Resolved>,
        config: &'a Config,
        purgeable: &'a BTreeMap<Slot, Vec<String>>,
        hand: Option<Slot>,
    ) -> Self {
        Self {
            grid,
            config,
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

    /// What to spawn here: the `[[slot]]` entry, or failing that the spec
    /// named by the `tl.spec` label of whatever last ran here.
    ///
    /// The label is the fallback rather than the other way round because
    /// the file is what you edited most recently and meant most
    /// deliberately. The label is how a session spawned from a shell —
    /// `anchor create --label tl.spec=claude` — stays respawnable without
    /// appearing in `slots.toml` at all.
    ///
    /// A label naming a spec the file no longer has resolves to `None`,
    /// which is the honest answer and the one the preview has to show:
    /// there is nothing left to run.
    pub fn launch(&self, at: Slot) -> Option<&'a Launch> {
        self.config.launch(at).or_else(|| {
            let spec = self.grid.get(&at)?.spec.as_deref()?;
            self.config.by_spec_name(spec)
        })
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
            spec: self.launch(at).is_some(),
            purgeable: !self.rows(at).is_empty(),
            in_hand: self.hand == Some(at),
        }
    }
}

/// The worst state in absolute row `abs`, or `None` if the row is empty.
///
/// One pad of the overview stands for eight slots, so seven-eighths of
/// the information is thrown away and what survives is the thing most
/// worth walking over to. The order is the order of that sentence:
/// something is asking, something is producing, something is alive,
/// something failed, something is lost.
///
/// Mute needs no rule here. A muted slot never resolves to
/// [`SlotState::NeedsAnswer`] at all ([`crate::signal::slot_state`]) — it
/// falls through to the state underneath — so it cannot put amber in a
/// row, for free and for the same reason it cannot reach the queue.
#[must_use]
pub fn row_summary(resolved: &BTreeMap<Slot, Resolved>, abs: u8) -> Option<SlotState> {
    fn rank(state: SlotState) -> u8 {
        match state {
            SlotState::NeedsAnswer { .. } => 5,
            SlotState::Working => 4,
            SlotState::Quiet => 3,
            SlotState::Failed { .. } => 2,
            SlotState::Lost { .. } => 1,
            // `resolve` never keeps one, but the match must be total and
            // "nothing" must not outrank anything.
            SlotState::Empty => 0,
        }
    }
    resolved
        .values()
        .filter(|r| r.slot.absolute_row() == abs)
        .map(|r| r.state)
        .max_by_key(|s| rank(*s))
}

impl View<'_> {
    /// The absolute row an overview pad addresses: reading order, one pad
    /// per row, no compression and no scrolling.
    ///
    /// Sixty-four pads onto sixty-four rows, so every pad is a real
    /// destination and there are no dead ones.
    #[must_use]
    pub fn overview_row(row: u8, col: u8) -> u8 {
        (row - 1) * 8 + col
    }

    /// Which grid row shows `at`, or `None` if it is scrolled off.
    ///
    /// The one place the window is turned into a position, so the renderer
    /// and the press cannot disagree about which pad is which.
    #[must_use]
    pub fn display_row(&self, at: Slot) -> Option<u8> {
        at.absolute_row()
            .checked_sub(self.offset)
            .filter(|row| (1..=8).contains(row))
    }

    /// The slot under grid row `row`, column `col`, or `None` past the end
    /// of the address space.
    #[must_use]
    pub fn slot_at(&self, row: u8, col: u8) -> Option<Slot> {
        Slot::at_absolute(self.offset.checked_add(row)?, col)
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
        if abs > offset + 8 {
            return abs.saturating_sub(8);
        }
        offset
    }
}

/// Build the frame, resolving and counting purgeable rows on the way.
pub fn frame(
    sessions: &[Session],
    derived: &BTreeMap<String, Derived>,
    config: &Config,
    view: &View<'_>,
) -> Frame {
    let resolved = resolve(sessions, derived, view.pages, view.now);
    let rows = purgeable(sessions, view.pages);
    frame_from(&Pads::new(&resolved, config, &rows, view.hand), view)
}

/// Build the frame from pads already assembled, so a caller that needs
/// the states as well as the picture does not resolve twice.
pub fn frame_from(pads: &Pads<'_>, view: &View<'_>) -> Frame {
    let resolved = pads.grid();
    let mut frame = Frame::dark();
    let mut offscreen = false;

    // The overview outranks both the grid and the held preview: holding
    // `Session` replaces what the pads *address*, not what they would do
    // to what they address.
    if view.overview {
        for row in 1..=8 {
            for col in 1..=8 {
                let abs = View::overview_row(row, col);
                let in_window = abs > view.offset && abs <= view.offset + 8;
                frame.set(
                    Button::Grid { row, col },
                    render::overview(row_summary(resolved, abs), in_window),
                );
            }
        }
    }

    for r in resolved.values() {
        let Some(row) = view.display_row(r.slot) else {
            // Off the window. It still counts for the logo, which is the
            // only thing that reports what you cannot see.
            offscreen |= matches!(r.state, SlotState::NeedsAnswer { .. });
            continue;
        };
        if view.held.is_none() && !view.overview {
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
        for row in 1..=8 {
            for col in 1..=8 {
                let Some(at) = view.slot_at(row, col) else {
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

    // The top row: `▲ ▼ ◄ ► · · · ·` (`docs/16-the-queue.md`). Every
    // button here is self-describing — dark does nothing, lit does exactly
    // one thing — so there is no state to remember and no mode to be in.
    frame.set(Button::Top(1), render::arrow(view.offset > 0));
    frame.set(
        Button::Top(2),
        render::arrow(view.offset < slot::MAX_OFFSET),
    );
    frame.set(Button::Top(3), render::back(view.can_go_back));
    frame.set(Button::Top(4), render::next(view.queued > 0));
    frame.set(Button::Top(5), render::overview_button(view.overview));
    frame.set(Button::Top(6), render::arts(view.art));
    for index in 7..=crate::surface::TOP_BUTTONS {
        // Carries nothing, and stays dark. An unassigned button is better
        // than a strained one.
        frame.set(Button::Top(index), crate::surface::Cell::Off);
    }

    // The right column: seven `>` launchers and the kill modifier at the
    // bottom (`docs/18-the-launch-column.md`). The launchers go dark
    // under the overview because the rows they name are not the rows on
    // the grid, and a button that would fire into a row the eye is not
    // looking at must not look live.
    for row in 1..=7u8 {
        frame.set(
            Button::Action(row),
            render::launch(!view.overview && view.launchable.contains(&row)),
        );
    }
    frame.set(
        Button::Action(8),
        // Held is asked of the **button**, not of the verb the mode
        // names: both verbs live on `CC19`, and a double-tap flips the
        // mode on the release, so a press can go down under one verb and
        // come up under another. `pressed` holds whichever verb was live
        // when it went down (`docs/22-the-mode-button.md`).
        render::action(Some(view.mode), !view.pressed.is_empty()),
    );

    // Attention you cannot see, because it is scrolled off the window.
    frame.set(
        Button::Logo,
        render::logo(Health {
            anchord_reachable: view.anchord_reachable,
            offscreen_attention: offscreen,
        }),
    );

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
        Session {
            id: id.to_owned(),
            state: state.to_owned(),
            exit_code: None,
            lost_reason: None,
            labels,
            output_bytes: 10,
            pid: None,
            cwd: None,
            command: None,
            login_shell: false,
        }
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
            overview: false,
            pressed: &NO_BUTTONS,
            art: false,
            mode: Action::Stop,
        }
    }

    static NO_BUTTONS: std::sync::LazyLock<BTreeSet<Action>> =
        std::sync::LazyLock::new(BTreeSet::new);
    /// No `slots.toml`. These tests are about what a session paints, not
    /// about what an empty pad could spawn; the pads that need a spec
    /// build their own config.
    static NO_SPECS: std::sync::LazyLock<Config> = std::sync::LazyLock::new(Config::default);

    #[test]
    fn a_labelled_session_lights_its_pad() {
        let pages = Pages::default();
        let sessions = [session("a", "running", Some("1:3:5"))];
        let f = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &view(&pages));
        assert_eq!(f.get(Button::Grid { row: 3, col: 5 }), Cell::Static(GREEN));
    }

    /// teleport's sessions are on the same anchord and must be invisible.
    #[test]
    fn a_session_with_no_slot_label_is_not_on_the_grid() {
        let pages = Pages::default();
        let sessions = [session("teleport-owned", "running", None)];
        let f = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &view(&pages));
        assert_eq!(f, {
            let mut empty = frame(&[], &BTreeMap::new(), &NO_SPECS, &view(&pages));
            empty.set(Button::Logo, f.get(Button::Logo));
            empty
        });
    }

    fn pad(state: Option<SlotState>) -> Pad {
        Pad {
            state,
            ..Pad::default()
        }
    }

    const ALIVE: [SlotState; 3] = [
        SlotState::Working,
        SlotState::Quiet,
        SlotState::NeedsAnswer {
            source: crate::signal::Source::Derived,
        },
    ];

    #[test]
    fn stop_ends_everything_alive_and_nothing_else() {
        for state in ALIVE {
            assert_eq!(
                effect(Action::Stop, pad(Some(state)), None),
                Effect::Acts(Consequence::Ends),
                "{state:?}"
            );
        }
        // Drawn dead with nothing anchord would forget. Unreachable in
        // practice and asserted anyway: it is the arm that keeps the
        // agreement between `resolve` and `purgeable` an observation.
        for dead in [
            SlotState::Failed { code: 1 },
            SlotState::Lost { reclaimable: false },
        ] {
            assert_eq!(
                effect(Action::Stop, pad(Some(dead)), None),
                Effect::Skips,
                "{dead:?}"
            );
        }
        assert_eq!(effect(Action::Stop, pad(None), None), Effect::None);
    }

    /// The whole claim of the merged button: every pad has **one**
    /// outcome, because alive and purgeable cannot both be true of the
    /// same pad (`docs/22-the-mode-button.md`).
    #[test]
    fn a_pad_is_never_both_a_kill_and_a_purge() {
        for state in [
            None,
            Some(SlotState::Working),
            Some(SlotState::Quiet),
            Some(SlotState::Failed { code: 1 }),
            Some(SlotState::Lost { reclaimable: true }),
        ] {
            for purgeable in [false, true] {
                let p = Pad {
                    state,
                    purgeable,
                    ..Pad::default()
                };
                // Alive wins, and the rows survive it.
                let want = match (p.alive(), purgeable, state.is_some()) {
                    (true, _, _) => Effect::Acts(Consequence::Ends),
                    (false, true, _) => Effect::Acts(Consequence::Forgotten),
                    (false, false, true) => Effect::Skips,
                    (false, false, false) => Effect::None,
                };
                assert_eq!(effect(Action::Stop, p, None), want, "{state:?}/{purgeable}");
            }
        }
    }

    /// Hand empty: what the hold offers is what you can pick up. Only
    /// what is **alive** — a session that has ended is not a thing you
    /// carry, because its address is the only record of where it ran.
    #[test]
    fn an_empty_hand_offers_what_is_alive() {
        for state in ALIVE {
            assert_eq!(
                effect(Action::Move, pad(Some(state)), None),
                Effect::Acts(Consequence::PickedUp),
                "{state:?}"
            );
        }
        for dead in [
            SlotState::Failed { code: 1 },
            SlotState::Lost { reclaimable: false },
        ] {
            assert_eq!(
                effect(Action::Move, pad(Some(dead)), None),
                Effect::Skips,
                "history does not move"
            );
        }
        assert_eq!(effect(Action::Move, pad(None), None), Effect::None);
    }

    /// Hand full: the same pads answer a different question. Dark pads
    /// become the targets and live ones go dim — the exact inverse of
    /// the empty-handed grid, which is why one colour can serve both.
    #[test]
    fn a_full_hand_offers_somewhere_to_put_it() {
        let held = Slot {
            page: 1,
            row: 2,
            col: 2,
        };
        let carried = Pad {
            state: Some(SlotState::Working),
            in_hand: true,
            ..Pad::default()
        };
        assert_eq!(
            effect(Action::Move, carried, Some(held)),
            Effect::Holds,
            "the pad in your hand is lit, not dimmed"
        );
        assert_eq!(
            effect(Action::Move, pad(None), Some(held)),
            Effect::Acts(Consequence::Dropped),
            "a dark pad is where it can land"
        );
        assert_eq!(
            effect(Action::Move, pad(Some(SlotState::Working)), Some(held)),
            Effect::Occupied,
            "no swap: two writes are two calls"
        );
        assert_eq!(
            effect(
                Action::Move,
                pad(Some(SlotState::Failed { code: 1 })),
                Some(held)
            ),
            Effect::Acts(Consequence::Dropped),
            "dead history underneath is what every reused pad looks like"
        );
    }

    /// Dark grey carries two meanings — *this gesture will not touch
    /// it* under a held Stop, and *you can put it down here* with a
    /// full hand — and the only thing keeping them apart is that they
    /// can never share a frame. That is this: **with a pad in hand,
    /// every pad is Holds, Occupied or Dropped**, and `Skips` is
    /// unreachable. If an arm is ever added that breaks this, the dim
    /// level starts lying (`docs/22-the-mode-button.md#the-colours`).
    #[test]
    fn a_full_hand_never_dims_for_the_other_reason() {
        let held = Slot {
            page: 1,
            row: 2,
            col: 2,
        };
        let carried = Pad {
            state: Some(SlotState::Working),
            in_hand: true,
            ..Pad::default()
        };
        let every_pad = [
            carried,
            pad(None),
            pad(Some(SlotState::Empty)),
            pad(Some(SlotState::Working)),
            pad(Some(SlotState::Quiet)),
            pad(Some(SlotState::NeedsAnswer {
                source: crate::signal::Source::Derived,
            })),
            pad(Some(SlotState::Failed { code: 1 })),
            pad(Some(SlotState::Lost { reclaimable: false })),
            Pad {
                purgeable: true,
                ..Pad::default()
            },
        ];
        for p in every_pad {
            let e = effect(Action::Move, p, Some(held));
            assert_ne!(e, Effect::Skips, "{p:?} dims for the wrong reason");
            assert!(
                matches!(
                    e,
                    Effect::Holds | Effect::Occupied | Effect::Acts(Consequence::Dropped)
                ),
                "{p:?} -> {e:?}"
            );
        }
    }

    /// Move never ends anything and never forgets anything. The two
    /// verbs share a button and must not share a consequence.
    #[test]
    fn move_is_not_destructive_anywhere_on_the_grid() {
        for hand in [
            None,
            Some(Slot {
                page: 1,
                row: 1,
                col: 1,
            }),
        ] {
            for state in [
                None,
                Some(SlotState::Working),
                Some(SlotState::Quiet),
                Some(SlotState::Failed { code: 1 }),
                Some(SlotState::Lost { reclaimable: true }),
            ] {
                for purgeable in [false, true] {
                    let p = Pad {
                        state,
                        purgeable,
                        ..Pad::default()
                    };
                    assert!(
                        !matches!(
                            effect(Action::Move, p, hand),
                            Effect::Acts(Consequence::Ends | Consequence::Forgotten)
                        ),
                        "{state:?}/{purgeable}/{hand:?}"
                    );
                }
            }
        }
    }

    /// The pad purge exists for: a clean exit keeps its `tl.slot` and its
    /// log, draws as nothing, and is invisible until the hold reveals it.
    #[test]
    fn a_pad_that_draws_dark_can_still_be_forgotten() {
        let hidden = Pad {
            state: None,
            purgeable: true,
            ..Pad::default()
        };
        assert_eq!(
            effect(Action::Stop, hidden, None),
            Effect::Acts(Consequence::Forgotten),
            "dark, and not empty"
        );
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
        let resting = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &view(&pages));
        assert_eq!(
            resting.get(Button::Grid { row: 3, col: 5 }),
            Cell::Static(GREEN)
        );

        let held = View {
            held: Some(Action::Stop),
            ..view(&pages)
        };
        let f = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &held);
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
            let f = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &view(&pages));
            // Only the controls. Named rather than counted, so a button
            // that starts lighting up says which one it is instead of
            // moving a number: `▼` (there is somewhere to scroll),
            // `Session` (always lit, there is always an overview),
            // `Drums` (always lit, there is always an art to start) and
            // `CC19` (kill, at rest). Every `>` is dark because an
            // unparseable label puts nothing in any row, and the rest of
            // the top row has nowhere to go and nothing to offer.
            let lit: Vec<Button> = Button::all().filter(|&b| f.get(b) != Cell::Off).collect();
            assert_eq!(
                lit,
                vec![
                    Button::Action(8),
                    Button::Top(2),
                    Button::Top(5),
                    Button::Top(6)
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
    /// one property `docs/22-the-mode-button.md#changing-the-mode` says
    /// it must never have.
    #[test]
    fn the_corner_wears_the_mode() {
        let pages = Pages::default();
        let sessions: [Session; 0] = [];
        for mode in Action::all() {
            let mut v = view(&pages);
            v.mode = mode;
            let f = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &v);
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
    /// froze the grid once (`docs/22-the-mode-button.md`).
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
                let f = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &v);
                assert_eq!(
                    f.get(Button::Action(8)),
                    Cell::Static(WHITE),
                    "{mode:?} with {down:?} down"
                );
            }
        }
    }

    /// A session wearing `tl.attention`: the declared half of the signal,
    /// which needs no derived state to be true.
    fn asking(id: &str, slot: &str) -> Session {
        let mut s = session(id, "running", Some(slot));
        let _declared = s
            .labels
            .insert(crate::anchor::LABEL_ATTENTION.to_owned(), "1".to_owned());
        s
    }

    /// Reading order, one pad per row, sixty-four onto sixty-four. Every
    /// pad is a real destination and there are no dead ones.
    #[test]
    fn the_overview_maps_every_pad_to_a_row() {
        let seen: Vec<u8> = (1..=8)
            .flat_map(|r| (1..=8).map(move |c| View::overview_row(r, c)))
            .collect();
        assert_eq!(seen, (1..=slot::ROWS).collect::<Vec<u8>>());
    }

    /// One pad stands for eight slots, and what survives the compression
    /// is the thing most worth walking over to.
    #[test]
    fn a_row_summarises_to_its_loudest_slot() {
        let pages = Pages::default();
        // Row 1: a working session and one that is asking. Amber wins.
        let sessions = [
            session("quiet", "running", Some("1:1:1")),
            asking("asks", "1:1:4"),
        ];
        let resolved = resolve(&sessions, &BTreeMap::new(), &pages, NOW);
        assert!(matches!(
            row_summary(&resolved, 1),
            Some(SlotState::NeedsAnswer { .. })
        ));
        assert_eq!(row_summary(&resolved, 2), None, "an empty row says nothing");
    }

    /// Holding `Session` replaces what the pads address. The window is
    /// marked by dim grey on its empty pads, and `>` goes dark because
    /// the rows it names are not the rows on the grid.
    #[test]
    fn the_overview_replaces_the_grid() {
        let pages = Pages::default();
        // Absolute row 13 — page 2, row 5 — is overview pad (2, 5).
        let sessions = [session("far", "running", Some("2:5:5"))];
        let v = View {
            overview: true,
            ..view(&pages)
        };
        let f = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &v);

        assert_eq!(
            f.get(Button::Grid { row: 2, col: 5 }),
            Cell::Static(GREEN),
            "row 13 is alive and says so on its own pad"
        );
        // The window is rows 1-8 at offset 0: pads (1,1)..(1,8). Row 1
        // holds nothing here, so every one of them carries the marker.
        for col in 1..=8 {
            assert_eq!(
                f.get(Button::Grid { row: 1, col }),
                Cell::Static(DARK_GREY),
                "the window is marked at col {col}"
            );
        }
        assert_eq!(
            f.get(Button::Grid { row: 3, col: 1 }),
            Cell::Off,
            "row 17 is empty and outside the window"
        );
        for row in 1..=7 {
            assert_eq!(f.get(Button::Action(row)), Cell::Off, "`>` {row} is dark");
        }
        assert_eq!(f.get(Button::Top(5)), Cell::Static(WHITE), "held");
    }

    /// State outranks position: a pad inside the window that has content
    /// keeps its own colour, and the marker is only ever a fallback.
    #[test]
    fn the_window_marker_never_hides_a_row() {
        let pages = Pages::default();
        let sessions = [session("near", "running", Some("1:1:1"))];
        let v = View {
            overview: true,
            ..view(&pages)
        };
        let f = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &v);
        assert_eq!(f.get(Button::Grid { row: 1, col: 1 }), Cell::Static(GREEN));
    }

    /// Two modifiers means none, and `Session` counts as one: the grid
    /// reverts to itself rather than showing a map or a preview.
    #[test]
    fn a_disarmed_overview_shows_the_plain_grid() {
        let pages = Pages::default();
        let sessions = [session("a", "running", Some("1:1:1"))];
        let v = View {
            overview: false,
            held: None,
            hand: None,
            ..view(&pages)
        };
        let f = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &v);
        assert_eq!(f.get(Button::Grid { row: 1, col: 1 }), Cell::Static(GREEN));
        assert_eq!(f.get(Button::Top(5)), Cell::Static(DARK_GREY), "not held");
    }

    /// anchor lists newest-first, so the first one seen owns the pad.
    #[test]
    fn two_sessions_on_one_pad_render_the_newest() {
        let pages = Pages::default();
        let mut newest = session("new", "running", Some("1:1:1"));
        newest.state = "running".to_owned();
        let mut older = session("old", "exited", Some("1:1:1"));
        older.exit_code = Some(1);
        let f = frame(&[newest, older], &BTreeMap::new(), &NO_SPECS, &view(&pages));
        assert_eq!(
            f.get(Button::Grid { row: 1, col: 1 }),
            Cell::Static(GREEN),
            "newest should win"
        );
    }

    /// A clean exit must not shadow a pad you are about to press.
    #[test]
    fn a_cleanly_exited_session_frees_its_pad() {
        let pages = Pages::default();
        let mut done = session("done", "exited", Some("1:2:2"));
        done.exit_code = Some(0);
        let f = frame(&[done], &BTreeMap::new(), &NO_SPECS, &view(&pages));
        assert_eq!(f.get(Button::Grid { row: 2, col: 2 }), Cell::Off);
    }

    #[test]
    fn a_failure_stays_visible() {
        let pages = Pages::default();
        let mut failed = session("bad", "exited", Some("1:2:2"));
        failed.exit_code = Some(1);
        let f = frame(&[failed], &BTreeMap::new(), &NO_SPECS, &view(&pages));
        assert_ne!(f.get(Button::Grid { row: 2, col: 2 }), Cell::Off);
    }

    #[test]
    fn only_the_current_page_is_on_the_grid() {
        let pages = Pages::default();
        let sessions = [session("a", "running", Some("2:3:5"))];
        let f = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &view(&pages));
        assert_eq!(
            f.get(Button::Grid { row: 3, col: 5 }),
            Cell::Off,
            "page 2 is not showing"
        );
        // The top row no longer says which page you are on, or which
        // page holds work. `▲` is dark because page 1 is the first, `▼`
        // is dim because there is somewhere to go (`docs/16-the-queue.md`).
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
            let f = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &v);
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
        let f = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &v);
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
                let at = v.slot_at(row, 4).expect("inside the address space");
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

    /// Attention scrolled off the window still reaches the logo — that is
    /// the only thing reporting what you cannot see.
    #[test]
    fn attention_scrolled_off_the_window_reaches_the_logo() {
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
        let f = frame(&sessions, &derived, &NO_SPECS, &v);
        assert_eq!(f.get(Button::Logo), Cell::Pulse(AMBER));
    }

    /// Dark does nothing: at the top there is nothing above, at the bottom
    /// nothing below.
    #[test]
    fn the_arrows_go_dark_at_the_ends_of_the_row_space() {
        let pages = Pages::default();
        let top = frame(
            &[],
            &BTreeMap::new(),
            &NO_SPECS,
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
            &NO_SPECS,
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
        let f = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &v);
        for index in 7..=crate::surface::TOP_BUTTONS {
            assert_eq!(f.get(Button::Top(index)), Cell::Off, "Top({index})");
        }
        assert_eq!(f.get(Button::Top(3)), Cell::Static(WHITE), "back is live");
        assert_eq!(f.get(Button::Top(4)), Cell::Pulse(AMBER), "next is waiting");
    }

    #[test]
    fn attention_on_another_page_reaches_the_logo() {
        let pages = Pages::default();
        let mut derived = BTreeMap::new();
        let mut d = Derived::starting_at(NOW);
        d.feed(b"ready?\x07", NOW);
        let _previous = derived.insert("a".to_owned(), d);

        let sessions = [session("a", "running", Some("4:1:1"))];
        let v = View {
            now: NOW + 5_000,
            ..view(&pages)
        };
        let f = frame(&sessions, &derived, &NO_SPECS, &v);

        // The logo still reports it. The top row no longer says *which*
        // page — `►` crosses pages, so you do not navigate to attention
        // any more (`docs/16-the-queue.md`).
        assert_eq!(f.get(Button::Logo), Cell::Pulse(AMBER), "logo must say so");
    }

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
        let f = frame(&sessions, &derived, &NO_SPECS, &v);
        assert_eq!(
            f.get(Button::Logo),
            Cell::Off,
            "you are already looking at it"
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
        let f = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &v);
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
    /// pressed to start one (`docs/21-the-arts.md`).
    #[test]
    fn an_art_lights_the_button_that_started_it() {
        let pages = Pages::default();
        let sessions = Vec::new();
        let playing = View {
            art: true,
            ..view(&pages)
        };
        let f = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &playing);
        assert_eq!(f.get(Button::Top(6)), render::arts(true));
        let resting = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &view(&pages));
        assert_eq!(resting.get(Button::Top(6)), render::arts(false));
        assert_ne!(f.get(Button::Top(6)), resting.get(Button::Top(6)));
    }
}
