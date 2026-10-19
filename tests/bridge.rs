//! The device over the bridge, end to end and with no hardware.
//!
//! `Device` on one side, the bridge's real session on the other, a
//! loopback socket between them and a fake Launchpad behind the bridge.
//! Every byte crosses the same codec it would between WSL and Windows;
//! the only thing not real is the USB cable (`docs/13-bridge.md`).

use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use tally::device::{self, Device, Error, Event};
use tally::surface::sysex;
use tally::surface::{Button, Frame};
use tallybridge::session::{self, Ended, Restore};
use tallybridge::wire::Hello;

/// The inquiry reply a Mini MK3 sent on 2026-10-16, byte for byte
/// (`docs/03-surface.md`).
const INQUIRY_REPLY: [u8; 17] = [
    0xF0, 0x7E, 0x00, 0x06, 0x02, 0x00, 0x20, 0x29, 0x13, 0x01, 0x00, 0x00, 0x00, 0x03, 0x04, 0x01,
    0xF7,
];

/// A Launchpad as far as the bytes can tell: it identifies itself, keeps
/// a mode, answers readbacks, and records everything it was sent.
#[derive(Clone)]
struct Launchpad {
    got: Arc<Mutex<Vec<Vec<u8>>>>,
    programmer: Arc<AtomicBool>,
    unplugged: Arc<AtomicBool>,
    to_host: Sender<Vec<u8>>,
}

impl Launchpad {
    fn new() -> (Self, Receiver<Vec<u8>>) {
        let (to_host, from_device) = mpsc::channel();
        let pad = Self {
            got: Arc::default(),
            programmer: Arc::default(),
            unplugged: Arc::default(),
            to_host,
        };
        (pad, from_device)
    }

    fn receive(&self, msg: &[u8]) -> Result<(), String> {
        if self.unplugged.load(Ordering::Relaxed) {
            return Err("the cable is out".to_owned());
        }
        self.got.lock().expect("the record").push(msg.to_vec());
        let answer = if msg == sysex::INQUIRY {
            Some(INQUIRY_REPLY.to_vec())
        } else if msg == sysex::PROGRAMMER || msg == sysex::LIVE {
            self.programmer
                .store(msg == sysex::PROGRAMMER, Ordering::Relaxed);
            None
        } else if msg == sysex::MODE_READBACK {
            let mode = u8::from(self.programmer.load(Ordering::Relaxed));
            Some(vec![0xF0, 0x00, 0x20, 0x29, 0x02, 0x0D, 0x0E, mode, 0xF7])
        } else {
            None
        };
        if let Some(reply) = answer {
            // A closed channel is a session that is over.
            drop(self.to_host.send(reply));
        }
        Ok(())
    }

    /// Something the user did on the hardware.
    fn press(&self, msg: &[u8]) {
        self.to_host
            .send(msg.to_vec())
            .expect("the bridge is listening");
    }

    fn sent(&self) -> Vec<Vec<u8>> {
        self.got.lock().expect("the record").clone()
    }

    /// What it was sent, once it has had `n` messages or two seconds.
    /// A write returning is not the bridge having delivered it.
    fn received(&self, n: usize) -> Vec<Vec<u8>> {
        let deadline = Instant::now() + Duration::from_secs(2);
        while self.sent().len() < n && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(2));
        }
        self.sent()
    }
}

/// A bridge serving one connection, with `pad` behind it.
fn bridge(
    pad: &Launchpad,
    from_device: Receiver<Vec<u8>>,
) -> (String, JoinHandle<(Ended, Restore)>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let addr = listener.local_addr().expect("its address").to_string();
    let pad = pad.clone();
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("tallyd connects");
        session::run(
            &stream,
            &mut |m| pad.receive(m),
            &from_device,
            session::SILENCE,
        )
    });
    (addr, handle)
}

/// A bridge that answers one connection with `hello` and hangs up.
fn refusing(hello: Hello) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let addr = listener.local_addr().expect("its address").to_string();
    thread::spawn(move || {
        let (stream, _) = listener.accept().expect("tallyd connects");
        session::refuse(&stream, &hello).expect("the hello goes out");
    });
    addr
}

/// Poll until `n` events arrived or two seconds passed.
fn events(device: &Device, n: usize) -> Vec<Event> {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut got = Vec::new();
    while got.len() < n && Instant::now() < deadline {
        got.extend(device.poll());
        thread::sleep(Duration::from_millis(5));
    }
    got
}

/// The whole life of a device over the bridge: the same start-up bytes in
/// the same order as a local port, a diff, presses, a clock pulse, and a
/// restore on the way out that the bridge then finds already done.
#[test]
fn a_device_over_the_bridge_says_what_a_local_one_says() {
    let (pad, from_device) = Launchpad::new();
    let (addr, served) = bridge(&pad, from_device);

    let mut device = Device::over_bridge(&addr).expect("the device opens through the bridge");
    assert_eq!(
        pad.received(4),
        [
            sysex::INQUIRY.to_vec(),
            sysex::PROGRAMMER.to_vec(),
            sysex::led_feedback(false, false),
            Frame::dark().repaint(),
        ],
        "identity, programmer mode, feedback off, a total frame"
    );
    assert!(pad.programmer.load(Ordering::Relaxed));

    let mut frame = Frame::dark();
    frame.set(
        Button::Grid { row: 8, col: 1 },
        tally::surface::Cell::Static(21),
    );
    device.render(&frame).unwrap();
    device.render(&frame).unwrap();
    device.clock().unwrap();
    let sent = pad.received(6);
    assert_eq!(
        sent.get(4..),
        Some([frame.diff(&Frame::dark()).unwrap(), vec![0xF8]].as_slice()),
        "one diff, nothing for an unchanged frame, one pulse"
    );

    // Two presses back to back: two events, not one lost blob.
    pad.press(&[0x90, 0x0B, 0x7F]);
    pad.press(&[0x90, 0x0B, 0x00]);
    let bottom_left = Button::Grid { row: 8, col: 1 };
    assert_eq!(
        events(&device, 2),
        [
            Event::Press {
                button: bottom_left,
                down: true
            },
            Event::Press {
                button: bottom_left,
                down: false
            },
        ]
    );

    drop(device);
    let (ended, restored) = served.join().unwrap();
    assert!(matches!(ended, Ended::Hangup), "{ended:?}");
    assert!(
        pad.sent().contains(&sysex::LIVE.to_vec()),
        "tallyd's own restore crossed the wire"
    );
    assert!(
        !pad.programmer.load(Ordering::Relaxed),
        "the Setup menu is back"
    );
    assert_eq!(
        restored,
        Restore::AlreadyLive,
        "so the bridge had nothing to do"
    );
}

/// Nothing listening is the bridge's unplugged cable: the ordinary,
/// silent `NotFound` that `Link` retries on the reconcile clock.
#[test]
fn no_bridge_is_not_found() {
    let addr = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().to_string()
    };
    assert!(matches!(Device::over_bridge(&addr), Err(Error::NotFound)));
}

/// A bridge with no Launchpad is the same state, reached a different way.
#[test]
fn a_bridge_without_a_launchpad_is_not_found() {
    let addr = refusing(Hello::Absent);
    assert!(matches!(Device::over_bridge(&addr), Err(Error::NotFound)));
}

/// A port another program holds is not an unplugged cable, and it says
/// so -- `Link` logs anything that is not `NotFound`.
#[test]
fn a_port_the_bridge_could_not_open_is_an_error_that_says_why() {
    let addr = refusing(Hello::Refused(
        "MIDIIN2 (LPMiniMK3 MIDI): in use".to_owned(),
    ));
    match Device::over_bridge(&addr) {
        Err(Error::Bridge(why)) => assert!(why.contains("in use"), "{why}"),
        other => panic!("expected a bridge error, got {other:?}"),
    }
}

/// Something listening that is not a bridge is an error, not a hang.
#[test]
fn a_listener_that_never_says_hello_is_an_error() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let held = thread::spawn(move || {
        let (s, _) = listener.accept().unwrap();
        thread::sleep(Duration::from_millis(2500));
        drop(s);
    });
    let started = Instant::now();
    assert!(matches!(Device::over_bridge(&addr), Err(Error::Bridge(_))));
    assert!(
        started.elapsed() < Duration::from_millis(2500),
        "bounded by the reply timeout"
    );
    held.join().unwrap();
}

/// When the bridge hangs up -- here because the cable came out under it --
/// the very next frame fails, even one with nothing new to send. That is
/// what lets `Link` drop the device on the next tick rather than after
/// seven seconds of silence.
#[test]
fn a_bridge_that_hangs_up_fails_the_next_frame() {
    let (pad, from_device) = Launchpad::new();
    let (addr, served) = bridge(&pad, from_device);
    let mut device = Device::over_bridge(&addr).unwrap();

    pad.unplugged.store(true, Ordering::Relaxed);
    device.clock().unwrap();
    let (ended, restored) = served.join().unwrap();
    assert!(matches!(ended, Ended::Device(_)), "{ended:?}");
    assert_eq!(
        restored,
        Restore::Unconfirmed,
        "the bridge cannot restore a device that is gone"
    );

    let deadline = Instant::now() + Duration::from_secs(2);
    let failed = loop {
        match device.render(&Frame::dark()) {
            Err(e) => break e,
            Ok(()) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            Ok(()) => panic!("the hang-up was never noticed"),
        }
    };
    assert!(matches!(failed, Error::Bridge(_)), "{failed:?}");
    device.abandon();
}

/// The bridge's copies of tally's constants, checked rather than trusted.
/// `tally`'s library does not build on Windows, so the bridge cannot use
/// these directly; this is what stops the copies drifting.
#[test]
fn the_bridges_copies_are_tallys_bytes() {
    assert_eq!(session::PORT_SUBSTRING, device::PORT_SUBSTRING);
    assert_eq!(session::LIVE, sysex::LIVE);
    assert_eq!(session::MODE_READBACK, sysex::MODE_READBACK);
    assert_eq!(session::STOP_SCROLL, sysex::STOP_SCROLL);
    for mode in [0x00, 0x01] {
        let reply = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0D, 0x0E, mode, 0xF7];
        assert_eq!(
            session::programmer_mode_from(&reply),
            sysex::programmer_mode_from(&reply)
        );
    }
}

/// A connection is one session: the bridge hangs up after it, so the next
/// `Device::over_bridge` is a new connection and a total frame.
#[test]
fn a_dropped_device_closes_its_socket() {
    let (pad, from_device) = Launchpad::new();
    let (addr, served) = bridge(&pad, from_device);
    let device = Device::over_bridge(&addr).unwrap();
    drop(device);
    // Joining at all is the assertion: a socket left half open would hold
    // the bridge for its whole silence.
    let started = Instant::now();
    let _ = served.join().unwrap();
    assert!(started.elapsed() < session::SILENCE);
    let _refused = TcpStream::connect(&addr).expect_err("that bridge served once");
}
