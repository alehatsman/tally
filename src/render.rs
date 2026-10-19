//! The colour language (`docs/05-interaction.md`): state in, light out.
//!
//! The constraint that decides everything here is that this must be
//! readable **from across the room, without focusing on it** — and, harder,
//! *memorable* without a legend. An earlier table used three greens
//! (working, quiet, exited-clean) and failed on the second count: you could
//! see the difference and could not recall which was which.
//!
//! So the rule is one sentence. **Green means alive**, anything not green
//! is gone, and the pad that wants you is the only amber thing on the
//! surface.
//!
//! **Nothing blinks**, and now almost nothing moves. Everything that does
//! breathes -- the device's two-beat ramp -- because this is a surface
//! somebody sits beside all day, and a hard 50% flash reads as urgent for
//! a minute and as stressful by the afternoon.
//!
//! Both of those are claims about the surface **at rest**. The one
//! exemption is the pad in your hand under a held Move, which does blink
//! for as long as your thumb is down — nothing sits beside *that* all day
//! (`docs/05-interaction.md#picking-something-up-flips-the-question-not-the-board`).
//!
//! Green used to move too: a pulse meant *producing output*, and it was
//! reported as **tiring** after a day beside it. Worse, the tempo is one
//! global clock ([`crate::tempo`]), so an unrelated pad waiting fifteen
//! minutes drove every green pulse to 140 bpm — urgency broadcast onto
//! pads with none.
//!
//! **Motion now means one thing: something wants you.**

use crate::chord::Action;
use crate::effect::{Consequence, Effect};
use crate::signal::SlotState;
use crate::surface::palette::{AMBER, DARK_GREY, DARK_RED, GREEN, OFF, RED, TURQUOISE, WHITE};
use crate::surface::Cell;

/// How one slot is lit.
pub fn slot(state: SlotState) -> Cell {
    match state {
        SlotState::Empty => Cell::Off,
        // Working and quiet are still *known* -- they drive the attention
        // signal and the queue's ranking -- they just stopped being
        // *shown*; the module header says what that bought.
        SlotState::Working | SlotState::Quiet => Cell::Static(GREEN),
        // Breathing, not blinking (module header).
        SlotState::NeedsAnswer { .. } => Cell::Pulse(AMBER),
        SlotState::Failed { .. } => Cell::Static(RED),
        // Dark red wherever the daemon had a problem — the same colour the
        // logo uses for an unreachable anchord. Static, because `lost`
        // arrives in bulk and sixty-four flashing pads is noise, not alarm.
        // Reclaimable or not, the colour is the same: the daemon still
        // cannot account for what this session did. What changes is
        // whether the pad can be claimed, which is a question for the
        // press, not for the eye (`docs/02-model.md`).
        SlotState::Lost { .. } => Cell::Static(DARK_RED),
    }
}

/// How `▲` and `▼` are lit.
///
/// Dim when you can move that way, dark at the ends. **Dark does
/// nothing** — that is the rule the whole top row is built on now, and it
/// is why these report *movement*, not *occupancy*: you navigate to an
/// empty page on purpose, to spawn into it.
pub fn arrow(movable: bool) -> Cell {
    if movable {
        Cell::Static(DARK_GREY)
    } else {
        Cell::Off
    }
}

/// How a `>` launcher is lit.
///
/// Dim when it would spawn something, dark when it would not ([`arrow`]).
/// A row with no session has no directory to inherit and a full row has
/// nowhere to put one; lit either way would be a promise it cannot keep.
pub fn launch(ready: bool) -> Cell {
    if ready {
        Cell::Static(DARK_GREY)
    } else {
        Cell::Off
    }
}

/// How `Drums` is lit: dim at rest, white while an art is playing.
///
/// It is never dark, because unlike the rest of the row it never has
/// nothing to do — there is always a reel to start ([`arrow`]). The white
/// is [`back`]'s white. Getting back out is `►`, not this button.
pub fn arts(playing: bool) -> Cell {
    if playing {
        Cell::Static(WHITE)
    } else {
        Cell::Static(DARK_GREY)
    }
}

/// How `►` is lit while an art is playing: white, always.
///
/// It is the only button in that mode that returns you to the working
/// grid, so it always does exactly one thing and is never dark
/// ([`arrow`], [`back`]).
pub fn leave() -> Cell {
    Cell::Static(WHITE)
}

/// How `◄` is lit: white when there is somewhere to go back to.
///
/// The canonical white: the same one the right column wears while a
/// modifier is held, and it says the same thing — *this is live right
/// now*.
pub fn back(available: bool) -> Cell {
    if available {
        Cell::Static(WHITE)
    } else {
        Cell::Off
    }
}

/// How `►` is lit: the reserved amber, pulsing, when something is waiting.
///
/// Amber and only amber, because amber means one thing on this surface and
/// `►` means that thing: *look at me*. It needs no colour of its own and
/// must not get one.
///
/// This row used to say which page wanted you, and the per-page amber
/// went with the page buttons: `►` crosses pages, so you no longer
/// navigate *to* attention. The number that decided it — pages used
/// 70 : 2 — is in `docs/05-interaction.md`, so it can be revisited if
/// pages ever become load-bearing.
pub fn next(waiting: bool) -> Cell {
    if waiting {
        Cell::Pulse(AMBER)
    } else {
        Cell::Off
    }
}

/// A right-column action button, coloured by the verb it is carrying.
///
/// **The colour is the mode**, and it is the only thing that says which
/// mode you are in — which is what makes a latched corner honest
/// (`docs/05-interaction.md#changing-the-mode`). Red for *Stop*, the
/// colour of what it does; turquoise for *Move*, which touches nothing
/// that is running.
///
/// White while held, unchanged: it is the only feedback that the
/// modifier is actually down, and it need not repeat the mode, because
/// holding repaints the whole grid in the mode's colour.
///
/// `CC49`-`CC79` carry no gesture and are dark ([`arrow`]).
pub fn action(button: Option<Action>, held: bool) -> Cell {
    match (button, held) {
        (None, _) => Cell::Off,
        (Some(_), true) => Cell::Static(WHITE),
        (Some(Action::Stop), false) => Cell::Static(RED),
        (Some(Action::Move), false) => Cell::Static(TURQUOISE),
    }
}

/// How a slot is lit while a modifier is held.
///
/// Three levels, and the middle one earns its keep: **dim** says "that
/// session is there and this gesture will not touch it", which is the
/// question you are actually asking. Off everywhere instead would leave
/// two thirds of the grid blank and you guessing which half of the reason
/// applied (`docs/05-interaction.md#the-held-preview`).
///
/// Each colour is the colour of the **consequence**, not of the button —
/// and it takes no [`Action`] at all, which is that sentence stated in
/// the type: one button carries two consequences, so a colour derived
/// from the button would have to be wrong about one of them.
///
/// Nothing here pulses — a preview lasts as long as your thumb does, and
/// motion on sixty pads for two seconds is noise. The pad in your hand
/// is the one exemption, and it flashes rather than pulses.
///
/// **Dim is not always the skipped level.** Under a held Stop, bright is
/// the target and dim is what will be spared; with a pad in hand it
/// inverts. The two readings cannot collide
/// ([`crate::effect::effect`]).
#[expect(
    clippy::match_same_arms,
    reason = "skipped and droppable are the same pixel for different reasons, \
              and they are kept apart so that stays an observation"
)]
pub fn preview(effect: Effect) -> Cell {
    match effect {
        Effect::None => Cell::Off,
        Effect::Skips => Cell::Static(DARK_GREY),
        // Red for what is about to be red. Dark red for what is about to
        // be forgotten — the colour `lost` already wears, on pads that
        // are already gone.
        Effect::Acts(Consequence::Ends) => Cell::Static(RED),
        Effect::Acts(Consequence::Forgotten) => Cell::Static(DARK_RED),
        // Turquoise is *a session is here*, and it means that in both
        // halves of a move: what you could pick up while your hand was
        // empty, and what is in the way once it is full. So the
        // occupied board does not repaint under you when you pick
        // something up — what changes is that the free pads light
        // instead (`docs/05-interaction.md#picking-something-up-flips-the-question-not-the-board`).
        Effect::Acts(Consequence::PickedUp) | Effect::Occupied => Cell::Static(TURQUOISE),
        // Where it can land, at the dimmest level the language has.
        // [`DARK_GREY`]'s own doc is where dim-rather-than-bright is
        // argued, and it is also the entry's canonical home.
        //
        // The same value means *this gesture will not touch it* under a
        // held Stop. The two never share a frame; [`crate::effect::effect`]
        // says why.
        Effect::Acts(Consequence::Dropped) => Cell::Static(DARK_GREY),
        // The pad in your hand: one of the occupied pads, and the one
        // you are carrying — hue says the first, motion says the second.
        //
        // **Flash, not pulse.** Both run on the one clock
        // ([`crate::tempo`]), so picking the shorter figure — one beat
        // against a pulse's two — is the only way to make this pad
        // faster than amber without dragging amber along.
        //
        // Against [`OFF`] rather than another colour: an occupied pad is
        // turquoise and a destination is a grey that never goes fully
        // out, so dark is the one gap here that cannot be misread.
        //
        // The single exemption to *nothing blinks* (module header).
        Effect::Holds => Cell::Flash {
            b: TURQUOISE,
            a: OFF,
        },
    }
}

/// The health of the stack behind the surface.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Health {
    /// `anchord` answered the last `list`.
    pub anchord_reachable: bool,
    /// Attention on a page that is not on the grid.
    pub offscreen_attention: bool,
}

/// `CC99` is not a slot — it is the status of everything behind it.
///
/// Unreachable **wins**. With no session list there is nothing that could
/// be known to want you, and a logo doing both at once would have to
/// alternate — a third animation, on the one light whose entire job is
/// being unambiguous.
pub fn logo(h: Health) -> Cell {
    if !h.anchord_reachable {
        Cell::Static(DARK_RED)
    } else if h.offscreen_attention {
        Cell::Pulse(AMBER)
    } else {
        Cell::Off
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal::Source;

    fn animated(c: Cell) -> bool {
        matches!(c, Cell::Flash { .. } | Cell::Pulse(_))
    }

    const ALL: [SlotState; 6] = [
        SlotState::Empty,
        SlotState::Working,
        SlotState::Quiet,
        SlotState::NeedsAnswer {
            source: Source::Derived,
        },
        SlotState::Failed { code: 1 },
        SlotState::Lost { reclaimable: false },
    ];

    /// At rest, one thing moves. (Under a held Move a second one does --
    /// the pad in your hand -- but that is a preview, not a slot, and
    /// `at_most_one_pad_in_a_preview_moves` is where it is pinned.)
    #[test]
    fn only_attention_moves_and_it_breathes() {
        let moving: Vec<_> = ALL.into_iter().filter(|&s| animated(slot(s))).collect();
        assert_eq!(moving.len(), 1, "{moving:?}");
        assert_eq!(slot(SlotState::Working), Cell::Static(GREEN));
        assert_eq!(
            slot(SlotState::NeedsAnswer {
                source: Source::Derived
            }),
            Cell::Pulse(AMBER)
        );
    }

    /// Nothing blinks among the things that are *there* when you are not
    /// touching the surface (module header). The pad in your hand is the
    /// one exemption and is pinned by
    /// `the_pad_in_your_hand_is_the_fastest_thing_on_the_grid` instead.
    #[test]
    fn nothing_on_the_surface_blinks() {
        let blinking: Vec<_> = ALL
            .into_iter()
            .filter(|&s| matches!(slot(s), Cell::Flash { .. }))
            .collect();
        assert!(blinking.is_empty(), "{blinking:?}");
        for control in [
            logo(Health {
                anchord_reachable: true,
                offscreen_attention: true,
            }),
            next(true),
            back(true),
            arrow(true),
            action(Some(Action::Stop), true),
            action(Some(Action::Stop), false),
        ] {
            assert!(!matches!(control, Cell::Flash { .. }), "{control:?} blinks");
        }
    }

    /// Amber is reserved. Nothing else on the surface uses it, so the one
    /// pad that wants you is the only one of its colour.
    #[test]
    fn amber_means_exactly_one_thing() {
        let amber = |c: Cell| matches!(c, Cell::Pulse(AMBER) | Cell::Static(AMBER));
        assert!(amber(slot(SlotState::NeedsAnswer {
            source: Source::Derived
        })));
        for other in ALL
            .into_iter()
            .filter(|s| !matches!(s, SlotState::NeedsAnswer { .. }))
        {
            assert!(!amber(slot(other)), "{other:?} is amber too");
        }
        assert!(!amber(action(Some(Action::Stop), false)));
        assert!(!amber(action(Some(Action::Stop), true)));
    }

    /// Green means alive, and nothing else does. The failure this guards is
    /// the one that was actually hit: a third green for "finished", which
    /// was legible and unmemorable.
    #[test]
    fn green_means_alive_and_only_alive() {
        let green = |s| matches!(slot(s), Cell::Pulse(GREEN) | Cell::Static(GREEN));
        assert!(green(SlotState::Working));
        assert!(green(SlotState::Quiet));
        for dead in [
            SlotState::Empty,
            SlotState::Failed { code: 1 },
            SlotState::Lost { reclaimable: false },
        ] {
            assert!(!green(dead), "{dead:?} is not alive and must not be green");
        }
    }

    /// A clean exit is not a colour. It is the absence of one.
    #[test]
    fn a_finished_session_leaves_no_mark() {
        assert_eq!(slot(SlotState::Empty), Cell::Off);
    }

    /// Where attention comes from changes nothing about how it looks --
    /// "nothing in the surface layer knows which one it got".
    #[test]
    fn attention_looks_the_same_from_either_source() {
        assert_eq!(
            slot(SlotState::NeedsAnswer {
                source: Source::Derived
            }),
            slot(SlotState::NeedsAnswer {
                source: Source::Declared
            })
        );
    }

    #[test]
    fn a_failure_is_red_whatever_the_code() {
        assert_eq!(slot(SlotState::Failed { code: 1 }), Cell::Static(RED));
        assert_eq!(slot(SlotState::Failed { code: 130 }), Cell::Static(RED));
    }

    /// `lost` must not read as a failure: after a reboot the whole grid is
    /// lost, and "twenty things failed" is a lie that sends you debugging.
    #[test]
    fn lost_does_not_look_like_a_failure() {
        assert_ne!(
            slot(SlotState::Lost { reclaimable: false }),
            slot(SlotState::Failed { code: 1 })
        );
        assert!(
            !animated(slot(SlotState::Lost { reclaimable: false })),
            "lost arrives in bulk; it must not flash"
        );
    }

    /// Dark red means "the daemon had a problem" wherever it appears.
    #[test]
    fn lost_slots_match_the_unreachable_logo() {
        let unreachable = logo(Health {
            anchord_reachable: false,
            offscreen_attention: false,
        });
        assert_eq!(slot(SlotState::Lost { reclaimable: false }), unreachable);
    }

    /// Every state is visually distinct from every other, with **one
    /// deliberate exception**.
    ///
    /// `Working` and `Quiet` are both plain green (module header). The
    /// exception is written here rather than removed from the invariant,
    /// so that collapsing a *second* pair still fails.
    #[test]
    fn no_two_states_share_a_cell() {
        let collapsed = [SlotState::Working, SlotState::Quiet];
        for (i, &a) in ALL.iter().enumerate() {
            for &b in &ALL[i + 1..] {
                if collapsed.contains(&a) && collapsed.contains(&b) {
                    assert_eq!(slot(a), slot(b), "alive is one colour now");
                    continue;
                }
                assert_ne!(slot(a), slot(b), "{a:?} and {b:?} look identical");
            }
        }
    }

    #[test]
    fn an_unreachable_anchord_outranks_attention_on_the_logo() {
        let both = Health {
            anchord_reachable: false,
            offscreen_attention: true,
        };
        assert_eq!(logo(both), Cell::Static(DARK_RED));
        assert!(
            !animated(logo(both)),
            "the health light must not alternate two meanings"
        );
    }

    #[test]
    fn a_healthy_quiet_stack_says_nothing() {
        assert_eq!(
            logo(Health {
                anchord_reachable: true,
                offscreen_attention: false
            }),
            Cell::Off
        );
    }

    #[test]
    fn the_logo_flashes_for_attention_you_cannot_see() {
        let h = Health {
            anchord_reachable: true,
            offscreen_attention: true,
        };
        assert!(animated(logo(h)));
    }

    /// Every top-row button is self-describing: dark does nothing, lit
    /// does exactly one thing. There is no state to remember and no mode
    /// to be in (`docs/05-interaction.md`).
    #[test]
    fn a_dark_top_row_button_does_nothing() {
        assert_eq!(arrow(false), Cell::Off);
        assert_eq!(back(false), Cell::Off);
        assert_eq!(next(false), Cell::Off);
    }

    /// `►` is the only one that moves, and it moves in the one colour
    /// this surface reserves.
    #[test]
    fn only_next_is_amber_and_only_next_animates() {
        assert!(animated(next(true)));
        assert!(!animated(back(true)));
        assert!(!animated(arrow(true)));
        assert_eq!(next(true), Cell::Pulse(AMBER));
    }

    /// Back is a control, not a slot state, so it takes a control colour.
    #[test]
    fn back_is_white_and_arrows_are_dim() {
        assert_eq!(back(true), Cell::Static(WHITE));
        assert_eq!(arrow(true), Cell::Static(DARK_GREY));
    }

    const EFFECTS: [Effect; 8] = [
        Effect::Acts(Consequence::Ends),
        Effect::Acts(Consequence::Forgotten),
        Effect::Acts(Consequence::PickedUp),
        Effect::Acts(Consequence::Dropped),
        Effect::Holds,
        Effect::Occupied,
        Effect::Skips,
        Effect::None,
    ];

    /// The one reservation the whole design rests on. A preview that
    /// borrowed amber would put a second amber thing on a surface whose
    /// promise is that there is only ever one.
    #[test]
    fn no_preview_is_ever_amber() {
        for effect in EFFECTS {
            assert_ne!(preview(effect), Cell::Static(AMBER));
            assert_ne!(preview(effect), Cell::Pulse(AMBER));
        }
    }

    /// **At most one pad in any preview moves**, and it is the one in
    /// your hand ([`preview`]).
    #[test]
    fn at_most_one_pad_in_a_preview_moves() {
        let moving: Vec<Effect> = EFFECTS
            .into_iter()
            .filter(|&e| animated(preview(e)))
            .collect();
        assert_eq!(moving, vec![Effect::Holds], "{moving:?}");
    }

    /// Three levels, and the middle one is the one that earns its keep.
    #[test]
    fn the_three_preview_levels_are_distinguishable() {
        let skips = preview(Effect::Skips);
        assert_eq!(skips, Cell::Static(DARK_GREY));
        assert_eq!(preview(Effect::None), Cell::Off);
        for c in [Consequence::Ends, Consequence::Forgotten] {
            assert_ne!(preview(Effect::Acts(c)), skips, "{c:?}");
            assert_ne!(preview(Effect::Acts(c)), Cell::Off, "{c:?}");
        }
    }

    /// One button, two consequences, and the only thing standing between
    /// them is that they do not look alike. A thumb one pad off forgets a
    /// row instead of doing nothing, so this is the assertion the merged
    /// button rests on (`docs/05-interaction.md`).
    #[test]
    fn ending_and_forgetting_do_not_look_alike() {
        assert_ne!(
            preview(Effect::Acts(Consequence::Ends)),
            preview(Effect::Acts(Consequence::Forgotten))
        );
    }

    /// The colour is the only thing that says which mode you are in
    /// ([`action`]), so the two must not look alike.
    #[test]
    fn the_corner_says_which_verb_it_is_carrying() {
        let stop = action(Some(Action::Stop), false);
        let mv = action(Some(Action::Move), false);
        assert_eq!(stop, Cell::Static(RED));
        assert_eq!(mv, Cell::Static(TURQUOISE));
        assert_ne!(stop, mv, "a latched mode that does not say which");
    }

    /// Held is held, whichever verb is loaded. The mode is not repeated
    /// under the thumb because holding repaints the whole grid in it.
    #[test]
    fn either_mode_goes_white_while_held() {
        for a in Action::all() {
            assert_eq!(action(Some(a), true), Cell::Static(WHITE), "{a:?}");
        }
    }

    /// Turquoise is *a session is here*, in both halves of a move
    /// ([`preview`]).
    #[test]
    fn turquoise_means_a_session_is_here_in_both_halves() {
        assert_eq!(
            preview(Effect::Acts(Consequence::PickedUp)),
            Cell::Static(TURQUOISE)
        );
        assert_eq!(preview(Effect::Occupied), Cell::Static(TURQUOISE));
        assert_eq!(
            preview(Effect::Holds),
            Cell::Flash {
                b: TURQUOISE,
                a: OFF
            }
        );
    }

    /// The rules it is exempt from were measured on a surface at rest,
    /// and this pad is the thing in your hand ([`preview`]).
    #[test]
    fn the_pad_in_your_hand_is_the_fastest_thing_on_the_grid() {
        let hand = preview(Effect::Holds);
        assert!(
            matches!(hand, Cell::Flash { .. }),
            "a pulse is half the rate: {hand:?}"
        );
        assert!(
            !matches!(
                slot(SlotState::NeedsAnswer {
                    source: Source::Derived
                }),
                Cell::Flash { .. }
            ),
            "and amber must not have been dragged along with it"
        );
        for e in EFFECTS {
            if e != Effect::Holds {
                assert!(!matches!(preview(e), Cell::Flash { .. }), "{e:?}");
            }
        }
    }

    /// Room is drawn at the **dimmest** level the language has, but it
    /// still has to be tellable from unlit, or "most of the grid" and
    /// "none of it" look the same.
    #[test]
    fn somewhere_to_land_is_dim_but_never_dark() {
        let lands = preview(Effect::Acts(Consequence::Dropped));
        assert_eq!(lands, Cell::Static(DARK_GREY));
        assert_ne!(lands, Cell::Off, "unlit is not an invitation");
        assert_ne!(lands, preview(Effect::Occupied));
        assert_ne!(lands, preview(Effect::Holds));
    }

    /// An unassigned button lit like a live one is an invitation to press
    /// it, and you press it twice before believing it does nothing.
    #[test]
    fn an_action_button_with_no_gesture_is_dark() {
        assert_eq!(action(None, false), Cell::Off);
        assert_eq!(action(None, true), Cell::Off);
        assert_eq!(action(Some(Action::Stop), false), Cell::Static(RED));
        assert_eq!(action(Some(Action::Stop), true), Cell::Static(WHITE));
    }

    /// Two brights side by side is what made the old page row unreadable
    /// — white for "you are here" against turquoise for "has sessions".
    /// The row no longer says where you are, but the rule survives it:
    /// **no bright button is ever competing with another.**
    #[test]
    fn at_most_one_top_row_button_is_bright() {
        let bright = |c: Cell| {
            matches!(c, Cell::Static(e) | Cell::Pulse(e)
                if e != OFF && e != DARK_GREY)
        };
        let lit = [arrow(true), arrow(true), back(true), next(true)];
        let count = lit.into_iter().filter(|&c| bright(c)).count();
        assert_eq!(count, 2, "back and next; the arrows stay dim");
        assert_ne!(back(true), next(true), "and they are not the same colour");
    }

    /// Dim is not dark: an arrow you can press has to look different from
    /// one you cannot, or the row answers only half a question.
    #[test]
    fn an_arrow_you_can_press_is_not_the_same_as_one_you_cannot() {
        assert_ne!(arrow(true), arrow(false));
    }

    /// Controls must not be mistakable for any slot state at a glance.
    ///
    /// The kill button is the one exemption, and it is deliberate: it
    /// rests on [`RED`], the colour of its own consequence and the
    /// colour a failed slot wears. It gets away with it because it is
    /// not on the grid — it is the bottom of the right column, where the
    /// silkscreen says *Stop*. Position disambiguates before hue has to.
    /// Every other control still has to be unique.
    #[test]
    fn control_colours_are_disjoint_from_slot_colours() {
        let slot_entries: Vec<Cell> = ALL.into_iter().map(slot).collect();
        for control in [Cell::Static(DARK_GREY), back(true)] {
            assert!(
                !slot_entries.contains(&control),
                "{control:?} collides with a slot colour"
            );
        }
    }

    /// `Drums` is the one top-row button that is never dark: it always
    /// has somewhere to go, which is back to the grid.
    #[test]
    fn drums_is_lit_either_way() {
        assert_eq!(arts(false), Cell::Static(DARK_GREY));
        assert_eq!(arts(true), Cell::Static(WHITE));
        assert_ne!(arts(true), arts(false), "a mode that does not say it is on");
    }
}
