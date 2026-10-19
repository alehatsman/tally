//! One line per attention transition.
//!
//! [M1's exit criterion](../ROADMAP.md) is *"a week of real use, counting
//! every time the amber was wrong"*, and a count you keep by remembering is
//! a count you do not keep. A week of remembering produces an opinion; a
//! week of transitions produces a number, and the number is the one
//! deciding whether the rest of the roadmap is worth building.
//!
//! Transitions only, so it is a few lines a day rather than a log. It is
//! the one thing `tallyd` writes that cannot be rebuilt from `anchor list`.
//!
//! It **is** read back, at startup and only at startup. The first draft
//! said nothing would, and that was the bug: who-is-asking lived in memory,
//! so a restart forgot it and the next tick wrote a second rising edge for
//! a slot that had never stopped asking — three rises against one fall in
//! the file as shipped, with `after_ms` measured from the wrong one
//! (`docs/04-signals.md`). The instrument has to be right more than it
//! has to be write-only.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::Resolved;
use crate::signal::{SlotState, Source};
use crate::slot::{Pages, Slot};

/// Remembers which slots were asking, so only changes are written.
///
/// It owns the file it writes to, so [`Sink`]'s rule is implemented once
/// rather than at each call site (`docs/08-daemon.md`).
#[derive(Debug, Default)]
pub struct Journal {
    /// slot -> (session id, when it started asking)
    asking: BTreeMap<Slot, (String, i64)>,
    /// Where the lines go, or nothing. See [`Sink`].
    sink: Sink,
}

/// The append handle, and the one rule about it.
///
/// **A write that fails retires the sink and the daemon carries on**: a
/// grid that lights up matters more than a log.
///
/// Unbuffered on purpose, and the flush is part of the rule rather than
/// the caller's job: this is a few lines a day, and a buffer would hold
/// them until a flush that `SIGKILL` never reaches — losing exactly the
/// transitions it exists to count.
#[derive(Debug, Default)]
pub struct Sink(Option<File>);

impl Sink {
    /// Open `path` for appending, creating its directory.
    ///
    /// A failure is reported once and yields a closed sink, not an error:
    /// there is no caller that would do anything else with one.
    #[must_use]
    pub fn open(path: &Path) -> Self {
        let opened = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| OpenOptions::new().create(true).append(true).open(path));
        match opened {
            Ok(file) => Self(Some(file)),
            Err(e) => {
                eprintln!("tallyd: no attention journal at {}: {e}", path.display());
                Self(None)
            }
        }
    }

    /// Write one record, flush it, and retire the sink if either failed.
    pub fn write(&mut self, line: impl FnOnce(&mut File) -> io::Result<()>) {
        let Some(file) = self.0.as_mut() else {
            return;
        };
        if let Err(e) = line(file).and_then(|()| file.flush()) {
            eprintln!("tallyd: attention journal: {e}");
            self.0 = None;
        }
    }

    /// Whether anything written here lands anywhere.
    ///
    /// Only the retire-on-failure tests ask: no caller would do anything
    /// different with the answer, which is the whole point of [`Sink`].
    #[cfg(test)]
    fn is_open(&self) -> bool {
        self.0.is_some()
    }
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
    /// Rebuild who is asking from a journal written before a restart.
    ///
    /// Replays the file in order and keeps, per slot, whichever transition
    /// came last. A slot whose last line is `needs_answer` is still asking
    /// — with **its original timestamp**, which is the half that makes
    /// `after_ms` true rather than merely making the rise count once.
    ///
    /// Unparsable lines are skipped rather than refused. The file is
    /// append-only and a truncated last line is what a `SIGKILL` mid-write
    /// leaves behind; losing one transition is a smaller error than
    /// refusing to recover at all.
    ///
    /// A missing file is an empty journal, not a failure: the first run
    /// has no history and that is the normal case, not an exception.
    ///
    /// **Only** a missing file. Every other failure is said out loud, the
    /// same policy [`Sink::open`] follows for the write half: a silent
    /// read failure is not a lost measurement but a false one, because
    /// [`Self::open`] goes on appending to a file it could not read and
    /// every restart re-raises every slot still asking.
    ///
    /// Bytes rather than a `String`, so a torn write that stopped inside
    /// a multi-byte sequence costs one line instead of the file —
    /// `read_to_string` refuses a whole file for one bad byte anywhere in
    /// it. `from_utf8_lossy` turns that tail into a line `serde_json`
    /// rejects, which is the path [`Self::replay`] already tests.
    #[must_use]
    pub fn resumed(path: &Path, pages: &Pages) -> Self {
        let text = match std::fs::read(path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Self::default(),
            Err(e) => {
                eprintln!("tallyd: cannot replay {}: {e}", path.display());
                return Self::default();
            }
        };
        Self::replay(&text, pages)
    }

    /// Resume the map and open the file to append to, in that order.
    ///
    /// The daemon's entry point, and the only one that produces a
    /// `Journal` that writes anything. Both halves are best effort and
    /// report their own failure: a missing file is a first run, and an
    /// unopenable one is a daemon that still lights the grid.
    ///
    /// Written as two statements rather than a functional update, because
    /// Rust evaluates field initialisers before the base: `sink:
    /// Sink::open(path), ..Self::resumed(..)` creates the file *before*
    /// reading it. Harmless while [`Sink::open`] only ever appends; a
    /// future `truncate(true)` there would destroy the history.
    #[must_use]
    pub fn open(path: &Path, pages: &Pages) -> Self {
        let mut journal = Self::resumed(path, pages);
        journal.sink = Sink::open(path);
        journal
    }

    /// The parsing half of [`Self::resumed`], separated so it can be tested
    /// against a string rather than a file.
    #[must_use]
    pub fn replay(text: &str, pages: &Pages) -> Self {
        let mut asking = BTreeMap::new();
        for line in text.lines() {
            let Ok(entry) = serde_json::from_str::<Line>(line) else {
                continue;
            };
            // A navigation line is a record of what you pressed, not of
            // what a slot did. Skipped explicitly rather than by failing
            // to parse, so the two kinds can diverge without this silently
            // starting to read one as the other.
            if entry.kind.as_deref().is_some_and(|k| k != "state") {
                continue;
            }
            let Some(slot) = parse_address(&entry.slot, pages) else {
                continue;
            };
            match entry.to.as_str() {
                "needs_answer" => {
                    let _rise = asking.insert(slot, (entry.session, entry.ms));
                }
                "answered" => {
                    let _fall = asking.remove(&slot);
                }
                // A verb written by a newer binary. Leaving the slot as it
                // was is the conservative answer: the worst case is one
                // duplicate rise, which is the bug we are already fixing,
                // rather than a fall we invented.
                _ => {}
            }
        }
        Self {
            asking,
            sink: Sink::default(),
        }
    }

    /// Write a line for every slot that started or stopped asking, to a
    /// sink the caller supplies.
    ///
    /// Kept over `impl Write` so the transitions can be asserted against
    /// a `Vec<u8>`. The daemon calls [`Self::tick`], which is this
    /// against the file this journal owns.
    ///
    /// # Errors
    /// Whatever the sink returns.
    #[cfg(test)]
    fn record(
        &mut self,
        resolved: &BTreeMap<Slot, Resolved>,
        pages: &Pages,
        now: i64,
        out: &mut impl Write,
    ) -> io::Result<()> {
        Self::transitions(&mut self.asking, resolved, pages, now, out)
    }

    /// Record this tick's transitions to the journal's own file.
    ///
    /// Nothing is returned: a write error retires the sink and is
    /// reported there ([`Sink`]), and there has never been a caller that
    /// would do anything else with one.
    ///
    /// `self` is destructured rather than passed whole so the map and the
    /// sink are borrowed separately.
    pub fn tick(&mut self, resolved: &BTreeMap<Slot, Resolved>, pages: &Pages, now: i64) {
        let Self { asking, sink } = self;
        sink.write(|out| Self::transitions(asking, resolved, pages, now, out));
    }

    /// The body of [`Self::record`], over the map rather than over
    /// `self`, so [`Self::tick`] can hold the sink at the same time.
    fn transitions(
        asking_now: &mut BTreeMap<Slot, (String, i64)>,
        resolved: &BTreeMap<Slot, Resolved>,
        pages: &Pages,
        now: i64,
        out: &mut impl Write,
    ) -> io::Result<()> {
        for (slot, r) in resolved {
            let SlotState::NeedsAnswer { source } = r.state else {
                continue;
            };
            if asking_now.contains_key(slot) {
                continue; // still asking; not a transition
            }
            let _previous = asking_now.insert(*slot, (r.session.clone(), now));
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
        //
        // Collected rather than iterated because the loop below mutates
        // `asking_now`.
        let done: Vec<Slot> = asking_now
            .keys()
            .filter(|s| {
                !matches!(
                    resolved.get(s).map(|r| r.state),
                    Some(SlotState::NeedsAnswer { .. })
                )
            })
            .copied()
            .collect();
        for slot in done {
            if let Some((session, since)) = asking_now.remove(&slot) {
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
    ///
    /// The replay tests' assertion. The daemon counts the queue through
    /// [`queue::depth`](crate::queue::depth) over [`Self::waiting`]
    /// instead, because that count excludes the muted and the pad you are
    /// already looking at and this one does not.
    #[cfg(test)]
    fn asking(&self) -> usize {
        self.asking.len()
    }

    /// Every slot that is asking, with the session and the moment it
    /// started.
    ///
    /// **This is the queue** (`docs/05-interaction.md`). It needed no new
    /// state because the map already holds the *original* timestamp across
    /// a restart — which was the fix for #7 and turns out to be the
    /// ordering the top row sorts by.
    pub fn waiting(&self) -> &BTreeMap<Slot, (String, i64)> {
        &self.asking
    }

    /// Record a press of `►`, to this journal's own file.
    pub fn jumped_next(&mut self, now: i64, slot: &str, waited_ms: i64, queued: usize) {
        self.sink
            .write(|out| nav_next(out, now, slot, waited_ms, queued));
    }

    /// Record a press of a **pad**, to this journal's own file.
    pub fn jumped_pad(&mut self, now: i64, slot: &str, waited_ms: Option<i64>, queued: usize) {
        self.sink
            .write(|out| nav_pad(out, now, slot, waited_ms, queued));
    }

    /// Record a press of `◄`, to this journal's own file.
    pub fn jumped_back(&mut self, now: i64, ok: bool) {
        self.sink.write(|out| nav_back(out, now, ok));
    }
}

/// Record a press of `►` or `◄`.
///
/// Navigation is written down because whether the queue works is a
/// question about what you actually press, and no other record answers it
/// (`docs/04-signals.md#the-instrument`).
///
/// # Errors
/// Whatever the sink returns.
fn nav_next(
    out: &mut impl Write,
    now: i64,
    slot: &str,
    waited_ms: i64,
    queued: usize,
) -> io::Result<()> {
    write_line(
        out,
        &Nav {
            slot: Some(slot),
            waited_ms: Some(waited_ms),
            queued: Some(queued),
            ..Nav::press(now, "next")
        },
    )
}

/// Record a press of a pad that went somewhere.
///
/// **The gesture the product exists for**, and the one the journal did
/// not write down until
/// [#43](https://github.com/alehatsman/tally/issues/43): the gate
/// (`docs/04-signals.md`) asks how many raised ambers you went to, and
/// most of them are answered by a plain tap rather than by `►`.
///
/// `waited_ms` is `None` when the slot was **not asking**, and that is the
/// field the whole line is for rather than a missing value: present means
/// you answered an amber, absent means you went to look at something
/// quiet.
///
/// # Errors
/// Whatever the sink returns.
fn nav_pad(
    out: &mut impl Write,
    now: i64,
    slot: &str,
    waited_ms: Option<i64>,
    queued: usize,
) -> io::Result<()> {
    write_line(
        out,
        &Nav {
            slot: Some(slot),
            waited_ms,
            queued: Some(queued),
            ..Nav::press(now, "pad")
        },
    )
}

/// Record a press of `◄`, and whether it had anywhere to go.
///
/// # Errors
/// Whatever the sink returns.
fn nav_back(out: &mut impl Write, now: i64, ok: bool) -> io::Result<()> {
    write_line(
        out,
        &Nav {
            ok: Some(ok),
            ..Nav::press(now, "back")
        },
    )
}

/// A press of a navigation button.
///
/// `kind` is what separates these from state lines, and it is written on
/// every one of them so a reader never has to infer the shape from which
/// fields happen to be present.
///
/// One struct with `Option` fields, which is the opposite of what
/// [`Asked`] and [`Answered`] do, and deliberately: the three nav shapes
/// share a `kind`, a `via` discriminator and a reader, so an absent field
/// is read as "that button carries no slot" rather than as a second
/// format. A rise and a fall share only the slot.
#[derive(Debug, Serialize)]
struct Nav<'a> {
    ms: i64,
    kind: &'static str,
    via: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    slot: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    waited_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    queued: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ok: Option<bool>,
}

impl Nav<'_> {
    /// The half every nav line shares, so the three that do not apply stop
    /// being typed out as `None` at each call site.
    ///
    /// `..Nav::press(..)` cannot reorder the wire format: serde writes
    /// fields in **declaration** order, not in the order a literal lists
    /// them, and `skip_serializing_if` already governs the absent ones.
    fn press(ms: i64, via: &'static str) -> Self {
        Self {
            ms,
            kind: "nav",
            via,
            slot: None,
            waited_ms: None,
            queued: None,
            ok: None,
        }
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
/// a transition *out* of asking. [`Nav`] goes the other way on purpose,
/// and says why.
#[derive(Debug, Serialize)]
struct Answered<'a> {
    ms: i64,
    session: &'a str,
    slot: &'a str,
    /// Always `answered`; see [`Asked::to`].
    to: &'static str,
    after_ms: i64,
}

/// Enough of any journal line to replay it.
///
/// Deliberately not [`Asked`] or [`Answered`]: reading needs three fields
/// that both shapes share, and a reader that insists on the rest would
/// reject the oldest lines in the file — which predate `title` — for
/// carrying less than the current writer emits.
#[derive(Debug, Deserialize)]
struct Line {
    ms: i64,
    session: String,
    slot: String,
    to: String,
    /// Absent means `"state"`, which is what every line written before
    /// `docs/16` is. A `"nav"` line records a press, not a slot, and the
    /// replay must not read one as a transition.
    #[serde(default)]
    kind: Option<String>,
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

/// The inverse of [`address`], and it lives here for that reason.
///
/// [`crate::slot::parse`] alone is not enough: this file writes a bare page
/// *number* when the page has no name, which is not a `tl.slot` value and
/// must not become one. Both spellings have to read back or a restart
/// rebuilds half the map.
///
/// A page renamed in `slots.toml` between runs stops matching, and that is
/// correct — it is a different address, and inventing a slot for it would
/// put a transition on a pad that never asked.
fn parse_address(text: &str, pages: &Pages) -> Option<Slot> {
    if let Some(slot) = crate::slot::parse(text, pages) {
        return Some(slot);
    }
    let mut parts = text.split(':');
    let page: u8 = parts.next()?.parse().ok()?;
    let row: u8 = parts.next()?.parse().ok()?;
    let col: u8 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let ok = (1..=crate::slot::PAGES).contains(&page)
        && (1..=crate::slot::ROWS_PER_PAGE).contains(&row)
        && (1..=crate::slot::COLS).contains(&col);
    ok.then_some(Slot { page, row, col })
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_760_000_000_000;

    /// A slot, without the three field names at every call site — the
    /// same helper `queue.rs::tests` already has.
    fn at(page: u8, row: u8, col: u8) -> Slot {
        Slot { page, row, col }
    }

    /// Reading a nav line as a transition would invent a rise nobody made.
    #[test]
    fn a_nav_line_is_not_a_state_transition() {
        let pages = Pages::default();
        let text = concat!(
            r#"{"ms":1000,"session":"a","slot":"1:3:3","to":"needs_answer","source":"declared","title":null}"#,
            "\n",
            r#"{"ms":1200,"kind":"nav","via":"next","slot":"1:3:3","waited_ms":200,"queued":2}"#,
            "\n",
            r#"{"ms":1300,"kind":"nav","via":"back","ok":true}"#,
            "\n",
        );
        let j = Journal::replay(text, &pages);
        assert_eq!(j.asking(), 1, "the nav lines changed nothing");
        let (_, since) = j.waiting().get(&at(1, 3, 3)).expect("still asking");
        assert_eq!(*since, 1000, "and did not move the clock");
    }

    /// Every line written before `docs/16` has no `kind`. They must keep
    /// parsing exactly as they did, or a restart loses the queue.
    #[test]
    fn a_line_written_before_kind_existed_still_parses() {
        let pages = Pages::default();
        let text = concat!(
            r#"{"ms":1792159699962,"session":"01M52FZ9Q0ZDGR8FH5K1P1PY5T","slot":"1:3:3","to":"needs_answer","source":"derived","title":null}"#,
            "\n",
        );
        assert_eq!(Journal::replay(text, &pages).asking(), 1);
    }

    /// The queue reads this map, so the session id has to come back with
    /// the timestamp.
    #[test]
    fn waiting_carries_the_session_and_the_original_moment() {
        let pages = Pages::default();
        let text = concat!(
            r#"{"ms":4242,"session":"ABC","slot":"1:1:1","to":"needs_answer","source":"declared","title":null}"#,
            "\n",
        );
        let j = Journal::replay(text, &pages);
        let (session, since) = j.waiting().get(&at(1, 1, 1)).expect("asking");
        assert_eq!(session, "ABC");
        assert_eq!(*since, 4242);
    }

    #[test]
    fn a_nav_line_says_which_button_and_what_it_cost() {
        let mut out = Vec::new();
        nav_next(&mut out, 9_000, "1:2:1", 219_722, 4).expect("write");
        nav_back(&mut out, 9_100, false).expect("write");
        let text = String::from_utf8(out).expect("utf8");
        let mut lines = text.lines();
        let next: serde_json::Value =
            serde_json::from_str(lines.next().expect("next line")).expect("json");
        assert_eq!(next["kind"], "nav");
        assert_eq!(next["via"], "next");
        assert_eq!(next["slot"], "1:2:1");
        assert_eq!(next["waited_ms"], 219_722);
        assert_eq!(next["queued"], 4);
        assert!(next.get("ok").is_none(), "next has no ok");

        let back: serde_json::Value =
            serde_json::from_str(lines.next().expect("back line")).expect("json");
        assert_eq!(back["via"], "back");
        assert_eq!(back["ok"], false);
        assert!(back.get("slot").is_none(), "back names no slot");
    }

    /// The gesture the product exists for, and the one that wrote nothing
    /// until [#43](https://github.com/alehatsman/tally/issues/43).
    ///
    /// **The presence of `waited_ms` is the measurement**, not a detail
    /// of serialisation: with it, the tap answered an amber and the line
    /// says how long that pad had been asking; without it, the tap went
    /// somewhere nothing was waiting. `►` can only ever do the first, so
    /// no existing line can tell the two apart.
    #[test]
    fn a_pad_press_says_whether_it_answered_anything() {
        let mut out = Vec::new();
        nav_pad(&mut out, 9_200, "1:2:1", Some(219_722), 4).expect("write");
        nav_pad(&mut out, 9_300, "1:3:7", None, 0).expect("write");
        let text = String::from_utf8(out).expect("utf8");
        let mut lines = text.lines();

        let answered: serde_json::Value =
            serde_json::from_str(lines.next().expect("line")).expect("json");
        assert_eq!(answered["kind"], "nav");
        assert_eq!(answered["via"], "pad", "not next, and not back");
        assert_eq!(answered["slot"], "1:2:1");
        assert_eq!(answered["waited_ms"], 219_722);
        assert_eq!(answered["queued"], 4);

        let quiet: serde_json::Value =
            serde_json::from_str(lines.next().expect("line")).expect("json");
        assert_eq!(quiet["via"], "pad");
        assert_eq!(quiet["slot"], "1:3:7");
        assert!(
            quiet.get("waited_ms").is_none(),
            "a tap on a pad that wanted nothing must not claim a wait"
        );
        assert_eq!(quiet["queued"], 0);
    }
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
                claude: None,
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

    /// The bug from #7, reproduced from the real file: three rising edges
    /// against one fall for the same session, because each restart began
    /// with an empty map. Replaying the first line is what stops the
    /// second one being written.
    #[test]
    fn a_restart_does_not_re_ask_a_slot_that_never_stopped() {
        let slot = at(1, 3, 3);
        let pages = Pages::default();
        let written = r#"{"ms":1792158972550,"session":"01M52FZ9","slot":"1:3:3","to":"needs_answer","source":"derived"}"#;

        let mut j = Journal::replay(written, &pages);
        assert_eq!(j.asking(), 1, "the rise in the file was not replayed");

        let mut out = Vec::new();
        j.record(
            &asking_at(slot, "01M52FZ9"),
            &pages,
            1_792_159_460_320,
            &mut out,
        )
        .expect("write");
        assert!(
            lines(&out).is_empty(),
            "wrote a second rising edge: {:?}",
            lines(&out)
        );
    }

    /// The other half, and the one that makes the number mean something:
    /// `after_ms` has to span the question, not the downtime.
    #[test]
    fn after_ms_is_measured_from_the_original_question() {
        let pages = Pages::default();
        let rise = 1_000_000;
        let written =
            format!(r#"{{"ms":{rise},"session":"01J","slot":"1:3:3","to":"needs_answer"}}"#);

        let mut j = Journal::replay(&written, &pages);
        let mut out = Vec::new();
        j.record(&BTreeMap::new(), &pages, rise + 90_347, &mut out)
            .expect("write");

        let line = lines(&out).pop().expect("a falling edge");
        assert!(line.contains(r#""after_ms":90347"#), "{line}");
        assert_eq!(j.asking(), 0);
    }

    /// An answered slot must not come back as asking, or every restart
    /// resurrects every question ever asked.
    #[test]
    fn the_last_transition_per_slot_is_the_one_that_counts() {
        let pages = Pages::default();
        let written = concat!(
            r#"{"ms":1,"session":"A","slot":"1:3:3","to":"needs_answer"}"#,
            "\n",
            r#"{"ms":2,"session":"A","slot":"1:3:3","to":"answered","after_ms":1}"#,
            "\n",
            r#"{"ms":3,"session":"B","slot":"1:6:2","to":"needs_answer"}"#,
            "\n",
        );
        let j = Journal::replay(written, &pages);
        assert_eq!(j.asking(), 1, "only 1:6:2 is still asking");
    }

    /// A SIGKILL mid-write leaves a half line. Losing one transition beats
    /// refusing to recover.
    #[test]
    fn a_truncated_last_line_is_skipped_not_fatal() {
        let pages = Pages::default();
        let written = concat!(
            r#"{"ms":1,"session":"A","slot":"1:3:3","to":"needs_answer"}"#,
            "\n",
            r#"{"ms":2,"session":"B","slot":"1:6:"#,
        );
        assert_eq!(Journal::replay(written, &pages).asking(), 1);
    }

    /// Both spellings `address` can write have to read back, or a restart
    /// rebuilds half the map. Named pages and bare numbers.
    #[test]
    fn both_address_spellings_replay() {
        let named = Pages::new(["work".to_owned()]);
        let written = r#"{"ms":1,"session":"A","slot":"work:3:3","to":"needs_answer"}"#;
        assert_eq!(Journal::replay(written, &named).asking(), 1);

        let numbered = r#"{"ms":1,"session":"A","slot":"1:3:3","to":"needs_answer"}"#;
        assert_eq!(Journal::replay(numbered, &Pages::default()).asking(), 1);
    }

    /// Off the grid is ignored, not clamped — the same rule a `tl.slot`
    /// label follows (`docs/02-model.md#limits`).
    #[test]
    fn an_address_off_the_grid_is_ignored() {
        let pages = Pages::default();
        for slot in ["9:1:1", "1:9:1", "1:1:0", "1:2:3:4", "nope"] {
            let written =
                format!(r#"{{"ms":1,"session":"A","slot":"{slot}","to":"needs_answer"}}"#);
            assert_eq!(
                Journal::replay(&written, &pages).asking(),
                0,
                "{slot} was accepted"
            );
        }
    }

    /// No file is the first run, which is normal rather than an error.
    #[test]
    fn a_missing_journal_resumes_empty() {
        let missing = Path::new("/nonexistent/tally/attention.jsonl");
        assert_eq!(Journal::resumed(missing, &Pages::default()).asking(), 0);
    }

    /// The SIGKILL that truncates a line does not stop at a character.
    /// `Sink` is unbuffered, so a short kernel write copies bytes, and the
    /// torn tail can sit inside a multi-byte sequence in a title.
    /// `a_truncated_last_line_is_skipped_not_fatal` only passes because it
    /// cuts on an ASCII boundary; this one cuts `é` in half, which
    /// `read_to_string` refused for the whole file — every slot forgotten,
    /// every one of them re-raised on the next tick.
    #[test]
    fn a_line_torn_mid_character_costs_one_line_not_the_file() {
        let path = std::env::temp_dir().join("tally-resume-torn-utf8.jsonl");
        let mut written = br#"{"ms":1,"session":"A","slot":"1:3:3","to":"needs_answer"}"#.to_vec();
        written.push(b'\n');
        written.extend_from_slice(br#"{"ms":2,"session":"B","slot":"1:6:2","title":"caf"#);
        written.push(0xC3); // the lead byte of `é`; the kernel stopped here
        std::fs::write(&path, &written).expect("seed");

        let j = Journal::resumed(&path, &Pages::default());
        assert_eq!(j.asking(), 1, "the intact line was thrown away too");
        std::fs::remove_file(&path).expect("clean up");
    }

    /// Not-found is the first run; every other read failure is a fault
    /// that still resumes empty, and [`Journal::resumed`] says why. The
    /// stderr report is not something a test can assert — what is locked
    /// here is that the fault path exists and stays survivable.
    ///
    /// Through `resumed` and not [`Journal::open`], which
    /// `an_unopenable_journal_is_still_a_journal` covers: the read half
    /// has to survive on its own, whatever the sink did.
    #[test]
    fn an_unreadable_journal_is_not_a_first_run() {
        // A directory is readable as a path and never as a file: EISDIR.
        let dir = std::env::temp_dir();
        assert_eq!(Journal::resumed(&dir, &Pages::default()).asking(), 0);
    }

    /// [`Journal::open`] resumes *before* it opens the sink. This passes
    /// under either order today, because [`Sink::open`] only appends — it
    /// is the guard that fails the day it stops being append-only, which
    /// is the day the wrong order eats the history.
    #[test]
    fn an_opened_journal_resumes_the_file_it_appends_to() {
        let path = std::env::temp_dir().join("tally-open-resumes.jsonl");
        std::fs::write(
            &path,
            br#"{"ms":1,"session":"A","slot":"1:3:3","to":"needs_answer"}"#,
        )
        .expect("seed");

        let j = Journal::open(&path, &Pages::default());
        assert_eq!(j.asking(), 1, "open forgot the history it appends to");
        assert!(j.sink.is_open(), "and it still writes");
        std::fs::remove_file(&path).expect("clean up");
    }

    #[test]
    fn a_pad_that_starts_asking_is_recorded_once() {
        let slot = at(1, 3, 5);
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
        let slot = at(1, 3, 5);
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
        let slot = at(1, 1, 1);
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
        let slot = at(1, 2, 4);
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
        let slot = at(1, 2, 4);
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
                claude: None,
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
        let slot = at(1, 1, 1);
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
        assert!(out.is_empty(), "{out:?}");
    }

    /// The sink's one rule, and the reason it is a type: a write that
    /// fails retires it and the daemon carries on
    /// (`docs/08-daemon.md`).
    ///
    /// The failure is made with a **read-only** handle, which is the
    /// cheapest honest way to get `write` to return an error without a
    /// full disk. What is asserted is not the error — it is that the
    /// sink closed itself and that writing to it afterwards is a no-op
    /// rather than a panic or a second report per press.
    #[test]
    fn a_failed_write_retires_the_sink_and_nothing_else() {
        let path = std::env::temp_dir().join("tally-sink-retires.jsonl");
        std::fs::write(&path, b"").expect("seed");
        let mut sink = Sink(Some(File::open(&path).expect("read-only handle")));
        assert!(sink.is_open());

        sink.write(|out| nav_back(out, 1, true));
        assert!(!sink.is_open(), "a failed write closes the sink");

        // Still callable, still silent, still writes nothing.
        sink.write(|out| nav_back(out, 2, true));
        assert_eq!(std::fs::read(&path).expect("read").len(), 0);
        std::fs::remove_file(&path).expect("clean up");
    }

    /// A journal that could not open its file still resumes its map. The
    /// grid is the half that is load-bearing; the log is not.
    #[test]
    fn an_unopenable_journal_is_still_a_journal() {
        // A directory is never an appendable file.
        let dir = std::env::temp_dir();
        let j = Journal::open(&dir, &Pages::default());
        assert!(!j.sink.is_open());
        assert_eq!(j.asking(), 0);
    }

    /// The nav lines are a **persisted** schema: F0 is computed from this
    /// file and its history, and the other tests here assert field
    /// *presence*, which a reordering would survive. This one pins the
    /// bytes, because [`Nav::press`] is a struct-update base and the
    /// obvious worry about one is that it moves a field.
    #[test]
    fn a_nav_line_is_written_byte_for_byte_as_it_always_was() {
        let mut out = Vec::new();
        nav_next(&mut out, 9_000, "1:2:1", 219_722, 4).expect("write");
        nav_pad(&mut out, 9_200, "1:2:1", Some(219_722), 4).expect("write");
        nav_pad(&mut out, 9_300, "1:3:7", None, 0).expect("write");
        nav_back(&mut out, 9_100, false).expect("write");
        assert_eq!(
            String::from_utf8(out).expect("utf8"),
            concat!(
                r#"{"ms":9000,"kind":"nav","via":"next","slot":"1:2:1","waited_ms":219722,"queued":4}"#,
                "\n",
                r#"{"ms":9200,"kind":"nav","via":"pad","slot":"1:2:1","waited_ms":219722,"queued":4}"#,
                "\n",
                r#"{"ms":9300,"kind":"nav","via":"pad","slot":"1:3:7","queued":0}"#,
                "\n",
                r#"{"ms":9100,"kind":"nav","via":"back","ok":false}"#,
                "\n",
            )
        );
    }
}
