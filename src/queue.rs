//! Which agent to answer first.
//!
//! A grouped layout rests on knowing *which project* is blocked before you
//! jump. The first hour of `attention.jsonl` answered a different question
//! instead: **61% of the time more than one agent is
//! waiting**, and when two or more are, the gap between the longest and
//! shortest wait is a median of 386 seconds.
//!
//! So the bottleneck is not finding the blocked agent — at five lit pads
//! out of sixty-four there is no finding problem. It is *choosing*, and
//! the thing that separates the choices is how long each has been waiting
//! (`docs/16-the-queue.md`).
//!
//! This module is pure, like [`crate::chord`]: it takes the map the
//! journal already keeps and produces a slot. It does not know that tmux
//! exists, that anchord exists, or that a press happened.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::Pads;
use crate::signal::SlotState;
use crate::slot::Slot;

/// The slot at the head of the queue, with its session and how long it has
/// been waiting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    /// Where it is.
    pub at: Slot,
    /// The session to jump to.
    pub session: String,
    /// When it started asking. **The original moment**, preserved across a
    /// `tallyd` restart by the journal readback (#7).
    pub since: i64,
    /// The conversation it is running, from the grid.
    ///
    /// Carried so that answering through `►` tags the view with it, the
    /// same as pressing the pad does — otherwise the one path you use
    /// when you are busy is the one that leaves the dead line short
    /// (`docs/26-the-dead-pane.md`).
    pub claude: Option<String>,
}

/// Every slot that is asking and not muted, oldest-first by nothing —
/// the iterator the three questions below are all asked of.
///
/// **The predicate, written once.** It used to be written three times,
/// character-identical in two of them, under a doc comment arguing that
/// one predicate is the whole point (`docs/29-what-the-loop-holds.md`).
/// Three copies agreed by coincidence of having been typed in one
/// sitting.
///
/// Re-checked against the grid rather than trusted: the journal map is
/// updated once per reconcile and a press can land between two of them.
/// Muted is the half that matters — mute is how you get a prompt you
/// will never answer out of the way, and a queue that ignored it would
/// hand it back forever.
fn answering<'a>(
    waiting: &'a BTreeMap<Slot, (String, i64)>,
    pads: &'a Pads<'_>,
) -> impl Iterator<Item = (&'a Slot, &'a (String, i64))> {
    waiting.iter().filter(|(at, _)| {
        let pad = pads.at(**at);
        matches!(pad.state, Some(SlotState::NeedsAnswer { .. })) && !pad.muted
    })
}

/// The longest-waiting slot that is asking, is not muted, and is not
/// already in front of somebody.
///
/// `None` is the ordinary answer, not an error — it is also exactly when
/// `►` is dark, which is the point: **one predicate, read by the renderer
/// and by the press.** Two would eventually disagree, and a button that
/// lights up and then does nothing is one you press twice.
#[must_use]
pub fn head(
    waiting: &BTreeMap<Slot, (String, i64)>,
    pads: &Pads<'_>,
    viewing: &BTreeSet<Slot>,
) -> Option<Head> {
    answering(waiting, pads)
        .filter(|(at, _)| !viewing.contains(*at))
        // Oldest first. `min_by_key` takes the first of equal keys and
        // `BTreeMap` iterates in slot order, so ties break by address and
        // the head does not flicker between two pads that rose in the
        // same tick.
        .min_by_key(|(_, (_, since))| *since)
        .map(|(at, (session, since))| Head {
            at: *at,
            session: session.clone(),
            since: *since,
            claude: pads.grid().get(at).and_then(|r| r.claude.clone()),
        })
}

/// How many slots the queue would offer — what `queued` reports in the
/// journal, so a week of lines can say whether there was ever a choice.
#[must_use]
pub fn depth(
    waiting: &BTreeMap<Slot, (String, i64)>,
    pads: &Pads<'_>,
    viewing: &BTreeSet<Slot>,
) -> usize {
    answering(waiting, pads)
        .filter(|(at, _)| !viewing.contains(*at))
        .count()
}

/// When the oldest thing still asking started asking.
///
/// **Not the queue's head.** The head excludes the slot in front of you,
/// because `►` must not offer you where you already are. The tempo does
/// not exclude it: a request you are sitting in front of and have not
/// answered is still a request that has been waiting, and the grid is
/// read by the room rather than by your current focus
/// (`docs/17-the-tempo.md`).
///
/// That difference is this function's whole content — it is
/// [`answering`] with the `viewing` filter the other two add left off,
/// and now that is visible here rather than inferable from diffing three
/// bodies.
///
/// Muted slots are excluded by both, for the same reason: mute means
/// *stop raising this*, and the beat is raising it.
#[must_use]
pub fn oldest(waiting: &BTreeMap<Slot, (String, i64)>, pads: &Pads<'_>) -> Option<i64> {
    answering(waiting, pads).map(|(_, (_, since))| *since).min()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::anchor::Session;
    use crate::config::Config;
    use crate::model::{self, Pads};
    use crate::signal::Source;
    use crate::slot::Pages;

    fn at(page: u8, row: u8, col: u8) -> Slot {
        Slot { page, row, col }
    }

    /// A session asking on `slot`, optionally muted.
    fn asking(id: &str, slot: &str, muted: bool) -> Session {
        let mut labels = BTreeMap::new();
        let _slot = labels.insert("tl.slot".to_owned(), slot.to_owned());
        let _att = labels.insert("tl.attention".to_owned(), "1".to_owned());
        if muted {
            let _m = labels.insert("tl.mute".to_owned(), "1".to_owned());
        }
        Session {
            id: id.to_owned(),
            state: "running".to_owned(),
            exit_code: None,
            lost_reason: None,
            labels,
            output_bytes: 0,
            pid: None,
            cwd: None,
            command: None,
            login_shell: false,
        }
    }

    struct Fixture {
        resolved: BTreeMap<Slot, model::Resolved>,
        rows: BTreeMap<Slot, Vec<String>>,
        config: Config,
    }

    impl Fixture {
        fn new(sessions: &[Session]) -> Self {
            let pages = Pages::default();
            Self {
                resolved: model::resolve(sessions, &BTreeMap::new(), &pages, 0),
                rows: BTreeMap::new(),
                config: Config::default(),
            }
        }
        fn pads(&self) -> Pads<'_> {
            Pads::new(&self.resolved, &self.config, &self.rows, None)
        }
    }

    fn waiting(entries: &[(Slot, &str, i64)]) -> BTreeMap<Slot, (String, i64)> {
        entries
            .iter()
            .map(|(s, id, since)| (*s, ((*id).to_owned(), *since)))
            .collect()
    }

    /// The whole point: oldest first, not newest, and not whatever order
    /// the map happens to iterate in.
    #[test]
    fn the_head_is_the_longest_waiting() {
        let f = Fixture::new(&[
            asking("A", "1:1:1", false),
            asking("B", "1:2:1", false),
            asking("C", "1:3:1", false),
        ]);
        let w = waiting(&[
            (at(1, 1, 1), "A", 5_000),
            (at(1, 2, 1), "B", 1_000), // the oldest
            (at(1, 3, 1), "C", 9_000),
        ]);
        let head = head(&w, &f.pads(), &BTreeSet::new()).expect("a head");
        assert_eq!(head.at, at(1, 2, 1));
        assert_eq!(head.session, "B");
        assert_eq!(head.since, 1_000);
    }

    /// Mute is the way out of a prompt you are never going to answer. A
    /// queue that ignored it would hand it back on every press, forever.
    #[test]
    fn a_muted_slot_is_never_the_head() {
        let f = Fixture::new(&[
            asking("A", "1:1:1", true), // oldest, but muted
            asking("B", "1:2:1", false),
        ]);
        let w = waiting(&[(at(1, 1, 1), "A", 1_000), (at(1, 2, 1), "B", 5_000)]);
        let head = head(&w, &f.pads(), &BTreeSet::new()).expect("a head");
        assert_eq!(head.at, at(1, 2, 1), "the muted one is skipped");
        assert_eq!(depth(&w, &f.pads(), &BTreeSet::new()), 1);
    }

    /// `►` must not offer you the thing already filling your terminal.
    #[test]
    fn the_slot_you_are_looking_at_is_never_the_head() {
        let f = Fixture::new(&[asking("A", "1:1:1", false), asking("B", "1:2:1", false)]);
        let w = waiting(&[(at(1, 1, 1), "A", 1_000), (at(1, 2, 1), "B", 5_000)]);
        let viewing: BTreeSet<Slot> = [at(1, 1, 1)].into_iter().collect();
        let head = head(&w, &f.pads(), &viewing).expect("a head");
        assert_eq!(head.at, at(1, 2, 1));
    }

    /// One asking, and you are on it: dark, and the press does nothing.
    #[test]
    fn nothing_to_offer_is_none_and_not_an_error() {
        let f = Fixture::new(&[asking("A", "1:1:1", false)]);
        let w = waiting(&[(at(1, 1, 1), "A", 1_000)]);
        let viewing: BTreeSet<Slot> = [at(1, 1, 1)].into_iter().collect();
        assert_eq!(head(&w, &f.pads(), &viewing), None);
        assert_eq!(depth(&w, &f.pads(), &viewing), 0);
        let empty = BTreeMap::new();
        assert_eq!(head(&empty, &f.pads(), &BTreeSet::new()), None);
    }

    /// The journal map is updated once a reconcile; a press can land
    /// between two of them. A slot that has since stopped asking must not
    /// be offered just because the map has not caught up.
    #[test]
    fn a_slot_that_stopped_asking_is_dropped_even_if_the_map_lags() {
        let f = Fixture::new(&[]); // nothing on the grid at all
        let w = waiting(&[(at(1, 1, 1), "A", 1_000)]);
        assert_eq!(head(&w, &f.pads(), &BTreeSet::new()), None);
    }

    /// Two pads that rose in the same tick must not make the head flicker
    /// between them on consecutive frames.
    #[test]
    fn ties_break_by_address_so_the_head_is_stable() {
        let f = Fixture::new(&[asking("A", "1:3:1", false), asking("B", "1:2:1", false)]);
        let w = waiting(&[(at(1, 3, 1), "A", 1_000), (at(1, 2, 1), "B", 1_000)]);
        for _ in 0..4 {
            let head = head(&w, &f.pads(), &BTreeSet::new()).expect("a head");
            assert_eq!(head.at, at(1, 2, 1), "lower address wins, every time");
        }
    }

    /// The tempo asks the room, not your terminal: a request you are
    /// looking at and have not answered has still been waiting.
    #[test]
    fn the_oldest_wait_counts_the_slot_you_are_looking_at() {
        let f = Fixture::new(&[asking("A", "1:1:1", false), asking("B", "1:2:1", false)]);
        let w = waiting(&[(at(1, 1, 1), "A", 1_000), (at(1, 2, 1), "B", 5_000)]);
        let viewing: BTreeSet<Slot> = [at(1, 1, 1)].into_iter().collect();
        assert_eq!(
            head(&w, &f.pads(), &viewing).expect("head").at,
            at(1, 2, 1),
            "the head skips it"
        );
        assert_eq!(oldest(&w, &f.pads()), Some(1_000), "the tempo does not");
    }

    /// Mute means stop raising this, and the beat is raising it.
    #[test]
    fn a_muted_slot_does_not_set_the_tempo() {
        let f = Fixture::new(&[asking("A", "1:1:1", true), asking("B", "1:2:1", false)]);
        let w = waiting(&[(at(1, 1, 1), "A", 1_000), (at(1, 2, 1), "B", 5_000)]);
        assert_eq!(oldest(&w, &f.pads()), Some(5_000));
    }

    #[test]
    fn nothing_asking_has_no_oldest() {
        let f = Fixture::new(&[]);
        assert_eq!(oldest(&BTreeMap::new(), &f.pads()), None);
    }

    #[test]
    fn depth_counts_what_the_queue_would_offer() {
        let f = Fixture::new(&[
            asking("A", "1:1:1", false),
            asking("B", "1:2:1", false),
            asking("C", "1:3:1", true),
        ]);
        let w = waiting(&[
            (at(1, 1, 1), "A", 1),
            (at(1, 2, 1), "B", 2),
            (at(1, 3, 1), "C", 3),
        ]);
        assert_eq!(depth(&w, &f.pads(), &BTreeSet::new()), 2, "muted excluded");
        let viewing: BTreeSet<Slot> = [at(1, 1, 1)].into_iter().collect();
        assert_eq!(depth(&w, &f.pads(), &viewing), 1);
    }

    /// Source does not rank. A declared amber and a derived one are the
    /// same request for your attention, and the only thing that orders
    /// them is how long they have waited.
    #[test]
    fn where_the_amber_came_from_does_not_change_its_place() {
        let f = Fixture::new(&[asking("A", "1:1:1", false), asking("B", "1:2:1", false)]);
        let pads = f.pads();
        assert!(matches!(
            pads.at(at(1, 1, 1)).state,
            Some(SlotState::NeedsAnswer {
                source: Source::Declared
            })
        ));
        let w = waiting(&[(at(1, 1, 1), "A", 9_000), (at(1, 2, 1), "B", 1_000)]);
        assert_eq!(
            head(&w, &pads, &BTreeSet::new()).expect("a head").at,
            at(1, 2, 1)
        );
    }

    /// The three questions are one predicate plus one filter, and this is
    /// the table that says so: every combination of muted / being-looked-at
    /// / not-asking-at-all, checked against all three entry points at once
    /// (`docs/29-what-the-loop-holds.md`).
    ///
    /// `oldest` deliberately counts the slot you are sitting in front of
    /// and `head`/`depth` deliberately do not, so the row where `viewing`
    /// holds the only asking slot is the one that would catch a filter
    /// applied in the wrong place.
    #[test]
    fn one_predicate_and_one_deliberate_difference() {
        let one = at(1, 1, 1);
        let two = at(1, 2, 2);
        let fx = Fixture::new(&[asking("a", "1:1:1", false), asking("b", "1:2:2", true)]);
        let pads = fx.pads();
        // `two` is muted, so it is not in the queue and not in the tempo.
        let w = waiting(&[(one, "a", 100), (two, "b", 50)]);

        let nobody = BTreeSet::new();
        assert_eq!(head(&w, &pads, &nobody).map(|h| h.at), Some(one));
        assert_eq!(depth(&w, &pads, &nobody), 1);
        assert_eq!(oldest(&w, &pads), Some(100), "muted is never the oldest");

        // Now look at the one slot that is asking. The queue has nothing
        // left to offer; the beat still counts the wait.
        let looking: BTreeSet<Slot> = [one].into_iter().collect();
        assert_eq!(head(&w, &pads, &looking), None);
        assert_eq!(depth(&w, &pads, &looking), 0);
        assert_eq!(
            oldest(&w, &pads),
            Some(100),
            "the tempo does not stop because you are sitting in front of it"
        );

        // A slot in the map that is not asking on the grid is in neither:
        // the map is a reconcile old and the grid is now.
        let stale = waiting(&[(at(1, 8, 8), "ghost", 1)]);
        assert_eq!(head(&stale, &pads, &nobody), None);
        assert_eq!(depth(&stale, &pads, &nobody), 0);
        assert_eq!(oldest(&stale, &pads), None);
    }
}
