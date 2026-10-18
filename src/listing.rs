//! The grid as text, for a reader that cannot see colour.
//!
//! The device has rendered sixty-four pads since M0 and never rendered
//! one as a character. An agent *on* a pad has no address and no map, and
//! six tool calls is what working that out by hand cost
//! (`docs/32-the-grid-in-text.md`).
//!
//! Pure, and in the library rather than the binary for the reason
//! [`crate::cli`] gives: the rules are worth testing without spawning a
//! process. Nothing here talks to anchord — it is handed a resolved grid
//! and says what it looks like.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::anchor::Session;
use crate::model::Resolved;
use crate::signal::SlotState;
use crate::slot::{Pages, Slot};

/// One occupied pad, with the facts a listing prints.
///
/// Assembled rather than taken from [`Resolved`] because `Resolved` has
/// no `cwd` — the grid never needed one, since a pad shows a colour and
/// not a path. The directory lives on the [`Session`], so a listing is a
/// join, and doing it here keeps `model` unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pad {
    /// Where it is.
    pub at: Slot,
    /// The session drawn on it.
    pub session: String,
    /// What the pad renders.
    pub state: SlotState,
    /// Where that session was started. `None` if anchord did not say.
    pub cwd: Option<String>,
    /// The conversation it is running, from `tl.claude`.
    pub claude: Option<String>,
}

/// Where the caller is, as far as a listing is concerned.
///
/// Three states and not an `Option<Slot>`, because the two ways of having
/// no address are not the same answer and the middle one is the useful
/// one: a session that let go of its pad is still a session you can label
/// back onto the grid, and it has an id worth printing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Caller<'a> {
    /// Not under anchor. The common case, and not an error: the grid is a
    /// thing you can look at from outside.
    Outside,
    /// Under anchor, holding no readable `tl.slot`.
    Unplaced(&'a str),
    /// On a pad.
    At(Slot),
}

/// Join the drawn grid against the session list, newest-wins already
/// applied by [`crate::model::resolve`].
///
/// In address order, which is the order a reader scans and the order the
/// picture above it is drawn in.
#[must_use]
pub fn pads(drawn: &BTreeMap<Slot, Resolved>, sessions: &[Session]) -> Vec<Pad> {
    drawn
        .values()
        .map(|held| Pad {
            at: held.slot,
            session: held.session.clone(),
            state: held.state,
            cwd: sessions
                .iter()
                .find(|s| s.id == held.session)
                .and_then(|s| s.cwd.clone()),
            claude: held.claude.clone(),
        })
        .collect()
}

/// How one slot is spelled, for a sentence rather than a pad.
///
/// **One vocabulary for occupancy.** `tally spawn` refuses with these
/// words and `tally ls` lists with them, so a pad that will not take a
/// spawn is described the same way in both places.
///
/// **`NeedsAnswer` is `wants you`, and the vagueness is the accurate
/// part.** `tl.attention` is raised by two hooks and the write is
/// hook-agnostic, so the label carries two states under one bit: *blocked*
/// on a dialog (`Notification`) and *finished with its turn*
/// (`Stop`). `Stop` fires every turn and `Notification` rarely, so a
/// finished turn is the common case and "waiting for an answer" -- what
/// this said until `ls` acquired readers that are programs -- was false
/// most of the times it appeared (`docs/14-attention.md`,
/// <https://github.com/alehatsman/tally/issues/23>). Whether the two
/// should be told apart at all is that issue's question; until it is
/// answered, this says only what the label actually means.
///
/// `Working` and `Quiet` are both `running`, which is what the device
/// says too — one green for alive (`crate::render::slot`). Here it is
/// also the only honest answer: the working/quiet split is derived from
/// PTY bytes by watchers that only tallyd runs, so a CLI resolving with
/// no `derived` map gets `Quiet` for every live session and would be
/// claiming an idleness it has no way to observe.
#[must_use]
pub fn words(state: SlotState) -> String {
    match state {
        SlotState::Empty => "nothing".to_owned(),
        SlotState::Working | SlotState::Quiet => "running".to_owned(),
        SlotState::NeedsAnswer { .. } => "wants you".to_owned(),
        SlotState::Failed { code } => format!("failed, exit {code}"),
        SlotState::Lost { reclaimable: true } => "lost to an anchord restart".to_owned(),
        SlotState::Lost { reclaimable: false } => "lost, and may still be running".to_owned(),
    }
}

/// The character a pad is drawn with.
///
/// **One glyph per colour the device uses**, and that correspondence is
/// the point rather than a convenience: the CLI already refuses to form
/// its own opinion about what a pad holds, and a listing with its own
/// vocabulary would be a third one (`docs/11-cli.md`).
#[must_use]
pub fn glyph(state: SlotState) -> char {
    match state {
        // off
        SlotState::Empty => '.',
        // green
        SlotState::Working | SlotState::Quiet => '#',
        // amber, pulsing
        SlotState::NeedsAnswer { .. } => '?',
        // red
        SlotState::Failed { .. } => '!',
        // dark red
        SlotState::Lost { .. } => 'x',
    }
}

/// The marker on the caller's own pad.
///
/// It replaces a glyph rather than sitting beside one, which costs
/// nothing: the session running this command is alive by construction, so
/// `@` hides only `#` — and the table below carries that row's state in
/// words anyway, including the one case where the caller raised its own
/// attention.
const YOU: char = '@';

/// The width the state column is padded to, so the directories line up.
/// The longest state is `lost, and may still be running`.
const STATE_WIDTH: usize = 30;

/// The grid, the key, and a line per occupied pad.
///
/// Rows holding something, plus the caller's row, in address order.
/// Sixty-four rows of dots is wallpaper, not a listing — and the trailing
/// line makes that omission a complete statement rather than a silent cap.
#[must_use]
pub fn text(pads: &[Pad], caller: Caller<'_>, pages: &Pages) -> String {
    let by_slot: BTreeMap<Slot, &Pad> = pads.iter().map(|p| (p.at, p)).collect();
    let rows = shown(pads, caller);
    let mut out = String::new();

    // The row label carries the page name, so the address a reader has to
    // type is the label and the column heading joined -- no arithmetic,
    // which was half of what the six tool calls were doing.
    let labels: BTreeMap<u8, String> = rows
        .iter()
        .filter_map(|&abs| Some((abs, row_label(pages, abs)?)))
        .collect();
    let width = labels.values().map(String::len).max().unwrap_or(3);

    let _header = writeln!(out, "  {:width$}  1  2  3  4  5  6  7  8", "");
    for abs in &rows {
        let Some(label) = labels.get(abs) else {
            continue; // an undeclared page has no name to print
        };
        let _row = write!(out, "  {label:width$}");
        for col in 1..=8 {
            let mark = match Slot::at_absolute(*abs, col) {
                Some(at) if caller == Caller::At(at) => YOU,
                Some(at) => by_slot.get(&at).map_or('.', |p| glyph(p.state)),
                // Past the end of the address space. Cannot happen for a
                // row we are drawing, and a dot is the honest answer.
                None => '.',
            };
            let _cell = write!(out, "  {mark}");
        }
        let _end = writeln!(out);
    }

    let _key = writeln!(
        out,
        "\n  {YOU} you   # running   ? wants you   ! failed   x lost   . free"
    );

    if !pads.is_empty() {
        let _blank = writeln!(out);
    }
    for pad in pads {
        let Some(address) = pad.at.label(pages) else {
            continue;
        };
        let mine = if caller == Caller::At(pad.at) {
            "  <- you"
        } else {
            ""
        };
        // The whole session id, not an elision: this is the argument
        // `anchor kill` and `anchor label` take, and a listing a reader
        // cannot act on is a listing that costs another command.
        let _line = writeln!(
            out,
            "  {address}  {}  {:STATE_WIDTH$}  {}{mine}",
            pad.session,
            words(pad.state),
            pad.cwd.as_deref().unwrap_or("-"),
        );
    }

    // Said rather than implied. Every row absent from the picture is
    // empty, and a reader that wants one can address it.
    let _omitted = writeln!(out, "\n  rows not shown hold nothing");

    if let Caller::Unplaced(id) = caller {
        // Naming the id is the point: it is the argument
        // `anchor label <id> tl.slot=…` needs to put this session on the
        // grid.
        let _line = writeln!(out, "  you are in {id}, which holds no pad");
    }
    out
}

/// Which absolute rows the picture shows.
fn shown(pads: &[Pad], caller: Caller<'_>) -> BTreeSet<u8> {
    let mut rows: BTreeSet<u8> = pads.iter().map(|p| p.at.absolute_row()).collect();
    if let Caller::At(at) = caller {
        let _mine = rows.insert(at.absolute_row());
    }
    rows
}

/// `page:row` — an address with its column left off.
fn row_label(pages: &Pages, abs: u8) -> Option<String> {
    let at = Slot::at_absolute(abs, 1)?;
    Some(format!("{}:{}", pages.name_of(at.page)?, at.row))
}

/// The same grid, for a caller that would otherwise parse a picture.
///
/// Occupied pads only — a free pad is the absence of an entry, and
/// sixty-four objects saying "nothing" is not a format.
///
/// `free` is stated rather than left to be inferred from `state`: it is
/// the one thing a reader is deciding on, and re-deriving
/// [`SlotState::frees_the_pad`] from a string would be exactly the second
/// opinion this module exists to avoid.
#[must_use]
pub fn json(pads: &[Pad], caller: Caller<'_>, pages: &Pages) -> String {
    let entries: Vec<serde_json::Value> = pads
        .iter()
        .filter_map(|pad| {
            let mut entry = serde_json::json!({
                "slot": pad.at.label(pages)?,
                "session": pad.session,
                "state": words(pad.state),
                "free": pad.state.frees_the_pad(),
            });
            let map = entry.as_object_mut()?;
            for (key, value) in [("cwd", pad.cwd.clone()), ("claude", pad.claude.clone())] {
                if let Some(value) = value {
                    let _set = map.insert(key.to_owned(), value.into());
                }
            }
            if let SlotState::Failed { code } = pad.state {
                let _set = map.insert("exit_code".to_owned(), code.into());
            }
            Some(entry)
        })
        .collect();
    let doc = serde_json::json!({
        "you": match caller {
            Caller::At(at) => serde_json::Value::from(at.label(pages)),
            Caller::Outside | Caller::Unplaced(_) => serde_json::Value::Null,
        },
        "unplaced": match caller {
            Caller::Unplaced(id) => serde_json::Value::from(id),
            Caller::Outside | Caller::At(_) => serde_json::Value::Null,
        },
        "pads": entries,
    });
    format!("{doc:#}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pages() -> Pages {
        Pages::default()
    }

    fn at(page: u8, row: u8, col: u8) -> Slot {
        Slot { page, row, col }
    }

    fn pad(id: &str, at: Slot, state: SlotState, cwd: &str) -> Pad {
        Pad {
            at,
            session: id.to_owned(),
            state,
            cwd: Some(cwd.to_owned()),
            claude: None,
        }
    }

    fn grid() -> Vec<Pad> {
        vec![
            pad("s1", at(1, 1, 1), SlotState::Quiet, "/repo/one"),
            pad("s2", at(1, 1, 2), SlotState::Quiet, "/repo/two"),
            pad(
                "s3",
                at(1, 3, 1),
                SlotState::Failed { code: 1 },
                "/repo/dead",
            ),
        ]
    }

    /// The whole point of the picture: the address a reader must type is
    /// the row label and a column heading, with no arithmetic in between.
    #[test]
    fn the_picture_puts_the_caller_on_a_row_whose_label_is_its_address() {
        let out = text(&grid(), Caller::At(at(1, 1, 4)), &pages());
        assert!(out.contains("  1:1  #  #  .  @  .  .  .  ."), "{out}");
        assert!(out.contains("  1:3  !  .  .  .  .  .  .  ."), "{out}");
        // Row 2 holds nothing and nobody is standing on it.
        assert!(!out.contains("  1:2  "), "{out}");
        assert!(out.contains("rows not shown hold nothing"), "{out}");
    }

    /// One glyph per colour the device uses. A break here is the text and
    /// the hardware disagreeing about what a pad holds.
    #[test]
    fn every_state_has_the_glyph_its_colour_has() {
        assert_eq!(glyph(SlotState::Empty), '.');
        assert_eq!(glyph(SlotState::Working), '#');
        assert_eq!(glyph(SlotState::Quiet), '#');
        assert_eq!(
            glyph(SlotState::NeedsAnswer {
                source: crate::signal::Source::Declared
            }),
            '?'
        );
        assert_eq!(glyph(SlotState::Failed { code: 1 }), '!');
        assert_eq!(glyph(SlotState::Lost { reclaimable: true }), 'x');
        assert_eq!(glyph(SlotState::Lost { reclaimable: false }), 'x');
    }

    /// The CLI resolves with no watchers, so every live session arrives
    /// here as `Quiet`. Saying "running, idle" would be claiming an
    /// idleness nothing in this process observed.
    #[test]
    fn a_live_session_is_running_and_never_idle() {
        assert_eq!(words(SlotState::Quiet), "running");
        assert_eq!(words(SlotState::Working), "running");
        assert!(!words(SlotState::Quiet).contains("idle"));
    }

    /// The label means "blocked on a dialog" **or** "finished its turn",
    /// and nothing downstream of the hook can tell which (`docs/14`, #23).
    /// A word that names only the first is wrong in the common case, and
    /// `tally ls` is read by programs that act on it.
    #[test]
    fn attention_does_not_claim_a_question_is_pending() {
        let declared = words(SlotState::NeedsAnswer {
            source: crate::signal::Source::Declared,
        });
        let derived = words(SlotState::NeedsAnswer {
            source: crate::signal::Source::Derived,
        });
        assert_eq!(declared, "wants you");
        assert_eq!(declared, derived, "the source is not the reader's business");
        assert!(
            !declared.contains("question") && !declared.contains("answer"),
            "a finished turn is not waiting for one: {declared}"
        );
    }

    /// Reclaimable and not are the same colour on the device and the same
    /// glyph here, so the words are the only place the difference is
    /// readable — and it is the difference that decides a spawn.
    #[test]
    fn the_two_kinds_of_lost_read_differently_even_though_they_look_alike() {
        assert_eq!(glyph(SlotState::Lost { reclaimable: true }), 'x');
        assert_eq!(glyph(SlotState::Lost { reclaimable: false }), 'x');
        assert_ne!(
            words(SlotState::Lost { reclaimable: true }),
            words(SlotState::Lost { reclaimable: false })
        );
        assert!(SlotState::Lost { reclaimable: true }.frees_the_pad());
        assert!(!SlotState::Lost { reclaimable: false }.frees_the_pad());
    }

    /// Not under anchor is not an error and not a blank page: the grid
    /// prints, and the only thing missing is the marker.
    #[test]
    fn a_caller_outside_the_grid_still_gets_the_grid() {
        let out = text(&grid(), Caller::Outside, &pages());
        assert!(out.contains("  1:1  #  #  .  .  .  .  .  ."), "{out}");
        // The key always spells the marker out; no pad wears it.
        assert_eq!(out.matches(YOU).count(), 1, "{out}");
        assert!(!out.contains("<- you"), "{out}");
        assert!(!out.contains("holds no pad"), "{out}");
    }

    /// A session that let go of its pad is still a session you can label
    /// back onto one, so the line names the id that `anchor label` takes.
    #[test]
    fn a_caller_holding_no_pad_is_told_so_by_id() {
        let out = text(&grid(), Caller::Unplaced("01M53KT"), &pages());
        assert!(
            out.contains("you are in 01M53KT, which holds no pad"),
            "{out}"
        );
        // The key spells the marker out, and nothing else does: a session
        // with no pad has no pad to mark.
        assert_eq!(out.matches(YOU).count(), 1, "{out}");
    }

    /// The table is what a reader acts on, so the id in it has to be the
    /// one `anchor kill` and `anchor label` take — whole, not elided.
    #[test]
    fn the_table_names_each_pad_its_session_and_its_directory() {
        let out = text(&grid(), Caller::At(at(1, 1, 1)), &pages());
        assert!(out.contains("1:1:1  s1  running"), "{out}");
        assert!(out.contains("/repo/one"), "{out}");
        assert!(out.contains("1:3:1  s3  failed, exit 1"), "{out}");
        assert!(out.contains("<- you"), "{out}");
    }

    /// Page names, not indices: a row label is the first two parts of an
    /// address and has to read like one.
    #[test]
    fn named_pages_label_the_rows_they_name() {
        let named = Pages::new(["work".to_owned(), "infra".to_owned()]);
        let pads = [pad("s", at(2, 4, 3), SlotState::Quiet, "/repo")];
        let out = text(&pads, Caller::Outside, &named);
        assert!(out.contains("  infra:4  .  .  #"), "{out}");
        assert!(out.contains("infra:4:3  s  running"), "{out}");
    }

    #[test]
    fn an_empty_grid_still_prints_the_key_and_says_so() {
        let out = text(&[], Caller::Outside, &pages());
        assert!(out.contains("1  2  3  4  5  6  7  8"), "{out}");
        assert!(out.contains("rows not shown hold nothing"), "{out}");
    }

    /// `free` is stated, because it is the one thing a reader decides on
    /// and re-deriving it from the state string would be a second opinion.
    #[test]
    fn json_states_free_rather_than_leaving_it_to_be_inferred() {
        let out = json(&grid(), Caller::At(at(1, 1, 1)), &pages());
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid json");
        assert_eq!(parsed["you"], "1:1:1");
        assert_eq!(parsed["unplaced"], serde_json::Value::Null);
        let pads = parsed["pads"].as_array().expect("an array");
        assert_eq!(pads.len(), 3);
        assert_eq!(pads[0]["slot"], "1:1:1");
        assert_eq!(pads[0]["free"], false);
        assert_eq!(pads[0]["cwd"], "/repo/one");
        // The failed pad is claimable, and carries the code that says why.
        assert_eq!(pads[2]["slot"], "1:3:1");
        assert_eq!(pads[2]["free"], true);
        assert_eq!(pads[2]["exit_code"], 1);
    }

    #[test]
    fn json_reports_a_caller_that_holds_no_pad_without_inventing_an_address() {
        let out = json(&grid(), Caller::Unplaced("01M53KT"), &pages());
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid json");
        assert_eq!(parsed["you"], serde_json::Value::Null);
        assert_eq!(parsed["unplaced"], "01M53KT");
    }
}
