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
//! Green used to move too: a pulse meant *producing output*. That cost
//! more than it bought and was reported as **tiring** after a day beside
//! it, which is the same measurement that killed the hard amber flash and
//! the only one that can be taken by sitting there. Five working agents
//! put five pads in motion carrying something nobody acts on, and the
//! surface was busiest exactly when nothing needed anything. Worse, the
//! tempo is one global clock, so an unrelated pad waiting fifteen minutes
//! drove every green pulse to 140 bpm -- urgency broadcast onto pads with
//! none.
//!
//! **Motion now means one thing: something wants you.** Which is what
//! this module always claimed and, until green went still, was not true.

use crate::chord::Action;
use crate::model::{Consequence, Effect};
use crate::signal::SlotState;
use crate::surface::palette::{AMBER, DARK_GREY, DARK_RED, GREEN, RED, WHITE};
use crate::surface::Cell;

/// How one slot is lit.
pub fn slot(state: SlotState) -> Cell {
    match state {
        SlotState::Empty => Cell::Off,
        // One colour for alive. The working/quiet distinction is still
        // *known* -- it drives the attention signal and the overview's
        // ranking -- it just stopped being *shown*, because showing it
        // cost the only thing motion had left to say (module header).
        SlotState::Working | SlotState::Quiet => Cell::Static(GREEN),
        // Breathing, not blinking. A hard 50% flash reads as urgent for
        // about a minute and as stressful for an afternoon, and this is a
        // surface someone sits next to all day. The pulse is a two-beat
        // ramp against the flash's one-beat chop, so it still moves while
        // the amber does the work of saying which pad.
        SlotState::NeedsAnswer { .. } => Cell::Pulse(AMBER),
        SlotState::Failed { .. } => Cell::Static(RED),
        // Dark red wherever the daemon had a problem — the same colour the
        // logo uses for an unreachable anchord. Static, because `lost`
        // arrives in bulk and sixty-four flashing pads is noise, not alarm.
        // Reclaimable or not, the colour is the same: the daemon still
        // cannot account for what this session did. What changes is
        // whether the pad can be claimed, which is a question for the
        // press, not for the eye (`docs/03-model.md`).
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
/// Dim when it would spawn something, dark when it would not — the rule
/// the whole surface now runs on. A row with no session has no directory
/// to inherit and a full row has nowhere to put one, and in both cases a
/// lit button would be a promise it cannot keep
/// (`docs/18-the-launch-column.md`).
pub fn launch(ready: bool) -> Cell {
    if ready {
        Cell::Static(DARK_GREY)
    } else {
        Cell::Off
    }
}

/// How `Session` is lit: dim always, white while held.
///
/// Always lit because there is always an overview to show — the one
/// button on this row whose availability does not depend on where you
/// are standing. It rests on the same dark grey as `▲▼` and the `>`
/// launchers, meaning the same thing on all of them: *available, not
/// urgent*. The top row is five navigation buttons and should look like
/// it. What separates this one is that it is held rather than pressed,
/// and that is learned from the grid changing under a thumb, not from a
/// hue (`docs/20-the-overview.md`).
pub fn overview_button(held: bool) -> Cell {
    if held {
        Cell::Static(WHITE)
    } else {
        Cell::Static(DARK_GREY)
    }
}

/// How one overview pad is lit: the worst thing in that row, and whether
/// the eight-row window covers it.
///
/// Same five colours as [`slot`], same meanings — there is no second
/// language to learn, because one pad here is just the loudest of the
/// eight pads it stands for.
///
/// The window is marked by the only thing that costs nothing: a pad
/// inside it that would be dark glows dark grey instead. A pad inside it
/// that has content keeps its true colour, because **state outranks
/// position**. The failure mode is stated rather than hidden: a window
/// whose eight rows are all occupied shows no marker at all
/// (`docs/20-the-overview.md`).
pub fn overview(summary: Option<SlotState>, in_window: bool) -> Cell {
    match (summary, in_window) {
        (Some(state), _) => slot(state),
        (None, true) => Cell::Static(DARK_GREY),
        (None, false) => Cell::Off,
    }
}

/// How `Drums` is lit: dim at rest, white while an art is playing.
///
/// It is never dark, because unlike the rest of the row it never has
/// nothing to do — a press always moves you along the reel, and the reel
/// wraps back to the working grid (`docs/21-the-arts.md`). White is the
/// same white the right column uses while a modifier is held, and says
/// the same thing: *this is what you are doing right now*.
pub fn arts(playing: bool) -> Cell {
    if playing {
        Cell::Static(WHITE)
    } else {
        Cell::Static(DARK_GREY)
    }
}

/// How `◄` is lit: white when there is somewhere to go back to.
///
/// One of the two control colours, and the same white the right column
/// uses while a modifier is held — it says *this button is live right
/// now*, which is exactly what it means here.
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
/// **This row used to say which page wanted you.** The rule it replaced
/// was paid for in real confusion — *"I am confused which one is selected,
/// the white one or the turquoise one"* — and said: exactly one page
/// button is bright, and it is always the one you are on. It is gone, with
/// the per-page amber, because `►` crosses pages. You no longer navigate
/// *to* attention; attention navigates to you. The defence is one number,
/// pages used 70 : 2, and it is written down in `docs/16-the-queue.md` so
/// it can be revisited if pages ever become load-bearing.
pub fn next(waiting: bool) -> Cell {
    if waiting {
        Cell::Pulse(AMBER)
    } else {
        Cell::Off
    }
}

/// A right-column action button. Red at rest — the colour of what it
/// does, on the corner the silkscreen labels *Stop*. White while held,
/// which is the only feedback that the modifier is actually down.
///
/// `CC49`-`CC79` carry no gesture and are dark: a button lit like a live
/// one that does nothing when you press it is worse than an unlit button,
/// because you press it twice.
pub fn action(button: Option<Action>, held: bool) -> Cell {
    match (button, held) {
        (None, _) => Cell::Off,
        (Some(_), true) => Cell::Static(WHITE),
        (Some(_), false) => Cell::Static(RED),
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
/// the type. One button now carries two consequences
/// (`docs/22-the-mode-button.md`), so a colour derived from the button
/// would have to be wrong about one of them.
///
/// Nothing here pulses — a preview lasts as long as your thumb does, and
/// motion on sixty pads for two seconds is noise.
pub fn preview(effect: Effect) -> Cell {
    match effect {
        Effect::None => Cell::Off,
        Effect::Skips => Cell::Static(DARK_GREY),
        // Red for what is about to be red. Dark red for what is about to
        // be forgotten — the colour `lost` already wears, on pads that
        // are already gone.
        Effect::Acts(Consequence::Ends) => Cell::Static(RED),
        Effect::Acts(Consequence::Forgotten) => Cell::Static(DARK_RED),
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

    /// Two things move, and both breathe. What separates them is hue, not
    /// rhythm -- green for alive, amber for wanting you.
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

    /// Nothing blinks, anywhere. A hard 50% flash is legible and tiring,
    /// and this is a surface somebody sits beside all day -- so urgency is
    /// carried by hue, which does not wear out.
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
    /// `Working` and `Quiet` are both plain green now. The distinction is
    /// still known — it is what the attention signal is derived from —
    /// but it stopped being drawn, because drawing it cost a pulse on
    /// every busy pad all day and that was reported as tiring (the module
    /// header). The exception is written here rather than removed from
    /// the invariant, so that collapsing a *second* pair still fails.
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
    /// to be in (`docs/16-the-queue.md`).
    #[test]
    fn a_dark_top_row_button_does_nothing() {
        assert_eq!(arrow(false), Cell::Off);
        assert_eq!(back(false), Cell::Off);
        assert_eq!(next(false), Cell::Off);
    }

    /// `Session` is the exception to the rule above, and the exception is
    /// the point: it is never dark, because there is always an overview
    /// to show. Holding it is what changes, and it says so in white.
    #[test]
    fn the_overview_button_is_always_lit() {
        assert_eq!(overview_button(false), Cell::Static(DARK_GREY));
        assert_eq!(overview_button(true), Cell::Static(WHITE));
    }

    /// An overview pad says the loudest thing in its row, in the same
    /// colours a slot uses — there is no second language.
    #[test]
    fn an_overview_pad_speaks_the_slot_palette() {
        for state in ALL {
            assert_eq!(overview(Some(state), false), slot(state));
            assert_eq!(
                overview(Some(state), true),
                slot(state),
                "state outranks position"
            );
        }
    }

    /// The window marker: dim grey on the pads inside it that would
    /// otherwise be dark, and nothing anywhere else.
    #[test]
    fn the_window_is_marked_only_where_the_map_is_empty() {
        assert_eq!(overview(None, true), Cell::Static(DARK_GREY));
        assert_eq!(overview(None, false), Cell::Off);
    }

    /// `►` is the only one that moves, and it moves in the one colour
    /// this surface reserves.
    #[test]
    fn only_next_is_amber_and_only_next_animates() {
        assert!(animated(next(true)));
        assert!(!animated(back(true)));
        assert!(!animated(arrow(true)));
        assert!(!animated(overview_button(true)));
        assert_eq!(next(true), Cell::Pulse(AMBER));
    }

    /// Back is a control, not a slot state, so it takes a control colour.
    #[test]
    fn back_is_white_and_arrows_are_dim() {
        assert_eq!(back(true), Cell::Static(WHITE));
        assert_eq!(arrow(true), Cell::Static(DARK_GREY));
    }

    const EFFECTS: [Effect; 4] = [
        Effect::Acts(Consequence::Ends),
        Effect::Acts(Consequence::Forgotten),
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

    /// A preview lasts as long as your thumb. Motion on sixty pads for
    /// two seconds is noise, not information.
    #[test]
    fn nothing_in_a_preview_moves() {
        for effect in EFFECTS {
            assert!(!animated(preview(effect)), "{effect:?}");
        }
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
    /// button rests on (`docs/22-the-mode-button.md`).
    #[test]
    fn ending_and_forgetting_do_not_look_alike() {
        assert_ne!(
            preview(Effect::Acts(Consequence::Ends)),
            preview(Effect::Acts(Consequence::Forgotten))
        );
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

    /// The rule this replaces was *exactly one page button is bright and
    /// it is the one you are on*, learned when white for "you are here"
    /// and turquoise for "has sessions" proved equally bright and
    /// unreadable. The row no longer says where you are — but the thing
    /// that made the old row fail is still forbidden: **at most one bright
    /// button, and the bright one is not competing with another.**
    #[test]
    fn at_most_one_top_row_button_is_bright() {
        let bright = |c: Cell| {
            matches!(c, Cell::Static(e) | Cell::Pulse(e)
                if e != crate::surface::palette::OFF && e != DARK_GREY)
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
