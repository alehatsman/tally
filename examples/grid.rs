//! A realistic grid, rendered through the real `render::slot` path.
//! Changing the colour language changes this picture — which is the point.

use tally::render::{self, Health};
use tally::signal::{SlotState, Source};
use tally::surface::{Button, Cell, Frame};

fn main() {
    use SlotState::{Failed, Lost, NeedsAnswer, Quiet, Working};
    let wants_you = NeedsAnswer {
        source: Source::Derived,
    };

    // (row, col, state) -- row 1 is the TOP.
    let slots = [
        (2, 2, Working),
        (2, 5, Working),
        (3, 1, Quiet),
        (3, 3, Working),
        (4, 6, wants_you), // the one that matters
        (5, 2, Working),
        (5, 4, Quiet),
        (6, 1, Working),
        (7, 3, Failed { code: 1 }),
        (7, 5, Quiet),
        (8, 8, Lost),
    ];

    let mut f = Frame::dark();
    for (row, col, state) in slots {
        f.set(Button::Grid { row, col }, render::slot(state));
    }
    // The top row: `▲ ▼ ◄ ► · · · ·` (docs/16-the-queue.md).
    f.set(Button::Top(1), render::arrow(false));
    f.set(Button::Top(2), render::arrow(true));
    f.set(Button::Top(3), render::back(true));
    f.set(Button::Top(4), render::next(true));
    for row in 1..=8 {
        f.set(
            Button::Action(row),
            render::action(tally::chord::Action::of(Button::Action(row)), false),
        );
    }
    f.set(
        Button::Logo,
        render::logo(Health {
            anchord_reachable: true,
            offscreen_attention: false,
        }),
    );

    let bytes = f.repaint();
    let moving = Button::all()
        .filter(|&b| matches!(f.get(b), Cell::Pulse(_) | Cell::Flash { .. }))
        .count();
    eprintln!("{} bytes, {moving} moving pads", bytes.len());
    println!(
        "{}",
        bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
}
