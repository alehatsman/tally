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
    /// `CC19` — end the session in this slot.
    Kill,
    /// `CC29` — run its spec again.
    Respawn,
    /// `CC39` — make anchord forget it.
    Purge,
    /// `CC89` — toggle `tl.mute`.
    Mute,
}

impl Action {
    /// Which action a right-column button carries, if any.
    ///
    /// `CC49`–`CC79` are unassigned and stay that way: an inert button
    /// that lights up like a live one is an invitation to press it.
    pub fn of(button: Button) -> Option<Self> {
        match button {
            Button::Action(8) => Some(Self::Kill),
            Button::Action(7) => Some(Self::Respawn),
            Button::Action(6) => Some(Self::Purge),
            Button::Action(1) => Some(Self::Mute),
            _ => None,
        }
    }

    /// The button this action lives on.
    pub fn button(self) -> Button {
        match self {
            Self::Kill => Button::Action(8),
            Self::Respawn => Button::Action(7),
            Self::Purge => Button::Action(6),
            Self::Mute => Button::Action(1),
        }
    }

    /// Every action, in top-row-to-bottom order.
    pub fn all() -> [Self; 4] {
        [Self::Mute, Self::Purge, Self::Respawn, Self::Kill]
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
}

/// What a top-row button does.
///
/// Four of the eight carry nothing and return `None`, which is the same
/// answer `Action::of` gives for `CC49`-`CC79` and for the same reason: an
/// inert button that lights up like a live one is an invitation to press
/// it (`docs/16-the-queue.md`).
///
/// None of these is a chord. `◄` and `►` commit nothing — they move you —
/// so there is nothing for a hold to protect, which is exactly why
/// `docs/13`'s *answer from the grid* needed one and this does not.
fn top(n: u8) -> Option<Gesture> {
    match n {
        1 => Some(Gesture::PageUp),
        2 => Some(Gesture::PageDown),
        3 => Some(Gesture::Back),
        4 => Some(Gesture::Next),
        _ => None,
    }
}

/// Which modifiers are down.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Chords {
    /// In press order. At most four, so a `Vec` is the right amount of
    /// machinery — a press that is already down is not pushed twice.
    held: Vec<Action>,
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
        if let Some(action) = Action::of(button) {
            if down {
                if !self.held.contains(&action) {
                    self.held.push(action);
                }
            } else {
                self.held.retain(|&a| a != action);
            }
            return None;
        }
        if !down {
            return None;
        }
        match button {
            Button::Grid { row, col } => match self.modifier() {
                Some(action) => Some(Gesture::Chord { action, row, col }),
                // Either nothing is held, or more than one is -- and an
                // ambiguous chord fires nothing. See `modifier`.
                None if self.held.is_empty() => Some(Gesture::Tap { row, col }),
                None => None,
            },
            // Switching page under a held modifier is allowed, and useful:
            // it is how you kill something that is not on the grid.
            Button::Top(n) => top(n),
            Button::Action(_) | Button::Logo => None,
        }
    }

    /// The modifier in effect, or `None`.
    ///
    /// **Two modifiers down means none.** Nothing in the hardware stops
    /// you brushing kill while holding purge, and the alternatives are
    /// both worse: picking the newest makes a fumble destructive, and
    /// picking the oldest makes it destructive and surprising.
    ///
    /// Doing nothing is safe, and — because the held preview is driven by
    /// this same function — it is *visible*. The second button down makes
    /// the preview **disappear**: the red pads you were aiming at revert
    /// to their resting colours while both buttons stay white. Confirmed
    /// on hardware, and it reads as unmistakably disarmed, because the
    /// grid you are now looking at is the one you see when nothing is
    /// held at all.
    pub fn modifier(&self) -> Option<Action> {
        match self.held.as_slice() {
            [only] => Some(*only),
            _ => None,
        }
    }

    /// Whether this button should render as held.
    pub fn is_held(&self, action: Action) -> bool {
        self.held.contains(&action)
    }

    /// Forget every held button.
    ///
    /// Called when the device goes away. A modifier that was down when the
    /// cable came out would otherwise still be down when it goes back in,
    /// and the first tap after a replug would be a kill — the stuck-chord
    /// failure M0 went looking for in the hardware and did not find, left
    /// for us to introduce in software instead.
    pub fn clear(&mut self) {
        self.held.clear();
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
            assert_eq!(Action::of(action.button()), Some(action));
        }
    }

    /// `CC49`-`CC79` do nothing, and must not pretend otherwise.
    #[test]
    fn the_middle_of_the_column_is_unassigned() {
        for row in 2..=5 {
            assert_eq!(Action::of(Button::Action(row)), None);
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
                action: Action::Kill,
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
        for button in [KILL, RESPAWN, PURGE, MUTE] {
            assert_eq!(c.feed(button, true), None);
            assert_eq!(c.feed(button, false), None);
        }
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
                    action: Action::Kill,
                    row: 1,
                    col
                })
            );
            assert_eq!(c.feed(Button::Grid { row: 1, col }, false), None);
        }
    }

    /// A fumble must not be destructive. Two modifiers down fires nothing
    /// — and the preview, which reads the same `modifier()`, shows no
    /// targets, so the dead zone is visible before you press.
    #[test]
    fn two_modifiers_down_fires_nothing() {
        let mut c = Chords::default();
        let _k = c.feed(KILL, true);
        let _p = c.feed(PURGE, true);
        assert_eq!(c.modifier(), None);
        assert_eq!(c.feed(PAD, true), None);
        assert!(c.is_held(Action::Kill) && c.is_held(Action::Purge));
    }

    /// Letting go of one of two leaves the other in effect, rather than
    /// leaving the surface inert until both are released.
    #[test]
    fn releasing_one_of_two_restores_the_other() {
        let mut c = Chords::default();
        let _k = c.feed(KILL, true);
        let _p = c.feed(PURGE, true);
        let _r = c.feed(PURGE, false);
        assert_eq!(c.modifier(), Some(Action::Kill));
        assert_eq!(
            c.feed(PAD, true),
            Some(Gesture::Chord {
                action: Action::Kill,
                row: 3,
                col: 5
            })
        );
    }

    /// The hardware sends exactly one release per press (M0), but a
    /// duplicate must not corrupt the held set either way.
    #[test]
    fn repeated_downs_and_strays_do_not_corrupt_the_held_set() {
        let mut c = Chords::default();
        let _a = c.feed(KILL, true);
        let _b = c.feed(KILL, true);
        assert_eq!(c.modifier(), Some(Action::Kill), "still exactly one");
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
        assert_eq!(c.modifier(), Some(Action::Kill), "still held");
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
        assert!(!c.is_held(Action::Kill));
        assert_eq!(c.feed(PAD, true), Some(Gesture::Tap { row: 3, col: 5 }));
    }
}
