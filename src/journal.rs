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

use serde::Serialize;

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
            let at = address(*slot, pages);
            write_line(
                out,
                &Asked {
                    ms: now,
                    session: &r.session,
                    slot: &at,
                    to: "needs_answer",
                    source,
                    title: r.title.as_deref(),
                },
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
                let at = address(slot, pages);
                write_line(
                    out,
                    &Answered {
                        ms: now,
                        session: &session,
                        slot: &at,
                        to: "answered",
                        after_ms: now.saturating_sub(since),
                    },
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

/// A slot that just started asking.
///
/// A struct rather than a `format!` because every string on this line is
/// one tally did not choose — a page name, an agent's session id, a
/// terminal title — and a single raw `"` in any of them turns the line
/// into something `jq` stops at. Hand-escaping covered the title and not
/// the other two. **Field order is declaration order** in serde, and the
/// order here is the one already written to `attention.jsonl`, so old and
/// new lines read alike.
#[derive(Debug, Serialize)]
struct Asked<'a> {
    ms: i64,
    session: &'a str,
    slot: &'a str,
    /// Always `needs_answer`. Constant per shape, but it is the field the
    /// week's count is grouped by, so it is written out.
    to: &'static str,
    source: &'static str,
    /// The title is what makes a week of these judgeable. A line naming
    /// only a ULID cannot be scored as a right or wrong amber seven days
    /// later; one carrying "edit tests/pty.rs?" can. Absent is normal and
    /// stays legal — `null`, as it has always been written.
    title: Option<&'a str>,
}

/// A slot that stopped asking: answered, exited, muted, or gone empty.
///
/// Its own struct rather than `Option` fields on [`Asked`], because the
/// two shapes genuinely differ: `source` and `title` have no meaning for
/// a transition *out* of asking, and emitting `"source":null` here would
/// be a third format rather than an honest one.
#[derive(Debug, Serialize)]
struct Answered<'a> {
    ms: i64,
    session: &'a str,
    slot: &'a str,
    /// Always `answered`; see [`Asked::to`].
    to: &'static str,
    after_ms: i64,
}

/// One line, newline-terminated — the journal is JSON Lines.
///
/// Serialising these structs cannot fail on its own (no non-string map
/// keys, no floats), so any error is the sink's and is reported as the
/// `io::Error` it is.
fn write_line(out: &mut impl Write, line: &impl Serialize) -> io::Result<()> {
    serde_json::to_writer(&mut *out, line).map_err(io::Error::from)?;
    writeln!(out)
}

/// `page:row:col`, falling back to the index for a page with no name.
///
/// The journal is read by an eye and by `jq`, so an unnamed page gets its
/// number rather than being dropped — unlike a `tl.slot` label, which has
/// to parse back ([`Slot::label`]).
fn address(slot: Slot, pages: &Pages) -> String {
    slot.label(pages)
        .unwrap_or_else(|| format!("{}:{}:{}", slot.page, slot.row, slot.col))
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
                title: Some("edit tests/pty.rs?".to_owned()),
                state: SlotState::NeedsAnswer {
                    source: Source::Derived,
                },
                muted: false,
                spec: None,
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
        assert!(got[0].contains(r#""title":"edit tests/pty.rs?""#));
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

    /// Every string on a journal line is one tally did not choose: a page
    /// name comes from `slots.toml`, a session id from anchor, a title
    /// from whatever the agent set. Hand-built, the line `{"slot":"wo"rk…`
    /// was not JSON — and `jq` stops at the first one of those, so one bad
    /// page name costs the whole week M1 is measured by.
    #[test]
    fn a_page_name_a_session_or_a_title_full_of_quotes_still_parses() {
        let pages = Pages::new([r#"wo"rk\"#.to_owned()]);
        let slot = Slot {
            page: 1,
            row: 2,
            col: 4,
        };
        let mut state = BTreeMap::new();
        let _p = state.insert(
            slot,
            Resolved {
                slot,
                session: r#"01J"X"#.to_owned(),
                title: Some("say \"hi\"\n\u{1}tab\there".to_owned()),
                state: SlotState::NeedsAnswer {
                    source: Source::Derived,
                },
                muted: false,
                spec: None,
            },
        );
        let mut out = Vec::new();
        let mut j = Journal::default();
        j.record(&state, &pages, T0, &mut out).expect("write");
        // ...and the line written when it stops asking, which carries the
        // same page name and session id.
        j.record(&BTreeMap::new(), &pages, T0 + 1_000, &mut out)
            .expect("write");

        let got = lines(&out);
        assert_eq!(got.len(), 2, "{got:?}");
        for line in &got {
            let v: serde_json::Value = serde_json::from_str(line).expect("valid JSON");
            assert_eq!(
                v.get("slot").and_then(serde_json::Value::as_str),
                Some(r#"wo"rk\:2:4"#),
                "{line}"
            );
            assert_eq!(
                v.get("session").and_then(serde_json::Value::as_str),
                Some(r#"01J"X"#),
                "{line}"
            );
        }
        let first = got.first().expect("first line");
        let v: serde_json::Value = serde_json::from_str(first).expect("valid JSON");
        assert_eq!(
            v.get("title").and_then(serde_json::Value::as_str),
            Some("say \"hi\"\n\u{1}tab\there"),
            "{first}"
        );
    }

    /// Absent is normal: an agent that set no title must still produce a
    /// line, and `null` is what the field has always carried.
    #[test]
    fn a_missing_title_is_written_as_null() {
        let slot = Slot {
            page: 1,
            row: 1,
            col: 1,
        };
        let mut state = asking_at(slot, "01J");
        if let Some(r) = state.get_mut(&slot) {
            r.title = None;
        }
        let mut out = Vec::new();
        Journal::default()
            .record(&state, &Pages::default(), T0, &mut out)
            .expect("write");
        let got = lines(&out);
        assert!(
            got.first().is_some_and(|l| l.contains(r#""title":null"#)),
            "{got:?}"
        );
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
