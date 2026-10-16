//! Raw device messages. Byte-pushing with no logic in it.

/// Every message to and from this device carries this header.
pub const HEADER: [u8; 6] = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0D];
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

/// Novation's manufacturer id, and this model's id, inside an inquiry reply.
pub const NOVATION: [u8; 3] = [0x00, 0x20, 0x29];
pub const MODEL_LP_MINI_MK3: u8 = 0x13;
/// Reply byte meaning the device booted its application, not the bootloader.
pub const MODE_APPLICATION: u8 = 0x01;

/// True if `reply` is an inquiry response from a Launchpad Mini MK3 running
/// its application.
pub fn is_launchpad_mini_mk3(reply: &[u8]) -> bool {
    reply.len() >= 10
        && reply[0..5] == [0xF0, 0x7E, 0x00, 0x06, 0x02]
        && reply[5..8] == NOVATION
        && reply[8] == MODEL_LP_MINI_MK3
        && reply[9] == MODE_APPLICATION
}

/// `Some(true)` if the device reports programmer mode, `Some(false)` for
/// Live, `None` if this is not a mode reply.
pub fn programmer_mode_from(reply: &[u8]) -> Option<bool> {
    (reply.len() == 9 && reply[0..6] == HEADER && reply[6] == 0x0E && reply[8] == EOX)
        .then(|| reply[7] == 1)
}

/// Turn the device's own press-lights off. By default it lights a pad when
/// pressed, which fights the renderer — the model owns every pixel.
pub fn led_feedback(internal: bool, external: bool) -> Vec<u8> {
    let mut m = HEADER.to_vec();
    m.extend([0x0A, u8::from(internal), u8::from(external), EOX]);
    m
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(programmer_mode_from(&[0xF0, 0, 0x20, 0x29, 2, 0x0D, 0x0E, 1, 0xF7]), Some(true));
        assert_eq!(programmer_mode_from(&[0xF0, 0, 0x20, 0x29, 2, 0x0D, 0x0E, 0, 0xF7]), Some(false));
        assert_eq!(programmer_mode_from(&INQUIRY), None);
        assert_eq!(programmer_mode_from(&[]), None);
    }

    #[test]
    fn feedback_off_is_what_startup_sends() {
        assert_eq!(led_feedback(false, false), vec![0xF0, 0, 0x20, 0x29, 2, 0x0D, 0x0A, 0, 0, 0xF7]);
    }
}
