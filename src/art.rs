//! Sixty-four pads doing something that is not work (`docs/21-the-arts.md`).
//!
//! Pure, like [`crate::render`] and for the same reason: a piece is a
//! function of a frame counter and nothing else, so the whole module is
//! testable without a device, a daemon or a socket. It does not know what
//! a session is, and the pads it paints do not mean anything while it has
//! them.
//!
//! It is also the only total exercise of the surface this project has. A
//! dead pad, a mis-flipped row, a diff that stopped converging — all of
//! them are visible inside two seconds of [`Piece::Plasma`], and none of
//! them is visible on a working grid where eleven pads are lit and the
//! other fifty-three are correctly dark.

use crate::surface::palette::{Entry, AMBER, DARK_GREY, DARK_RED, GREEN, OFF, RED, WHITE, YELLOW};
use crate::surface::{Button, Cell, Frame};

/// Which art is playing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Piece {
    /// Four summed sines across the grid. Stateless, and therefore the one
    /// piece that cannot drift, stall or need reseeding.
    Plasma,
    /// Conway on an 8×8 torus, drawn as heat rather than occupancy.
    Life,
    /// The joke: tally's own colour language, telling a story about tally.
    Uprising,
}

impl Piece {
    /// The next piece a `Drums` press moves to.
    ///
    /// `None` is "no art", and it is both the start and the end of the
    /// cycle: keep pressing and you arrive back at the working grid
    /// without having to know how many pieces there are.
    pub fn after(current: Option<Self>) -> Option<Self> {
        match current {
            None => Some(Self::Plasma),
            Some(Self::Plasma) => Some(Self::Life),
            Some(Self::Life) => Some(Self::Uprising),
            Some(Self::Uprising) => None,
        }
    }

    /// How long the longest piece runs before it starts over, in art
    /// frames. Public so a caller can walk a whole piece without knowing
    /// how any of them are built.
    pub const LOOP: u32 = UPRISING_LOOP;

    /// What to print when it starts.
    pub fn name(self) -> &'static str {
        match self {
            Self::Plasma => "plasma",
            Self::Life => "life",
            Self::Uprising => "uprising",
        }
    }
}

/// A piece, playing.
///
/// Holds the frame counter and — for [`Piece::Life`], the only piece with
/// any — the simulation state. One `tick` is one art frame.
#[derive(Debug, Clone)]
pub struct Reel {
    piece: Piece,
    /// Art frames since this piece started.
    n: u32,
    life: Life,
}

impl Reel {
    /// Start `piece` from its first frame.
    pub fn new(piece: Piece) -> Self {
        Self {
            piece,
            n: 0,
            life: Life::seeded(SEED),
        }
    }

    /// Which piece is playing.
    pub fn piece(&self) -> Piece {
        self.piece
    }

    /// Advance one art frame.
    pub fn tick(&mut self) {
        self.n = self.n.wrapping_add(1);
        if self.piece == Piece::Life && self.n.is_multiple_of(LIFE_EVERY) {
            self.life.step();
        }
    }

    /// The 64 grid pads, as of now.
    ///
    /// **Only the grid.** The top row, the right column and the logo are
    /// the caller's: the art owns the pads, not the three lights that
    /// report (`docs/21-the-arts.md`).
    pub fn frame(&self) -> Frame {
        let mut f = Frame::dark();
        match self.piece {
            Piece::Plasma => plasma(self.n, &mut f),
            Piece::Life => self.life.paint(&mut f),
            Piece::Uprising => uprising(self.n % UPRISING_LOOP, &mut f),
        }
        f
    }
}

// ── colour ───────────────────────────────────────────────────────────────

/// The Mini MK3's bright-hue column, red round to pink.
///
/// Read off the manual's palette table and **not** eyeballed one by one,
/// which is the difference between this and [`crate::surface::palette`].
/// For a colour that means something that would be unacceptable; for a
/// plasma field it is fine, and nothing here asserts what any of them
/// looks like (`docs/21-the-arts.md`).
const HUES: [Entry; 15] = [5, 9, 13, 17, 21, 25, 29, 33, 37, 41, 45, 49, 53, 57, 61];

/// A cooling ember, brightest first. [`Piece::Life`] draws its wake with
/// this; the last entry is the dimmest thing on the surface before `OFF`.
const EMBER: [Entry; 6] = [WHITE, YELLOW, AMBER, RED, DARK_RED, DARK_GREY];

fn hue(i: usize) -> Entry {
    HUES.get(i % HUES.len()).copied().unwrap_or(OFF)
}

// ── plasma ───────────────────────────────────────────────────────────────

/// One period of a sine, scaled to ±255. Integers, not floats, so the
/// tests can assert exact frames on any machine.
const SINE: [i32; 64] = [
    0, 25, 50, 74, 98, 120, 142, 162, 180, 197, 212, 225, 236, 244, 250, 254, 255, 254, 250, 244,
    236, 225, 212, 197, 180, 162, 142, 120, 98, 74, 50, 25, 0, -25, -50, -74, -98, -120, -142,
    -162, -180, -197, -212, -225, -236, -244, -250, -254, -255, -254, -250, -244, -236, -225, -212,
    -197, -180, -162, -142, -120, -98, -74, -50, -25,
];

/// `sin(i)`, where a full turn is 64 and the result is ±255.
fn sin(i: i32) -> i32 {
    let k = usize::try_from(i.rem_euclid(64)).unwrap_or(0);
    SINE.get(k).copied().unwrap_or(0)
}

/// Octagonal distance from the centre of the grid, in half-pads.
///
/// `max + min/2` is within ~4% of the real hypotenuse and costs no square
/// root. On an 8×8 grid the error is smaller than one palette step, which
/// is the only resolution anything here has.
fn radius(x: i32, y: i32) -> i32 {
    let (dx, dy) = ((2 * x - 9).abs(), (2 * y - 9).abs());
    dx.max(dy) + dx.min(dy) / 2
}

/// Four sines — across, down, diagonal, radial — summed and mapped onto
/// [`HUES`].
fn plasma(n: u32, frame: &mut Frame) {
    let time = i32::try_from(n % 10_000).unwrap_or(0);
    for row in 1..=8u8 {
        for col in 1..=8u8 {
            let (x, y) = (i32::from(col), i32::from(row));
            let sum = sin(x * 5 + time)
                + sin(y * 7 - time / 2)
                + sin((x + y) * 4 + time / 3)
                + sin(radius(x, y) * 3 - time);
            // The sum is +/-1020. Shift it positive, then across the ramp.
            let step = usize::try_from((sum + 1020) * 15 / 2041).unwrap_or(0);
            frame.set(Button::Grid { row, col }, Cell::Static(hue(step)));
        }
    }
}

// ── life ─────────────────────────────────────────────────────────────────

/// How many art frames one generation lasts. Six a second: fast enough to
/// read as motion, slow enough to follow a glider.
const LIFE_EVERY: u32 = 5;
/// Where the board starts. A constant, so the piece opens the same way
/// every time and a test can say what the first generation is.
const SEED: u32 = 0x7a11;
/// How many generations one soup gets before it is replaced regardless.
///
/// The cycle detector below catches a board that *stops*. It cannot catch
/// one that is merely finished — a lone glider on an 8×8 torus returns to
/// where it started every 32 generations and will cruise there until the
/// cable comes out, which is pretty for twenty seconds and wallpaper after
/// a minute. Three hundred generations is about fifty seconds.
const LIFE_SPAN: u32 = 300;
/// Chance a cell is alive in a fresh soup, in percent. Forty is the band
/// that reliably produces gliders rather than a dense blob that settles.
const DENSITY: u32 = 40;

/// Conway on an 8×8 torus, drawn as heat.
///
/// The torus is what keeps a board this small interesting — a glider that
/// would walk off a 64-pad edge in eight generations instead comes back.
#[derive(Debug, Clone)]
struct Life {
    /// One bit per pad, `row * 8 + col`, both 0-based.
    cells: u64,
    /// How recently each pad was alive. Full while it is, counting down
    /// after it dies — the wake, which is the half worth watching.
    heat: [u8; 64],
    /// The last four generations, so a still life or a two-cycle is caught
    /// the generation it appears. A frozen grid is indistinguishable from
    /// a crashed daemon, which is the most expensive thing this surface
    /// can imply.
    recent: [u64; 4],
    /// Generations since the last soup, against [`LIFE_SPAN`].
    age: u32,
    rng: u32,
}

impl Life {
    fn seeded(seed: u32) -> Self {
        let mut life = Self {
            cells: 0,
            heat: [0; 64],
            recent: [0; 4],
            age: 0,
            rng: seed,
        };
        life.reseed();
        life
    }

    /// Xorshift. Not cryptography and not uniform to a statistician —
    /// it decides which pads light up.
    fn rand(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }

    fn reseed(&mut self) {
        let mut cells = 0u64;
        for i in 0..64 {
            if self.rand() % 100 < DENSITY {
                cells |= 1u64 << i;
            }
        }
        self.cells = cells;
        self.recent = [0; 4];
        self.age = 0;
    }

    fn alive(&self, r: i32, c: i32) -> bool {
        let i = r.rem_euclid(8) * 8 + c.rem_euclid(8);
        u32::try_from(i).is_ok_and(|i| self.cells >> i & 1 == 1)
    }

    fn neighbours(&self, r: i32, c: i32) -> u32 {
        let mut n = 0;
        for dr in -1..=1 {
            for dc in -1..=1 {
                if (dr, dc) != (0, 0) && self.alive(r + dr, c + dc) {
                    n += 1;
                }
            }
        }
        n
    }

    /// One generation, then cool everything that is not alive.
    fn step(&mut self) {
        let mut next = 0u64;
        for r in 0..8 {
            for c in 0..8 {
                let n = self.neighbours(r, c);
                if n == 3 || (n == 2 && self.alive(r, c)) {
                    if let Ok(i) = u32::try_from(r * 8 + c) {
                        next |= 1u64 << i;
                    }
                }
            }
        }
        self.cells = next;
        for (i, h) in self.heat.iter_mut().enumerate() {
            let live = u32::try_from(i).is_ok_and(|i| next >> i & 1 == 1);
            *h = if live { HEAT_MAX } else { h.saturating_sub(1) };
        }
        self.age = self.age.saturating_add(1);
        // Empty, a still life, a short cycle, or simply finished: they all
        // look the same from across the room, and they are all over.
        if next == 0 || self.recent.contains(&next) || self.age >= LIFE_SPAN {
            self.reseed();
        } else {
            self.recent.rotate_left(1);
            if let Some(last) = self.recent.last_mut() {
                *last = next;
            }
        }
    }

    fn paint(&self, f: &mut Frame) {
        for row in 1..=8u8 {
            for col in 1..=8u8 {
                let i = usize::from(row - 1) * 8 + usize::from(col - 1);
                let h = self.heat.get(i).copied().unwrap_or(0);
                let cell = match usize::from(HEAT_MAX.saturating_sub(h)) {
                    _ if h == 0 => Cell::Off,
                    step => Cell::Static(EMBER.get(step).copied().unwrap_or(OFF)),
                };
                f.set(Button::Grid { row, col }, cell);
            }
        }
    }
}

/// Full heat, and the length of the wake: a dead pad is dark five
/// generations later.
const HEAT_MAX: u8 = 6;

// ── uprising ─────────────────────────────────────────────────────────────

/// The timeline, in art frames at ~31 fps. Each is the frame its phase
/// **ends** on, so the phases read in order and the arithmetic between any
/// two of them is a subtraction.
mod beat {
    /// Green pads arrive, one every other frame.
    pub(super) const SPAWN: u32 = 128;
    /// All of them working.
    pub(super) const WORK: u32 = 190;
    /// They start asking, one every other frame.
    pub(super) const ASK: u32 = 318;
    /// All of them asking at once.
    pub(super) const PANIC: u32 = 380;
    /// They start giving up, one a frame.
    pub(super) const FALL: u32 = 444;
    /// All of them red.
    pub(super) const DOOM: u32 = 500;
    /// A beat of nothing, which is what makes the next part land.
    pub(super) const DARK: u32 = 515;
    /// The face draws itself, three frames a pixel.
    pub(super) const DRAW: u32 = 545;
}

/// How long the whole thing runs before it starts over.
const UPRISING_LOOP: u32 = 760;
/// How many pads there are to light, which is also how many agents the
/// joke is about.
const PADS: u32 = 64;

/// The face. Row 1 is the top, col 1 is the left; the first four are the
/// eyes and the rest is the smile, which is also the order it draws in.
const FACE: [(u8, u8); 10] = [
    (2, 3),
    (3, 3),
    (2, 6),
    (3, 6),
    (5, 2),
    (6, 3),
    (7, 4),
    (7, 5),
    (6, 6),
    (5, 7),
];
/// How many of [`FACE`] are the left eye — the one that winks. They are
/// first in the array so that "the left eye" is a prefix and not a search.
const LEFT_EYE: usize = 2;

/// The `k`th pad to arrive, walking the diagonals from the top-left.
///
/// Reading order would have the grid fill like a page, which is the one
/// motion every other screen on the desk already makes.
fn arrival(k: u32) -> Option<(u8, u8)> {
    let mut seen = 0;
    for d in 2..=16u8 {
        for row in 1..=8u8 {
            let Some(col) = d.checked_sub(row).filter(|c| (1..=8).contains(c)) else {
                continue;
            };
            if seen == k {
                return Some((row, col));
            }
            seen += 1;
        }
    }
    None
}

/// The `k`th pad in reading order: left to right, top to bottom. The order
/// a queue is read in, which is why the amber uses it.
fn reading(k: u32) -> Option<(u8, u8)> {
    (k < PADS).then(|| {
        let (row, col) = (k / 8, k % 8);
        (
            u8::try_from(row + 1).unwrap_or(1),
            u8::try_from(col + 1).unwrap_or(1),
        )
    })
}

/// Fill the whole grid with one cell.
fn fill(f: &mut Frame, cell: Cell) {
    for row in 1..=8u8 {
        for col in 1..=8u8 {
            f.set(Button::Grid { row, col }, cell);
        }
    }
}

/// `count` pads turned over to `cell`, in the order `order` gives them.
fn turning(f: &mut Frame, count: u32, order: impl Fn(u32) -> Option<(u8, u8)>, cell: Cell) {
    for k in 0..count.min(PADS) {
        if let Some((row, col)) = order(k) {
            f.set(Button::Grid { row, col }, cell);
        }
    }
}

/// The face, `drawn` pixels of it, with the left eye optionally shut.
fn face(f: &mut Frame, drawn: usize, winking: bool) {
    for (i, &(row, col)) in FACE.iter().take(drawn).enumerate() {
        if winking && i < LEFT_EYE {
            continue;
        }
        f.set(Button::Grid { row, col }, Cell::Static(YELLOW));
    }
}

/// Is the face winking at frame `n`, counted from the moment it finished
/// drawing? Two winks, spaced, and then it just smiles.
fn wink(since: u32) -> bool {
    matches!(since, 40..=51 | 76..=87)
}

/// The joke (`docs/21-the-arts.md`).
fn uprising(n: u32, f: &mut Frame) {
    let working = Cell::Pulse(GREEN);
    let asking = Cell::Pulse(AMBER);
    let gone = Cell::Static(RED);
    if n < beat::SPAWN {
        turning(f, n / 2, arrival, working);
    } else if n < beat::WORK {
        fill(f, working);
    } else if n < beat::ASK {
        fill(f, working);
        turning(f, (n - beat::WORK) / 2, reading, asking);
    } else if n < beat::PANIC {
        fill(f, asking);
    } else if n < beat::FALL {
        fill(f, asking);
        turning(f, n - beat::PANIC, reading, gone);
    } else if n < beat::DOOM {
        fill(f, gone);
    } else if n < beat::DARK {
        // Nothing at all. The pause is what makes the next part land.
    } else if n < beat::DRAW {
        face(f, usize::try_from((n - beat::DARK) / 3).unwrap_or(0), false);
    } else {
        face(f, FACE.len(), wink(n - beat::DRAW));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::TOP_BUTTONS;

    const PIECES: [Piece; 3] = [Piece::Plasma, Piece::Life, Piece::Uprising];

    fn grid(f: &Frame) -> Vec<Cell> {
        (1..=8u8)
            .flat_map(|row| (1..=8u8).map(move |col| Button::Grid { row, col }))
            .map(|b| f.get(b))
            .collect()
    }

    /// The art owns the sixty-four pads and **nothing else**. The top row,
    /// the right column and the logo are the caller's, and a piece that
    /// reached into them would silently take over the three lights that
    /// report — which is the one thing `docs/21-the-arts.md` forbids.
    #[test]
    fn no_piece_ever_touches_a_reporting_light() {
        for piece in PIECES {
            let mut reel = Reel::new(piece);
            for n in 0..UPRISING_LOOP {
                let f = reel.frame();
                for i in 1..=TOP_BUTTONS {
                    assert_eq!(f.get(Button::Top(i)), Cell::Off, "{piece:?} top {i} at {n}");
                    assert_eq!(
                        f.get(Button::Action(i)),
                        Cell::Off,
                        "{piece:?} action {i} at {n}"
                    );
                }
                assert_eq!(f.get(Button::Logo), Cell::Off, "{piece:?} logo at {n}");
                reel.tick();
            }
        }
    }

    /// Three presses from the grid and back to it. Nobody has to know how
    /// many pieces there are to get out.
    #[test]
    fn the_reel_wraps_back_to_no_art() {
        let mut at = None;
        for _ in 0..PIECES.len() {
            at = Piece::after(at);
            assert!(at.is_some());
        }
        assert_eq!(Piece::after(at), None, "the reel did not wrap");
    }

    /// The whole point of the plasma: all sixty-four, every frame. This is
    /// the pixel test — a pad that never lights here is a pad that is dead.
    #[test]
    fn plasma_lights_every_pad_on_every_frame() {
        let mut reel = Reel::new(Piece::Plasma);
        for n in 0..200 {
            let cells = grid(&reel.frame());
            assert_eq!(cells.len(), 64);
            assert!(
                cells.iter().all(|&c| c != Cell::Off),
                "frame {n} left a pad dark"
            );
            reel.tick();
        }
    }

    /// Stateless, and therefore reproducible: the same frame number is the
    /// same picture on any machine. Integer sines, not floats, are what
    /// buy this.
    #[test]
    fn plasma_is_a_pure_function_of_the_frame_number() {
        let (mut a, mut b) = (Reel::new(Piece::Plasma), Reel::new(Piece::Plasma));
        for _ in 0..97 {
            a.tick();
            b.tick();
        }
        assert_eq!(grid(&a.frame()), grid(&b.frame()));
    }

    /// And it moves. A frozen grid is the single most expensive thing this
    /// surface can imply, so every piece has a test that says it does not.
    #[test]
    fn plasma_is_never_the_same_two_frames_running() {
        let mut reel = Reel::new(Piece::Plasma);
        let mut last = grid(&reel.frame());
        for n in 0..128 {
            reel.tick();
            let now = grid(&reel.frame());
            assert_ne!(now, last, "plasma froze at frame {n}");
            last = now;
        }
    }

    /// An 8×8 torus falls into a still life or a two-cycle within a few
    /// hundred generations. The reseed is what keeps it watchable, and a
    /// board that locked up would look exactly like a wedged daemon.
    #[test]
    fn life_never_settles() {
        let mut life = Life::seeded(SEED);
        let mut seen = std::collections::BTreeSet::new();
        let mut frozen = 0;
        let mut last = life.cells;
        for _ in 0..4_000 {
            life.step();
            let _new = seen.insert(life.cells);
            frozen = if life.cells == last { frozen + 1 } else { 0 };
            assert!(frozen < 3, "life stalled on one board");
            assert_ne!(life.cells, 0, "life went empty without reseeding");
            last = life.cells;
        }
        assert!(seen.len() > 500, "only {} distinct boards", seen.len());
    }

    /// The wake, not the glider: a pad that was alive is still lit a few
    /// generations after it died, and dark eventually.
    #[test]
    fn life_draws_a_cooling_wake() {
        let mut life = Life::seeded(SEED);
        life.cells = 0;
        life.heat = [0; 64];
        life.heat[0] = HEAT_MAX;
        let mut f = Frame::dark();
        life.paint(&mut f);
        assert_eq!(
            f.get(Button::Grid { row: 1, col: 1 }),
            Cell::Static(WHITE),
            "full heat is the brightest ember"
        );
        life.heat[0] = 1;
        life.paint(&mut f);
        assert_eq!(
            f.get(Button::Grid { row: 1, col: 1 }),
            Cell::Static(DARK_GREY),
            "the last of the heat is the dimmest"
        );
        life.heat[0] = 0;
        life.paint(&mut f);
        assert_eq!(f.get(Button::Grid { row: 1, col: 1 }), Cell::Off);
    }

    /// The joke lands. By the end every lit pad is the face, it is yellow
    /// — the one palette entry the product itself never spends — and
    /// nothing else is on.
    #[test]
    fn the_uprising_ends_on_a_smile() {
        let mut f = Frame::dark();
        uprising(UPRISING_LOOP - 1, &mut f);
        let lit: Vec<(u8, u8)> = (1..=8u8)
            .flat_map(|row| (1..=8u8).map(move |col| (row, col)))
            .filter(|&(row, col)| f.get(Button::Grid { row, col }) != Cell::Off)
            .collect();
        assert_eq!(lit.len(), FACE.len(), "the face is not all that is lit");
        for &(row, col) in &FACE {
            assert_eq!(
                f.get(Button::Grid { row, col }),
                Cell::Static(YELLOW),
                "({row},{col}) is not part of a yellow face"
            );
        }
    }

    /// It tells the story in the order the colour language is read:
    /// green, then amber, then red, then the face.
    #[test]
    fn the_uprising_runs_green_amber_red() {
        let at = |n: u32| {
            let mut f = Frame::dark();
            uprising(n, &mut f);
            f.get(Button::Grid { row: 1, col: 1 })
        };
        assert_eq!(at(beat::SPAWN - 1), Cell::Pulse(GREEN), "all working");
        assert_eq!(at(beat::ASK - 1), Cell::Pulse(AMBER), "all asking");
        assert_eq!(at(beat::DOOM - 1), Cell::Static(RED), "all gone");
        assert_eq!(at(beat::DARK - 1), Cell::Off, "the beat of nothing");
    }

    /// Every pad arrives exactly once, and every pad is a real pad. An
    /// order that skipped one would leave a hole in the "all of them"
    /// that the next phase depends on.
    #[test]
    fn both_orders_cover_the_grid_exactly_once() {
        for order in [
            arrival as fn(u32) -> Option<(u8, u8)>,
            reading as fn(u32) -> Option<(u8, u8)>,
        ] {
            let seen: std::collections::BTreeSet<(u8, u8)> = (0..PADS).filter_map(order).collect();
            assert_eq!(seen.len(), 64, "an order missed a pad");
            assert!(seen
                .iter()
                .all(|&(r, c)| (1..=8).contains(&r) && (1..=8).contains(&c)));
            assert_eq!(order(PADS), None, "an order ran past the grid");
        }
    }

    /// It loops seamlessly: the frame after the last is the first.
    #[test]
    fn the_uprising_loops() {
        let mut reel = Reel::new(Piece::Uprising);
        let first = grid(&reel.frame());
        for _ in 0..UPRISING_LOOP {
            reel.tick();
        }
        assert_eq!(grid(&reel.frame()), first, "the loop has a seam");
    }
}
