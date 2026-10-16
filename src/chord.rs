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
    /// A top-row button was tapped.
    Page(u8),
    /// A slot has been held past [`HOLD_MS`] with no modifier down.
    /// Scroll its title until [`Gesture::Released`].
    Hold {
        /// Row 1-8, 1 is the top.
        row: u8,
        /// Col 1-8, 1 is the left.
        col: u8,
    },
    /// The held slot came back up. Put the grid back.
    Released,
}

/// How long a slot must be down before it is a hold rather than a tap.
///
/// The **only** timing in the whole surface, and it is forgiving in the
/// right direction: a slow tap scrolls a title, which is harmless, rather
/// than spawning something (`docs/05-interaction.md`).
pub const HOLD_MS: i64 = 400;

/// Which modifiers are down.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Chords {
    /// In press order. At most four, so a `Vec` is the right amount of
    /// machinery — a press that is already down is not pushed twice.
    held: Vec<Action>,
    /// The bare slot press that has not resolved yet.
    pending: Option<Pending>,
}

/// A slot that is down with no modifier, waiting to become a tap or a hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pending {
    row: u8,
    col: u8,
    /// When it went down.
    since: i64,
    /// [`Gesture::Hold`] has already been emitted for it.
    scrolling: bool,
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
    pub fn feed(&mut self, button: Button, down: bool, now: i64) -> Option<Gesture> {
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
        match button {
            Button::Grid { row, col } if down => {
                // A chord resolves at press-down and never becomes a hold:
                // a held slot under a held modifier is a kill you are
                // being slow about, not a title request (rule 3). One
                // gesture per press, decided here.
                if let Some(action) = self.modifier() {
                    self.pending = None;
                    return Some(Gesture::Chord { action, row, col });
                }
                if !self.held.is_empty() {
                    return None; // ambiguous; see `modifier`
                }
                // A bare press commits to nothing yet. Which gesture it
                // was is a question about how long you keep it down, and
                // that is not answerable until you let go or 400ms pass.
                let stale = self.pending.replace(Pending {
                    row,
                    col,
                    since: now,
                    scrolling: false,
                });
                // A second pad pressed while the first is still down ends
                // whatever the first one started.
                stale.and_then(|p| p.scrolling.then_some(Gesture::Released))
            }
            Button::Grid { row, col } => {
                let Some(p) = self.pending.take() else {
                    return None; // a release for a press that was a chord
                };
                if (p.row, p.col) != (row, col) {
                    self.pending = Some(p); // some other pad came up
                    return None;
                }
                if p.scrolling {
                    return Some(Gesture::Released);
                }
                // Short press, no modifier *at press time* -- rule 1:
                // grabbing kill after the slot went down is too late, and
                // the modifier that matters was read when it went down.
                Some(Gesture::Tap { row, col })
            }
            // Switching page under a held modifier is allowed, and useful:
            // it is how you kill something that is not on the grid.
            Button::Page(n) if down => Some(Gesture::Page(n)),
            Button::Page(_) | Button::Action(_) | Button::Logo => None,
        }
    }

    /// Call every tick. Emits [`Gesture::Hold`] once, when a bare press
    /// has been down past [`HOLD_MS`].
    ///
    /// Time has to come from outside because this module is pure, and
    /// pure is what lets a 400ms threshold be tested without waiting
    /// 400ms.
    pub fn tick(&mut self, now: i64) -> Option<Gesture> {
        let p = self.pending.as_mut()?;
        if p.scrolling || now.saturating_sub(p.since) < HOLD_MS {
            return None;
        }
        p.scrolling = true;
        Some(Gesture::Hold {
            row: p.row,
            col: p.col,
        })
    }

    /// Whether a title is currently being scrolled.
    pub fn scrolling(&self) -> bool {
        self.pending.is_some_and(|p| p.scrolling)
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
        self.pending = None;
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

    /// A fixed clock. Nothing below waits 400ms; the threshold is a
    /// number this module is handed, which is the point of it being pure.
    const T0: i64 = 1_760_000_000_000;

    /// Press and release inside the hold window.
    fn tap(c: &mut Chords, b: Button) -> Option<Gesture> {
        let pressed = c.feed(b, true, T0);
        assert_eq!(pressed, None, "a bare press commits to nothing yet");
        c.feed(b, false, T0 + 50)
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

    /// A bare tap resolves on **release**, not press-down, and that is
    /// what makes the 400ms hold forgiving in the right direction: a slow
    /// press scrolls a title instead of spawning something. Rule 2's
    /// press-down firing is about chords, which a modifier already makes
    /// deliberate.
    #[test]
    fn a_bare_tap_resolves_when_you_let_go() {
        let mut c = Chords::default();
        assert_eq!(c.feed(PAD, true, T0), None, "nothing yet");
        assert_eq!(
            c.feed(PAD, false, T0 + 50),
            Some(Gesture::Tap { row: 3, col: 5 })
        );
    }

    #[test]
    fn a_tap_under_a_held_modifier_is_a_chord() {
        let mut c = Chords::default();
        assert_eq!(c.feed(KILL, true, T0), None, "holding fires nothing");
        assert_eq!(
            c.feed(PAD, true, T0),
            Some(Gesture::Chord {
                action: Action::Kill,
                row: 3,
                col: 5
            })
        );
    }

    /// Rule 1, and it survives tap-on-release. The modifier that counts
    /// is the one down when the *slot* went down; grabbing kill while the
    /// pad is already held is too late and the press is still a tap.
    /// Order is the difference between a gesture and a race.
    #[test]
    fn grabbing_the_modifier_after_the_slot_is_too_late() {
        let mut c = Chords::default();
        assert_eq!(c.feed(PAD, true, T0), None);
        assert_eq!(c.feed(KILL, true, T0 + 10), None);
        assert_eq!(
            c.feed(PAD, false, T0 + 50),
            Some(Gesture::Tap { row: 3, col: 5 }),
            "still a tap, not a kill"
        );
        assert_eq!(c.feed(KILL, false, T0 + 60), None);
    }

    /// Rule 2. Press-down, not release — so sliding off the pad cannot
    /// silently cancel a gesture you already committed to.
    /// Rule 2, which is about **chords** specifically: a chord is
    /// deliberate by construction, so there is nothing left for a release
    /// to confirm, and firing on release would let sliding off the pad
    /// cancel it silently.
    #[test]
    fn a_chord_fires_on_press_not_release() {
        let mut c = Chords::default();
        let _held = c.feed(KILL, true, T0);
        assert!(c.feed(PAD, true, T0).is_some());
        assert_eq!(
            c.feed(PAD, false, T0 + 50),
            None,
            "the release adds nothing"
        );
    }

    /// Rule 4, and the entire reason hold-then-tap beats a prefix: you can
    /// change your mind with the hand that is already committed.
    #[test]
    fn letting_go_without_tapping_does_nothing() {
        let mut c = Chords::default();
        for button in [KILL, RESPAWN, PURGE, MUTE] {
            assert_eq!(c.feed(button, true, T0), None);
            assert_eq!(c.feed(button, false, T0), None);
        }
        assert_eq!(c.modifier(), None);
    }

    /// "The modifier is a mode for as long as it is down, not a one-shot."
    #[test]
    fn a_held_modifier_fires_on_every_slot_you_tap() {
        let mut c = Chords::default();
        let _held = c.feed(KILL, true, T0);
        for col in 1..=3 {
            assert_eq!(
                c.feed(Button::Grid { row: 1, col }, true, T0),
                Some(Gesture::Chord {
                    action: Action::Kill,
                    row: 1,
                    col
                })
            );
            assert_eq!(c.feed(Button::Grid { row: 1, col }, false, T0), None);
        }
    }

    /// A fumble must not be destructive. Two modifiers down fires nothing
    /// — and the preview, which reads the same `modifier()`, shows no
    /// targets, so the dead zone is visible before you press.
    #[test]
    fn two_modifiers_down_fires_nothing() {
        let mut c = Chords::default();
        let _k = c.feed(KILL, true, T0);
        let _p = c.feed(PURGE, true, T0);
        assert_eq!(c.modifier(), None);
        assert_eq!(c.feed(PAD, true, T0), None);
        assert_eq!(c.feed(PAD, false, T0 + 50), None, "and the release too");
        assert!(c.is_held(Action::Kill) && c.is_held(Action::Purge));
    }

    /// Letting go of one of two leaves the other in effect, rather than
    /// leaving the surface inert until both are released.
    #[test]
    fn releasing_one_of_two_restores_the_other() {
        let mut c = Chords::default();
        let _k = c.feed(KILL, true, T0);
        let _p = c.feed(PURGE, true, T0);
        let _r = c.feed(PURGE, false, T0);
        assert_eq!(c.modifier(), Some(Action::Kill));
        assert_eq!(
            c.feed(PAD, true, T0),
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
        let _a = c.feed(KILL, true, T0);
        let _b = c.feed(KILL, true, T0);
        assert_eq!(c.modifier(), Some(Action::Kill), "still exactly one");
        let _d = c.feed(KILL, false, T0);
        assert_eq!(c.modifier(), None);
        let _e = c.feed(KILL, false, T0); // a release for nothing
        assert_eq!(c.modifier(), None);
    }

    /// Switching page under a held modifier is how you reach a session
    /// that is not currently on the grid.
    #[test]
    fn a_page_button_still_works_under_a_modifier() {
        let mut c = Chords::default();
        let _held = c.feed(KILL, true, T0);
        assert_eq!(c.feed(Button::Page(4), true, T0), Some(Gesture::Page(4)));
        assert_eq!(c.modifier(), Some(Action::Kill), "still held");
    }

    #[test]
    fn the_logo_is_not_a_button_you_press() {
        let mut c = Chords::default();
        assert_eq!(tap(&mut c, Button::Logo), None);
        let _held = c.feed(KILL, true, T0);
        assert_eq!(c.feed(Button::Logo, true, T0), None);
    }

    /// Held past the threshold, the press becomes a scroll — and the
    /// release that follows puts the grid back instead of spawning.
    #[test]
    fn a_long_press_scrolls_instead_of_tapping() {
        let mut c = Chords::default();
        assert_eq!(c.feed(PAD, true, T0), None);
        assert_eq!(c.tick(T0 + HOLD_MS - 1), None, "not yet");
        assert_eq!(c.tick(T0 + HOLD_MS), Some(Gesture::Hold { row: 3, col: 5 }));
        assert!(c.scrolling());
        assert_eq!(c.tick(T0 + 5_000), None, "only once");
        assert_eq!(c.feed(PAD, false, T0 + 5_000), Some(Gesture::Released));
        assert!(!c.scrolling());
    }

    /// Rule 3. A held slot under a held modifier is a kill you are being
    /// slow about, not a title request. One gesture per press, decided at
    /// press-down by whether a modifier is down.
    #[test]
    fn the_hold_does_not_apply_while_a_modifier_is_held() {
        let mut c = Chords::default();
        let _held = c.feed(KILL, true, T0);
        assert!(c.feed(PAD, true, T0).is_some(), "fired as a chord");
        assert_eq!(c.tick(T0 + 10_000), None, "and never becomes a scroll");
        assert!(!c.scrolling());
        assert_eq!(c.feed(PAD, false, T0 + 10_000), None);
    }

    /// A release for a chord must not also arrive as a tap — otherwise
    /// every kill would be followed by a spawn on the pad you just killed.
    #[test]
    fn a_chords_release_is_not_a_second_gesture() {
        let mut c = Chords::default();
        let _held = c.feed(KILL, true, T0);
        let _chord = c.feed(PAD, true, T0);
        assert_eq!(c.feed(PAD, false, T0 + 600), None);
        assert_eq!(c.feed(KILL, false, T0 + 700), None);
    }

    /// Nothing is left armed when a second pad interrupts the first.
    #[test]
    fn a_second_pad_ends_what_the_first_one_started() {
        let mut c = Chords::default();
        let _down = c.feed(PAD, true, T0);
        let _hold = c.tick(T0 + HOLD_MS);
        let other = Button::Grid { row: 1, col: 1 };
        assert_eq!(c.feed(other, true, T0 + 500), Some(Gesture::Released));
        assert!(!c.scrolling(), "the new press is its own gesture");
        assert_eq!(
            c.feed(other, false, T0 + 540),
            Some(Gesture::Tap { row: 1, col: 1 })
        );
    }

    /// A release for a pad that is not the pending one changes nothing.
    /// The hardware sends one release per press (M0), but a surface that
    /// drops a gesture on a stray event is a surface that feels broken.
    #[test]
    fn a_release_for_some_other_pad_is_ignored() {
        let mut c = Chords::default();
        let _down = c.feed(PAD, true, T0);
        assert_eq!(
            c.feed(Button::Grid { row: 8, col: 8 }, false, T0 + 10),
            None
        );
        assert_eq!(
            c.feed(PAD, false, T0 + 50),
            Some(Gesture::Tap { row: 3, col: 5 }),
            "the real pad still resolves"
        );
    }

    /// A clock that jumps backwards (NTP, a sleep/wake) must not make a
    /// press scroll instantly, nor arm one forever.
    #[test]
    fn a_clock_that_goes_backwards_does_not_fire_a_hold() {
        let mut c = Chords::default();
        let _down = c.feed(PAD, true, T0);
        assert_eq!(c.tick(T0 - 60_000), None);
        assert_eq!(
            c.feed(PAD, false, T0 - 60_000),
            Some(Gesture::Tap { row: 3, col: 5 })
        );
    }

    /// Unplugged mid-scroll: the release never arrives, and a `Pending`
    /// left behind would make the first press after a replug a release.
    #[test]
    fn a_disconnect_clears_a_scroll_in_progress() {
        let mut c = Chords::default();
        let _down = c.feed(PAD, true, T0);
        let _hold = c.tick(T0 + HOLD_MS);
        assert!(c.scrolling());
        c.clear();
        assert!(!c.scrolling());
        assert_eq!(c.tick(T0 + 10_000), None);
    }

    /// Unplugged with kill held: the release never arrives. Without a
    /// clear, the first tap after the replug is a kill.
    #[test]
    fn a_disconnect_clears_a_stuck_modifier() {
        let mut c = Chords::default();
        let _held = c.feed(KILL, true, T0);
        c.clear();
        assert_eq!(c.modifier(), None);
        assert!(!c.is_held(Action::Kill));
        assert_eq!(tap(&mut c, PAD), Some(Gesture::Tap { row: 3, col: 5 }));
    }
}
