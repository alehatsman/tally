//! What a held gesture would do to one pad.
//!
//! The vocabulary three modules share: [`crate::model`] builds the pads,
//! [`crate::render`] colours them, [`crate::act`] carries them out. It
//! lives below all three because it used to live in `model`, which made
//! `render` import `model` and `model` import `render` — the only cycle
//! this crate has had, and the reason `act` reached through a 1,900-line
//! module for a four-variant enum (`docs/08-daemon.md`).
//!
//! Pure, and depending on nothing but an address, a state and an action.

use crate::chord::Action;
use crate::signal::SlotState;
use crate::slot::Slot;

/// What a held gesture would do to one pad.
///
/// **This is read twice**: once to colour the pad while the modifier is
/// held, once to decide what the tap actually does. One function, so the
/// preview cannot promise something the press does not deliver — which is
/// the whole claim `docs/05` makes for putting a destructive action on a
/// button with no label.
///
/// One function is only half of it, and the half that is easy to see.
/// [`crate::model::Pads`] is the other half: one function handed two differently-built
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
    /// (`docs/05-interaction.md`).
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
    /// (`docs/05-interaction.md#picking-something-up-flips-the-question-not-the-board`).
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
/// (`docs/05-interaction.md#stop-does-kill-and-purge`).
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
    /// This is the pad currently **in your hand** under a held
    /// [`Action::Move`]: picked up, not yet put down.
    ///
    /// Positional rather than a property of the session, which is why it
    /// is resolved by [`crate::model::Pads`] alongside everything else a pad means. A
    /// gesture that spans two taps needs somewhere to keep the first one,
    /// and the only honest place is the same struct the second tap will
    /// be judged against (`docs/05-interaction.md#move-is-the-missing-verb`).
    pub in_hand: bool,
    /// anchord holds at least one row for this pad that it would forget.
    /// Includes rows that are **not drawn**: a clean exit keeps its
    /// `tl.slot` and its log, and its pad is dark
    /// (`docs/02-model.md#exited-rows-accumulate-and-purge-is-the-answer`).
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
///
/// One action now (`docs/05-interaction.md`), and the function stays
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
        // (`docs/05-interaction.md#what-the-corner-deliberately-does-not-carry`).
        // It stays occupancy-coloured rather than dim, because that is
        // the shape the empty space is read against.
        Action::Move if pad.alive() => Effect::Occupied,
        Action::Move => Effect::Acts(Consequence::Dropped),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    /// same pad (`docs/05-interaction.md`).
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
    /// level starts lying (`docs/05-interaction.md#picking-something-up-flips-the-question-not-the-board`).
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
}
