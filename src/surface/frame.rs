//! A frame is what the whole surface looks like, and it is a pure function
//! of the model. Rendering is `model -> Frame`; sending is `diff -> bytes`.

use super::addr::{Button, SURFACE_LEN};
use super::palette::{Entry, OFF};
use super::sysex::{EOX, HEADER};

/// How one button is lit.
///
/// Flashing and pulsing run on the **device's** clock, so a host that wants
/// a blinking pad sets it once and stops thinking about it — no timer, no
/// repaint loop, no drift (`docs/02-surface.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cell {
    /// Unlit.
    #[default]
    Off,
    /// One colour, held.
    Static(Entry),
    /// Alternates `b` and `a` at one beat, 50% duty.
    Flash {
        /// The colour flashed *to*.
        b: Entry,
        /// The colour flashed *from*.
        a: Entry,
    },
    /// Ramps dark to full over two beats.
    Pulse(Entry),
}

impl Cell {
    /// The colourspec bytes for this cell at `index`.
    ///
    /// The `SysEx` form takes **both** flash colours, where channel-2
    /// note-on sets one and inherits the other from whatever was on the pad.
    /// Taking both is what makes a frame total, and therefore a pure
    /// function of the model with no dependence on history.
    fn encode(self, index: u8, out: &mut Vec<u8>) {
        match self {
            Self::Off => out.extend([0x00, index, OFF]),
            Self::Static(c) => out.extend([0x00, index, c]),
            Self::Flash { b, a } => out.extend([0x01, index, b, a]),
            Self::Pulse(c) => out.extend([0x02, index, c]),
        }
    }
}

/// The lit state of all 81 positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    cells: [Cell; SURFACE_LEN],
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            cells: [Cell::Off; SURFACE_LEN],
        }
    }
}

impl Frame {
    /// Every position dark.
    pub fn dark() -> Self {
        Self::default()
    }

    /// Light one button.
    pub fn set(&mut self, button: Button, cell: Cell) {
        if let Some(slot) = self.cells.get_mut(button.slot()) {
            *slot = cell;
        }
    }

    /// How one button is currently lit.
    pub fn get(&self, button: Button) -> Cell {
        self.cells.get(button.slot()).copied().unwrap_or_default()
    }

    /// One `SysEx` painting every position. Used on connect and on replug,
    /// where repainting everything is both correct and cheap.
    pub fn repaint(&self) -> Vec<u8> {
        Self::message(Button::all().map(|b| (b, self.get(b))))
            .expect("a full repaint always has 81 changes")
    }

    /// One `SysEx` carrying only what changed since `prev`, or `None` if
    /// nothing did — the steady state, where the right number of bytes to
    /// send is zero.
    pub fn diff(&self, prev: &Self) -> Option<Vec<u8>> {
        Self::message(
            Button::all()
                .map(|b| (b, self.get(b)))
                .filter(|&(b, cell)| prev.get(b) != cell),
        )
    }

    fn message(changes: impl Iterator<Item = (Button, Cell)>) -> Option<Vec<u8>> {
        let mut msg = HEADER.to_vec();
        msg.push(0x03);
        let mut any = false;
        for (button, cell) in changes {
            cell.encode(button.index(), &mut msg);
            any = true;
        }
        any.then(|| {
            msg.push(EOX);
            msg
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::palette::*;

    fn hex(bytes: &[u8]) -> String {
        bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The manual's own third worked example, byte for byte. If the encoder
    /// cannot reproduce Novation's example it is wrong, and that is knowable
    /// without hardware (`docs/09-testing.md`).
    #[test]
    fn reproduces_the_manuals_sysex_example() {
        let mut f = Frame::dark();
        // Pads 0B, 0C, 0D -- the bottom-left three, i.e. logical row 8.
        f.set(Button::Grid { row: 8, col: 1 }, Cell::Static(YELLOW));
        f.set(
            Button::Grid { row: 8, col: 2 },
            Cell::Flash { b: 0x0C, a: YELLOW },
        );
        f.set(Button::Grid { row: 8, col: 3 }, Cell::Pulse(YELLOW));

        let msg = f.diff(&Frame::dark()).unwrap();
        assert_eq!(
            hex(&msg),
            "F0 00 20 29 02 0D 03 00 0B 0D 01 0C 0C 0D 02 0D 0D F7",
            "encoder does not match the manual"
        );
    }

    /// M0 measured these on the wire; docs/02's byte budget quotes them.
    #[test]
    fn full_repaint_byte_budget() {
        let mut static_frame = Frame::dark();
        let mut flashing = Frame::dark();
        for b in Button::all() {
            static_frame.set(b, Cell::Static(GREEN));
            flashing.set(b, Cell::Flash { b: GREEN, a: OFF });
        }
        // 8 bytes of overhead -- header(6) + command(1) + EOX(1).
        assert_eq!(static_frame.repaint().len(), 8 + 81 * 3);
        assert_eq!(static_frame.repaint().len(), 251);
        assert_eq!(flashing.repaint().len(), 8 + 81 * 4);
        assert_eq!(flashing.repaint().len(), 332);
    }

    #[test]
    fn a_repaint_covers_the_logo() {
        // Skipping index 99 on shutdown leaves the health light burning,
        // which asserts "anchord unreachable" on the way out.
        assert!(Frame::dark()
            .repaint()
            .windows(3)
            .any(|w| w == [0x00, 99, OFF]));
    }

    #[test]
    fn an_unchanged_frame_sends_nothing() {
        let f = Frame::dark();
        assert_eq!(f.diff(&f), None);
    }

    #[test]
    fn a_diff_carries_only_what_changed() {
        let prev = Frame::dark();
        let mut next = prev.clone();
        next.set(Button::Grid { row: 1, col: 1 }, Cell::Static(AMBER));
        let msg = next.diff(&prev).unwrap();
        assert_eq!(hex(&msg), "F0 00 20 29 02 0D 03 00 51 09 F7");
        assert_eq!(msg.len(), 8 + 3, "one changed pad is one colourspec");
    }

    #[test]
    fn diff_then_apply_equals_repaint() {
        let mut a = Frame::dark();
        a.set(Button::Logo, Cell::Static(DARK_RED));
        a.set(Button::Action(8), Cell::Static(BLUE));
        a.set(
            Button::Top(3),
            Cell::Flash {
                b: AMBER,
                a: TURQUOISE,
            },
        );
        // Every position in a repaint; only the three set here in a diff.
        assert_eq!(a.repaint().len(), 8 + 78 * 3 + 2 * 3 + 4);
        assert_eq!(a.diff(&Frame::dark()).unwrap().len(), 8 + 2 * 3 + 4);
    }

    /// No byte in a `SysEx` body may have the high bit set: the device reads
    /// it as a status byte and ends the message early.
    #[test]
    fn no_body_byte_is_ever_a_status_byte() {
        let mut f = Frame::dark();
        for (i, b) in Button::all().enumerate() {
            f.set(b, Cell::Static(Entry::try_from(i * 7 % 128).unwrap_or(0)));
        }
        let msg = f.repaint();
        assert_eq!(msg[0], 0xF0);
        assert_eq!(*msg.last().unwrap(), EOX);
        assert!(msg[1..msg.len() - 1].iter().all(|&b| b < 0x80));
    }
}
