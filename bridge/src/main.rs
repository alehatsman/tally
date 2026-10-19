//! `tallybridge` -- the Windows half of tally.
//!
//! Owns the Launchpad's MIDI port and moves its bytes to `tallyd` in WSL
//! over a loopback socket, one connection at a time. It holds no model:
//! everything a pad means is decided in `tallyd` (`docs/13-bridge.md`).
//!
//! ```text
//! tallybridge [--listen 127.0.0.1:7373] [--port SUBSTRING]
//! tallybridge --list [--port SUBSTRING]
//! ```

use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::ExitCode;
use std::sync::mpsc::{self, Receiver, Sender};

use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use tallybridge::session::{self, Ended, Restore};
use tallybridge::wire::{self, Hello};

const USAGE: &str = "\
usage: tallybridge [--listen ADDR] [--port SUBSTRING]
       tallybridge --list [--port SUBSTRING]

  --listen ADDR   where tallyd connects; loopback only (default 127.0.0.1:7373)
  --port TEXT     the substring naming the Launchpad's port (default \"LPMiniMK3 MIDI\")
  --list          print every MIDI port and which two would be used, then exit";

/// The command line, parsed.
#[derive(Debug)]
struct Args {
    listen: SocketAddr,
    port: String,
    list: bool,
}

fn parse(mut argv: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut args = Args {
        listen: wire::DEFAULT_ADDR
            .parse()
            .map_err(|e| format!("the default address: {e}"))?,
        port: session::PORT_SUBSTRING.to_owned(),
        list: false,
    };
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "--listen" => {
                let v = argv.next().ok_or("--listen needs an address")?;
                args.listen = v.parse().map_err(|e| format!("--listen {v}: {e}"))?;
            }
            "--port" => args.port = argv.next().ok_or("--port needs a substring")?,
            "--list" => args.list = true,
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    // The security decision, enforced rather than documented. Under WSL's
    // mirrored networking a loopback listener is reachable from WSL and
    // from nothing on the network (docs/13-bridge.md, question 1). Any
    // other address puts an unauthenticated socket that drives the grid
    // on the LAN, and there is no flag for that.
    if !args.listen.ip().is_loopback() {
        return Err(format!(
            "--listen {}: not a loopback address. The socket is unauthenticated, so it \
             listens on loopback only; WSL reaches it under mirrored networking",
            args.listen
        ));
    }
    Ok(args)
}

fn main() -> ExitCode {
    let args = match parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("tallybridge: {e}");
            }
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    if args.list {
        return list(&args.port);
    }

    let listener = match TcpListener::bind(args.listen) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("tallybridge: {}: {e}", args.listen);
            return ExitCode::FAILURE;
        }
    };
    stop::install(args.listen);

    // A bridge killed hard (`taskkill /F`) leaves the device in
    // programmer mode, and nothing it did can have run. Recovering on
    // start is the half that is reliable (docs/08-daemon.md), so the
    // first thing a new bridge does is put back whatever the last one
    // left.
    if let Ok(mut midi) = Midi::open(&args.port) {
        let restored = session::restore(&mut |m| send(&mut midi.out, m), &midi.from_device);
        if restored != Restore::AlreadyLive {
            eprintln!("tallybridge: found the Launchpad out of Live mode at start: {restored:?}");
        }
    }

    eprintln!(
        "tallybridge: listening on {} for tallyd; Launchpad port matches {:?}",
        args.listen, args.port
    );
    let mut absent = false;
    for conn in listener.incoming() {
        if stop::requested() {
            break;
        }
        match conn {
            Ok(stream) => serve(&stream, &args.port, &mut absent),
            Err(e) => eprintln!("tallybridge: accept: {e}"),
        }
        if stop::requested() {
            break;
        }
    }
    stop::finished();
    ExitCode::SUCCESS
}

/// One connection from `tallyd`, start to finish.
///
/// The port is opened **per connection** and closed after it. Between
/// sessions the device is in Live mode and the bridge holds nothing, so
/// a bridge with no `tallyd` is a bridge that is not in anyone's way.
fn serve(stream: &TcpStream, port: &str, absent: &mut bool) {
    let peer = stream
        .peer_addr()
        .map_or_else(|_| "?".to_owned(), |a| a.to_string());
    let mut midi = match Midi::open(port) {
        Ok(m) => m,
        Err(hello) => {
            // tallyd asks every two seconds while the cable is out; say
            // it once, not every time.
            if hello != Hello::Absent || !*absent {
                eprintln!("tallybridge: {peer}: no session: {hello:?}");
            }
            *absent = hello == Hello::Absent;
            if let Err(e) = session::refuse(stream, &hello) {
                eprintln!("tallybridge: {peer}: {e}");
            }
            return;
        }
    };
    *absent = false;
    eprintln!(
        "tallybridge: {peer}: session on {:?} / {:?}",
        midi.in_name, midi.out_name
    );
    stop::watch(Some(stream));
    let (ended, restored) = session::run(
        stream,
        &mut |m| send(&mut midi.out, m),
        &midi.from_device,
        session::SILENCE,
    );
    stop::watch(None);
    let why = match &ended {
        Ended::Hangup => "tallyd hung up".to_owned(),
        Ended::Silence => format!("nothing from tallyd for {:?}", session::SILENCE),
        Ended::Device(e) => format!("the device refused a write ({e})"),
        Ended::Broken(e) => format!("the socket broke ({e})"),
    };
    match restored {
        Restore::AlreadyLive | Restore::Restored => {
            eprintln!("tallybridge: {peer}: {why}; Launchpad in Live mode ({restored:?})");
        }
        Restore::Unconfirmed => eprintln!(
            "tallybridge: {peer}: {why}; the Launchpad did not confirm Live mode -- if it is \
             still plugged in, its Setup menu may be disabled until a replug"
        ),
    }
}

/// The open port pair, and what the device has sent.
struct Midi {
    out: MidiOutputConnection,
    /// Held so the callback keeps running; never read directly.
    _input: MidiInputConnection<Sender<Vec<u8>>>,
    from_device: Receiver<Vec<u8>>,
    in_name: String,
    out_name: String,
}

impl Midi {
    /// Open the Launchpad's ports, or say in a hello why not.
    fn open(substring: &str) -> Result<Self, Hello> {
        let refused =
            |what: &str, e: &dyn std::fmt::Display| Hello::Refused(format!("{what}: {e}"));
        let mut input = MidiInput::new("tallybridge").map_err(|e| refused("midi in", &e))?;
        // SysEx is how the device identifies itself and reports its mode.
        input.ignore(midir::Ignore::None);
        let output = MidiOutput::new("tallybridge").map_err(|e| refused("midi out", &e))?;

        // In and Out are named from the device's side, so our input reads
        // the port the device calls Out. `pick` only needs the substring.
        let in_ports = input.ports();
        let in_names: Vec<String> = in_ports
            .iter()
            .map(|p| input.port_name(p).unwrap_or_default())
            .collect();
        let out_ports = output.ports();
        let out_names: Vec<String> = out_ports
            .iter()
            .map(|p| output.port_name(p).unwrap_or_default())
            .collect();
        let in_name = session::pick(&in_names, substring).ok_or(Hello::Absent)?;
        let out_name = session::pick(&out_names, substring).ok_or(Hello::Absent)?;
        let in_port = in_names
            .iter()
            .position(|n| n == in_name)
            .and_then(|i| in_ports.get(i))
            .ok_or(Hello::Absent)?;
        let out_port = out_names
            .iter()
            .position(|n| n == out_name)
            .and_then(|i| out_ports.get(i))
            .ok_or(Hello::Absent)?;

        let (tx, from_device) = mpsc::channel();
        let input_conn = input
            .connect(
                in_port,
                "tallybridge-in",
                |_t, bytes, tx: &mut Sender<Vec<u8>>| {
                    // A closed receiver means the session is over.
                    drop(tx.send(bytes.to_vec()));
                },
                tx,
            )
            .map_err(|e| refused(in_name, &e))?;
        let out = output
            .connect(out_port, "tallybridge-out")
            .map_err(|e| refused(out_name, &e))?;
        Ok(Self {
            out,
            _input: input_conn,
            from_device,
            in_name: in_name.to_owned(),
            out_name: out_name.to_owned(),
        })
    }
}

/// One message to the device. A free function over the output alone, so a
/// closure calling it borrows the port and leaves `from_device` free.
fn send(out: &mut MidiOutputConnection, bytes: &[u8]) -> Result<(), String> {
    out.send(bytes).map_err(|e| e.to_string())
}

/// `--list`: every port, and which two a session would open.
fn list(substring: &str) -> ExitCode {
    let (Ok(input), Ok(output)) = (
        MidiInput::new("tallybridge"),
        MidiOutput::new("tallybridge"),
    ) else {
        eprintln!("tallybridge: the MIDI stack would not open");
        return ExitCode::FAILURE;
    };
    let ins: Vec<String> = input
        .ports()
        .iter()
        .map(|p| input.port_name(p).unwrap_or_default())
        .collect();
    let outs: Vec<String> = output
        .ports()
        .iter()
        .map(|p| output.port_name(p).unwrap_or_default())
        .collect();
    let chosen = (
        session::pick(&ins, substring),
        session::pick(&outs, substring),
    );
    for (side, names, pick) in [("in ", &ins, chosen.0), ("out", &outs, chosen.1)] {
        for n in names {
            let mark = if Some(n.as_str()) == pick {
                "  <- used"
            } else {
                ""
            };
            println!("{side}  {n}{mark}");
        }
    }
    if chosen.0.is_none() || chosen.1.is_none() {
        eprintln!("tallybridge: no port pair matches {substring:?}; is it plugged in? try --port");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// Ctrl+C, or the console window closing.
///
/// Both kill a console program before any destructor runs, which would
/// leave the device in programmer mode -- so the handler ends the current
/// session (the restore runs on the normal path, as it does for `tallyd`'s
/// signals) and waits for it. `taskkill /F` runs nothing at all and is
/// not covered; the next start covers it.
mod stop {
    use std::net::{Shutdown, SocketAddr, TcpStream};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Condvar, Mutex, OnceLock};
    use std::time::Duration;

    static REQUESTED: AtomicBool = AtomicBool::new(false);
    /// The session's socket, so the handler can end it.
    static ACTIVE: Mutex<Option<TcpStream>> = Mutex::new(None);
    /// Where to connect to wake an `accept` with nobody on the line.
    static LISTEN: OnceLock<SocketAddr> = OnceLock::new();
    static FINISHED: Mutex<bool> = Mutex::new(false);
    static FINISHED_CV: Condvar = Condvar::new();

    /// Windows gives a closing console about five seconds; the restore
    /// needs a readback, which is two seconds at worst.
    const GRACE: Duration = Duration::from_secs(4);

    pub(crate) fn requested() -> bool {
        REQUESTED.load(Ordering::Relaxed)
    }

    /// The session in progress, or none.
    pub(crate) fn watch(stream: Option<&TcpStream>) {
        if let Ok(mut active) = ACTIVE.lock() {
            *active = stream.and_then(|s| s.try_clone().ok());
        }
    }

    /// The loop is out and the device is back. Release the handler.
    pub(crate) fn finished() {
        if let Ok(mut done) = FINISHED.lock() {
            *done = true;
            FINISHED_CV.notify_all();
        }
    }

    /// End the session, wake the accept, and wait for the loop to say the
    /// device is back. Runs on the thread Windows starts for the handler.
    #[cfg_attr(
        not(windows),
        expect(dead_code, reason = "only a console handler calls it")
    )]
    fn request() {
        REQUESTED.store(true, Ordering::Relaxed);
        if let Ok(active) = ACTIVE.lock() {
            if let Some(s) = active.as_ref() {
                let _ended = s.shutdown(Shutdown::Both);
            }
        }
        if let Some(addr) = LISTEN.get() {
            let _woken = TcpStream::connect_timeout(addr, Duration::from_millis(200));
        }
        if let Ok(done) = FINISHED.lock() {
            let _waited = FINISHED_CV.wait_timeout_while(done, GRACE, |d| !*d);
        }
    }

    #[cfg(windows)]
    pub(crate) fn install(listen: SocketAddr) {
        #[link(name = "kernel32")]
        extern "system" {
            fn SetConsoleCtrlHandler(
                handler: Option<unsafe extern "system" fn(u32) -> i32>,
                add: i32,
            ) -> i32;
        }

        /// Every event a console sends -- Ctrl+C, Ctrl+Break, close,
        /// logoff, shutdown -- means the same thing here.
        extern "system" fn on_ctrl(_event: u32) -> i32 {
            request();
            1
        }

        let _set = LISTEN.set(listen);
        // SAFETY: `on_ctrl` matches `PHANDLER_ROUTINE`, lives for the
        // whole program, and touches only statics that are themselves
        // thread-safe; Windows calls it on a thread of its own, not in a
        // signal context.
        #[expect(unsafe_code, reason = "registering a console handler needs kernel32")]
        let installed = unsafe { SetConsoleCtrlHandler(Some(on_ctrl), 1) };
        if installed == 0 {
            eprintln!("tallybridge: no Ctrl+C handler; stopping it will skip the restore");
        }
    }

    /// Elsewhere the bridge only runs as a test fixture, and a signal
    /// killing it is fine.
    #[cfg(not(windows))]
    pub(crate) fn install(listen: SocketAddr) {
        let _set = LISTEN.set(listen);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Result<Args, String> {
        parse(a.iter().map(|&s| s.to_owned()))
    }

    #[test]
    fn the_defaults_are_loopback_and_the_shared_substring() {
        let a = args(&[]).unwrap();
        assert_eq!(a.listen.to_string(), wire::DEFAULT_ADDR);
        assert_eq!(a.port, session::PORT_SUBSTRING);
        assert!(!a.list);
    }

    /// The bind address is a security decision and the parser makes it:
    /// nothing that is not loopback gets a listener.
    #[test]
    fn a_non_loopback_address_is_refused() {
        for addr in ["0.0.0.0:7373", "192.168.1.156:7373", "[::]:7373"] {
            let e = args(&["--listen", addr]).unwrap_err();
            assert!(e.contains("loopback"), "{addr}: {e}");
        }
        args(&["--listen", "127.0.0.1:9000"]).unwrap();
        args(&["--listen", "[::1]:9000"]).unwrap();
    }

    #[test]
    fn unknown_and_incomplete_arguments_are_errors() {
        args(&["--frobnicate"]).unwrap_err();
        args(&["--listen"]).unwrap_err();
        args(&["--port"]).unwrap_err();
        assert_eq!(
            args(&["--port", "LPX MIDI", "--list"]).unwrap().port,
            "LPX MIDI"
        );
    }
}
