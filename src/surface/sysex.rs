//! Raw device messages. Byte-pushing with no logic in it.

/// The `SysEx` header every message to and from this device carries.
pub const HEADER: [u8; 6] = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0D];
/// End-of-exclusive: the byte that closes every `SysEx`.
pub const EOX: u8 = 0xF7;

/// Enter programmer mode. **Disables the device's Setup menu** until Live
/// mode is restored — see `docs/08-lifecycle.md`.
pub const PROGRAMMER: [u8; 9] = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0D, 0x0E, 0x01, 0xF7];
/// Return to Live mode. Not optional, and not only on a clean exit.
pub const LIVE: [u8; 9] = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0D, 0x0E, 0x00, 0xF7];
/// Ask which mode the device is in. The reply is the only way to know the
/// restore above actually landed; a write is not an acknowledgement.
pub const MODE_READBACK: [u8; 8] = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0D, 0x0E, 0xF7];
/// Stop a running text scroll.
pub const STOP_SCROLL: [u8; 8] = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0D, 0x07, 0xF7];
/// Universal identity request. Confirms we found a Launchpad Mini MK3 and
/// not something else whose port name happened to match.
pub const INQUIRY: [u8; 6] = [0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7];

/// Novation's MIDI manufacturer id, as it appears in an inquiry reply.
pub const NOVATION: [u8; 3] = [0x00, 0x20, 0x29];
/// This model's id. Confirms we found a Mini MK3 and not another Launchpad.
pub const MODEL_LP_MINI_MK3: u8 = 0x13;
/// Reply byte meaning the device booted its application, not the bootloader.
pub const MODE_APPLICATION: u8 = 0x01;

/// True if `reply` is an inquiry response from a Launchpad Mini MK3 running
/// its application.
pub fn is_launchpad_mini_mk3(reply: &[u8]) -> bool {
    reply.get(0..5) == Some(&[0xF0, 0x7E, 0x00, 0x06, 0x02])
        && reply.get(5..8) == Some(&NOVATION)
        && reply.get(8) == Some(&MODEL_LP_MINI_MK3)
        && reply.get(9) == Some(&MODE_APPLICATION)
}

/// `Some(true)` if the device reports programmer mode, `Some(false)` for
/// Live, `None` if this is not a mode reply.
pub fn programmer_mode_from(reply: &[u8]) -> Option<bool> {
    let ok = reply.len() == 9
        && reply.get(0..6) == Some(&HEADER)
        && reply.get(6) == Some(&0x0E)
        && reply.get(8) == Some(&EOX);
    ok.then(|| reply.get(7) == Some(&1))
}

/// Turn the device's own press-lights off. By default it lights a pad when
/// pressed, which fights the renderer — the model owns every pixel.
pub fn led_feedback(internal: bool, external: bool) -> Vec<u8> {
    let mut m = HEADER.to_vec();
    m.extend([0x0A, u8::from(internal), u8::from(external), EOX]);
    m
}

/// How many characters of a title the device is asked to scroll.
///
/// 64 is about nine seconds at [`SCROLL_SPEED`], which is already longer
/// than anybody holds a pad. The cap is not about the byte budget; it is
/// about not asking the device to spell out a path nobody will wait for.
pub const SCROLL_MAX: usize = 64;

/// Pads per second. 7 is brisk enough to read a short title in one hold
/// and slow enough to actually read it.
pub const SCROLL_SPEED: u8 = 7;

/// Scroll `text` across the grid once, in `colour`.
///
/// `F0 00 20 29 02 0D 07 <loop> <speed> 00 <palette> <ascii...> F7`
///
/// **Never loops.** A hold is a question with an end — you let go — and a
/// looping scroll would keep running if a release were ever missed, which
/// is the one failure this surface must not have: text covering the grid
/// with no way to ask it to stop.
///
/// The text is sanitised by [`scroll_ascii`] on the way through, because
/// a byte ≥ `0x80` inside a `SysEx` is read as a status byte and ends the
/// message early — which would leave the device parsing the rest of our
/// frame as commands.
pub fn scroll(text: &str, colour: u8) -> Vec<u8> {
    let mut m = HEADER.to_vec();
    m.extend([0x07, 0x00, SCROLL_SPEED, 0x00, colour & 0x7F]);
    m.extend(scroll_ascii(text).into_bytes());
    m.push(EOX);
    m
}

/// A title, made safe to put inside a `SysEx`.
///
/// One rule, from `docs/05-interaction.md#the-title-scroll`: printable
/// ASCII (`0x20`–`0x7E`) passes, everything else becomes a space, runs of
/// spaces collapse, and the result is trimmed and capped at
/// [`SCROLL_MAX`].
///
/// Dropping the em-dashes and box-drawing characters an agent puts in its
/// title is a better outcome than refusing to scroll, and a far better one
/// than sending the byte and watching the device execute the remains of
/// the message.
///
/// Collapsing runs matters more than it looks: a title like
/// `claude — edit tests/pty.rs` becomes three spaces where the dash was,
/// and three pads of nothing scrolling past is read as the end of the text.
pub fn scroll_ascii(text: &str) -> String {
    let mut out = String::with_capacity(text.len().min(SCROLL_MAX));
    let mut pending_space = false;
    for c in text.chars() {
        let printable = matches!(c, ' '..='~');
        if !printable || c == ' ' {
            // Leading spaces are dropped by never emitting one first.
            pending_space = !out.is_empty();
            continue;
        }
        if pending_space {
            if out.len() + 2 > SCROLL_MAX {
                break;
            }
            out.push(' ');
            pending_space = false;
        }
        if out.len() + 1 > SCROLL_MAX {
            break;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape from `docs/02-surface.md`, and the `F7` that closes it.
    #[test]
    fn a_scroll_is_the_documented_message() {
        let m = scroll("hi", 21);
        assert_eq!(
            m,
            vec![
                0xF0,
                0x00,
                0x20,
                0x29,
                0x02,
                0x0D,
                0x07,
                0x00,
                SCROLL_SPEED,
                0x00,
                21,
                b'h',
                b'i',
                0xF7
            ]
        );
    }

    /// A hold is a question with an end. A looping scroll would keep
    /// running if a release were ever missed, covering the grid with no
    /// way left to ask it to stop.
    #[test]
    fn a_scroll_never_loops() {
        assert_eq!(scroll("x", 5).get(7), Some(&0x00), "the loop byte");
    }

    /// The hard one. Any byte >= 0x80 inside a `SysEx` is a status byte: the
    /// device ends the message there and reads the rest of our frame as
    /// commands. This is the whole reason the sanitiser exists.
    #[test]
    fn nothing_above_7f_ever_reaches_the_wire() {
        for title in [
            "claude — edit tests/pty.rs",
            "╭─ building ─╮",
            "日本語のタイトル",
            "emoji \u{1f600} here",
            "\u{7f}\u{80}\u{ff}",
        ] {
            let m = scroll(title, 21);
            let body = m.get(11..m.len() - 1).expect("body");
            assert!(
                body.iter().all(|&b| (0x20..=0x7E).contains(&b)),
                "{title:?} produced {body:02X?}"
            );
            assert_eq!(m.last(), Some(&EOX), "still terminated");
        }
    }

    /// Three spaces where a dash was reads as the end of the text, so
    /// runs collapse rather than being replaced one for one.
    #[test]
    fn runs_of_unprintables_collapse_to_one_space() {
        assert_eq!(scroll_ascii("claude — edit pty.rs"), "claude edit pty.rs");
        assert_eq!(scroll_ascii("a\t\t\tb"), "a b");
        assert_eq!(scroll_ascii("a     b"), "a b");
    }

    #[test]
    fn surrounding_space_is_dropped() {
        assert_eq!(scroll_ascii("   hi   "), "hi");
        assert_eq!(scroll_ascii("\u{a0}hi\u{a0}"), "hi");
        assert_eq!(scroll_ascii(""), "");
        assert_eq!(scroll_ascii("———"), "", "nothing printable is nothing");
    }

    /// Capped in characters, and never mid-way into a trailing space.
    #[test]
    fn a_long_title_is_cut_to_something_you_would_wait_for() {
        let long = "word ".repeat(40);
        let cut = scroll_ascii(&long);
        assert!(cut.len() <= SCROLL_MAX, "{} chars", cut.len());
        assert!(!cut.ends_with(' '), "{cut:?}");
        // The cap is on the text, so the message stays well inside the
        // budget `docs/02-surface.md` works out for a frame.
        assert!(scroll(&long, 21).len() <= SCROLL_MAX + 12);
    }

    /// `STOP_SCROLL` is the same message with no payload, and that is how
    /// a release puts the grid back.
    #[test]
    fn stopping_is_the_same_message_with_nothing_in_it() {
        assert_eq!(STOP_SCROLL.get(6), Some(&0x07));
        assert_eq!(STOP_SCROLL.len(), 8);
        assert_eq!(
            scroll("", 21).len(),
            STOP_SCROLL.len() + 4,
            "loop, speed, and a two-byte colourspec"
        );
    }

    /// The exact bytes the device sent in M0.
    const M0_INQUIRY_REPLY: [u8; 17] = [
        0xF0, 0x7E, 0x00, 0x06, 0x02, 0x00, 0x20, 0x29, 0x13, 0x01, 0x00, 0x00, 0x00, 0x03, 0x04,
        0x01, 0xF7,
    ];

    #[test]
    fn recognises_the_real_inquiry_reply() {
        assert!(is_launchpad_mini_mk3(&M0_INQUIRY_REPLY));
    }

    #[test]
    fn rejects_the_bootloader_and_other_devices() {
        let mut boot = M0_INQUIRY_REPLY;
        boot[9] = 0x11; // bootloader: a different device, from our point of view
        assert!(!is_launchpad_mini_mk3(&boot));

        let mut other = M0_INQUIRY_REPLY;
        other[8] = 0x03; // some other Novation product
        assert!(!is_launchpad_mini_mk3(&other));

        assert!(!is_launchpad_mini_mk3(&[]));
        assert!(!is_launchpad_mini_mk3(&M0_INQUIRY_REPLY[..6]));
    }

    #[test]
    fn mode_replies_decode() {
        // Both observed on hardware in M0.
        assert_eq!(
            programmer_mode_from(&[0xF0, 0, 0x20, 0x29, 2, 0x0D, 0x0E, 1, 0xF7]),
            Some(true)
        );
        assert_eq!(
            programmer_mode_from(&[0xF0, 0, 0x20, 0x29, 2, 0x0D, 0x0E, 0, 0xF7]),
            Some(false)
        );
        assert_eq!(programmer_mode_from(&INQUIRY), None);
        assert_eq!(programmer_mode_from(&[]), None);
    }

    #[test]
    fn feedback_off_is_what_startup_sends() {
        assert_eq!(
            led_feedback(false, false),
            vec![0xF0, 0, 0x20, 0x29, 2, 0x0D, 0x0A, 0, 0, 0xF7]
        );
    }
}
