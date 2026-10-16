//! Sessions in, frame out. The only place that knows both halves.
//!
//! This is a **pure function**: `anchor list` plus derived signals plus
//! which page you are looking at, producing 81 lit cells. Nothing is
//! cached, nothing is incremental. That is what makes `tallyd` disposable —
//! it holds no state that cannot be rebuilt from a `list` and a config file
//! (`docs/08-lifecycle.md`).

use std::collections::BTreeMap;

use crate::anchor::Session;
use crate::render::{self, Health, Page};
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
        if state == SlotState::Empty {
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
            },
        );
    }
    taken
}

/// Build the frame.
pub fn frame(sessions: &[Session], derived: &BTreeMap<String, Derived>, view: &View<'_>) -> Frame {
    frame_from(&resolve(sessions, derived, view.pages, view.now), view)
}

/// Build the frame from an already-resolved grid, so a caller that needs
/// the states as well as the picture does not resolve twice.
pub fn frame_from(resolved: &BTreeMap<Slot, Resolved>, view: &View<'_>) -> Frame {
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
        if r.slot.page == view.current {
            frame.set(r.slot.button(), render::slot(r.state));
        }
    }

    for index in 1..=slot::PAGES {
        let at = usize::from(index - 1);
        frame.set(
            Button::Page(index),
            render::page(Page {
                current: index == view.current,
                occupied: occupied.get(at).copied().unwrap_or(false),
                attention: wants_you.get(at).copied().unwrap_or(false),
            }),
        );
    }

    for row in 1..=8 {
        frame.set(Button::Action(row), render::action(false));
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
    use crate::surface::palette::{AMBER, DARK_RED, GREEN, TURQUOISE, WHITE};
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
        }
    }

    fn view(pages: &Pages) -> View<'_> {
        View {
            pages,
            current: 1,
            anchord_reachable: true,
            now: NOW,
        }
    }

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
        assert_eq!(f, {
            let mut empty = frame(&[], &BTreeMap::new(), &view(&pages));
            empty.set(Button::Logo, f.get(Button::Logo));
            empty
        });
    }

    #[test]
    fn a_malformed_slot_is_ignored_not_clamped() {
        let pages = Pages::default();
        for bad in ["1:9:1", "nope:1:1", "garbage"] {
            let sessions = [session("x", "running", Some(bad))];
            let f = frame(&sessions, &BTreeMap::new(), &view(&pages));
            let lit = Button::all().filter(|&b| f.get(b) != Cell::Off).count();
            // Only the controls: the current page button and 8 actions.
            assert_eq!(lit, 9, "{bad:?} lit a pad");
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
        let f = frame(&[newest, older], &BTreeMap::new(), &view(&pages));
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
        assert_eq!(
            f.get(Button::Page(2)),
            Cell::Static(TURQUOISE),
            "but its button says it holds work"
        );
        assert_eq!(
            f.get(Button::Page(1)),
            Cell::Static(WHITE),
            "and page 1 is where you are"
        );
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
        let f = frame(&sessions, &derived, &v);

        assert!(
            matches!(f.get(Button::Logo), Cell::Flash { b: AMBER, .. }),
            "logo must say so"
        );
        assert!(
            matches!(f.get(Button::Page(4)), Cell::Flash { b: AMBER, .. }),
            "and which page"
        );
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
        let f = frame(&sessions, &derived, &v);
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
}
