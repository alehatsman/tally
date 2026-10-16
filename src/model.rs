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
    /// Which page is on the grid, 1-8.
    pub current: u8,
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
    /// The modifier being held, if exactly one is
    /// (`crate::chord::Chords::modifier`). The grid recolours to show
    /// what it would do.
    pub held: Option<Action>,
    /// Every action button physically down — which is **not** the same as
    /// [`Self::held`]: two buttons down lights both white and previews
    /// neither, so a fumble is visible rather than merely inert
    /// (`crate::chord::Chords::modifier`).
    pub pressed: &'a BTreeSet<Action>,
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
    /// It will act on this pad.
    Acts,
    /// Mute only: `tl.mute` is set and the tap would clear it.
    Unmutes,
    /// Something is here and this gesture does not apply to it. Dim.
    Skips,
    /// Nothing here for this gesture. Dark.
    None,
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

/// What `action` would do to `pad`.
pub fn effect(action: Action, pad: Pad) -> Effect {
    let occupied = pad.state.is_some();
    let skip = if occupied {
        Effect::Skips
    } else {
        Effect::None
    };
    match action {
        Action::Kill if pad.alive() => Effect::Acts,
        // Respawn needs somewhere to put the new session and something to
        // put there. It deliberately does **not** touch `lost`: a lost
        // session may still be running and orphaned, and a second agent on
        // the same address gives you two processes and one pad to watch
        // them with. Purge first -- the purge is the act of deciding it is
        // really gone.
        Action::Respawn
            if pad.spec && matches!(pad.state, None | Some(SlotState::Failed { .. })) =>
        {
            Effect::Acts
        }
        // A dark pad can still be purgeable, and that is the point: clean
        // exits pile up behind one invisibly. Holding purge is how you see
        // them.
        Action::Purge if pad.purgeable && !pad.alive() => Effect::Acts,
        Action::Mute if pad.alive() => {
            if pad.muted {
                Effect::Unmutes
            } else {
                Effect::Acts
            }
        }
        Action::Kill | Action::Respawn | Action::Purge | Action::Mute => skip,
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
}

impl<'a> Pads<'a> {
    /// Borrow a resolved grid, the config behind it, and the rows anchord
    /// would forget ([`purgeable`]).
    pub fn new(
        grid: &'a BTreeMap<Slot, Resolved>,
        config: &'a Config,
        purgeable: &'a BTreeMap<Slot, Vec<String>>,
    ) -> Self {
        Self {
            grid,
            config,
            purgeable,
        }
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
        }
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
    frame_from(&Pads::new(&resolved, config, &rows), view)
}

/// Build the frame from pads already assembled, so a caller that needs
/// the states as well as the picture does not resolve twice.
pub fn frame_from(pads: &Pads<'_>, view: &View<'_>) -> Frame {
    let resolved = pads.grid();
    let mut frame = Frame::dark();
    let mut occupied = [false; slot::PAGES as usize];
    let mut wants_you = [false; slot::PAGES as usize];

    for r in resolved.values() {
        if let Some(page) = usize::from(r.slot.page).checked_sub(1) {
            if let Some(flag) = occupied.get_mut(page) {
                *flag = true;
            }
            if matches!(r.state, SlotState::NeedsAnswer { .. }) {
                if let Some(flag) = wants_you.get_mut(page) {
                    *flag = true;
                }
            }
        }
        if r.slot.page == view.current && view.held.is_none() {
            frame.set(r.slot.button(), render::slot(r.state));
        }
    }

    // A held modifier replaces the grid with the answer to "what happens
    // if I press this". Every pad on the page is asked, not just the lit
    // ones: a dark pad can be a spawn or a pile of clean exits waiting to
    // be forgotten, and both are things the preview exists to reveal.
    if let Some(action) = view.held {
        for row in 1..=8 {
            for col in 1..=8 {
                let at = Slot {
                    page: view.current,
                    row,
                    col,
                };
                let pad = pads.at(at);
                frame.set(at.button(), render::preview(action, effect(action, pad)));
            }
        }
    }

    // The top row: `▲ ▼ ◄ ► · · · ·` (`docs/16-the-queue.md`). Every
    // button here is self-describing — dark does nothing, lit does exactly
    // one thing — so there is no state to remember and no mode to be in.
    frame.set(Button::Top(1), render::arrow(view.current > 1));
    frame.set(Button::Top(2), render::arrow(view.current < slot::PAGES));
    frame.set(Button::Top(3), render::back(view.can_go_back));
    frame.set(Button::Top(4), render::next(view.queued > 0));
    for index in 5..=slot::PAGES {
        // Carries nothing, and stays dark. An unassigned button is better
        // than a strained one.
        frame.set(Button::Top(index), crate::surface::Cell::Off);
    }

    for row in 1..=8 {
        let button = Button::Action(row);
        let carries = Action::of(button);
        frame.set(
            button,
            render::action(carries, carries.is_some_and(|a| view.pressed.contains(&a))),
        );
    }

    // Attention you cannot see, on a page that is not on the grid.
    let offscreen = wants_you
        .iter()
        .enumerate()
        .any(|(at, &w)| w && u8::try_from(at + 1).is_ok_and(|p| p != view.current));
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
        }
    }

    /// The resting view: nothing held, nothing pressed. Tests that care
    /// about those build on top of it.
    fn view(pages: &Pages) -> View<'_> {
        View {
            pages,
            current: 1,
            anchord_reachable: true,
            now: NOW,
            can_go_back: false,
            queued: 0,
            held: None,
            pressed: &NO_BUTTONS,
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
    fn kill_acts_on_everything_alive_and_nothing_else() {
        for state in ALIVE {
            assert_eq!(
                effect(Action::Kill, pad(Some(state))),
                Effect::Acts,
                "{state:?}"
            );
        }
        for dead in [SlotState::Failed { code: 1 }, SlotState::Lost] {
            assert_eq!(
                effect(Action::Kill, pad(Some(dead))),
                Effect::Skips,
                "{dead:?}"
            );
        }
        assert_eq!(effect(Action::Kill, pad(None)), Effect::None);
    }

    /// A lost session may still be running and orphaned, so respawning it
    /// would give you two agents in one repo and one pad to watch them
    /// with. Purge is the act of deciding it is really gone.
    #[test]
    fn respawn_refuses_a_lost_pad_even_with_a_spec() {
        let with_spec = |state| Pad {
            state,
            spec: true,
            ..Pad::default()
        };
        assert_eq!(effect(Action::Respawn, with_spec(None)), Effect::Acts);
        assert_eq!(
            effect(
                Action::Respawn,
                with_spec(Some(SlotState::Failed { code: 1 }))
            ),
            Effect::Acts
        );
        assert_eq!(
            effect(Action::Respawn, with_spec(Some(SlotState::Lost))),
            Effect::Skips
        );
        for state in ALIVE {
            assert_eq!(
                effect(Action::Respawn, with_spec(Some(state))),
                Effect::Skips
            );
        }
    }

    /// Without a spec there is nothing to run, so a dark pad stays dark
    /// under respawn rather than lighting and then doing nothing.
    #[test]
    fn respawn_needs_something_to_run() {
        assert_eq!(effect(Action::Respawn, pad(None)), Effect::None);
    }

    /// The case the colour language cannot show any other way: rows pile
    /// up behind a dark pad, and holding purge is how you see them.
    #[test]
    fn purge_lights_a_dark_pad_that_has_rows_behind_it() {
        let hidden = Pad {
            state: None,
            purgeable: true,
            ..Pad::default()
        };
        assert_eq!(effect(Action::Purge, hidden), Effect::Acts);
        assert_eq!(effect(Action::Purge, pad(None)), Effect::None);
    }

    /// anchord refuses a running session with `already_exited`, so the
    /// preview must not offer it — the press would fail, visibly, for a
    /// reason the surface never hinted at.
    #[test]
    fn purge_never_offers_a_live_session() {
        for state in ALIVE {
            let live = Pad {
                state: Some(state),
                purgeable: true,
                ..Pad::default()
            };
            assert_eq!(effect(Action::Purge, live), Effect::Skips, "{state:?}");
        }
    }

    #[test]
    fn mute_toggles_and_says_which_way() {
        for state in ALIVE {
            let on = Pad {
                state: Some(state),
                ..Pad::default()
            };
            assert_eq!(effect(Action::Mute, on), Effect::Acts);
            assert_eq!(
                effect(Action::Mute, Pad { muted: true, ..on }),
                Effect::Unmutes
            );
        }
        // A dead session cannot ring, so there is nothing to silence.
        assert_eq!(
            effect(Action::Mute, pad(Some(SlotState::Failed { code: 1 }))),
            Effect::Skips
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

    /// `Unmutes` is mute's alone. Nothing else on the surface toggles,
    /// so nothing else may claim the variant that says which way.
    #[test]
    fn only_mute_ever_reports_a_toggle() {
        let every_pad = [
            None,
            Some(SlotState::Working),
            Some(SlotState::Quiet),
            Some(SlotState::Failed { code: 1 }),
            Some(SlotState::Lost),
        ];
        for action in [Action::Kill, Action::Respawn, Action::Purge] {
            for state in every_pad {
                for muted in [false, true] {
                    for spec in [false, true] {
                        for rows in [false, true] {
                            let p = Pad {
                                state,
                                muted,
                                spec,
                                purgeable: rows,
                            };
                            assert_ne!(effect(action, p), Effect::Unmutes, "{action:?} {p:?}");
                        }
                    }
                }
            }
        }
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
            held: Some(Action::Kill),
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

    /// The health light and the top row are not part of the question the
    /// preview asks, so they must not answer it.
    #[test]
    fn the_preview_leaves_the_top_row_and_the_logo_alone() {
        let pages = Pages::default();
        let sessions = [session("a", "running", Some("1:3:5"))];
        let resting = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &view(&pages));
        let held = frame(
            &sessions,
            &BTreeMap::new(),
            &NO_SPECS,
            &View {
                held: Some(Action::Purge),
                ..view(&pages)
            },
        );
        assert_eq!(resting.get(Button::Logo), held.get(Button::Logo));
        for page in 1..=slot::PAGES {
            assert_eq!(
                resting.get(Button::Top(page)),
                held.get(Button::Top(page)),
                "page {page}"
            );
        }
    }

    /// Two buttons down lights both white and previews neither. The grid
    /// reverts to its resting colours, which is what makes the fumble
    /// visible rather than merely inert: you are suddenly looking at the
    /// surface you see when nothing is held.
    #[test]
    fn two_pressed_buttons_light_without_a_preview() {
        let pages = Pages::default();
        let sessions = [session("a", "running", Some("1:3:5"))];
        let pressed: BTreeSet<Action> = [Action::Kill, Action::Purge].into_iter().collect();
        let f = frame(
            &sessions,
            &BTreeMap::new(),
            &NO_SPECS,
            &View {
                can_go_back: false,
                queued: 0,
                held: None, // what `Chords::modifier` reports for two
                pressed: &pressed,
                ..view(&pages)
            },
        );
        assert_eq!(
            f.get(Button::Grid { row: 3, col: 5 }),
            Cell::Static(GREEN),
            "the grid is untouched"
        );
        for action in [Action::Kill, Action::Purge] {
            assert_eq!(
                f.get(action.button()),
                Cell::Static(WHITE),
                "{action:?} reads as held"
            );
        }
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

    #[test]
    fn a_malformed_slot_is_ignored_not_clamped() {
        let pages = Pages::default();
        for bad in ["1:9:1", "nope:1:1", "garbage"] {
            let sessions = [session("x", "running", Some(bad))];
            let f = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &view(&pages));
            let lit = Button::all().filter(|&b| f.get(b) != Cell::Off).count();
            // Only the controls: the current page button, plus the four
            // action buttons that carry a gesture. `CC49`-`CC79` are dark.
            assert_eq!(lit, 5, "{bad:?} lit a pad");
        }
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

    /// The four buttons that carry nothing stay dark, in every state. A
    /// lit button that does nothing is one you press twice.
    #[test]
    fn the_unassigned_half_of_the_top_row_is_always_dark() {
        let pages = Pages::default();
        let sessions = [session("a", "running", Some("1:3:5"))];
        let v = View {
            can_go_back: true,
            queued: 3,
            ..view(&pages)
        };
        let f = frame(&sessions, &BTreeMap::new(), &NO_SPECS, &v);
        for index in 5..=slot::PAGES {
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
}
