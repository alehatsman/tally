//! Print an art as text, so the shapes can be checked without a device.
//!
//! The hardware tells you whether a colour reads; this tells you whether
//! the picture is the picture. `cargo run --example arts -- uprising 700`
//!
//! Digits are palette entries; `.` is an unlit pad.

use tally::art::{Piece, Reel};
use tally::surface::{Button, Cell};

fn main() {
    let mut args = std::env::args().skip(1);
    let name = args.next().unwrap_or_else(|| "uprising".to_owned());
    let at: u32 = args
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(Piece::LOOP - 1);
    let piece = Piece::REEL
        .into_iter()
        .find(|p| p.name() == name)
        .unwrap_or(Piece::Uprising);

    let mut reel = Reel::new(piece);
    for _ in 0..at {
        reel.tick();
    }
    let frame = reel.frame();
    println!("{} @ {at}", piece.name());
    for row in 1..=8u8 {
        let line: String = (1..=8u8)
            .map(|col| match frame.get(Button::Grid { row, col }) {
                Cell::Off => "  . ".to_owned(),
                Cell::Static(c) => format!("{c:3} "),
                Cell::Pulse(c) => format!("{c:3}~"),
                Cell::Flash { b, .. } => format!("{b:3}!"),
            })
            .collect();
        println!("{line}");
    }
}
