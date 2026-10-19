//! The Launchpad itself: finding it, driving it, and — the part that is not
//! optional — putting it back.
//!
//! Programmer mode **disables the device's Setup menu**, so a process that
//! exits without sending Live mode leaves the user with a Launchpad that
//! has lost a feature and no obvious way to get it back short of a power
//! cycle (`docs/08-daemon.md`). Everything here is arranged so that the
//! restore happens on every path out: normal return, error, panic unwind,
//! and a signal that the main loop turns into a normal return.
//!
//! The device is usually on this machine. When it is on the far side of
//! the WSL line, [`BRIDGE_ENV`] names the bridge that holds it and the
//! same bytes cross a socket instead (`docs/13-bridge.md`). **Nothing
//! above this module can tell which**: it is one type with two ways of
//! moving bytes, not two devices.

use std::fmt;
use std::io;
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use tallybridge::wire::{self, Hello, Kind};

use crate::surface::sysex;
use crate::surface::{Button, Frame};

/// How long to wait for a `SysEx` reply.
///
/// Generous on purpose: the first round-trip after connecting is cold
/// (~345ms measured in M0) where a warm one is ~2ms.
const REPLY_TIMEOUT: Duration = Duration::from_secs(2);

/// MIDI realtime clock. Not `SysEx`, so it is named here and not beside
/// [`sysex::PROGRAMMER`] — but an unnamed byte on the wire is a byte
/// nothing can check.
const CLOCK: [u8; 1] = [0xF8];
/// MIDI realtime start: the transport is running.
const CLOCK_START: [u8; 1] = [0xFA];
/// MIDI realtime stop: the transport is not.
const CLOCK_STOP: [u8; 1] = [0xFC];

/// The substring that identifies the port we want.
///
/// Never an exact string and never an index. M0 confirmed this matches both
/// names `CoreMIDI` exposes (`LPMiniMK3 MIDI Out` and the longer
/// `Launchpad Mini MK3 LPMiniMK3 MIDI Out`), so it survives whichever one
/// `midir` reports.
pub const PORT_SUBSTRING: &str = "LPMiniMK3 MIDI";

/// Set to an address, and the device is reached through `tallybridge` at
/// that address rather than through a local MIDI port -- the Launchpad on
/// a Windows desk, `tallyd` in WSL (`docs/13-bridge.md`).
///
/// Unset is the local port, exactly as before. **A Mac with a Launchpad
/// on it never reaches the other path.**
pub const BRIDGE_ENV: &str = "TALLY_BRIDGE";

/// How long a connect to the bridge may take.
///
/// It bounds a loop tick, not a wait: a loopback connect answers in under
/// a millisecond (0.4-0.5 ms measured, WSL to Windows), and nothing
/// listening is refused at once.
const CONNECT_TIMEOUT: Duration = Duration::from_millis(500);

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
    /// The bridge could not be reached, or said something other than
    /// hello. Nothing listening is [`Error::NotFound`] instead: that is
    /// the bridge's unplugged cable.
    Bridge(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "no MIDI port matching {PORT_SUBSTRING:?}"),
            Self::NotALaunchpad => write!(f, "port matched but the device is not a Mini MK3"),
            Self::Midi(e) => write!(f, "midi: {e}"),
            Self::Bridge(e) => write!(f, "bridge: {e}"),
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
    port: Port,
    events: Receiver<Event>,
    /// The last frame put on the wire, so the next one can be a diff.
    last: Frame,
    /// Whether `Drop` should try to put the device back. Armed by
    /// `open()` only once `PROGRAMMER` has been sent, because before that
    /// there is no programmer mode to undo.
    restore_on_drop: bool,
}

impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Device")
            .field("port", &self.port)
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
        match std::env::var(BRIDGE_ENV) {
            Ok(addr) if !addr.trim().is_empty() => Self::over_bridge(addr.trim()),
            _ => Self::local(),
        }
    }

    /// [`Device::open`], through the bridge at `addr` rather than a local
    /// port. Everything after the connection -- identity, programmer mode,
    /// the first repaint -- is the same bytes in the same order.
    ///
    /// # Errors
    /// [`Error::NotFound`] if nothing listens at `addr` or the bridge has
    /// no Launchpad, [`Error::Bridge`] if it cannot be reached or does not
    /// speak the wire, and [`Device::open`]'s errors after that.
    pub fn over_bridge(addr: &str) -> Result<Self, Error> {
        let (bridge, events) = Bridge::connect(addr)?;
        Self::take(Port::Bridge(bridge), events)
    }

    /// The local MIDI port.
    fn local() -> Result<Self, Error> {
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
        Self::take(
            Port::Midi {
                out,
                _input: listening,
            },
            events,
        )
    }

    /// Take the device into programmer mode, whatever carries the bytes.
    fn take(port: Port, events: Receiver<Event>) -> Result<Self, Error> {
        let mut device = Self {
            port,
            events,
            last: Frame::dark(),
            // Not armed yet: there is nothing to put back until
            // `PROGRAMMER` is on the wire. Arming here instead cost four
            // seconds a go — a port that matches but does not identify (a
            // Mini MK3 in its bootloader, which `sysex::MODE_APPLICATION`
            // names, or a port that enumerates a moment before the device
            // can answer) burned `REPLY_TIMEOUT` on the inquiry and then
            // another one on a `restore()` of a device still in Live mode,
            // and printed the Setup-menu alarm at the end of it. That is
            // the ordinary case firing the alarm that `abandon()`'s doc
            // says must stay rare.
            restore_on_drop: false,
        };
        device.confirm_identity()?;
        device.send(&sysex::PROGRAMMER)?;
        // From here on the device is in a mode we put it in, so every exit
        // owes it a restore — including the two sends below failing.
        device.restore_on_drop = true;
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
        // A frame with nothing new sends nothing, so without this a bridge
        // that hung up would go unnoticed until the ping went quiet --
        // seven seconds of every clock pulse failing in the log. Over a
        // socket a hang-up is a fact; over midir it is not knowable, and
        // this is a no-op (D17).
        self.port.alive()?;
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
        self.send(&CLOCK)
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
        self.send(&CLOCK_START)
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
        self.send(&CLOCK_STOP)
    }

    fn send(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.port.send(bytes)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        if !self.restore_on_drop {
            return;
        }
        // Over the bridge, a bridge that already hung up is not a failed
        // restore: putting the device back is then the bridge's own duty,
        // and it has done it (`docs/13-bridge.md`).
        if !self.restore() && self.port.alive().is_ok() {
            // Not a panic: we may already be unwinding, and a double panic
            // aborts without running anything else's destructors.
            eprintln!("tally: the Launchpad did not confirm Live mode -- its Setup menu may still be disabled");
        }
    }
}

/// What carries the bytes to the device.
enum Port {
    /// A MIDI port on this machine.
    Midi {
        out: MidiOutputConnection,
        /// Held so the callback keeps running; never read directly.
        _input: MidiInputConnection<Sender<Event>>,
    },
    /// A socket to `tallybridge`, which holds the port on another machine.
    Bridge(Bridge),
}

impl fmt::Debug for Port {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Midi { .. } => write!(f, "{PORT_SUBSTRING:?}"),
            Self::Bridge(b) => match b.stream.peer_addr() {
                Ok(addr) => write!(f, "bridge at {addr}"),
                Err(_) => write!(f, "bridge, disconnected"),
            },
        }
    }
}

impl Port {
    fn send(&mut self, bytes: &[u8]) -> Result<(), Error> {
        match self {
            Self::Midi { out, .. } => out.send(bytes).map_err(|e| Error::Midi(e.to_string())),
            Self::Bridge(b) => {
                b.alive()?;
                wire::write(&mut b.stream, Kind::Midi, bytes)
                    .map_err(|e| Error::Bridge(e.to_string()))
            }
        }
    }

    /// Whether the far end is still there, where that can be known.
    fn alive(&self) -> Result<(), Error> {
        match self {
            Self::Midi { .. } => Ok(()),
            Self::Bridge(b) => b.alive(),
        }
    }
}

/// The socket to `tallybridge`, and whether its reader has seen it close.
///
/// A thread reads frames and turns each into an [`Event`] on the same
/// channel the `midir` callback feeds, so [`Device::poll`] and the reply
/// waits cannot tell the two apart.
struct Bridge {
    stream: TcpStream,
    /// Cleared by the reader when the bridge hangs up.
    open: Arc<AtomicBool>,
}

impl Bridge {
    /// Connect, read the hello, start the reader.
    fn connect(addr: &str) -> Result<(Self, Receiver<Event>), Error> {
        let named = |e: &dyn fmt::Display| Error::Bridge(format!("{BRIDGE_ENV}={addr}: {e}"));
        let target = addr
            .to_socket_addrs()
            .map_err(|e| named(&e))?
            .next()
            .ok_or_else(|| named(&"names no address"))?;
        let mut stream = match TcpStream::connect_timeout(&target, CONNECT_TIMEOUT) {
            Ok(s) => s,
            // The bridge is not running: its version of a cable that is
            // out, ordinary, and resolved by a later tick.
            Err(e) if e.kind() == io::ErrorKind::ConnectionRefused => return Err(Error::NotFound),
            Err(e) => return Err(named(&e)),
        };
        // Not an optimisation: the breath is single-byte clock pulses and
        // Nagle exists to hold those back (`docs/13-bridge.md`).
        stream.set_nodelay(true).map_err(|e| named(&e))?;
        stream
            .set_read_timeout(Some(REPLY_TIMEOUT))
            .map_err(|e| named(&e))?;
        match wire::read(&mut stream) {
            Ok(Some((Kind::Hello, payload))) => {
                match Hello::decode(&payload).map_err(|e| named(&e))? {
                    Hello::Present => {}
                    Hello::Absent => return Err(Error::NotFound),
                    Hello::Refused(why) => return Err(named(&why)),
                }
            }
            Ok(Some((kind, _))) => return Err(named(&format!("{kind:?} before hello"))),
            Ok(None) => return Err(named(&"hung up before hello")),
            // The bridge serves one connection at a time, so a silent one
            // is usually busy with someone else.
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Err(named(&"no hello -- is another tallyd connected to it?"))
            }
            Err(e) => return Err(named(&e)),
        }
        stream.set_read_timeout(None).map_err(|e| named(&e))?;

        let mut reader = stream.try_clone().map_err(|e| named(&e))?;
        let open = Arc::new(AtomicBool::new(true));
        let seen = Arc::clone(&open);
        let (tx, events) = mpsc::channel();
        thread::Builder::new()
            .name("tally-bridge".to_owned())
            .spawn(move || {
                // One frame is one message, so `decode` sees exactly what a
                // `midir` callback would have handed it.
                while let Ok(Some((Kind::Midi, msg))) = wire::read(&mut reader) {
                    if let Some(event) = decode(&msg) {
                        if tx.send(event).is_err() {
                            break;
                        }
                    }
                }
                seen.store(false, Ordering::Relaxed);
            })
            .map_err(|e| named(&e))?;
        Ok((Self { stream, open }, events))
    }

    fn alive(&self) -> Result<(), Error> {
        if self.open.load(Ordering::Relaxed) {
            Ok(())
        } else {
            Err(Error::Bridge("the bridge hung up".to_owned()))
        }
    }
}

impl Drop for Bridge {
    /// Close both halves. The reader holds a clone of the socket, so
    /// dropping this one alone would leave it open, and the bridge would
    /// wait out its silence instead of seeing the hang-up.
    fn drop(&mut self) {
        let _closed = self.stream.shutdown(Shutdown::Both);
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
        assert_eq!(decode(&CLOCK), None);
        assert_eq!(decode(&[0xC0, 1, 0]), None); // program change
        assert_eq!(decode(&[0x90, 200, 127]), None); // off-surface index
    }
}
