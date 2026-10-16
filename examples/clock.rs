//! The one measurement M9 is gated on: **does the Launchpad Mini MK3 sync
//! its LED animations to MIDI clock on the port `tally` already opens?**
//!
//! `docs/02` records that both animations run at 120 bpm *when no MIDI
//! clock is supplied*, which implies they follow one when it is. It does
//! not say so, and `tally` opens `LPMiniMK3 MIDI` rather than the DAW
//! port — so the whole of `docs/17` rests on a sentence nobody has tested.
//!
//! This sends nothing but clock. It paints no pads and touches no state,
//! so it can run **while `tallyd` is running** and you watch the pads
//! `tallyd` is already pulsing. If the green breathing changes speed, the
//! answer is yes.
//!
//! ```sh
//! cargo run --example clock
//! ```
//!
//! Watch a **pulsing** pad — a green one is ideal. Call out what you see
//! in each phase; the phases are printed as they start.

use std::io::Write as _;
use std::thread::sleep;
use std::time::{Duration, Instant};

use midir::MidiOutput;

/// MIDI realtime: one clock pulse. 24 of these per quarter note.
const CLOCK: u8 = 0xF8;
/// MIDI realtime: start. Some devices only follow clock while "playing".
const START: u8 = 0xFA;
/// MIDI realtime: stop. Hands the beat back to the device's own 120.
const STOP: u8 = 0xFC;

/// Pulses per quarter note, by the MIDI spec. Not negotiable.
const PPQN: u64 = 24;

fn tick(bpm: u64) -> Duration {
    Duration::from_nanos(60_000_000_000 / (bpm * PPQN))
}

fn main() {
    let output = MidiOutput::new("tally-clock").expect("midi output");
    let port = output
        .ports()
        .into_iter()
        .find(|p| {
            output
                .port_name(p)
                .is_ok_and(|n| n.contains(tally::device::PORT_SUBSTRING))
        })
        .expect("no Launchpad Mini MK3 — is it plugged in?");
    println!("port: {}\n", output.port_name(&port).unwrap_or_default());
    let mut out = output.connect(&port, "tally-clock").expect("connect");

    // A baseline first, so "it changed" is a comparison and not a memory.
    quiet("1/5  no clock at all — the device's own 120 bpm", 15);

    // START before the stream: some devices only follow clock while they
    // believe something is playing. If the answer turns out to be "only
    // with START", that is worth knowing too.
    let _started = out.send(&[START]);
    for (label, bpm, secs) in [
        (
            "2/5  clock at  60 bpm — should breathe SLOWER",
            60u64,
            20u64,
        ),
        ("3/5  clock at 150 bpm — should breathe FASTER", 150, 20),
        ("4/5  clock at  60 bpm again — slow once more", 60, 20),
    ] {
        let gap = tick(bpm);
        announce(&format!("{label}   ({} pulses/s)", bpm * PPQN / 60));
        let until = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < until {
            if out.send(&[CLOCK]).is_err() {
                eprintln!("send failed — is the device still plugged in?");
                return;
            }
            sleep(gap);
        }
    }
    let _stopped = out.send(&[STOP]);
    quiet("5/5  clock stopped — back to the device's own 120", 15);

    println!("\nverdict: did 2 and 4 breathe slower, and 3 faster, than 1 and 5?");
    println!("  yes -> docs/17 is buildable on the port we already open");
    println!("  no  -> try the DAW port, and docs/17 costs a second connection");
}

/// Announce a phase.
fn announce(label: &str) {
    println!("{label}");
    let _flushed = std::io::stdout().flush();
}

/// A phase that sends nothing, and therefore has to wait by itself.
fn quiet(label: &str, secs: u64) {
    announce(&format!("{label}   [{secs}s]"));
    sleep(Duration::from_secs(secs));
}
