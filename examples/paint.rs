//! Dump a frame's bytes as hex so they can be pushed at a real device.
//! M1 scaffolding: proves the encoder against hardware before `midir` is
//! anywhere near the picture.

use tally::surface::palette::*;
use tally::surface::{Button, Cell, Frame};

fn main() {
    let mut f = Frame::dark();

    // The row flip, made falsifiable. Logical row 1 is the TOP.
    // If the arithmetic is inverted, this paints upside down and you can
    // see it from across the room.
    for col in 1..=8 {
        f.set(Button::Grid { row: 1, col }, Cell::Static(AMBER)); // top
        f.set(Button::Grid { row: 8, col }, Cell::Static(GREEN)); // bottom
    }
    // Column 1 is the LEFT: a single white pad on the left of row 4.
    f.set(Button::Grid { row: 4, col: 1 }, Cell::Static(WHITE));
    // CC89 is the top of the right column, CC19 the bottom (kill lives there).
    f.set(Button::Action(1), Cell::Static(BLUE));
    f.set(Button::Action(8), Cell::Static(RED));
    // Page 1 is the leftmost of the top row.
    f.set(Button::Page(1), Cell::Static(TURQUOISE));
    // The health light.
    f.set(Button::Logo, Cell::Static(DARK_RED));

    let bytes = f.repaint();
    eprintln!("{} bytes", bytes.len());
    println!("{}", bytes.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" "));
}
