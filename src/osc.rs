//! An incremental scanner for the OSC sequences PTY output carries, ported
//! from `teleport/daemon/src/session/osc.rs` with the OSC 8 half removed —
//! a Claude resume id is teleport's business, and writing its prefix is
//! something `docs/07-seams.md` forbids.
//!
//! **This is not here for the title.** It is here because `BEL` (`0x07`) is
//! an OSC *string terminator*, so `ESC ] 0 ; some title BEL` contains a BEL
//! that is not a bell. An agent that sets its terminal title — Claude Code
//! does, constantly — would otherwise look like one that rang and went
//! quiet, and the pad would flash amber for nothing. That is exactly the
//! "wrong amber" M1 exists to count, so the scanner earns its place in the
//! attention path rather than in the title-scrolling one.
//!
//! Not a general VT parser: every other escape sequence is recognised only
//! well enough to skip cleanly past without misreading its payload.
//!
//! The usual caveat, inherited: this rests on Claude Code's current output
//! format, which is not a documented or versioned contract. A release that
//! changes it degrades to "no title" — never a parse error, never a broken
//! session.

use std::mem;

/// Caps how much of one OSC payload is ever buffered. A stream that opens a
/// sequence and never closes it must not grow this without bound; real
/// titles are a few dozen bytes.
const MAX_PAYLOAD_BYTES: usize = 512;

const BEL: u8 = 0x07;
const ESC: u8 = 0x1b;

#[derive(Debug, Default, Clone)]
enum State {
    #[default]
    Ground,
    /// Saw `ESC`; still deciding whether this is an OSC at all.
    Esc,
    /// Saw `ESC ]`; accumulating the numeric parameter before the first `;`.
    Param(u16),
    /// Saw `ESC ] Ps ;`; buffering the payload until a terminator.
    Payload { code: u16, buf: Vec<u8> },
    /// Mid-payload, just saw `ESC` — is it `ESC \` (String Terminator) or a
    /// byte that happens to be `0x1b`?
    PayloadEsc { code: u16, buf: Vec<u8> },
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
}

impl Scanner {
    /// Fold one chunk of raw PTY output in.
    pub fn feed(&mut self, chunk: &[u8]) -> Scan {
        let mut scan = Scan::default();
        for (at, &b) in chunk.iter().enumerate() {
            match &mut self.state {
                State::Ground => match b {
                    ESC => self.state = State::Esc,
                    // The only place a BEL means what it says.
                    BEL => scan.last_bell = Some(at),
                    _ => {}
                },
                State::Esc => {
                    self.state = if b == b']' {
                        State::Param(0)
                    } else {
                        State::Ground
                    };
                }
                State::Param(code) => match b {
                    b'0'..=b'9' => {
                        *code = code.saturating_mul(10).saturating_add(u16::from(b - b'0'));
                    }
                    b';' => {
                        self.state = State::Payload {
                            code: *code,
                            buf: Vec::new(),
                        }
                    }
                    // Malformed. Bail to ground rather than guess — and do
                    // not count a BEL here as a bell: it is still closing an
                    // escape sequence, however badly formed.
                    _ => self.state = State::Ground,
                },
                State::Payload { code, buf } => match b {
                    BEL => {
                        if let Some(title) = finish(*code, buf) {
                            scan.titles.push(title);
                        }
                        self.state = State::Ground;
                    }
                    ESC => {
                        self.state = State::PayloadEsc {
                            code: *code,
                            buf: mem::take(buf),
                        };
                    }
                    _ if buf.len() < MAX_PAYLOAD_BYTES => buf.push(b),
                    // Far longer than any real title; abandon it rather
                    // than buffer forever.
                    _ => self.state = State::Ground,
                },
                State::PayloadEsc { code, buf } => {
                    if b == b'\\' {
                        if let Some(title) = finish(*code, buf) {
                            scan.titles.push(title);
                        }
                    }
                    // Either a real ST closed it, or that ESC was not one
                    // and the sequence is malformed. Either way, done.
                    self.state = State::Ground;
                }
            }
        }
        scan
    }
}

impl Scanner {
    /// How many payload bytes are currently buffered mid-sequence.
    ///
    /// Only the no-unbounded-growth test uses this: the interesting claim
    /// is that a stream of ordinary output leaves nothing retained here.
    #[cfg(test)]
    pub(crate) fn buffered(&self) -> usize {
        match &self.state {
            State::Payload { buf, .. } | State::PayloadEsc { buf, .. } => buf.len(),
            _ => 0,
        }
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
        assert!(got.titles.is_empty());
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
        assert!(s.feed(b"\x1b]0;edit tests/").titles.is_empty());
        assert!(s.feed(b"pty.rs?").titles.is_empty());
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
        assert!(got.titles.is_empty());
        assert_eq!(got.last_bell, None);
    }

    #[test]
    fn a_runaway_sequence_is_abandoned_not_buffered() {
        let mut s = Scanner::default();
        let huge = vec![b'x'; MAX_PAYLOAD_BYTES * 4];
        let got = s.feed(&[b"\x1b]0;".as_slice(), &huge].concat());
        assert!(got.titles.is_empty());
        // Back in ground: a later real bell is still seen.
        assert_eq!(s.feed(b"\x07").last_bell, Some(0));
    }

    #[test]
    fn a_malformed_param_does_not_swallow_the_stream() {
        let mut s = Scanner::default();
        assert!(s.feed(b"\x1b]bogus").titles.is_empty());
        assert_eq!(s.feed(b"ping\x07").last_bell, Some(4));
    }

    #[test]
    fn plain_output_produces_nothing() {
        let got = scan(&[b"just some tokens being emitted\n"]);
        assert_eq!(got, Scan::default());
    }
}
