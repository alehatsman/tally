//! The colour language (`docs/05-interaction.md`): state in, light out.
//!
//! The constraint that decides everything here is that this must be
//! readable **from across the room, without focusing on it** — and, harder,
//! *memorable* without a legend. An earlier table used three greens
//! (working, quiet, exited-clean) and failed on the second count: you could
//! see the difference and could not recall which was which.
//!
//! So the rule is one sentence. **Green means alive**; motion inside green
//! means producing output. Anything not green is gone, and the pad that
//! wants you is the only amber thing on the surface.
//!
//! **Nothing blinks.** Everything that moves breathes -- the device's
//! two-beat ramp -- because this is a surface somebody sits beside all day,
//! and a hard 50% flash reads as urgent for a minute and as stressful by
//! the afternoon. Urgency is carried by hue, which does not get tiring.

use crate::chord::Action;
use crate::model::Effect;
use crate::signal::SlotState;
use crate::surface::palette::{
    Entry, AMBER, BLUE, DARK_GREY, DARK_RED, GREEN, RED, TURQUOISE, WHITE,
};
use crate::surface::Cell;

/// How one slot is lit.
pub fn slot(state: SlotState) -> Cell {
    match state {
        SlotState::Empty => Cell::Off,
        SlotState::Working => Cell::Pulse(GREEN),
        SlotState::Quiet => Cell::Static(GREEN),
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
        SlotState::Lost => Cell::Static(DARK_RED),
    }
}

/// The colour a held pad's title scrolls in.
///
/// The slot's **own** colour, so the text that appears is visibly coming
/// from the pad under your finger rather than from the surface in
/// general. A dark pad has no title to scroll, but the fallback is green
/// rather than off: text in colour 0 is text you cannot see, and a
/// request that silently produces nothing reads as a broken button.
pub fn scroll_colour(state: SlotState) -> Entry {
    match slot(state) {
        Cell::Static(c) | Cell::Pulse(c) | Cell::Flash { b: c, .. } => c,
        Cell::Off => GREEN,
    }
}

/// What a page button in the top row shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Page {
    /// The page currently on the grid.
    pub current: bool,
    /// Holds at least one session.
    pub occupied: bool,
    /// Something on this page wants you, and you are not looking at it.
    pub attention: bool,
}

/// How a page button in the top row is lit.
pub fn page(p: Page) -> Cell {
    if p.current {
        // You are looking at it, so there is no off-screen attention to
        // report: walking onto a page is how you stop being told about it.
        Cell::Static(WHITE)
    } else if p.attention {
        Cell::Pulse(AMBER)
    } else if p.occupied {
        Cell::Static(TURQUOISE)
    } else {
        Cell::Off
    }
}

/// A right-column action button. White while held, which is the only
/// feedback that the modifier is actually down.
///
/// `CC49`-`CC79` carry no gesture and are dark: a button lit like a live
/// one that does nothing when you press it is worse than an unlit button,
/// because you press it twice.
pub fn action(button: Option<Action>, held: bool) -> Cell {
    match (button, held) {
        (None, _) => Cell::Off,
        (Some(_), true) => Cell::Static(WHITE),
        (Some(_), false) => Cell::Static(BLUE),
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
/// Each colour is the colour of the **consequence**, not of the button:
/// kill reds the pads that are about to be red, respawn greens the ones
/// about to be green. Nothing here pulses — a preview lasts as long as
/// your thumb does, and motion on sixty pads for two seconds is noise.
pub fn preview(action: Action, effect: Effect) -> Cell {
    match effect {
        Effect::None => Cell::Off,
        Effect::Skips => Cell::Static(DARK_GREY),
        // Mute is the only two-colour preview, because it is the only
        // toggle. Blue and white are the *control* colours: you are about
        // to change how tally treats the pad, not what is running on it.
        Effect::Unmutes => Cell::Static(WHITE),
        Effect::Acts => Cell::Static(match action {
            Action::Kill => RED,
            Action::Respawn => GREEN,
            Action::Purge => DARK_RED,
            Action::Mute => BLUE,
        }),
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
        SlotState::Lost,
    ];

    /// Two things move, and both breathe. What separates them is hue, not
    /// rhythm -- green for alive, amber for wanting you.
    #[test]
    fn only_two_slot_states_move_and_both_breathe() {
        let moving: Vec<_> = ALL.into_iter().filter(|&s| animated(slot(s))).collect();
        assert_eq!(moving.len(), 2, "{moving:?}");
        assert_eq!(slot(SlotState::Working), Cell::Pulse(GREEN));
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
            page(Page {
                occupied: true,
                attention: true,
                ..Page::default()
            }),
            action(Some(Action::Kill), true),
            action(Some(Action::Kill), false),
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
        assert!(!amber(action(Some(Action::Kill), false)));
        assert!(!amber(action(Some(Action::Kill), true)));
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
            SlotState::Lost,
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
        assert_ne!(slot(SlotState::Lost), slot(SlotState::Failed { code: 1 }));
        assert!(
            !animated(slot(SlotState::Lost)),
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
        assert_eq!(slot(SlotState::Lost), unreachable);
    }

    /// Every state is visually distinct from every other.
    #[test]
    fn no_two_states_share_a_cell() {
        for (i, &a) in ALL.iter().enumerate() {
            for &b in &ALL[i + 1..] {
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

    #[test]
    fn the_current_page_does_not_report_attention_to_itself() {
        let here = Page {
            current: true,
            occupied: true,
            attention: true,
        };
        assert_eq!(page(here), Cell::Static(WHITE));
    }

    #[test]
    fn page_buttons_escalate() {
        assert_eq!(page(Page::default()), Cell::Off);
        assert_eq!(
            page(Page {
                occupied: true,
                ..Page::default()
            }),
            Cell::Static(TURQUOISE)
        );
        let wants = Page {
            occupied: true,
            attention: true,
            ..Page::default()
        };
        assert!(animated(page(wants)));
    }

    const EFFECTS: [Effect; 4] = [Effect::Acts, Effect::Unmutes, Effect::Skips, Effect::None];

    /// The one reservation the whole design rests on. A preview that
    /// borrowed amber would put a second amber thing on a surface whose
    /// promise is that there is only ever one.
    #[test]
    fn no_preview_is_ever_amber() {
        for action in Action::all() {
            for effect in EFFECTS {
                assert_ne!(preview(action, effect), Cell::Static(AMBER));
                assert_ne!(preview(action, effect), Cell::Pulse(AMBER));
            }
        }
    }

    /// A preview lasts as long as your thumb. Motion on sixty pads for
    /// two seconds is noise, not information.
    #[test]
    fn nothing_in_a_preview_moves() {
        for action in Action::all() {
            for effect in EFFECTS {
                assert!(!animated(preview(action, effect)), "{action:?}/{effect:?}");
            }
        }
    }

    /// Each colour is the colour of the consequence: kill reds the pads
    /// that are about to be red, respawn greens the ones about to be green.
    #[test]
    fn a_preview_wears_the_colour_of_what_it_would_do() {
        assert_eq!(preview(Action::Kill, Effect::Acts), Cell::Static(RED));
        assert_eq!(preview(Action::Respawn, Effect::Acts), Cell::Static(GREEN));
        assert_eq!(preview(Action::Purge, Effect::Acts), Cell::Static(DARK_RED));
        assert_eq!(
            slot(SlotState::Failed { code: 1 }),
            preview(Action::Kill, Effect::Acts)
        );
        assert_eq!(slot(SlotState::Lost), preview(Action::Purge, Effect::Acts));
    }

    /// Three levels, and the middle one is the one that earns its keep.
    #[test]
    fn the_three_preview_levels_are_distinguishable() {
        for action in Action::all() {
            let acts = preview(action, Effect::Acts);
            let skips = preview(action, Effect::Skips);
            assert_eq!(skips, Cell::Static(DARK_GREY), "{action:?}");
            assert_eq!(preview(action, Effect::None), Cell::Off, "{action:?}");
            assert_ne!(acts, skips, "{action:?}");
        }
    }

    /// Mute is the only two-colour preview, because it is the only
    /// toggle — and both of its colours are control colours, not slot
    /// colours: you are changing how tally treats the pad, not what is
    /// running on it.
    #[test]
    fn mute_is_the_only_preview_that_says_which_way_it_goes() {
        assert_eq!(preview(Action::Mute, Effect::Acts), Cell::Static(BLUE));
        assert_eq!(preview(Action::Mute, Effect::Unmutes), Cell::Static(WHITE));
        let slots: Vec<Cell> = ALL.into_iter().map(slot).collect();
        for c in [Cell::Static(BLUE), Cell::Static(WHITE)] {
            assert!(!slots.contains(&c), "{c:?} is also a slot colour");
        }
    }

    /// An unassigned button lit like a live one is an invitation to press
    /// it, and you press it twice before believing it does nothing.
    #[test]
    fn an_action_button_with_no_gesture_is_dark() {
        assert_eq!(action(None, false), Cell::Off);
        assert_eq!(action(None, true), Cell::Off);
        assert_eq!(action(Some(Action::Kill), false), Cell::Static(BLUE));
        assert_eq!(action(Some(Action::Kill), true), Cell::Static(WHITE));
    }

    /// Text in colour 0 is text you cannot see, and a hold that silently
    /// produces nothing reads as a broken button.
    #[test]
    fn a_scroll_is_never_invisible() {
        for state in ALL {
            assert_ne!(
                scroll_colour(state),
                crate::surface::palette::OFF,
                "{state:?}"
            );
        }
    }

    /// The pad's own colour, so the text is visibly coming from the thing
    /// under your finger.
    #[test]
    fn a_title_scrolls_in_its_slots_colour() {
        assert_eq!(scroll_colour(SlotState::Working), GREEN);
        assert_eq!(scroll_colour(SlotState::Quiet), GREEN);
        assert_eq!(
            scroll_colour(SlotState::NeedsAnswer {
                source: Source::Derived
            }),
            AMBER
        );
        assert_eq!(scroll_colour(SlotState::Failed { code: 1 }), RED);
        assert_eq!(scroll_colour(SlotState::Lost), DARK_RED);
    }

    /// Controls must not be mistakable for any slot state at a glance.
    #[test]
    fn control_colours_are_disjoint_from_slot_colours() {
        let slot_entries: Vec<Cell> = ALL.into_iter().map(slot).collect();
        for control in [action(Some(Action::Kill), false), Cell::Static(TURQUOISE)] {
            assert!(
                !slot_entries.contains(&control),
                "{control:?} collides with a slot colour"
            );
        }
    }
}
