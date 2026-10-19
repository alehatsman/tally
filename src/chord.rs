//! Hold-then-tap: the only modifier scheme the surface has.
//!
//! tmux's prefix is right for a keyboard — `C-t` then `x` — because a
//! keyboard has no notion of *still holding*. A grid does, and that buys
//! one property a prefix cannot have: **you can change your mind by
//! letting go** (`docs/05-interaction.md`). For a surface where one of the
//! gestures kills a running agent, that is worth more than the keystroke.
//!
//! This module is pure. It takes presses and releases and produces
//! gestures; it does not know what is in a slot, whether anything can be
//! killed, or that anchord exists. The four rules it enforces are the four
//! in `docs/05-interaction.md#the-exact-rules-of-a-chord`, and between
//! them they decide whether a fat finger kills an agent.
//!
//! **And it has no `now`.** That is the half of the purity the rest of the
//! crate kept restating, so it is stated here once: there is no clock in
//! this file, so every gesture that is *timed* — the `▲` hold behind
//! [`Gesture::Home`], the `CC19` double-tap behind [`Chords::toggle_mode`]
//! — is held by `tallyd`'s loop instead, which is what lets the four rules
//! be tested with no device and no timeline.

use crate::surface::Button;

/// What a right-column button does while it is held.
///
/// The positions come from `docs/05-interaction.md`. Note where they land:
/// `CC19` is the **bottom** of the column and `CC89` the top
/// (`crate::surface::addr`), so the three gestures that change what is
/// running sit together at the bottom and the harmless one is a hand's
/// width away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Action {
    /// `CC19` — **end what is running, forget what has ended.**
    ///
    /// Two consequences on one button, and they can never land on the
    /// same pad: alive and ended are disjoint sets, so the preview shows
    /// exactly one colour per pad and the press does exactly what it
    /// showed (`docs/05-interaction.md#stop-does-kill-and-purge`).
    ///
    /// It is called *Stop* rather than *Kill* because killing is only
    /// half of what it does, and because that is the word the silkscreen
    /// puts under the corner it lives in.
    Stop,
    /// `CC19` in its other mode — **pick a pad up and put it down.**
    ///
    /// One `tl.slot` write; the agent never learns it happened
    /// (`docs/05-interaction.md#move-is-the-missing-verb`).
    ///
    /// Two taps inside one hold, which needs no new grammar: this module
    /// already emits one [`Gesture::Chord`] per tap and the pairing lives
    /// above it. Letting go with something still in hand changes nothing,
    /// which is rule 4 doing the job it was put there for.
    Move,
}

impl Action {
    /// Which action a right-column button carries, if any.
    ///
    /// Exactly one does. `CC89`–`CC29` became the seven `>` launchers
    /// when the column stopped being four hold-modifiers
    /// (`docs/05-interaction.md`), so the corner is the only one left.
    ///
    /// **Takes the mode**, because the corner carries two verbs now and
    /// which one is live is surface state, not a property of the button
    /// (`docs/05-interaction.md#changing-the-mode`).
    pub fn of(button: Button, mode: Self) -> Option<Self> {
        match button {
            Button::Action(8) => Some(mode),
            // `Action(1)`-`Action(7)` are `>`, and they are **taps**, not
            // modifiers: they destroy nothing, so there is nothing for a
            // hold to protect (`docs/05-interaction.md`).
            _ => None,
        }
    }

    /// The button this action lives on.
    pub fn button(self) -> Button {
        match self {
            // Both modes, one button. That is the whole design: there
            // were no spare buttons left (`docs/05-interaction.md`).
            Self::Stop | Self::Move => Button::Action(8),
        }
    }

    /// Every action, in top-to-bottom order.
    pub fn all() -> [Self; 2] {
        [Self::Stop, Self::Move]
    }

    /// The other mode. A toggle rather than a cycle: two fit, a third
    /// would need you to remember the order
    /// (`docs/05-interaction.md#what-the-corner-deliberately-does-not-carry`).
    #[must_use]
    pub fn toggled(self) -> Self {
        match self {
            Self::Stop => Self::Move,
            Self::Move => Self::Stop,
        }
    }
}

/// What a press resolved to.
///
/// Deliberately says nothing about what is *in* a slot: whether a tap is a
/// spawn or a jump depends on the session list, which is the caller's to
/// read. Mixing the two would put a model lookup inside a state machine
/// whose whole value is being testable without one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
    /// A slot was tapped with no modifier down. Spawn or jump.
    Tap {
        /// Row 1-8, 1 is the top.
        row: u8,
        /// Col 1-8, 1 is the left.
        col: u8,
    },
    /// A slot was tapped under exactly one held modifier.
    Chord {
        /// Which modifier was down.
        action: Action,
        /// Row 1-8, 1 is the top.
        row: u8,
        /// Col 1-8, 1 is the left.
        col: u8,
    },
    /// Go to the previous page (`▲`).
    PageUp,
    /// Go to the next page (`▼`).
    PageDown,
    /// Put me back where I was standing (`◄`).
    Back,
    /// Take me to the longest-waiting amber (`►`).
    Next,
    /// Scroll back to row 1 (`⌂`).
    ///
    /// **This module never emits it**: `▲` held past a threshold needs a
    /// clock, so `tallyd` produces it ([module docs](self)).
    Home,
    /// Move to the next art, or back to the grid (`Drums`).
    ///
    /// The only button on this surface that does nothing useful, which is
    /// the whole argument for it being there (`docs/05-interaction.md`).
    Arts,
    /// `>` beside a grid row: spawn into it (`docs/05-interaction.md`).
    Launch {
        /// Grid row 1-7, 1 is the top. There is no `>` beside row 8.
        row: u8,
    },
}

/// What a top-row button does.
///
/// Two of the eight carry nothing and return `None`, which is the same
/// answer [`Action::of`] gives for `CC49`-`CC79` and for the same reason: an
/// inert button that lights up like a live one is an invitation to press
/// it (`docs/05-interaction.md`).
///
/// None of these is a chord: navigation commits nothing, so there is
/// nothing for a hold to protect. That is how it won these buttons over
/// *answer from the grid*, which would have needed one.
fn top(n: u8) -> Option<Gesture> {
    match n {
        1 => Some(Gesture::PageUp),
        2 => Some(Gesture::PageDown),
        3 => Some(Gesture::Back),
        4 => Some(Gesture::Next),
        // 5 is `Session`: it held the overview until #41 cut it, and is
        // left carrying nothing rather than given a verb to justify
        // itself.
        6 => Some(Gesture::Arts),
        _ => None,
    }
}

/// What the right column is doing: the modifier that is down, and the
/// verb the corner is carrying.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chords {
    /// The modifier under the thumb, if any.
    ///
    /// **One, not a set**, because [`Action::of`] leaves exactly one
    /// modifier on the surface. If a second is ever restored, the rule
    /// that goes with it — *two modifiers down fires nothing*,
    /// `docs/05-interaction.md` — has to be rebuilt here: what this shape
    /// gives you instead is newest-wins, and newest-wins makes a fumble
    /// destructive.
    held: Option<Action>,
    /// Which verb `CC19` is carrying.
    ///
    /// Surface state, so it lives with the other surface state and not in
    /// the model. It **latches**, which is allowed here for a reason the
    /// ROADMAP's rejection of latched modes does not reach: nothing is
    /// invisible. The state is the colour of the button that set it, and
    /// the grid does not change until you hold — at which point it
    /// repaints entirely in the mode's colour under your thumb, before
    /// anything can fire (`docs/05-interaction.md#changing-the-mode`).
    mode: Action,
}

impl Default for Chords {
    fn default() -> Self {
        Self {
            held: None,
            // Stop is what the corner carried before `Move` existed.
            mode: Action::Stop,
        }
    }
}

impl Chords {
    /// Feed one button transition. Returns the gesture it completed, if any.
    ///
    /// Rules 1, 2 and 4 are all consequences of this shape rather than
    /// code that checks for them:
    ///
    /// - **The modifier must be down before the slot** (1), because the
    ///   modifier is read at the instant the slot goes down and there is
    ///   nothing to revisit afterwards.
    /// - **The action fires on press-down** (2). A chord is deliberate by
    ///   construction — you are holding something with your other hand —
    ///   so there is nothing left for a release to confirm, and firing on
    ///   release would let sliding off the pad cancel it silently.
    /// - **Releasing a modifier without tapping is a no-op** (4), because
    ///   releases emit nothing at all. That is the escape hatch that
    ///   justifies hold-then-tap over a prefix.
    pub fn feed(&mut self, button: Button, down: bool) -> Option<Gesture> {
        if let Some(action) = Action::of(button, self.mode) {
            // A release lets go of the **button**, not of the verb. The
            // mode can change between the press and the release — a
            // double-tap toggles it on the release itself — and a release
            // that cleared only the verb the mode names right now
            // stranded the old one in `held` forever. That left
            // [`Self::modifier`] answering `Some` with nothing down, so
            // the grid froze in a preview with no way out
            // (`docs/05-interaction.md`). Clearing the field outright is
            // that fix by construction rather than by a predicate.
            self.held = down.then_some(action);
            return None;
        }
        if !down {
            return None;
        }
        match button {
            Button::Grid { row, col } => match self.held {
                Some(action) => Some(Gesture::Chord { action, row, col }),
                None => Some(Gesture::Tap { row, col }),
            },
            // Switching page under a held modifier is allowed, and useful:
            // it is how you kill something that is not on the grid.
            Button::Top(n) => top(n),
            // `>` fires on press-down like everything else here. It is a
            // tap and not a chord, so a held modifier owns the surface and
            // suppresses it: you are mid-kill, not mid-launch.
            Button::Action(row) if row <= 7 => {
                self.held.is_none().then_some(Gesture::Launch { row })
            }
            Button::Action(_) | Button::Logo => None,
        }
    }

    /// The modifier in effect, or `None`. Drives the held preview.
    pub fn modifier(&self) -> Option<Action> {
        self.held
    }

    /// Whether this button should render as held.
    pub fn is_held(&self, action: Action) -> bool {
        self.held == Some(action)
    }

    /// Which verb the corner is carrying.
    pub fn mode(&self) -> Action {
        self.mode
    }

    /// Flip the corner to its other verb.
    ///
    /// **Called by `tallyd`, never from here**: a double-tap is a
    /// threshold, and thresholds need a clock ([module docs](self)).
    pub fn toggle_mode(&mut self) {
        self.mode = self.mode.toggled();
    }

    /// Forget every held button.
    ///
    /// Called when the device goes away. A modifier that was down when the
    /// cable came out would otherwise still be down when it goes back in,
    /// and the first tap after a replug would be a kill — the stuck-chord
    /// failure M0 went looking for in the hardware and did not find, left
    /// for us to introduce in software instead.
    pub fn clear(&mut self) {
        self.held = None;
        // The mode deliberately survives: it is not a button that can be
        // stuck down, it is what the corner means, and a replug that
        // silently moved it back to Stop would be a surface lying about
        // itself.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KILL: Button = Button::Action(8);
    const RESPAWN: Button = Button::Action(7);
    const PURGE: Button = Button::Action(6);
    const MUTE: Button = Button::Action(1);
    const PAD: Button = Button::Grid { row: 3, col: 5 };

    fn tap(c: &mut Chords, b: Button) -> Option<Gesture> {
        let fired = c.feed(b, true);
        assert_eq!(c.feed(b, false), None, "a release never fires");
        fired
    }

    /// The addresses in `docs/05` are CC numbers, and `CC19` is the
    /// *bottom* of the right column. Worth a test because reading the
    /// table top-down is the natural mistake.
    #[test]
    fn the_cc_numbers_map_where_the_manual_says() {
        assert_eq!(KILL.index(), 19);
        assert_eq!(RESPAWN.index(), 29);
        assert_eq!(PURGE.index(), 39);
        assert_eq!(MUTE.index(), 89);
        for action in Action::all() {
            assert_eq!(Action::of(action.button(), action), Some(action));
        }
    }

    /// `Session` is inert, not repurposed: it emits nothing on press or
    /// release, and changes nothing about what the pads around it mean.
    #[test]
    fn session_carries_nothing() {
        let mut c = Chords::default();
        assert_eq!(
            c.feed(Button::Top(5), true),
            None,
            "the press fires nothing"
        );
        assert_eq!(c.feed(Button::Top(5), false), None, "and the release too");
        assert_eq!(
            c.feed(PAD, true),
            Some(Gesture::Tap { row: 3, col: 5 }),
            "and a pad is still a plain tap"
        );
        assert_eq!(
            c.feed(Button::Action(3), true),
            Some(Gesture::Launch { row: 3 }),
            "and a launcher still launches"
        );
    }

    /// `⌂` is the one gesture this module does not produce: it needs a clock.
    #[test]
    fn home_is_never_emitted_here() {
        let mut c = Chords::default();
        for n in 1..=crate::surface::TOP_BUTTONS {
            for down in [true, false] {
                assert_ne!(c.feed(Button::Top(n), down), Some(Gesture::Home));
            }
        }
    }

    /// `CC49`-`CC79` do nothing, and must not pretend otherwise.
    #[test]
    fn the_middle_of_the_column_is_unassigned() {
        for row in 2..=5 {
            assert_eq!(Action::of(Button::Action(row), Action::Stop), None);
        }
    }

    #[test]
    fn a_bare_tap_is_a_tap() {
        let mut c = Chords::default();
        assert_eq!(tap(&mut c, PAD), Some(Gesture::Tap { row: 3, col: 5 }));
    }

    #[test]
    fn a_tap_under_a_held_modifier_is_a_chord() {
        let mut c = Chords::default();
        assert_eq!(c.feed(KILL, true), None, "holding fires nothing");
        assert_eq!(
            c.feed(PAD, true),
            Some(Gesture::Chord {
                action: Action::Stop,
                row: 3,
                col: 5
            })
        );
    }

    /// Rule 1. Tapping a slot and *then* grabbing kill does nothing: the
    /// slot press already resolved as a tap. Order is the difference
    /// between a gesture and a race.
    #[test]
    fn grabbing_the_modifier_after_the_slot_is_too_late() {
        let mut c = Chords::default();
        assert_eq!(c.feed(PAD, true), Some(Gesture::Tap { row: 3, col: 5 }));
        assert_eq!(c.feed(KILL, true), None);
        assert_eq!(c.feed(PAD, false), None);
        assert_eq!(c.feed(KILL, false), None);
    }

    /// Rule 2. Press-down, not release — so sliding off the pad cannot
    /// silently cancel a gesture you already committed to.
    #[test]
    fn the_chord_fires_on_press_not_release() {
        let mut c = Chords::default();
        let _held = c.feed(KILL, true);
        assert!(c.feed(PAD, true).is_some());
        assert_eq!(c.feed(PAD, false), None, "the release adds nothing");
    }

    /// Rule 4, and the entire reason hold-then-tap beats a prefix: you can
    /// change your mind with the hand that is already committed.
    #[test]
    fn letting_go_without_tapping_does_nothing() {
        let mut c = Chords::default();
        assert_eq!(c.feed(KILL, true), None);
        assert_eq!(c.feed(KILL, false), None);
        assert_eq!(c.modifier(), None);
    }

    /// "The modifier is a mode for as long as it is down, not a one-shot."
    #[test]
    fn a_held_modifier_fires_on_every_slot_you_tap() {
        let mut c = Chords::default();
        let _held = c.feed(KILL, true);
        for col in 1..=3 {
            assert_eq!(
                c.feed(Button::Grid { row: 1, col }, true),
                Some(Gesture::Chord {
                    action: Action::Stop,
                    row: 1,
                    col
                })
            );
            assert_eq!(c.feed(Button::Grid { row: 1, col }, false), None);
        }
    }

    /// The hardware sends exactly one release per press (M0), but a
    /// duplicate must not corrupt the held set either way.
    #[test]
    fn repeated_downs_and_strays_do_not_corrupt_the_held_set() {
        let mut c = Chords::default();
        let _a = c.feed(KILL, true);
        let _b = c.feed(KILL, true);
        assert_eq!(c.modifier(), Some(Action::Stop), "still exactly one");
        let _d = c.feed(KILL, false);
        assert_eq!(c.modifier(), None);
        let _e = c.feed(KILL, false); // a release for nothing
        assert_eq!(c.modifier(), None);
    }

    /// Switching page under a held modifier is how you reach a session
    /// that is not currently on the grid.
    #[test]
    fn a_page_button_still_works_under_a_modifier() {
        let mut c = Chords::default();
        let _held = c.feed(KILL, true);
        assert_eq!(c.feed(Button::Top(4), true), Some(Gesture::Next));
        assert_eq!(c.feed(Button::Top(6), true), Some(Gesture::Arts));
        for n in 7..=8 {
            assert_eq!(
                c.feed(Button::Top(n), true),
                None,
                "Top({n}) carries nothing"
            );
        }
        assert_eq!(c.modifier(), Some(Action::Stop), "still held");
    }

    #[test]
    fn the_logo_is_not_a_button_you_press() {
        let mut c = Chords::default();
        assert_eq!(tap(&mut c, Button::Logo), None);
        let _held = c.feed(KILL, true);
        assert_eq!(c.feed(Button::Logo, true), None);
    }

    /// Unplugged with kill held: the release never arrives. Without a
    /// clear, the first tap after the replug is a kill.
    #[test]
    fn a_disconnect_clears_a_stuck_modifier() {
        let mut c = Chords::default();
        let _held = c.feed(KILL, true);
        c.clear();
        assert_eq!(c.modifier(), None);
        assert!(!c.is_held(Action::Stop));
        assert_eq!(c.feed(PAD, true), Some(Gesture::Tap { row: 3, col: 5 }));
    }

    /// `Drums` and nothing else; the two buttons beside it stay inert.
    #[test]
    fn drums_is_the_only_art_button() {
        let mut c = Chords::default();
        assert_eq!(c.feed(Button::Top(6), true), Some(Gesture::Arts));
        for n in [7, 8] {
            assert_eq!(c.feed(Button::Top(n), true), None, "Top({n}) is not inert");
        }
        assert_eq!(
            c.feed(Button::Top(6), false),
            None,
            "a release fires nothing"
        );
    }

    /// Same button, same gesture, different word.
    #[test]
    fn the_corner_answers_with_the_mode_it_is_in() {
        for mode in Action::all() {
            assert_eq!(Action::of(Button::Action(8), mode), Some(mode), "{mode:?}");
        }
    }

    #[test]
    fn the_corner_rests_on_stop_and_toggles_to_move() {
        let mut c = Chords::default();
        assert_eq!(c.mode(), Action::Stop);
        c.toggle_mode();
        assert_eq!(c.mode(), Action::Move);
        c.toggle_mode();
        assert_eq!(c.mode(), Action::Stop, "a toggle, not a cycle");
    }

    /// A hold under `Move` emits the same [`Gesture::Chord`] a hold under
    /// `Stop` does; the pairing lives above this module ([`Action::Move`]).
    #[test]
    fn a_move_chord_is_an_ordinary_chord() {
        let mut c = Chords::default();
        c.toggle_mode();
        assert_eq!(c.feed(Button::Action(8), true), None);
        assert_eq!(
            c.feed(Button::Grid { row: 3, col: 4 }, true),
            Some(Gesture::Chord {
                action: Action::Move,
                row: 3,
                col: 4
            })
        );
        assert_eq!(
            c.feed(Button::Grid { row: 5, col: 6 }, true),
            Some(Gesture::Chord {
                action: Action::Move,
                row: 5,
                col: 6
            }),
            "and a second tap inside the same hold is another one"
        );
    }

    /// The mode is not a button that can be stuck down ([`Chords::clear`]).
    #[test]
    fn a_replug_forgets_the_hold_and_keeps_the_mode() {
        let mut c = Chords::default();
        c.toggle_mode();
        assert_eq!(c.feed(Button::Action(8), true), None);
        c.clear();
        assert!(!c.is_held(Action::Move), "the hold is gone");
        assert_eq!(c.mode(), Action::Move, "the mode is not");
    }

    /// The bug that froze the grid: a press that went down under one verb
    /// and came up under another ([`Chords::feed`]).
    #[test]
    fn toggling_the_mode_on_a_release_strands_nothing() {
        let mut c = Chords::default();
        assert_eq!(c.feed(Button::Action(8), true), None);
        // This is what `tallyd` does when the second tap completes: the
        // mode flips while the button is still physically down.
        c.toggle_mode();
        assert_eq!(c.feed(Button::Action(8), false), None);
        for a in Action::all() {
            assert!(!c.is_held(a), "{a:?} survived the release");
        }
        assert_eq!(c.modifier(), None, "nothing is held, so nothing previews");
    }

    /// And it recovers from either direction, repeatedly — the state
    /// that froze the surface was cumulative.
    #[test]
    fn the_corner_survives_a_hundred_toggles() {
        let mut c = Chords::default();
        for _ in 0..100 {
            let _down = c.feed(Button::Action(8), true);
            c.toggle_mode();
            let _up = c.feed(Button::Action(8), false);
            assert_eq!(c.modifier(), None);
        }
        assert_eq!(c.held, None, "nothing accumulated");
    }

    /// No clock in this file ([module docs](self)), so no sequence of
    /// presses can reach one.
    #[test]
    fn no_sequence_of_presses_alone_changes_the_mode() {
        let mut c = Chords::default();
        for _ in 0..8 {
            let _down = c.feed(Button::Action(8), true);
            let _up = c.feed(Button::Action(8), false);
        }
        assert_eq!(c.mode(), Action::Stop, "chord.rs never toggles it");
    }
}
