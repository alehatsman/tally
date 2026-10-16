//! One line per attention transition.
//!
//! [M1's exit criterion](../ROADMAP.md) is *"a week of real use, counting
//! every time the amber was wrong"*, and a count you keep by remembering is
//! a count you do not keep. A week of remembering produces an opinion; a
//! week of transitions produces a number, and the number is the one
//! deciding whether the rest of the roadmap is worth building.
//!
//! Transitions only, so it is a few lines a day rather than a log. It is
//! the one thing `tallyd` writes that cannot be rebuilt from `anchor list`,
//! which is acceptable precisely because **nothing reads it back**.

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::model::Resolved;
use crate::signal::{SlotState, Source};
use crate::slot::{Pages, Slot};

/// Remembers which slots were asking, so only changes are written.
#[derive(Debug, Default)]
pub struct Journal {
    /// slot -> (session id, when it started asking)
    asking: BTreeMap<Slot, (String, i64)>,
}

/// `~/.local/state/tally/attention.jsonl`.
pub fn default_path() -> PathBuf {
    let state = crate::env_path("XDG_STATE_HOME").unwrap_or_else(|| {
        crate::env_path("HOME")
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".local/state")
    });
    state.join("tally/attention.jsonl")
}

impl Journal {
    /// Write a line for every slot that started or stopped asking.
    ///
    /// # Errors
    /// Whatever the sink returns.
    pub fn record(
        &mut self,
        resolved: &BTreeMap<Slot, Resolved>,
        pages: &Pages,
        now: i64,
        out: &mut impl Write,
    ) -> io::Result<()> {
        for (slot, r) in resolved {
            let SlotState::NeedsAnswer { source } = r.state else {
                continue;
            };
            if self.asking.contains_key(slot) {
                continue; // still asking; not a transition
            }
            let _previous = self.asking.insert(*slot, (r.session.clone(), now));
            let source = match source {
                Source::Derived => "derived",
                Source::Declared => "declared",
            };
            writeln!(
                out,
                r#"{{"ms":{now},"session":"{}","slot":"{}","to":"needs_answer","source":"{source}"}}"#,
                r.session,
                address(*slot, pages),
            )?;
        }

        // Stopped asking: answered, exited, muted, or the pad went empty.
        let still: Vec<Slot> = resolved
            .iter()
            .filter(|(_, r)| matches!(r.state, SlotState::NeedsAnswer { .. }))
            .map(|(s, _)| *s)
            .collect();
        let done: Vec<Slot> = self
            .asking
            .keys()
            .filter(|s| !still.contains(s))
            .copied()
            .collect();
        for slot in done {
            if let Some((session, since)) = self.asking.remove(&slot) {
                writeln!(
                    out,
                    r#"{{"ms":{now},"session":"{session}","slot":"{}","to":"answered","after_ms":{}}}"#,
                    address(slot, pages),
                    now.saturating_sub(since),
                )?;
            }
        }
        Ok(())
    }

    /// How many slots are currently asking.
    pub fn asking(&self) -> usize {
        self.asking.len()
    }
}

/// `page:row:col`, using the page's name where it has one.
fn address(slot: Slot, pages: &Pages) -> String {
    let page = pages
        .name_of(slot.page)
        .map_or_else(|| slot.page.to_string(), str::to_owned);
    format!("{page}:{}:{}", slot.row, slot.col)
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_760_000_000_000;

    fn asking_at(slot: Slot, session: &str) -> BTreeMap<Slot, Resolved> {
        let mut m = BTreeMap::new();
        let _p = m.insert(
            slot,
            Resolved {
                slot,
                session: session.to_owned(),
                state: SlotState::NeedsAnswer {
                    source: Source::Derived,
                },
            },
        );
        m
    }

    fn lines(buf: &[u8]) -> Vec<String> {
        String::from_utf8_lossy(buf)
            .lines()
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn a_pad_that_starts_asking_is_recorded_once() {
        let slot = Slot {
            page: 1,
            row: 3,
            col: 5,
        };
        let mut j = Journal::default();
        let pages = Pages::default();
        let state = asking_at(slot, "01J");
        let mut out = Vec::new();

        j.record(&state, &pages, T0, &mut out).expect("write");
        // Still asking on the next tick, and the one after.
        j.record(&state, &pages, T0 + 2_000, &mut out)
            .expect("write");
        j.record(&state, &pages, T0 + 4_000, &mut out)
            .expect("write");

        let got = lines(&out);
        assert_eq!(got.len(), 1, "transitions only: {got:?}");
        assert!(got[0].contains(r#""to":"needs_answer""#));
        assert!(got[0].contains(r#""slot":"1:3:5""#));
        assert!(got[0].contains(r#""source":"derived""#));
        assert_eq!(j.asking(), 1);
    }

    #[test]
    fn answering_records_how_long_it_waited() {
        let slot = Slot {
            page: 1,
            row: 3,
            col: 5,
        };
        let mut j = Journal::default();
        let pages = Pages::default();
        let mut out = Vec::new();

        j.record(&asking_at(slot, "01J"), &pages, T0, &mut out)
            .expect("write");
        j.record(&BTreeMap::new(), &pages, T0 + 61_000, &mut out)
            .expect("write");

        let got = lines(&out);
        assert_eq!(got.len(), 2, "{got:?}");
        assert!(got[1].contains(r#""to":"answered""#));
        assert!(got[1].contains(r#""after_ms":61000"#));
        assert_eq!(j.asking(), 0);
    }

    #[test]
    fn a_second_ask_on_the_same_pad_is_a_second_line() {
        let slot = Slot {
            page: 1,
            row: 1,
            col: 1,
        };
        let mut j = Journal::default();
        let pages = Pages::default();
        let mut out = Vec::new();

        j.record(&asking_at(slot, "01J"), &pages, T0, &mut out)
            .expect("write");
        j.record(&BTreeMap::new(), &pages, T0 + 1_000, &mut out)
            .expect("write");
        j.record(&asking_at(slot, "01J"), &pages, T0 + 2_000, &mut out)
            .expect("write");

        assert_eq!(lines(&out).len(), 3);
    }

    #[test]
    fn a_named_page_is_written_by_name() {
        let pages = Pages::new(["work".to_owned()]);
        let slot = Slot {
            page: 1,
            row: 2,
            col: 4,
        };
        let mut out = Vec::new();
        Journal::default()
            .record(&asking_at(slot, "01J"), &pages, T0, &mut out)
            .expect("write");
        assert!(lines(&out)[0].contains(r#""slot":"work:2:4""#));
    }

    #[test]
    fn a_quiet_grid_writes_nothing() {
        let mut out = Vec::new();
        Journal::default()
            .record(&BTreeMap::new(), &Pages::default(), T0, &mut out)
            .expect("write");
        assert!(out.is_empty());
    }
}
