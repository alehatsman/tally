//! What the bridge does with one connection from `tallyd`, and the one
//! thing it does on its own.
//!
//! **Generic over the device on purpose, and not through a trait.** A
//! session needs a way to send bytes to the Launchpad and a channel of
//! what came back, which is what `midir` gives the binary and what a test
//! can give it with a closure. Nothing here knows a MIDI port exists, so
//! all of it runs without hardware, on both sides of the line.

use std::io::{self, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use crate::wire::{self, Hello, Kind};

/// The substring that identifies the Launchpad's port.
///
/// A copy of `tally::device::PORT_SUBSTRING`, because `tally`'s library
/// does not build on Windows. A test in `tally` asserts the two are equal,
/// so the copy cannot drift.
pub const PORT_SUBSTRING: &str = "LPMiniMK3 MIDI";

/// Live mode: the message that gives the Setup menu back.
///
/// This and the two below are copied from `tally::surface::sysex` for the
/// same reason as [`PORT_SUBSTRING`], and checked by the same test. They
/// are fixed bytes out of Novation's manual and do not move.
pub const LIVE: [u8; 9] = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0D, 0x0E, 0x00, 0xF7];
/// Ask which mode the device is in. The reply is the only proof a
/// restore happened.
pub const MODE_READBACK: [u8; 8] = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0D, 0x0E, 0xF7];
/// Stop any text scroll, so a device left mid-scroll is put back too.
pub const STOP_SCROLL: [u8; 8] = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0D, 0x07, 0xF7];

/// How long `tallyd` may say nothing before the bridge stops waiting.
///
/// `tallyd` pings every 2 s and clocks the breath at 24 PPQN in between,
/// so a quiet socket is not an idle one. It is a `tallyd` that died
/// without the socket closing -- which is what a WSL shutdown can look
/// like from Windows. Seven seconds, the same as `tallyd`'s own
/// `DEVICE_SILENCE` for the same job in the other direction.
pub const SILENCE: Duration = Duration::from_secs(7);

/// How long to wait for the device to answer a mode readback. The same
/// generous figure `tallyd` uses: a cold round trip measured ~345 ms.
pub const REPLY_TIMEOUT: Duration = Duration::from_secs(2);

/// How often the forwarding thread looks up to see whether the session is
/// over. Bounds how long the end of a session takes, nothing else.
const POLL: Duration = Duration::from_millis(50);

/// Why a session ended. Every one of them is followed by [`restore`].
#[derive(Debug)]
pub enum Ended {
    /// `tallyd` closed the socket: a clean exit, a crash or a `SIGKILL`.
    Hangup,
    /// Nothing from `tallyd` for [`SILENCE`].
    Silence,
    /// The device refused a write: usually, the cable came out.
    Device(String),
    /// The socket broke, or the peer spoke something that is not the wire.
    Broken(io::Error),
}

/// What [`restore`] found and did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restore {
    /// The device already said Live mode. `tallyd` put it back itself on
    /// the way out, which is the clean path.
    AlreadyLive,
    /// It was in programmer mode, and a readback confirmed it no longer
    /// is.
    Restored,
    /// No confirmation. The device may be gone; if it is not, its Setup
    /// menu may still be disabled.
    Unconfirmed,
}

/// Serve one connection whose device is already open: say hello, pipe
/// both ways until `tallyd` goes, then put the device back.
///
/// `send` writes one message to the device. `from_device` is every
/// message the device sent, one per item, as its MIDI callback received
/// them.
///
/// **One message per frame.** A message read from the device becomes
/// exactly one frame; a frame from `tallyd` becomes exactly one `send`.
/// Nothing is parsed, merged or split.
pub fn run<S>(
    stream: &TcpStream,
    send: &mut S,
    from_device: &Receiver<Vec<u8>>,
    silence: Duration,
) -> (Ended, Restore)
where
    S: FnMut(&[u8]) -> Result<(), String> + Send,
{
    let ended = pipe(stream, send, from_device, silence);
    // Whatever happened, the socket is finished: say so to the other end
    // too, so a `tallyd` that is still alive sees a hang-up rather than a
    // write into a buffer nobody drains.
    let _closed = stream.shutdown(Shutdown::Both);
    (ended, restore(send, from_device))
}

/// Tell `tallyd` there is no session to be had, and why.
///
/// # Errors
/// Whatever the socket said.
pub fn refuse(mut stream: &TcpStream, why: &Hello) -> io::Result<()> {
    wire::write(&mut stream, Kind::Hello, &why.encode())?;
    stream.flush()
}

fn pipe<S>(
    stream: &TcpStream,
    send: &mut S,
    from_device: &Receiver<Vec<u8>>,
    silence: Duration,
) -> Ended
where
    S: FnMut(&[u8]) -> Result<(), String> + Send,
{
    // Not an optimisation. The breath is single-byte `F8` pulses, and
    // Nagle's algorithm exists to hold small writes back until the last
    // one is acknowledged (docs/13-bridge.md, question 5).
    if let Err(e) = stream.set_nodelay(true) {
        return Ended::Broken(e);
    }
    if let Err(e) = stream.set_read_timeout(Some(silence)) {
        return Ended::Broken(e);
    }
    let mut hello = stream;
    if let Err(e) = wire::write(&mut hello, Kind::Hello, &Hello::Present.encode()) {
        return Ended::Broken(e);
    }

    // The device side stays on this thread because a `Receiver` cannot be
    // shared; the socket side is the one that moves.
    let done = AtomicBool::new(false);
    thread::scope(|scope| {
        let inbound = scope.spawn(|| {
            let ended = inbound(stream, send);
            done.store(true, Ordering::Relaxed);
            ended
        });
        forward(stream, from_device, &done);
        inbound
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}

/// Device to `tallyd`: one message, one frame, until the session ends or
/// the socket will not take another.
fn forward(mut stream: &TcpStream, from_device: &Receiver<Vec<u8>>, done: &AtomicBool) {
    while !done.load(Ordering::Relaxed) {
        match from_device.recv_timeout(POLL) {
            Ok(msg) => {
                if wire::write(&mut stream, Kind::Midi, &msg).is_err() {
                    return;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

/// `tallyd` to the device, until something ends the session.
fn inbound<S>(mut stream: &TcpStream, send: &mut S) -> Ended
where
    S: FnMut(&[u8]) -> Result<(), String> + Send,
{
    loop {
        match wire::read(&mut stream) {
            Ok(Some((Kind::Midi, msg))) => {
                if let Err(e) = send(&msg) {
                    return Ended::Device(e);
                }
            }
            Ok(Some((Kind::Hello, _))) => {
                return Ended::Broken(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "tallyd sent a hello, which only the bridge says",
                ))
            }
            Ok(None) => return Ended::Hangup,
            // A read timeout is `WouldBlock` on unix and `TimedOut` on
            // Windows; both mean the same silence.
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Ended::Silence
            }
            Err(e) => return Ended::Broken(e),
        }
    }
}

/// Put the device back in Live mode, unless it already is.
///
/// **The one place the bridge is more than a pipe**, and it is on purpose.
/// Programmer mode disables the device's Setup menu, and `tallyd`'s own
/// restore runs on its way out -- over this socket. If `tallyd` was
/// killed, or WSL shut down, or the socket dropped, the moment the restore
/// is needed is the moment it can no longer be sent from there.
///
/// It **asks first**. After a clean `tallyd` exit the device is already in
/// Live mode, and nothing about re-sending Live mode to a device in Live
/// mode has been measured; a readback has. Otherwise: stop any scroll,
/// Live mode, and read it back, because a write is not an acknowledgement
/// (`docs/08-daemon.md`). The dark repaint `tallyd` sends first is left
/// out -- measured on hardware to make no difference to where the device
/// ends up, and it would be the bridge's only copy of a frame.
pub fn restore<S>(send: &mut S, from_device: &Receiver<Vec<u8>>) -> Restore
where
    S: FnMut(&[u8]) -> Result<(), String>,
{
    // Presses and replies from the session that just ended are nobody's.
    while from_device.try_recv().is_ok() {}

    if send(&MODE_READBACK).is_ok() && await_mode(from_device) == Some(false) {
        return Restore::AlreadyLive;
    }
    for msg in [STOP_SCROLL.as_slice(), &LIVE, &MODE_READBACK] {
        if send(msg).is_err() {
            return Restore::Unconfirmed;
        }
    }
    match await_mode(from_device) {
        Some(false) => Restore::Restored,
        _ => Restore::Unconfirmed,
    }
}

/// Wait for a mode readback, discarding anything else.
fn await_mode(from_device: &Receiver<Vec<u8>>) -> Option<bool> {
    let deadline = Instant::now() + REPLY_TIMEOUT;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return None;
        }
        match from_device.recv_timeout(left) {
            Ok(reply) => {
                if let Some(programmer) = programmer_mode_from(&reply) {
                    return Some(programmer);
                }
            }
            Err(_) => return None,
        }
    }
}

/// `Some(true)` for a readback saying programmer mode, `Some(false)` for
/// Live, `None` for anything that is not a readback.
pub fn programmer_mode_from(reply: &[u8]) -> Option<bool> {
    match reply {
        [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0D, 0x0E, mode, 0xF7] => Some(*mode == 0x01),
        _ => None,
    }
}

/// Which of the ports a MIDI stack lists is the Launchpad's.
///
/// Every name containing `substring` is a candidate, and **one** is the
/// common case -- macOS and Linux name the two interfaces `... DAW ...`
/// and `... MIDI ...`, and only the second contains the substring. Windows
/// names ports differently: the classic USB-MIDI driver gives a device's
/// first interface its bare name and the second `MIDIIN2 (...)` /
/// `MIDIOUT2 (...)`. If both of the Launchpad's land on the substring,
/// the numbered one is the second interface, and the second interface is
/// the MIDI one.
///
/// ⚠️ **Unverified on hardware.** No Launchpad has been plugged into the
/// Windows desk yet; `tallybridge --list` prints what this would choose,
/// and `--port` overrides it (docs/13-bridge.md, the checklist).
pub fn pick<'a>(names: &'a [String], substring: &str) -> Option<&'a str> {
    let mut matching = names.iter().filter(|n| n.contains(substring));
    let first = matching.next()?;
    let numbered = |n: &&String| n.starts_with("MIDIIN") || n.starts_with("MIDIOUT");
    if numbered(&first) {
        return Some(first);
    }
    Some(matching.find(numbered).unwrap_or(first))
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;
    use std::sync::mpsc::{self, Sender};
    use std::sync::{Arc, Mutex};

    use super::*;

    /// A Launchpad as far as a session can tell: it records what it was
    /// sent and answers a mode readback with the mode it is in.
    #[derive(Clone)]
    struct Fake {
        got: Arc<Mutex<Vec<Vec<u8>>>>,
        programmer: Arc<AtomicBool>,
        reply: Sender<Vec<u8>>,
        unplugged: Arc<AtomicBool>,
    }

    impl Fake {
        fn new(programmer: bool) -> (Self, Receiver<Vec<u8>>) {
            let (reply, from_device) = mpsc::channel();
            let fake = Self {
                got: Arc::default(),
                programmer: Arc::new(AtomicBool::new(programmer)),
                reply,
                unplugged: Arc::default(),
            };
            (fake, from_device)
        }

        fn send(&self, msg: &[u8]) -> Result<(), String> {
            if self.unplugged.load(Ordering::Relaxed) {
                return Err("the device is gone".to_owned());
            }
            self.got.lock().unwrap().push(msg.to_vec());
            match msg {
                [.., 0x0E, 0x01, 0xF7] => self.programmer.store(true, Ordering::Relaxed),
                [.., 0x0E, 0x00, 0xF7] => self.programmer.store(false, Ordering::Relaxed),
                m if m == MODE_READBACK => {
                    let mode = u8::from(self.programmer.load(Ordering::Relaxed));
                    let _sent = self
                        .reply
                        .send(vec![0xF0, 0x00, 0x20, 0x29, 0x02, 0x0D, 0x0E, mode, 0xF7]);
                }
                _ => {}
            }
            Ok(())
        }

        fn sent(&self) -> Vec<Vec<u8>> {
            self.got.lock().unwrap().clone()
        }
    }

    /// A session on a real loopback socket, with `fake` behind it.
    fn serve(
        fake: Fake,
        from_device: Receiver<Vec<u8>>,
        silence: Duration,
    ) -> (TcpStream, thread::JoinHandle<(Ended, Restore)>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let addr = listener.local_addr().expect("its address");
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("tallyd connects");
            run(&stream, &mut |m| fake.send(m), &from_device, silence)
        });
        let client = TcpStream::connect(addr).expect("connect to the bridge");
        client
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("a bounded test");
        (client, handle)
    }

    #[test]
    fn a_session_says_hello_then_pipes_both_ways() {
        let (fake, from_device) = Fake::new(true);
        let press = fake.reply.clone();
        let (mut tallyd, bridge) = serve(fake.clone(), from_device, SILENCE);

        assert_eq!(
            wire::read(&mut tallyd).unwrap(),
            Some((Kind::Hello, Hello::Present.encode()))
        );

        // Two presses back to back are two frames, not one.
        press.send(vec![0x90, 0x0B, 0x7F]).unwrap();
        press.send(vec![0x90, 0x0B, 0x00]).unwrap();
        assert_eq!(
            wire::read(&mut tallyd).unwrap().unwrap().1,
            [0x90, 0x0B, 0x7F]
        );
        assert_eq!(
            wire::read(&mut tallyd).unwrap().unwrap().1,
            [0x90, 0x0B, 0x00]
        );

        // And what tallyd sends reaches the device one message at a time.
        wire::write(&mut tallyd, Kind::Midi, &[0xF8]).unwrap();
        wire::write(&mut tallyd, Kind::Midi, &[0xF8]).unwrap();
        drop(tallyd);

        let (ended, restored) = bridge.join().unwrap();
        assert!(matches!(ended, Ended::Hangup), "{ended:?}");
        assert_eq!(
            restored,
            Restore::Restored,
            "it was left in programmer mode"
        );
        let sent = fake.sent();
        assert_eq!(sent.get(..2), Some([vec![0xF8], vec![0xF8]].as_slice()));
        assert!(sent.contains(&LIVE.to_vec()), "{sent:02X?}");
    }

    /// The clean path: `tallyd` restored the device itself, so the bridge
    /// asks, hears Live, and sends nothing else.
    #[test]
    fn a_device_already_in_live_mode_is_left_alone() {
        let (fake, from_device) = Fake::new(false);
        let (tallyd, bridge) = serve(fake.clone(), from_device, SILENCE);
        drop(tallyd);
        let (_, restored) = bridge.join().unwrap();
        assert_eq!(restored, Restore::AlreadyLive);
        assert_eq!(
            fake.sent(),
            [MODE_READBACK.to_vec()],
            "a question, no orders"
        );
    }

    /// A `tallyd` that went away without closing the socket -- WSL shut
    /// down under it -- is noticed by its silence, and the device is
    /// restored anyway.
    #[test]
    fn silence_ends_a_session_and_still_restores() {
        let (fake, from_device) = Fake::new(true);
        let (mut tallyd, bridge) = serve(fake.clone(), from_device, Duration::from_millis(200));
        let _hello = wire::read(&mut tallyd).unwrap();
        // Held open, saying nothing.
        let (ended, restored) = bridge.join().unwrap();
        assert!(matches!(ended, Ended::Silence), "{ended:?}");
        assert_eq!(restored, Restore::Restored);
        assert_eq!(
            wire::read(&mut tallyd).unwrap(),
            None,
            "and tallyd is told, if it is still there to hear"
        );
    }

    /// An unplugged cable ends the session, so `tallyd` sees a hang-up
    /// and goes back to looking for a device. The restore cannot work and
    /// says so rather than claiming it did.
    #[test]
    fn a_device_that_refuses_a_write_ends_the_session() {
        let (fake, from_device) = Fake::new(true);
        let (mut tallyd, bridge) = serve(fake.clone(), from_device, SILENCE);
        let _hello = wire::read(&mut tallyd).unwrap();
        fake.unplugged.store(true, Ordering::Relaxed);
        wire::write(&mut tallyd, Kind::Midi, &[0xF8]).unwrap();
        let (ended, restored) = bridge.join().unwrap();
        assert!(matches!(ended, Ended::Device(_)), "{ended:?}");
        assert_eq!(restored, Restore::Unconfirmed);
        assert_eq!(wire::read(&mut tallyd).unwrap(), None);
    }

    #[test]
    fn a_peer_that_is_not_tallyd_is_hung_up_on() {
        let (fake, from_device) = Fake::new(false);
        let (mut tallyd, bridge) = serve(fake, from_device, SILENCE);
        tallyd.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();
        let (ended, _) = bridge.join().unwrap();
        assert!(matches!(ended, Ended::Broken(_)), "{ended:?}");
    }

    #[test]
    fn a_refusal_is_one_hello_and_a_hang_up() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let bridge = thread::spawn(move || {
            let (s, _) = listener.accept().unwrap();
            refuse(&s, &Hello::Absent).unwrap();
        });
        let mut tallyd = TcpStream::connect(addr).unwrap();
        bridge.join().unwrap();
        assert_eq!(
            wire::read(&mut tallyd).unwrap(),
            Some((Kind::Hello, Hello::Absent.encode()))
        );
        assert_eq!(wire::read(&mut tallyd).unwrap(), None);
    }

    #[test]
    fn only_a_readback_reads_as_a_mode() {
        assert_eq!(
            programmer_mode_from(&[0xF0, 0, 0x20, 0x29, 2, 0x0D, 0x0E, 1, 0xF7]),
            Some(true)
        );
        assert_eq!(
            programmer_mode_from(&[0xF0, 0, 0x20, 0x29, 2, 0x0D, 0x0E, 0, 0xF7]),
            Some(false)
        );
        assert_eq!(programmer_mode_from(&LIVE[..8]), None);
        assert_eq!(programmer_mode_from(&[0x90, 0x0B, 0x7F]), None);
    }

    fn names(n: &[&str]) -> Vec<String> {
        n.iter().map(|&s| s.to_owned()).collect()
    }

    #[test]
    fn the_port_with_the_substring_is_picked() {
        // CoreMIDI's two spellings of the input, and the DAW port beside it.
        let mac = names(&["LPMiniMK3 DAW Out", "LPMiniMK3 MIDI Out"]);
        assert_eq!(pick(&mac, PORT_SUBSTRING), Some("LPMiniMK3 MIDI Out"));
        let long = names(&["Launchpad Mini MK3 LPMiniMK3 MIDI Out"]);
        assert_eq!(
            pick(&long, PORT_SUBSTRING),
            Some("Launchpad Mini MK3 LPMiniMK3 MIDI Out")
        );
        assert_eq!(
            pick(&names(&["Microsoft GS Wavetable Synth"]), PORT_SUBSTRING),
            None
        );
    }

    /// Windows' numbered name is the second interface, whichever order the
    /// stack lists them in.
    #[test]
    fn on_windows_the_numbered_port_is_the_second_interface() {
        for order in [
            names(&["LPMiniMK3 MIDI", "MIDIIN2 (LPMiniMK3 MIDI)"]),
            names(&["MIDIIN2 (LPMiniMK3 MIDI)", "LPMiniMK3 MIDI"]),
        ] {
            assert_eq!(
                pick(&order, PORT_SUBSTRING),
                Some("MIDIIN2 (LPMiniMK3 MIDI)")
            );
        }
        let out = names(&["LPMiniMK3 MIDI", "MIDIOUT2 (LPMiniMK3 MIDI)"]);
        assert_eq!(
            pick(&out, PORT_SUBSTRING),
            Some("MIDIOUT2 (LPMiniMK3 MIDI)")
        );
    }
}
