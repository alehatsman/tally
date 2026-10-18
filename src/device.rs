//! The Launchpad itself: finding it, driving it, and — the part that is not
//! optional — putting it back.
//!
//! Programmer mode **disables the device's Setup menu**, so a process that
//! exits without sending Live mode leaves the user with a Launchpad that
//! has lost a feature and no obvious way to get it back short of a power
//! cycle (`docs/08-daemon.md`). Everything here is arranged so that the
//! restore happens on every path out: normal return, error, panic unwind,
//! and a signal that the main loop turns into a normal return.

use std::fmt;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};

use crate::surface::sysex;
use crate::surface::{Button, Frame};

/// How long to wait for a `SysEx` reply.
///
/// Generous on purpose: the first round-trip after connecting is cold
/// (~345ms measured in M0) where a warm one is ~2ms.
const REPLY_TIMEOUT: Duration = Duration::from_secs(2);

/// The substring that identifies the port we want.
///
/// Never an exact string and never an index. M0 confirmed this matches both
/// names `CoreMIDI` exposes (`LPMiniMK3 MIDI Out` and the longer
/// `Launchpad Mini MK3 LPMiniMK3 MIDI Out`), so it survives whichever one
/// `midir` reports.
pub const PORT_SUBSTRING: &str = "LPMiniMK3 MIDI";

/// Something the device, or the MIDI stack under it, would not do.
#[derive(Debug)]
pub enum Error {
    /// No port matching [`PORT_SUBSTRING`]. Usually: it is unplugged.
    NotFound,
    /// A port matched, but the thing behind it did not identify as a
    /// Launchpad Mini MK3 running its application.
    NotALaunchpad,
    /// The MIDI stack refused.
    Midi(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "no MIDI port matching {PORT_SUBSTRING:?}"),
            Self::NotALaunchpad => write!(f, "port matched but the device is not a Mini MK3"),
            Self::Midi(e) => write!(f, "midi: {e}"),
        }
    }
}

impl std::error::Error for Error {}

/// Something that arrived from the device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A button went down or came up.
    ///
    /// Every press is followed by exactly one release, confirmed on
    /// hardware in M0 — including under a held chord, which is what makes
    /// hold-then-tap safe to build a kill gesture on.
    Press {
        /// Which button.
        button: Button,
        /// `true` on the way down.
        down: bool,
    },
    /// A `SysEx` reply: a device inquiry or a mode readback.
    Reply(Vec<u8>),
}

/// An open connection to the Launchpad, in programmer mode.
///
/// Dropping it restores Live mode. That is a `Drop` impl rather than a
/// method you must remember to call, because the failure it prevents is
/// invisible until the next time you reach for the Setup menu.
pub struct Device {
    out: MidiOutputConnection,
    /// Held so the callback keeps running; never read directly.
    _input: MidiInputConnection<Sender<Event>>,
    events: Receiver<Event>,
    /// The last frame put on the wire, so the next one can be a diff.
    last: Frame,
    /// Whether `Drop` should try to put the device back.
    restore_on_drop: bool,
}

impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Device")
            .field("port", &PORT_SUBSTRING)
            .finish_non_exhaustive()
    }
}

impl Device {
    /// Find the device, confirm it is one, and take it into programmer mode.
    ///
    /// Start-up is **idempotent and total**: programmer mode is re-entered,
    /// the device's own press-lighting is turned off, and all 81 LEDs are
    /// painted. Whatever state a previous process died in is overwritten
    /// rather than inherited — which matters more than exiting cleanly,
    /// because only one of the two is reliable.
    ///
    /// # Errors
    /// [`Error::NotFound`] if nothing matches, [`Error::NotALaunchpad`] if
    /// the device does not identify correctly, [`Error::Midi`] otherwise.
    pub fn open() -> Result<Self, Error> {
        let mut input = MidiInput::new("tally").map_err(|e| Error::Midi(e.to_string()))?;
        // We depend on SysEx for identity, mode readback and every frame.
        input.ignore(midir::Ignore::None);
        let output = MidiOutput::new("tally").map_err(|e| Error::Midi(e.to_string()))?;

        // In and Out are named from the DEVICE's point of view, so our
        // input reads the source called "...MIDI Out". Matching the wrong
        // way round opens a port that never delivers a press, silently.
        let in_port = input
            .ports()
            .into_iter()
            .find(|p| input.port_name(p).is_ok_and(|n| n.contains(PORT_SUBSTRING)))
            .ok_or(Error::NotFound)?;
        let out_port = output
            .ports()
            .into_iter()
            .find(|p| {
                output
                    .port_name(p)
                    .is_ok_and(|n| n.contains(PORT_SUBSTRING))
            })
            .ok_or(Error::NotFound)?;

        let (tx, events) = mpsc::channel();
        let listening = input
            .connect(
                &in_port,
                "tally-in",
                |_t, bytes, tx: &mut Sender<Event>| {
                    if let Some(event) = decode(bytes) {
                        // A closed receiver means we are shutting down.
                        drop(tx.send(event));
                    }
                },
                tx,
            )
            .map_err(|e| Error::Midi(e.to_string()))?;
        let out = output
            .connect(&out_port, "tally-out")
            .map_err(|e| Error::Midi(e.to_string()))?;

        let mut device = Self {
            out,
            _input: listening,
            events,
            last: Frame::dark(),
            restore_on_drop: true,
        };
        device.confirm_identity()?;
        device.send(&sysex::PROGRAMMER)?;
        // The device lights a pad when pressed unless told otherwise, which
        // fights the renderer: the model owns every pixel.
        device.send(&sysex::led_feedback(false, false))?;
        device.repaint(&Frame::dark())?;
        Ok(device)
    }

    fn confirm_identity(&mut self) -> Result<(), Error> {
        self.send(&sysex::INQUIRY)?;
        if self.await_reply(sysex::is_launchpad_mini_mk3).is_some() {
            return Ok(());
        }
        Err(Error::NotALaunchpad)
    }

    /// Wait up to [`REPLY_TIMEOUT`] for a `SysEx` reply matching `wanted`,
    /// discarding any presses that arrive meanwhile.
    ///
    /// A timeout on one `recv` is **not** the end of waiting. The device's
    /// first round-trip after connecting is cold -- ~345ms measured in M0,
    /// against ~2ms warm -- so treating the first quiet interval as "no
    /// device" rejects a perfectly healthy Launchpad on every start. That
    /// is how this was first written, and running it is what found it.
    fn await_reply(&self, wanted: impl Fn(&[u8]) -> bool) -> Option<Vec<u8>> {
        let deadline = Instant::now() + REPLY_TIMEOUT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return None;
            }
            match self.events.recv_timeout(left) {
                Ok(Event::Reply(r)) if wanted(&r) => return Some(r),
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return None,
            }
        }
    }

    /// Give up on a device that is already gone.
    ///
    /// Restoring Live mode on an unplugged Launchpad cannot work, and
    /// *saying* it failed is worse than silence — it tells the user their
    /// Setup menu may be broken when the truth is that the cable is out.
    /// An alarm that fires for the ordinary case is an alarm people learn
    /// to ignore.
    pub fn abandon(&mut self) {
        self.restore_on_drop = false;
    }

    /// Ask the device which mode it is in, without waiting for the answer.
    ///
    /// This is the liveness probe, and it exists because **a failed write is
    /// not how you learn the device is gone**. `CoreMIDI` accepts sends to a
    /// removed endpoint without error, so `render` returning `Ok` says
    /// nothing about whether anything is listening. Only a reply does.
    ///
    /// It doubles as a mode check. A Launchpad that was unplugged and
    /// plugged back in comes up in **Live mode**, and a host still holding
    /// a connection to it would otherwise keep diffing against a frame the
    /// device no longer has — leaving the grid dark forever, because
    /// nothing in the model changed.
    ///
    /// # Errors
    /// [`Error::Midi`] if the write fails.
    pub fn ping(&mut self) -> Result<(), Error> {
        self.send(&sysex::MODE_READBACK)
    }

    /// Re-enter programmer mode and forget what we believe is on the pads,
    /// so the next render is total.
    ///
    /// # Errors
    /// [`Error::Midi`] if a write fails.
    pub fn reprogram(&mut self) -> Result<(), Error> {
        self.send(&sysex::PROGRAMMER)?;
        self.send(&sysex::led_feedback(false, false))?;
        self.last = Frame::dark();
        Ok(())
    }

    /// Put `frame` on the device, sending only what changed.
    ///
    /// # Errors
    /// [`Error::Midi`] if the write fails.
    pub fn render(&mut self, frame: &Frame) -> Result<(), Error> {
        if let Some(msg) = frame.diff(&self.last) {
            self.send(&msg)?;
        }
        self.last = frame.clone();
        Ok(())
    }

    /// Paint every LED, ignoring what we believe is already there. Used on
    /// connect and after a replug, where a full frame is one message anyway.
    ///
    /// # Errors
    /// [`Error::Midi`] if the write fails.
    pub fn repaint(&mut self, frame: &Frame) -> Result<(), Error> {
        self.send(&frame.repaint())?;
        self.last = frame.clone();
        Ok(())
    }

    /// Presses and replies since the last call. Never blocks.
    pub fn poll(&self) -> impl Iterator<Item = Event> + '_ {
        self.events.try_iter()
    }

    /// Put the device back: stop any scroll, clear all 81 LEDs, Live mode.
    ///
    /// Returns whether the device **confirmed** Live mode on a readback. A
    /// write is not an acknowledgement, and a restore that failed silently
    /// is the whole failure this module exists to prevent.
    pub fn restore(&mut self) -> bool {
        // Clearing covers index 99. Skipping the logo would exit while
        // asserting "anchord unreachable", which is a lie on the way out.
        for msg in [
            sysex::STOP_SCROLL.as_slice(),
            &Frame::dark().repaint(),
            &sysex::LIVE,
        ] {
            if self.send(msg).is_err() {
                return false;
            }
        }
        if self.send(&sysex::MODE_READBACK).is_err() {
            return false;
        }
        self.await_reply(|r| sysex::programmer_mode_from(r).is_some())
            .and_then(|r| sysex::programmer_mode_from(&r))
            .is_some_and(|programmer| !programmer)
    }

    /// One MIDI clock pulse. Twenty-four of these make a quarter note,
    /// and the device's two-beat pulse ramp follows them
    /// (`docs/05-interaction.md`).
    ///
    /// ✅ Confirmed on hardware that the Mini MK3 honours clock on this
    /// port — `LPMiniMK3 MIDI`, not the DAW port — which was the one
    /// sentence the tempo rested on.
    ///
    /// # Errors
    /// Whatever the MIDI stack said.
    pub fn clock(&mut self) -> Result<(), Error> {
        self.send(&[0xF8])
    }

    /// Begin owning the beat.
    ///
    /// Sent once before the stream because some devices only follow clock
    /// while they believe something is playing. Harmless where it is not
    /// needed.
    ///
    /// # Errors
    /// Whatever the MIDI stack said.
    pub fn clock_start(&mut self) -> Result<(), Error> {
        self.send(&[0xFA])
    }

    /// Hand the beat back to the device's own 120 bpm.
    ///
    /// Sent on a clean shutdown so the grid is left breathing normally
    /// rather than holding our last tempo. **Not** sent when `tallyd` is
    /// killed — then the breathing stops mid-ramp, which is the point:
    /// a wedged daemon used to leave a perfectly normal-looking grid
    /// showing a stale snapshot.
    ///
    /// # Errors
    /// Whatever the MIDI stack said.
    pub fn clock_stop(&mut self) -> Result<(), Error> {
        self.send(&[0xFC])
    }

    fn send(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.out.send(bytes).map_err(|e| Error::Midi(e.to_string()))
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        if !self.restore_on_drop {
            return;
        }
        if !self.restore() {
            // Not a panic: we may already be unwinding, and a double panic
            // aborts without running anything else's destructors.
            eprintln!("tally: the Launchpad did not confirm Live mode -- its Setup menu may still be disabled");
        }
    }
}

/// Classify one inbound MIDI message.
///
/// Note-off arrives as either `8n`, or `9n` with velocity 0; both are
/// handled because the manual says both occur and M0 did not pin down which
/// this device uses.
fn decode(bytes: &[u8]) -> Option<Event> {
    match bytes {
        [0xF0, ..] => Some(Event::Reply(bytes.to_vec())),
        &[status, index, velocity] => {
            let down = match status {
                0x90 | 0xB0 => velocity > 0,
                0x80 => false,
                _ => return None,
            };
            Some(Event::Press {
                button: Button::from_index(index)?,
                down,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grid_press_decodes_to_the_pad_under_the_finger() {
        // 0x0B is the bottom-left pad, confirmed on hardware in M0.
        assert_eq!(
            decode(&[0x90, 0x0B, 127]),
            Some(Event::Press {
                button: Button::Grid { row: 8, col: 1 },
                down: true
            })
        );
    }

    /// Both note-off encodings must work; the manual permits either.
    #[test]
    fn note_off_arrives_two_ways() {
        let up = Some(Event::Press {
            button: Button::Grid { row: 8, col: 1 },
            down: false,
        });
        assert_eq!(decode(&[0x90, 0x0B, 0]), up, "9n with velocity 0");
        assert_eq!(decode(&[0x80, 0x0B, 0]), up, "8n");
    }

    #[test]
    fn control_change_carries_the_modifiers_and_pages() {
        // CC89 is the top of the right column; CC19 is kill.
        assert_eq!(
            decode(&[0xB0, 89, 127]),
            Some(Event::Press {
                button: Button::Action(1),
                down: true
            })
        );
        assert_eq!(
            decode(&[0xB0, 19, 127]),
            Some(Event::Press {
                button: Button::Action(8),
                down: true
            })
        );
        assert_eq!(
            decode(&[0xB0, 91, 127]),
            Some(Event::Press {
                button: Button::Top(1),
                down: true
            })
        );
    }

    #[test]
    fn sysex_replies_are_kept_whole() {
        let reply = [
            0xF0, 0x7E, 0x00, 0x06, 0x02, 0x00, 0x20, 0x29, 0x13, 0x01, 0xF7,
        ];
        assert_eq!(decode(&reply), Some(Event::Reply(reply.to_vec())));
    }

    #[test]
    fn noise_is_ignored_rather_than_guessed_at() {
        assert_eq!(decode(&[]), None);
        assert_eq!(decode(&[0xF8]), None); // clock
        assert_eq!(decode(&[0xC0, 1, 0]), None); // program change
        assert_eq!(decode(&[0x90, 200, 127]), None); // off-surface index
    }
}
