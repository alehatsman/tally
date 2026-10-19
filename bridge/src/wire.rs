//! The framing on the socket between `tallyd` and `tallybridge`.
//!
//! One byte of kind, two bytes of big-endian length, then the payload:
//!
//! ```text
//! kind  len    payload
//! 00    00 02  01 01               bridge -> tallyd, first frame: hello, present
//! 01    00 01  F8                  tallyd -> bridge: one clock pulse
//! 01    01 06  F0 00 20 29 ...     tallyd -> bridge: a full repaint
//! 01    00 03  90 0B 7F            bridge -> tallyd: a press
//! ```
//!
//! **One MIDI message per frame, in both directions.** `decode` in
//! tally's `device.rs` matches a whole message, exactly as `midir` hands
//! its callback one; if TCP delivered two presses in one read and the
//! bridge forwarded them as one blob, both would vanish. The frame
//! boundary *is* the message boundary, so neither end ever re-splits a
//! MIDI stream.
//!
//! The length is a `u16`, which caps a frame at 64 KiB by construction.
//! The largest real frame is a repaint of 81 flashing pads, 332 bytes, so
//! no real frame comes near the cap, and no peer can make the other end
//! allocate more than 64 KiB for one.

use std::io::{self, Read, Write};

/// The version of this framing. Carried in the hello, so a `tallyd` and a
/// bridge built from different commits say so instead of misreading each
/// other.
pub const VERSION: u8 = 1;

/// Where the bridge listens unless told otherwise, and where `tallyd`
/// looks for it.
pub const DEFAULT_ADDR: &str = "127.0.0.1:7373";

/// Bytes before the payload: kind, then a big-endian `u16` length.
pub const HEADER_LEN: usize = 3;

/// What a frame carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The bridge's first frame on every connection, and only that: see
    /// [`Hello`].
    Hello,
    /// One MIDI message, uninspected. To the device from `tallyd`, from
    /// the device to `tallyd`.
    Midi,
}

impl Kind {
    const fn byte(self) -> u8 {
        match self {
            Self::Hello => 0x00,
            Self::Midi => 0x01,
        }
    }

    const fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x00 => Some(Self::Hello),
            0x01 => Some(Self::Midi),
            _ => None,
        }
    }
}

/// One frame as bytes.
///
/// # Errors
/// [`io::ErrorKind::InvalidInput`] if the payload does not fit a `u16`.
pub fn encode(kind: Kind, payload: &[u8]) -> io::Result<Vec<u8>> {
    let len = u16::try_from(payload.len()).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("a {}-byte frame does not fit the wire: {e}", payload.len()),
        )
    })?;
    let mut frame = Vec::with_capacity(HEADER_LEN + payload.len());
    frame.push(kind.byte());
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(payload);
    Ok(frame)
}

/// Write one frame.
///
/// **One `write_all` of the whole frame**, not a header then a payload:
/// with `TCP_NODELAY` on, two writes are two segments, and a clock pulse
/// should be one.
///
/// # Errors
/// Whatever the writer said, or [`encode`]'s error.
pub fn write(w: &mut impl Write, kind: Kind, payload: &[u8]) -> io::Result<()> {
    w.write_all(&encode(kind, payload)?)
}

/// Read one frame. `Ok(None)` is a clean hang-up **between** frames.
///
/// A frame split across reads reassembles, because TCP will do that and a
/// test socket usually will not. A hang-up **inside** a frame is an error
/// rather than a short message, and an unknown kind is an error rather
/// than something skipped: a peer speaking something else is hung up on.
///
/// # Errors
/// [`io::ErrorKind::UnexpectedEof`] for a truncated frame,
/// [`io::ErrorKind::InvalidData`] for an unknown kind, and whatever the
/// reader said otherwise -- including its timeout.
pub fn read(r: &mut impl Read) -> io::Result<Option<(Kind, Vec<u8>)>> {
    let mut first = [0u8; 1];
    loop {
        match r.read(&mut first) {
            Ok(0) => return Ok(None),
            Ok(_) => break,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    let [kind] = first;
    let kind = Kind::from_byte(kind).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unknown frame kind {kind:#04x}"),
        )
    })?;
    let mut len = [0u8; 2];
    r.read_exact(&mut len)?;
    let mut payload = vec![0u8; usize::from(u16::from_be_bytes(len))];
    r.read_exact(&mut payload)?;
    Ok(Some((kind, payload)))
}

/// The bridge's first word on a connection: is there a Launchpad behind
/// it.
///
/// Over a socket there are two silences where a local port has one --
/// *the bridge is down* and *the bridge is up and the cable is out* --
/// and they want the same grid and different logs. A refused connect is
/// the first; [`Hello::Absent`] is the second.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hello {
    /// The port is open and the bridge is piping it.
    Present,
    /// No port matched. The bridge hangs up after saying so.
    Absent,
    /// A port matched and would not open -- on Windows, usually another
    /// program holding it. Says why, then hangs up.
    Refused(String),
}

impl Hello {
    const ABSENT: u8 = 0;
    const PRESENT: u8 = 1;
    const REFUSED: u8 = 2;

    /// The payload of a [`Kind::Hello`] frame: version, status, and for
    /// [`Hello::Refused`] the reason as UTF-8.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Present => vec![VERSION, Self::PRESENT],
            Self::Absent => vec![VERSION, Self::ABSENT],
            Self::Refused(why) => {
                let mut p = vec![VERSION, Self::REFUSED];
                p.extend_from_slice(why.as_bytes());
                p
            }
        }
    }

    /// Read a hello payload.
    ///
    /// # Errors
    /// [`io::ErrorKind::InvalidData`] for a different [`VERSION`] -- named
    /// in the message, because the fix is rebuilding one side -- or a
    /// payload that is not a hello.
    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        let invalid = |what: String| io::Error::new(io::ErrorKind::InvalidData, what);
        match payload {
            [VERSION, Self::PRESENT] => Ok(Self::Present),
            [VERSION, Self::ABSENT] => Ok(Self::Absent),
            [VERSION, Self::REFUSED, why @ ..] => {
                Ok(Self::Refused(String::from_utf8_lossy(why).into_owned()))
            }
            [v, ..] if *v != VERSION => Err(invalid(format!(
                "the bridge speaks wire version {v} and this side speaks {VERSION}: rebuild one of them"
            ))),
            _ => Err(invalid(format!("not a hello: {payload:02X?}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reader that hands out one byte per call: the worst split TCP is
    /// allowed to make, and the one a loopback test socket never does.
    struct Trickle<'a>(&'a [u8]);

    impl Read for Trickle<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let Some((&b, rest)) = self.0.split_first() else {
                return Ok(0);
            };
            let Some(slot) = buf.first_mut() else {
                return Ok(0);
            };
            *slot = b;
            self.0 = rest;
            Ok(1)
        }
    }

    #[test]
    fn every_kind_round_trips() {
        for (kind, payload) in [
            (Kind::Midi, vec![0xF8]),
            (Kind::Midi, vec![0x90, 0x0B, 0x7F]),
            (Kind::Hello, Hello::Present.encode()),
            (Kind::Midi, vec![]),
        ] {
            let bytes = encode(kind, &payload).unwrap();
            assert_eq!(bytes.len(), HEADER_LEN + payload.len());
            let got = read(&mut bytes.as_slice()).unwrap();
            assert_eq!(got, Some((kind, payload)));
        }
    }

    /// The test that matters: TCP may deliver a frame a byte at a time.
    #[test]
    fn a_frame_split_across_reads_reassembles() {
        let mut stream = encode(Kind::Midi, &[0x90, 0x0B, 0x7F]).unwrap();
        stream.extend(encode(Kind::Midi, &[0xB0, 19, 0x7F]).unwrap());
        let mut r = Trickle(&stream);
        assert_eq!(
            read(&mut r).unwrap(),
            Some((Kind::Midi, vec![0x90, 0x0B, 0x7F]))
        );
        assert_eq!(
            read(&mut r).unwrap(),
            Some((Kind::Midi, vec![0xB0, 19, 0x7F]))
        );
        assert_eq!(read(&mut r).unwrap(), None, "a clean end between frames");
    }

    /// Two presses written back to back stay two messages. Coalescing
    /// would hand `decode` six bytes and lose both.
    #[test]
    fn back_to_back_frames_stay_separate() {
        let mut stream = encode(Kind::Midi, &[0x90, 0x0B, 0x7F]).unwrap();
        stream.extend(encode(Kind::Midi, &[0x90, 0x0B, 0x00]).unwrap());
        let mut r = stream.as_slice();
        assert_eq!(read(&mut r).unwrap().unwrap().1, [0x90, 0x0B, 0x7F]);
        assert_eq!(read(&mut r).unwrap().unwrap().1, [0x90, 0x0B, 0x00]);
    }

    /// A hang-up mid-frame is not a short message, and it does not block:
    /// a slice reader ends where a dead socket would.
    #[test]
    fn a_truncated_frame_is_an_error_not_a_message() {
        let whole = encode(Kind::Midi, &[0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7]).unwrap();
        for cut in 1..whole.len() {
            let got = read(&mut whole.get(..cut).unwrap()).unwrap_err();
            assert_eq!(got.kind(), io::ErrorKind::UnexpectedEof, "cut at {cut}");
        }
    }

    #[test]
    fn an_unknown_kind_is_hung_up_on() {
        let got = read(&mut [0x7E, 0x00, 0x00].as_slice()).unwrap_err();
        assert_eq!(got.kind(), io::ErrorKind::InvalidData);
    }

    /// The cap is the type. Nothing over `u16::MAX` can be encoded, so
    /// nothing over it can be announced by a well-formed peer.
    #[test]
    fn an_oversize_payload_is_refused_rather_than_truncated() {
        let big = vec![0u8; usize::from(u16::MAX) + 1];
        let got = encode(Kind::Midi, &big).unwrap_err();
        assert_eq!(got.kind(), io::ErrorKind::InvalidInput);
        let largest = encode(Kind::Midi, &big[1..]).expect("the largest that fits");
        assert_eq!(largest.len(), HEADER_LEN + usize::from(u16::MAX));
    }

    #[test]
    fn hellos_round_trip() {
        for h in [
            Hello::Present,
            Hello::Absent,
            Hello::Refused("MIDIIN2 (LPMiniMK3 MIDI): already open".to_owned()),
        ] {
            assert_eq!(Hello::decode(&h.encode()).unwrap(), h);
        }
    }

    /// The version is the first byte so a mismatch reads as one, not as a
    /// malformed hello.
    #[test]
    fn a_hello_from_another_version_says_so() {
        let got = Hello::decode(&[VERSION + 1, 1]).unwrap_err();
        assert!(got.to_string().contains("rebuild"), "{got}");
        let _empty = Hello::decode(&[]).unwrap_err();
        let _unknown = Hello::decode(&[VERSION, 9]).unwrap_err();
    }
}
