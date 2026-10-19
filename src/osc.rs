//! An incremental scanner for the OSC sequences PTY output carries, ported
//! from `teleport/daemon/src/session/osc.rs` with the OSC 8 half removed —
//! a Claude resume id is teleport's business, and writing its prefix is
//! something `docs/01-scope.md` forbids.
//!
//! **This is not here for the title.** It is here because `BEL` (`0x07`) is
//! an OSC *string terminator*, so `ESC ] 0 ; some title BEL` contains a BEL
//! that is not a bell. An agent that sets its terminal title — Claude Code
//! does, constantly — would otherwise look like one that rang and went
//! quiet, and the pad would flash amber for nothing. That is exactly the
//! "wrong amber" M1 exists to count, so the scanner earns its place in the
//! attention path rather than in the title-scrolling one.
//!
//! Not a general VT parser. What it knows is exactly this: OSC is parsed,
//! and the other four *string* sequences — DCS (`ESC P`), SOS (`ESC X`),
//! PM (`ESC ^`) and APC (`ESC _`) — are skipped to their terminator
//! without being read. Those four earn the skip because their payloads are
//! arbitrary bytes rather than text, so a `0x07` inside one (tmux
//! passthrough, sixel) would otherwise be scanned as a ground-state ring.
//! Everything else — CSI, charset designators, single shifts — is left to
//! run through ground state, which is safe because their payloads are
//! printable and can hold neither a `BEL` nor an `ESC`.
//!
//! The usual caveat, inherited: this rests on Claude Code's current output
//! format, which is not a documented or versioned contract. A release that
//! changes it degrades to "no title" — never a parse error, never a broken
//! session.

/// Caps how much of one OSC payload is ever buffered. A stream that opens a
/// sequence and never closes it must not grow this without bound; real
/// titles are a few dozen bytes.
const MAX_PAYLOAD_BYTES: usize = 512;

const BEL: u8 = 0x07;
const ESC: u8 = 0x1b;

#[derive(Debug, Default, Clone, Copy)]
enum State {
    #[default]
    Ground,
    /// Saw `ESC`; still deciding whether this is an OSC at all.
    Esc,
    /// Saw `ESC ]`; accumulating the numeric parameter before the first `;`.
    Param(u16),
    /// Inside a string sequence, reading towards its terminator.
    /// `Some(code)` is capturing the payload into [`Scanner::buf`];
    /// `None` is reading it and throwing it away.
    ///
    /// **Abandoning a sequence is not the same as leaving it.** Every arm
    /// that gives up mid-sequence lands on `Payload(None)` rather than in
    /// `Ground`, because the rest of that sequence is still escape-sequence
    /// bytes and the `BEL` that closes it is still a terminator. Dropping
    /// straight to `Ground` counted that `BEL` as a ring — a clipboard
    /// write over the payload cap, or a `tmux` passthrough, flashed the pad
    /// amber for a session that rang nothing. `None` also clears the
    /// buffer, so it costs nothing and the no-unbounded-growth property is
    /// unchanged.
    Payload(Option<u16>),
    /// Mid-payload, just saw `ESC` — is it `ESC \\` (String Terminator) or a
    /// byte that happens to be `0x1b`?
    PayloadEsc(Option<u16>),
}

/// What one chunk contained.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Scan {
    /// Titles completed in this chunk, in order.
    pub titles: Vec<String>,
    /// Index of the last **real** bell — one seen in ground state, not one
    /// consumed as an OSC terminator. `None` if the chunk rang no bell.
    pub last_bell: Option<usize>,
}

/// Chunk-boundary-safe scanner. One per watched session.
///
/// `Clone` so a [`crate::signal::Derived`] can be snapshotted for
/// rendering without locking the whole watcher map for the duration. A
/// cloned scanner keeps its mid-sequence state, which is the only correct
/// thing to copy — a half-read title must not become two.
#[derive(Debug, Default, Clone)]
pub struct Scanner {
    state: State,
    /// The payload of the sequence currently open, when it is one we are
    /// capturing. Lives on the scanner rather than in `State` so that a
    /// sequence can be abandoned mid-payload without moving a `Vec`
    /// between two variants.
    buf: Vec<u8>,
}

impl Scanner {
    /// Fold one chunk of raw PTY output in.
    pub fn feed(&mut self, chunk: &[u8]) -> Scan {
        let mut scan = Scan::default();
        for (at, &b) in chunk.iter().enumerate() {
            match self.state {
                State::Ground => match b {
                    ESC => self.state = State::Esc,
                    // The only place a BEL means what it says.
                    BEL => scan.last_bell = Some(at),
                    _ => {}
                },
                State::Esc => match b {
                    b']' => self.state = State::Param(0),
                    // DCS, SOS, PM, APC. Not parsed, but not ignored
                    // either: their payloads are arbitrary bytes, and a
                    // `tmux` passthrough carrying a `0x07` read as a ring
                    // in ground state.
                    b'P' | b'X' | b'^' | b'_' => self.abandon(),
                    // `ESC ESC` is a fresh start, not an escape that needs
                    // consuming — staying here is how the byte gets
                    // re-dispatched without the loop having to back up.
                    // Dropping it lost the `]` of the sequence behind it.
                    ESC => {}
                    _ => self.state = State::Ground,
                },
                State::Param(code) => match b {
                    b'0'..=b'9' => {
                        self.state = State::Param(
                            code.saturating_mul(10).saturating_add(u16::from(b - b'0')),
                        );
                    }
                    b';' => self.state = State::Payload(Some(code)),
                    // Malformed. Stop guessing at the parameter, but keep
                    // reading as a sequence to its end: the BEL that closes
                    // it is still a terminator, however badly formed what
                    // came before it was.
                    _ => self.abandon(),
                },
                State::Payload(code) => match b {
                    BEL => {
                        self.take_title(code, &mut scan);
                        self.ground();
                    }
                    ESC => self.state = State::PayloadEsc(code),
                    // Already abandoned: read the bytes, keep none.
                    _ if code.is_none() => {}
                    _ if self.buf.len() < MAX_PAYLOAD_BYTES => self.buf.push(b),
                    // Far longer than any real title; abandon the payload
                    // rather than buffer forever — but skip to the real end
                    // of the sequence, because OSC 52 clipboard writes blow
                    // this cap routinely and end in a BEL that is not a
                    // ring.
                    _ => self.abandon(),
                },
                State::PayloadEsc(code) => match b {
                    b'\\' => {
                        self.take_title(code, &mut scan);
                        self.ground();
                    }
                    // BEL closes an OSC string too, so the sequence really
                    // is over; the stray ESC was payload junk and the title
                    // is not trustworthy.
                    BEL => self.ground(),
                    // Still an ESC — the `\` may yet be next, and the
                    // sequence is not over until one of its terminators
                    // arrives. The capture goes, because an ESC in the
                    // middle of a payload makes the title untrustworthy
                    // for the same reason the BEL arm above drops it. All
                    // three places that can see a doubled ESC now hold it
                    // rather than dropping it; they used to disagree, and
                    // `ESC ] 0 ; x ESC ESC \` left the sequence open until
                    // the next terminator because of it.
                    ESC => {
                        self.buf.clear();
                        self.state = State::PayloadEsc(None);
                    }
                    // That ESC was not a terminator, so the sequence is
                    // still open and now malformed. Same reason as the arms
                    // above: skip to its end rather than read the rest of
                    // it as output.
                    _ => self.abandon(),
                },
            }
        }
        scan
    }

    /// Give up on this sequence's payload without leaving the sequence.
    /// See [`State::Payload`] for why the difference matters.
    fn abandon(&mut self) {
        self.buf.clear();
        self.state = State::Payload(None);
    }

    /// The sequence is over; nothing is retained between sequences.
    fn ground(&mut self) {
        self.buf.clear();
        self.state = State::Ground;
    }

    /// Emit the buffered payload as a title if this sequence was being
    /// captured and its code is one of the title codes.
    fn take_title(&self, code: Option<u16>, scan: &mut Scan) {
        if let Some(title) = code.and_then(|c| finish(c, &self.buf)) {
            scan.titles.push(title);
        }
    }
}

impl Scanner {
    /// How many payload bytes are currently buffered mid-sequence.
    ///
    /// Only the no-unbounded-growth test uses this: the interesting claim
    /// is that a stream of ordinary output leaves nothing retained here.
    #[cfg(test)]
    pub(crate) fn buffered(&self) -> usize {
        self.buf.len()
    }
}

/// OSC 0 and 2 are "set window title"; 0 sets the icon name too, which
/// nothing here has a use for.
fn finish(code: u16, buf: &[u8]) -> Option<String> {
    matches!(code, 0 | 2).then(|| String::from_utf8_lossy(buf).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(chunks: &[&[u8]]) -> Scan {
        let mut s = Scanner::default();
        let mut last = Scan::default();
        for c in chunks {
            last = s.feed(c);
        }
        last
    }

    #[test]
    fn a_title_arrives_whole() {
        let got = scan(&[b"\x1b]0;\xe2\x9c\xb3 Claude Code\x07"]);
        assert_eq!(got.titles, ["✳ Claude Code"]);
    }

    /// The reason this module exists in the attention path at all.
    #[test]
    fn a_titles_terminator_is_not_a_bell() {
        let got = scan(&[b"\x1b]0;working on it\x07"]);
        assert_eq!(got.last_bell, None, "a title's BEL must not read as a ring");
        assert_eq!(got.titles.len(), 1);
    }

    #[test]
    fn a_real_bell_is_still_a_bell() {
        let got = scan(&[b"what next?\x07"]);
        assert_eq!(got.last_bell, Some(10));
        assert!(got.titles.is_empty(), "{:?}", got.titles);
    }

    #[test]
    fn a_title_and_then_a_real_bell_in_one_chunk() {
        let chunk = b"\x1b]0;tally\x07ready?\x07";
        let got = scan(&[chunk]);
        assert_eq!(got.titles, ["tally"]);
        // The last BEL is the real one, at the very end.
        assert_eq!(got.last_bell, Some(chunk.len() - 1));
    }

    /// The chunk boundary is the whole reason this is a state machine.
    #[test]
    fn a_title_split_across_chunks_is_one_title() {
        let mut s = Scanner::default();
        let opened = s.feed(b"\x1b]0;edit tests/");
        assert!(opened.titles.is_empty(), "{:?}", opened.titles);
        let continued = s.feed(b"pty.rs?");
        assert!(continued.titles.is_empty(), "{:?}", continued.titles);
        let got = s.feed(b"\x07");
        assert_eq!(got.titles, ["edit tests/pty.rs?"]);
        assert_eq!(
            got.last_bell, None,
            "still a terminator, even across chunks"
        );
    }

    #[test]
    fn the_string_terminator_form_works_too() {
        let got = scan(&[b"\x1b]2;via ST\x1b\\"]);
        assert_eq!(got.titles, ["via ST"]);
        assert_eq!(got.last_bell, None);
    }

    #[test]
    fn other_osc_codes_are_skipped_not_misread() {
        // OSC 8 hyperlink: teleport's business, not ours. Its payload must
        // not become a title, and its terminator must not become a bell.
        let got = scan(&[b"\x1b]8;;https://claude.ai/code/session_abc\x07"]);
        assert!(got.titles.is_empty(), "{:?}", got.titles);
        assert_eq!(got.last_bell, None);
    }

    /// Two properties, and they are not the same one. The old version of
    /// this test asserted that the `\x07` arriving straight after the
    /// overflow was a bell — which is how the scanner behaved, and which
    /// was the bug: that byte is the abandoned sequence's own terminator.
    /// What it was *trying* to say is that the scanner does not wedge, so
    /// that is what it says now, and it says it with a bell that really is
    /// one.
    #[test]
    fn a_runaway_sequence_is_abandoned_not_buffered() {
        let mut s = Scanner::default();
        let huge = vec![b'x'; MAX_PAYLOAD_BYTES * 4];
        let got = s.feed(&[b"\x1b]0;".as_slice(), &huge].concat());
        assert!(got.titles.is_empty(), "{:?}", got.titles);
        // Stronger than capped: abandoning drops the buffer outright.
        assert_eq!(s.buffered(), 0, "nothing retained for an abandoned payload");
        // The sequence's own terminator, not a ring.
        assert_eq!(s.feed(b"\x07").last_bell, None);
        // And the scanner did not wedge: the next real bell is seen.
        assert_eq!(s.feed(b"what next?\x07").last_bell, Some(10));
    }

    /// Same correction as above: the old assertion took the `\x07` that
    /// closed the malformed sequence and called it a ring. A badly formed
    /// OSC is still an OSC, and skipping to its end is the whole point.
    #[test]
    fn a_malformed_param_does_not_swallow_the_stream() {
        let mut s = Scanner::default();
        let bogus = s.feed(b"\x1b]bogus");
        assert!(bogus.titles.is_empty(), "{:?}", bogus.titles);
        assert_eq!(s.feed(b"ping\x07").last_bell, None, "still terminating");
        assert_eq!(s.feed(b"ready?\x07").last_bell, Some(6));
    }

    /// OSC 52 is the reachable one: base64 of anything over ~384 bytes
    /// blows `MAX_PAYLOAD_BYTES`, and `tmux set-clipboard`, vim and
    /// `pbcopy` wrappers all emit it BEL-terminated. Before an abandoned
    /// payload stayed inside its sequence, the pad went amber for every
    /// clipboard write.
    #[test]
    fn a_clipboard_write_over_the_cap_rings_nothing() {
        let payload = vec![b'A'; 2_000];
        let got = scan(&[&[b"\x1b]52;c;".as_slice(), &payload, b"\x07"].concat()]);
        assert_eq!(got.last_bell, None, "OSC 52's terminator is not a ring");
        assert!(got.titles.is_empty(), "{:?}", got.titles);
    }

    /// Same arm, reached by a title rather than a clipboard — and the
    /// over-long title must not become a title either.
    #[test]
    fn a_title_over_the_cap_is_neither_a_title_nor_a_bell() {
        let long = vec![b't'; 600];
        let got = scan(&[&[b"\x1b]0;".as_slice(), &long, b"\x07"].concat()]);
        assert_eq!(got.last_bell, None);
        assert!(got.titles.is_empty(), "{:?}", got.titles);
    }

    /// The overflow and the terminator can land in different reads. An
    /// abandoned payload has to survive the chunk boundary exactly as a
    /// captured one does, or the scanner is back where it started.
    #[test]
    fn an_overflow_split_across_chunks_still_eats_its_terminator() {
        let mut s = Scanner::default();
        let huge = vec![b'x'; MAX_PAYLOAD_BYTES * 2];
        let opened = s.feed(&[b"\x1b]0;".as_slice(), &huge].concat());
        assert!(opened.titles.is_empty(), "{:?}", opened.titles);
        assert_eq!(s.feed(b"\x07").last_bell, None, "terminator, a chunk later");
        assert_eq!(s.feed(b"\x07").last_bell, Some(0), "and now ground again");
    }

    /// `ESC ESC` used to drop the second `ESC` on the floor, so the `]`
    /// behind it started nothing and the whole sequence — terminator
    /// included — was read as output.
    #[test]
    fn a_doubled_escape_does_not_lose_the_sequence_behind_it() {
        let got = scan(&[b"\x1b\x1b]0;t\x07"]);
        assert_eq!(got.last_bell, None);
        assert_eq!(got.titles, ["t"]);
    }

    /// DCS carries arbitrary bytes — `tmux` passthrough and sixel both do
    /// — so a `0x07` inside one is payload, not a ring. Nothing other than
    /// OSC was recognised before, and the module doc claimed otherwise.
    #[test]
    fn a_dcs_payload_is_skipped_not_scanned() {
        let got = scan(&[b"\x1bPtmux;\x07\x1b\\"]);
        assert_eq!(got.last_bell, None, "a DCS payload byte is not a ring");
        assert!(got.titles.is_empty(), "{:?}", got.titles);
    }

    /// The other two string sequences take the same arm, and afterwards
    /// ground state still works.
    #[test]
    fn apc_and_pm_are_skipped_too() {
        let mut s = Scanner::default();
        assert_eq!(s.feed(b"\x1b_payload\x07").last_bell, None);
        assert_eq!(s.feed(b"\x1b^more\x1b\\").last_bell, None);
        assert_eq!(s.feed(b"done\x07").last_bell, Some(4));
    }

    /// CSI is deliberately *not* skipped the way the string sequences
    /// are: its payload is printable, so it cannot hide a terminator, and
    /// a real bell on the same line must still be seen.
    #[test]
    fn a_csi_sequence_still_lets_a_real_bell_through() {
        let chunk = b"\x1b[31mred\x1b[0m ready?\x07";
        let got = scan(&[chunk]);
        assert_eq!(got.last_bell, Some(chunk.len() - 1));
    }

    /// A doubled `ESC` is held in all three places it can appear: ground
    /// state, an abandoned sequence and a captured payload. The title
    /// goes, because an `ESC` mid-payload is junk, but the sequence still
    /// ends where it says it ends — otherwise `ESC ] 0 ; x ESC ESC \\`
    /// stays open until some later terminator closes it.
    #[test]
    fn a_doubled_escape_inside_a_payload_still_terminates_the_sequence() {
        let mut s = Scanner::default();
        let got = s.feed(b"\x1b]0;t\x1b\x1b\\");
        assert!(got.titles.is_empty(), "{:?}", got.titles);
        assert_eq!(got.last_bell, None);
        // Back in ground state: the next bell is a real one.
        assert_eq!(s.feed(b"ready?\x07").last_bell, Some(6));
    }

    #[test]
    fn plain_output_produces_nothing() {
        let got = scan(&[b"just some tokens being emitted\n"]);
        assert_eq!(got, Scan::default());
    }
}
