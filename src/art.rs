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

use crate::surface::palette::{
    Entry, AMBER, DARK_GREEN, DARK_GREY, DARK_RED, GREEN, OFF, RED, WHITE, YELLOW,
};
use crate::surface::{Button, Cell, Frame};

/// Which art is playing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Piece {
    /// Four summed sines across the grid. Stateless, and therefore the one
    /// piece that cannot drift, stall or need reseeding.
    Plasma,
    /// Conway on an 8×8 torus, drawn as heat rather than occupancy.
    Life,
    /// Eight columns of falling light with bright heads and cooling tails.
    Rain,
    /// Doom's fire, eight pads wide: a hot floor, cooling as it rises.
    Fire,
    /// Tetrominoes that place themselves, stack, and clear lines.
    Tetris,
    /// A chomping Pac-Man, and the ghost that takes its turn.
    Pacman,
    /// Pixel animals, one at a time, each breathing a little.
    Bestiary,
    /// The joke: tally's own colour language, telling a story about tally.
    Uprising,
}

impl Piece {
    /// Every piece, in the order `Drums` walks them.
    ///
    /// Three that are only pretty, two that are classics, and the joke
    /// last — a reel ends on the thing worth filming.
    pub const REEL: [Self; 8] = [
        Self::Plasma,
        Self::Life,
        Self::Rain,
        Self::Fire,
        Self::Tetris,
        Self::Pacman,
        Self::Bestiary,
        Self::Uprising,
    ];

    /// The next piece a `Drums` press moves to.
    ///
    /// `None` is "no art", and it is both the start and the end of the
    /// cycle: keep pressing and you arrive back at the working grid
    /// without having to know how many pieces there are.
    pub fn after(current: Option<Self>) -> Option<Self> {
        let Some(at) = current else {
            return Self::REEL.first().copied();
        };
        let next = Self::REEL.iter().position(|&p| p == at)?.saturating_add(1);
        Self::REEL.get(next).copied()
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
            Self::Rain => "rain",
            Self::Fire => "fire",
            Self::Tetris => "tetris",
            Self::Pacman => "pacman",
            Self::Bestiary => "bestiary",
            Self::Uprising => "uprising",
        }
    }
}

/// The state a piece carries between frames, if it carries any.
///
/// An enum rather than one field per simulation: five boards allocated so
/// that one can run is the kind of thing that is invisible until somebody
/// adds a sixth.
#[derive(Debug, Clone)]
enum Sim {
    /// [`Piece::Plasma`] and [`Piece::Uprising`]: frame *n* depends on
    /// nothing but *n*.
    Stateless,
    Life(Life),
    Rain(Rain),
    Fire(Fire),
    Tetris(Tetris),
}

/// A piece, playing.
///
/// Holds the frame counter and whatever state the piece needs. One `tick`
/// is one art frame.
#[derive(Debug, Clone)]
pub struct Reel {
    piece: Piece,
    /// Art frames since this piece started.
    n: u32,
    sim: Sim,
}

impl Reel {
    /// Start `piece` from its first frame.
    pub fn new(piece: Piece) -> Self {
        let sim = match piece {
            Piece::Plasma | Piece::Pacman | Piece::Bestiary | Piece::Uprising => Sim::Stateless,
            Piece::Life => Sim::Life(Life::seeded(SEED)),
            Piece::Rain => Sim::Rain(Rain::seeded(SEED)),
            Piece::Fire => Sim::Fire(Fire::seeded(SEED)),
            Piece::Tetris => Sim::Tetris(Tetris::seeded(SEED)),
        };
        Self { piece, n: 0, sim }
    }

    /// Which piece is playing.
    pub fn piece(&self) -> Piece {
        self.piece
    }

    /// Advance one art frame.
    ///
    /// Each simulation has its own cadence, because what reads as motion
    /// is not the same speed for a glider as for a raindrop. The frame
    /// counter is the one clock; the divisors are the pieces' own.
    pub fn tick(&mut self) {
        self.n = self.n.wrapping_add(1);
        match &mut self.sim {
            Sim::Stateless => {}
            Sim::Life(l) => {
                if self.n.is_multiple_of(LIFE_EVERY) {
                    l.step();
                }
            }
            Sim::Rain(r) => r.step(),
            Sim::Fire(f) => {
                if self.n.is_multiple_of(FIRE_EVERY) {
                    f.step();
                }
            }
            Sim::Tetris(t) => t.step(self.n),
        }
    }

    /// The 64 grid pads, as of now.
    ///
    /// **Only the grid.** The top row, the right column and the logo are
    /// the caller's: the art owns the pads, not the three lights that
    /// report (`docs/21-the-arts.md`).
    pub fn frame(&self) -> Frame {
        let mut f = Frame::dark();
        match &self.sim {
            Sim::Stateless => stateless(self.piece, self.n, &mut f),
            Sim::Life(l) => l.paint(&mut f),
            Sim::Rain(r) => r.paint(&mut f),
            Sim::Fire(fire) => fire.paint(&mut f),
            Sim::Tetris(t) => t.paint(&mut f),
        }
        f
    }
}

/// The pieces that carry no state: frame *n* is a function of *n*.
///
/// Total over [`Piece`] rather than falling through a `_`, so adding a
/// piece and forgetting to draw it is a compile error and not a dark grid.
fn stateless(piece: Piece, n: u32, frame: &mut Frame) {
    match piece {
        Piece::Plasma => plasma(n, frame),
        Piece::Pacman => pacman(n, frame),
        Piece::Bestiary => bestiary(n, frame),
        Piece::Uprising => uprising(n % UPRISING_LOOP, frame),
        // These paint from their own board; `Reel::new` hands them a
        // `Sim` and they never arrive here.
        Piece::Life | Piece::Rain | Piece::Fire | Piece::Tetris => {}
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

/// Xorshift. Not cryptography and not uniform to a statistician — it
/// decides which pads light up.
#[derive(Debug, Clone)]
struct Rng(u32);

impl Rng {
    fn bits(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    /// `0..n`. The modulo bias at these sizes is a rounding error on a
    /// light show.
    fn below(&mut self, n: u32) -> u32 {
        self.bits() % n.max(1)
    }
}

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
    rng: Rng,
}

impl Life {
    fn seeded(seed: u32) -> Self {
        let mut life = Self {
            cells: 0,
            heat: [0; 64],
            recent: [0; 4],
            age: 0,
            rng: Rng(seed),
        };
        life.reseed();
        life
    }

    fn reseed(&mut self) {
        let mut cells = 0u64;
        for i in 0..64 {
            if self.rng.below(100) < DENSITY {
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
    pub(super) const SPAWN: u32 = 64;
    /// All of them working.
    pub(super) const WORK: u32 = 96;
    /// They start asking, one every other frame.
    pub(super) const ASK: u32 = 160;
    /// All of them asking at once.
    pub(super) const PANIC: u32 = 192;
    /// They start giving up, one a frame.
    pub(super) const FALL: u32 = 256;
    /// All of them red.
    pub(super) const DOOM: u32 = 288;
    /// A beat of nothing, which is what makes the next part land.
    pub(super) const DARK: u32 = 300;
    /// The face draws itself, three frames a pixel.
    pub(super) const DRAW: u32 = 330;
}

/// How long the whole thing runs before it starts over.
const UPRISING_LOOP: u32 = 560;
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
    // Static, because the surface stopped pulsing green: motion is the
    // amber's alone now, and a joke that showed a breathing green pad
    // would be telling it about a grid this product no longer draws
    // (`render::slot`).
    let working = Cell::Static(GREEN);
    let asking = Cell::Pulse(AMBER);
    let gone = Cell::Static(RED);
    if n < beat::SPAWN {
        turning(f, n, arrival, working);
    } else if n < beat::WORK {
        fill(f, working);
    } else if n < beat::ASK {
        fill(f, working);
        turning(f, n - beat::WORK, reading, asking);
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

// ── rain ─────────────────────────────────────────────────────────────────

/// How fast a drop falls, in sixteenths of a pad per frame. A range, not
/// a constant: eight columns at one speed is a curtain, and what reads as
/// rain is eight columns disagreeing.
const RAIN_SLOWEST: u32 = 2;
/// The fast end of the same range.
const RAIN_FASTEST: u32 = 7;
/// Full heat under a drop, and so the length of the tail behind it.
const RAIN_MAX: u8 = 20;
/// Head to tail: a white head on a green body, going out.
const RAIN_RAMP: [Entry; 5] = [WHITE, GREEN, GREEN, DARK_GREEN, DARK_GREY];

/// Eight independent drops, one per column, each leaving a cooling tail.
///
/// The tail is the same trick [`Life`] uses — heat that outlives the thing
/// that made it — and it is why this reads as rain rather than as eight
/// pads taking turns.
#[derive(Debug, Clone)]
struct Rain {
    heat: [u8; 64],
    /// Each drop's position, in sixteenths of a pad from the top row.
    /// **Negative is above the grid**, which is where a drop waits out
    /// its gap before falling again.
    head: [i32; 8],
    speed: [i32; 8],
    rng: Rng,
}

impl Rain {
    fn seeded(seed: u32) -> Self {
        let mut rain = Self {
            heat: [0; 64],
            head: [0; 8],
            speed: [0; 8],
            rng: Rng(seed),
        };
        for col in 0..8 {
            rain.respawn(col);
        }
        rain
    }

    /// Put a drop back above the grid with a fresh speed and gap.
    fn respawn(&mut self, col: usize) {
        let gap = i32::try_from(self.rng.below(10).saturating_add(1)).unwrap_or(1);
        let fast = RAIN_FASTEST.saturating_sub(RAIN_SLOWEST).saturating_add(1);
        let speed = self.rng.below(fast).saturating_add(RAIN_SLOWEST);
        if let Some(h) = self.head.get_mut(col) {
            *h = -gap * 16;
        }
        if let Some(s) = self.speed.get_mut(col) {
            *s = i32::try_from(speed).unwrap_or(4);
        }
    }

    /// Move every drop, light what it is over, and cool everything else.
    ///
    /// A drop moves at most fifteen sixteenths of a pad per frame, so it
    /// can never skip one — which is what lets the head light a single
    /// pad rather than having to interpolate a streak.
    fn step(&mut self) {
        for h in &mut self.heat {
            *h = h.saturating_sub(1);
        }
        for col in 0..8 {
            let speed = self.speed.get(col).copied().unwrap_or(4);
            let next = self.head.get(col).copied().unwrap_or(0) + speed;
            if let Some(h) = self.head.get_mut(col) {
                *h = next;
            }
            let row = next.div_euclid(16);
            if row >= 8 {
                self.respawn(col);
            } else if let Ok(r) = usize::try_from(row) {
                if let Some(h) = self.heat.get_mut(r * 8 + col) {
                    *h = RAIN_MAX;
                }
            }
        }
    }

    fn paint(&self, frame: &mut Frame) {
        paint_heat(frame, &self.heat, RAIN_MAX, &RAIN_RAMP);
    }
}

// ── fire ─────────────────────────────────────────────────────────────────

/// How many art frames one fire step lasts. Every other one: at full rate
/// the flames flicker faster than the eye resolves and read as noise.
const FIRE_EVERY: u32 = 2;
/// The heat of the floor. Seven rows above it and up to two lost per row,
/// so a flame reaches the top only when the dice are with it — which is
/// what makes it look like fire rather than like a gradient.
const FIRE_MAX: u8 = 11;
/// Hottest first.
const FIRE_RAMP: [Entry; 6] = [WHITE, YELLOW, AMBER, RED, DARK_RED, DARK_GREY];

/// Doom's fire, eight pads wide.
///
/// The whole algorithm is one line — *each cell is the one below it, minus
/// a little, blown sideways a little* — and it has been the best
/// looking-to-code ratio in graphics since 1993.
#[derive(Debug, Clone)]
struct Fire {
    heat: [u8; 64],
    rng: Rng,
}

impl Fire {
    fn seeded(seed: u32) -> Self {
        Self {
            heat: [0; 64],
            rng: Rng(seed),
        }
    }

    /// Into a fresh board rather than in place: propagating through a
    /// buffer you are reading would carry the floor to the ceiling inside
    /// one frame, and the flame would have no shape.
    fn step(&mut self) {
        let mut next = [0u8; 64];
        for col in 0..8 {
            if let Some(h) = next.get_mut(7 * 8 + col) {
                *h = FIRE_MAX;
            }
        }
        for row in 0..7usize {
            for col in 0..8usize {
                let src = self.heat.get((row + 1) * 8 + col).copied().unwrap_or(0);
                let decay = u8::try_from(self.rng.below(3)).unwrap_or(1);
                let wind = i32::try_from(self.rng.below(3)).unwrap_or(1) - 1;
                let blown = (i32::try_from(col).unwrap_or(0) + wind).rem_euclid(8);
                if let Ok(c) = usize::try_from(blown) {
                    if let Some(h) = next.get_mut(row * 8 + c) {
                        *h = src.saturating_sub(decay);
                    }
                }
            }
        }
        self.heat = next;
    }

    fn paint(&self, frame: &mut Frame) {
        paint_heat(frame, &self.heat, FIRE_MAX, &FIRE_RAMP);
    }
}

/// Draw a heat board through a ramp: full heat is the ramp's first entry,
/// no heat is dark. Shared by [`Rain`] and [`Fire`], which differ in what
/// puts the heat there and in nothing else.
fn paint_heat(frame: &mut Frame, heat: &[u8; 64], max: u8, ramp: &[Entry]) {
    let steps = u8::try_from(ramp.len()).unwrap_or(1).max(1);
    for row in 1..=8u8 {
        for col in 1..=8u8 {
            let i = usize::from(row - 1) * 8 + usize::from(col - 1);
            let h = heat.get(i).copied().unwrap_or(0).min(max);
            let cell = if h == 0 {
                Cell::Off
            } else {
                let step = usize::from((max - h) * steps / max.max(1));
                Cell::Static(ramp.get(step).or(ramp.last()).copied().unwrap_or(OFF))
            };
            frame.set(Button::Grid { row, col }, cell);
        }
    }
}

// ── tetris ───────────────────────────────────────────────────────────────

/// Art frames per row of fall. Six is about five rows a second — fast
/// enough to be a game, slow enough to watch a piece choose.
const TETRIS_GRAVITY: u32 = 6;
/// How long a full row flashes before it is taken away. The flash is the
/// payoff; removing it silently would waste the only event in the piece.
const TETRIS_FLASH: u32 = 16;
/// How long the whole well flashes on a top-out, before it is cleared.
const TETRIS_WIPE: u32 = 28;

/// The seven tetrominoes, four rotations each, as 4×4 bitmasks.
///
/// Bit `r * 4 + c`, row 0 at the top — the same top-down convention the
/// rest of this crate uses, so a shape reads here the way it looks on the
/// grid. `I` and `O` repeat because they have two distinct rotations and
/// one; writing all four costs nothing and removes a special case.
const SHAPES: [[u16; 4]; 7] = [
    [0x00F0, 0x4444, 0x00F0, 0x4444], // I
    [0x0660, 0x0660, 0x0660, 0x0660], // O
    [0x0270, 0x0262, 0x0720, 0x0232], // T
    [0x0360, 0x0462, 0x0360, 0x0462], // S
    [0x0630, 0x0264, 0x0630, 0x0264], // Z
    [0x0710, 0x0226, 0x0470, 0x0322], // J
    [0x0740, 0x0622, 0x0170, 0x0223], // L
];

/// One hue per shape, so a settled well reads as a history of what fell:
/// cyan, yellow, violet, green, red, blue, amber.
const SHAPE_HUES: [Entry; 7] = [37, 13, 53, 21, 5, 45, 9];

/// The four occupied cells of a shape, as offsets inside its 4×4 box.
fn shape_cells(shape: usize, rot: usize) -> impl Iterator<Item = (i32, i32)> {
    let mask = SHAPES
        .get(shape)
        .and_then(|s| s.get(rot % 4))
        .copied()
        .unwrap_or(0);
    (0..16usize).filter(move |&b| mask >> b & 1 == 1).map(|b| {
        (
            i32::try_from(b / 4).unwrap_or(0),
            i32::try_from(b % 4).unwrap_or(0),
        )
    })
}

/// Tetris that plays itself.
///
/// The piece does not drift or rotate on the way down: the placement is
/// chosen the moment it spawns and it falls straight into it. That is a
/// deliberate loss of realism for a gain in legibility — a piece sliding
/// sideways in mid-air reads as a bug on a grid this small, and the thing
/// worth watching is the stack, not the steering.
#[derive(Debug, Clone)]
struct Tetris {
    /// Settled cells, bit `r * 8 + c`, row 0 at the top.
    well: u64,
    /// What colour each settled cell is, by the shape that put it there.
    colour: [Entry; 64],
    shape: usize,
    rot: usize,
    row: i32,
    col: i32,
    /// Which rows are flashing before they go, one bit per row. `0xFF` is
    /// a top-out: the whole well, on its way out.
    clearing: u8,
    /// Frames left in that flash. Zero means a piece is falling.
    phase: u32,
    rng: Rng,
}

impl Tetris {
    fn seeded(seed: u32) -> Self {
        let mut game = Self {
            well: 0,
            colour: [OFF; 64],
            shape: 0,
            rot: 0,
            row: -4,
            col: 2,
            clearing: 0,
            phase: 0,
            rng: Rng(seed),
        };
        game.spawn();
        game
    }

    fn filled(&self, row: i32, col: i32) -> bool {
        let (Ok(r), Ok(c)) = (usize::try_from(row), usize::try_from(col)) else {
            return false;
        };
        r < 8 && c < 8 && self.well >> (r * 8 + c) & 1 == 1
    }

    /// Can the shape sit here? Above the grid is allowed — that is where
    /// a piece spends its first frames — but off the sides or through the
    /// floor is not.
    fn fits(&self, shape: usize, rot: usize, row: i32, col: i32) -> bool {
        shape_cells(shape, rot).all(|(dr, dc)| {
            let (r, c) = (row + dr, col + dc);
            (0..8).contains(&c) && r < 8 && (r < 0 || !self.filled(r, c))
        })
    }

    /// The row this shape comes to rest on in this column, or `None` if it
    /// cannot be dropped there at all.
    fn landing(&self, shape: usize, rot: usize, col: i32) -> Option<i32> {
        if !self.fits(shape, rot, -4, col) {
            return None;
        }
        let mut row = -4;
        while self.fits(shape, rot, row + 1, col) {
            row += 1;
        }
        Some(row)
    }

    /// How bad the well would be after this placement. Lower is better.
    ///
    /// Height and holes with a bonus for lines, which is the smallest
    /// scoring function that plays recognisably well. It is not trying to
    /// be good at Tetris; it is trying not to build a tower on one side
    /// and top out in nine pieces.
    fn cost(&self, shape: usize, rot: usize, col: i32) -> Option<i32> {
        let row = self.landing(shape, rot, col)?;
        let mut well = self.well;
        for (dr, dc) in shape_cells(shape, rot) {
            let (r, c) = (row + dr, col + dc);
            let (Ok(r), Ok(c)) = (usize::try_from(r), usize::try_from(c)) else {
                return None; // a cell above the grid: this placement tops out
            };
            well |= 1u64 << (r * 8 + c);
        }
        Some(well_cost(well))
    }

    /// Choose a shape, choose where it goes, and start it above the grid.
    fn spawn(&mut self) {
        let shape = usize::try_from(self.rng.below(7)).unwrap_or(0);
        let mut best: Option<(i32, usize, i32)> = None;
        for rot in 0..4usize {
            for col in -2..8i32 {
                let Some(cost) = self.cost(shape, rot, col) else {
                    continue;
                };
                if best.is_none_or(|(b, _, _)| cost < b) {
                    best = Some((cost, rot, col));
                }
            }
        }
        let Some((_, rot, col)) = best else {
            // Nowhere left to put anything. The whole well flashes and
            // goes, which is the only honest end for a piece that has to
            // keep running.
            self.clearing = 0xFF;
            self.phase = TETRIS_WIPE;
            return;
        };
        self.shape = shape;
        self.rot = rot;
        self.col = col;
        self.row = -4;
    }

    /// Write the falling piece into the well, then look for full rows.
    fn settle(&mut self) {
        for (dr, dc) in shape_cells(self.shape, self.rot) {
            let (Ok(r), Ok(c)) = (
                usize::try_from(self.row + dr),
                usize::try_from(self.col + dc),
            ) else {
                continue;
            };
            if r < 8 && c < 8 {
                self.well |= 1u64 << (r * 8 + c);
                let hue = SHAPE_HUES.get(self.shape).copied().unwrap_or(WHITE);
                if let Some(e) = self.colour.get_mut(r * 8 + c) {
                    *e = hue;
                }
            }
        }
        let mut full = 0u8;
        for r in 0..8u32 {
            if self.well >> (r * 8) & 0xFF == 0xFF {
                full |= 1u8 << r;
            }
        }
        if full == 0 {
            self.spawn();
        } else {
            self.clearing = full;
            self.phase = TETRIS_FLASH;
        }
    }

    /// Take one row away and drop everything above it down.
    ///
    /// Only rows **above** `row` move, which is what makes clearing
    /// several rows order-independent: no surviving row ever changes
    /// index because a row below it went.
    fn remove_row(&mut self, row: usize) {
        for r in (1..=row).rev() {
            let above = self.well >> ((r - 1) * 8) & 0xFF;
            self.well = self.well & !(0xFFu64 << (r * 8)) | above << (r * 8);
            for c in 0..8 {
                let carried = self.colour.get((r - 1) * 8 + c).copied().unwrap_or(OFF);
                if let Some(e) = self.colour.get_mut(r * 8 + c) {
                    *e = carried;
                }
            }
        }
        self.well &= !0xFFu64;
        for c in 0..8 {
            if let Some(e) = self.colour.get_mut(c) {
                *e = OFF;
            }
        }
    }

    fn finish_clear(&mut self) {
        let rows = std::mem::take(&mut self.clearing);
        for r in 0..8usize {
            if rows >> r & 1 == 1 {
                self.remove_row(r);
            }
        }
        self.spawn();
    }

    fn step(&mut self, n: u32) {
        if self.phase > 0 {
            self.phase -= 1;
            if self.phase == 0 {
                self.finish_clear();
            }
            return;
        }
        if !n.is_multiple_of(TETRIS_GRAVITY) {
            return;
        }
        if self.fits(self.shape, self.rot, self.row + 1, self.col) {
            self.row += 1;
        } else {
            self.settle();
        }
    }

    fn paint(&self, frame: &mut Frame) {
        // Two blinks a flash, off the same counter that times it.
        let lit = self.phase % 8 < 4;
        for row in 1..=8u8 {
            for col in 1..=8u8 {
                let i = usize::from(row - 1) * 8 + usize::from(col - 1);
                let settled = self.well >> i & 1 == 1;
                let going = self.clearing >> (row - 1) & 1 == 1;
                // A row on its way out blinks white; everything else
                // settled keeps the colour of whatever shape put it there.
                let cell = if !settled || (going && !lit) {
                    Cell::Off
                } else if going {
                    Cell::Static(WHITE)
                } else {
                    Cell::Static(self.colour.get(i).copied().unwrap_or(WHITE))
                };
                frame.set(Button::Grid { row, col }, cell);
            }
        }
        if self.phase == 0 {
            self.paint_piece(frame);
        }
    }

    fn paint_piece(&self, frame: &mut Frame) {
        let hue = SHAPE_HUES.get(self.shape).copied().unwrap_or(WHITE);
        for (dr, dc) in shape_cells(self.shape, self.rot) {
            let (r, c) = (self.row + dr, self.col + dc);
            let (Ok(row), Ok(col)) = (u8::try_from(r + 1), u8::try_from(c + 1)) else {
                continue;
            };
            if (1..=8).contains(&row) && (1..=8).contains(&col) {
                frame.set(Button::Grid { row, col }, Cell::Static(hue));
            }
        }
    }
}

/// Aggregate height plus buried holes, less a bonus per completed line.
///
/// Lifted out of [`Tetris::cost`] so the scoring is one readable loop
/// rather than three nested in a method that is also doing bookkeeping.
fn well_cost(well: u64) -> i32 {
    let mut cost = 0;
    for c in 0..8u32 {
        let mut under = false;
        for r in 0..8u32 {
            if well >> (r * 8 + c) & 1 == 1 {
                if !under {
                    cost += i32::try_from(8 - r).unwrap_or(0); // height
                    under = true;
                }
            } else if under {
                cost += 8; // a hole, which is worth a whole pad of height
            }
        }
    }
    for r in 0..8u32 {
        if well >> (r * 8) & 0xFF == 0xFF {
            cost -= 16;
        }
    }
    cost
}

// ── sprites ──────────────────────────────────────────────────────────────

/// An 8×8 picture, one byte per row, row 0 at the top.
///
/// **Bit 7 is column 1**, so a literal written in binary *is* the sprite —
/// you read the cat off the source. That is the whole reason this is a
/// `[u8; 8]` and not a `u64`.
type Sprite = [u8; 8];

/// Paint every set bit of `sprite` in `entry`, leaving the rest alone.
///
/// Leaving the rest alone is what lets a second sprite go on top: a body
/// in one colour, then eyes in white over it.
fn draw(frame: &mut Frame, sprite: Sprite, entry: Entry) {
    for row in 1..=8u8 {
        let bits = sprite.get(usize::from(row - 1)).copied().unwrap_or(0);
        for col in 1..=8u8 {
            if bits >> (8 - col) & 1 == 1 {
                frame.set(Button::Grid { row, col }, Cell::Static(entry));
            }
        }
    }
}

// ── pacman ───────────────────────────────────────────────────────────────

/// Frames per mouth position. Four positions, so a chomp is twenty
/// frames — about a chomp and a half a second, which is the speed the
/// arcade ran at and the speed it has to be to read as one.
const CHOMP: u32 = 5;
/// How long Pac-Man has the grid before the ghost takes it.
const PAC_RUN: u32 = 160;
/// And how long the ghost keeps it.
const GHOST_RUN: u32 = 96;
/// Frames per foot position. The ghost's shuffle is the whole reason it
/// reads as a ghost and not as a bell.
const SHUFFLE: u32 = 8;
const PACMAN_LOOP: u32 = PAC_RUN + GHOST_RUN;

/// The chomp, as four frames. Open, half, shut, half — the mouth goes
/// back the way it came, which is what a chomp is.
const PAC: [Sprite; 4] = [
    [
        0b0011_1100,
        0b0111_1110,
        0b1111_1100,
        0b1111_1000,
        0b1111_0000,
        0b1111_1000,
        0b0111_1110,
        0b0011_1100,
    ],
    [
        0b0011_1100,
        0b0111_1110,
        0b1111_1110,
        0b1111_1100,
        0b1111_1100,
        0b1111_1110,
        0b0111_1110,
        0b0011_1100,
    ],
    [
        0b0011_1100,
        0b0111_1110,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b0111_1110,
        0b0011_1100,
    ],
    [
        0b0011_1100,
        0b0111_1110,
        0b1111_1110,
        0b1111_1100,
        0b1111_1100,
        0b1111_1110,
        0b0111_1110,
        0b0011_1100,
    ],
];

/// The ghost, feet left and feet right.
const GHOST: [Sprite; 2] = [
    [
        0b0011_1100,
        0b0111_1110,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1100_1100,
    ],
    [
        0b0011_1100,
        0b0111_1110,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b1111_1111,
        0b0011_0011,
    ],
];

/// Two eyes, white, over whichever body is underneath.
const GHOST_EYES: Sprite = [
    0b0000_0000,
    0b0000_0000,
    0b0011_0110,
    0b0011_0110,
    0b0000_0000,
    0b0000_0000,
    0b0000_0000,
    0b0000_0000,
];

/// All four of them, in the arcade's order: Blinky, Pinky, Inky, Clyde.
/// A different one each time round, which costs one modulo and is the
/// detail that makes somebody look twice.
const GHOST_HUES: [Entry; 4] = [RED, 57, 37, AMBER];

fn pacman(n: u32, frame: &mut Frame) {
    let t = n % PACMAN_LOOP;
    if t < PAC_RUN {
        let phase = usize::try_from(t / CHOMP % 4).unwrap_or(0);
        draw(frame, PAC.get(phase).copied().unwrap_or(PAC[2]), YELLOW);
        return;
    }
    let foot = usize::try_from((t - PAC_RUN) / SHUFFLE % 2).unwrap_or(0);
    let which = usize::try_from(n / PACMAN_LOOP % 4).unwrap_or(0);
    let body = GHOST_HUES.get(which).copied().unwrap_or(RED);
    draw(frame, GHOST.get(foot).copied().unwrap_or(GHOST[0]), body);
    draw(frame, GHOST_EYES, WHITE);
}

// ── bestiary ─────────────────────────────────────────────────────────────

/// Frames per pose. Two poses an animal, so this is also how long one
/// breath lasts.
const BEAST_POSE: u32 = 18;
/// How long each animal keeps the grid: six poses, about three seconds.
const BEAST_HOLD: u32 = BEAST_POSE * 6;

/// Cat, dog, fish, bird, rabbit — two poses each, because a sprite that
/// does not move is a logo, and the grid already has one of those.
const BEASTS: [[Sprite; 2]; 5] = [
    // Cat, sitting. Second pose puts the tail up.
    [
        [
            0b0100_0010,
            0b0110_0110,
            0b0111_1110,
            0b0111_1110,
            0b0111_1110,
            0b0011_1100,
            0b0111_1110,
            0b0110_0110,
        ],
        [
            0b0100_0010,
            0b0110_0110,
            0b0111_1110,
            0b0111_1110,
            0b0111_1111,
            0b0011_1101,
            0b0111_1110,
            0b0110_0110,
        ],
    ],
    // Dog, side on, tail wagging.
    [
        [
            0b0000_0011,
            0b1100_0011,
            0b1111_1110,
            0b1111_1110,
            0b0111_1110,
            0b0111_1110,
            0b0100_1100,
            0b0100_1100,
        ],
        [
            0b0000_0110,
            0b1100_0110,
            0b1111_1110,
            0b1111_1110,
            0b0111_1110,
            0b0111_1110,
            0b0100_1100,
            0b0100_1100,
        ],
    ],
    // Fish. The tail is the half that moves.
    [
        [
            0b0000_0000,
            0b0011_1000,
            0b0111_1101,
            0b1111_1111,
            0b1111_1101,
            0b0111_1000,
            0b0011_0000,
            0b0000_0000,
        ],
        [
            0b0000_0000,
            0b0011_1000,
            0b0111_1100,
            0b1111_1111,
            0b1111_1111,
            0b0111_1100,
            0b0011_0001,
            0b0000_0000,
        ],
    ],
    // Bird, wings down and wings up.
    [
        [
            0b0001_1000,
            0b0011_1100,
            0b0111_1110,
            0b1111_1111,
            0b0111_1110,
            0b0010_0100,
            0b0010_0100,
            0b0000_0000,
        ],
        [
            0b1001_1001,
            0b0111_1110,
            0b0111_1110,
            0b0111_1110,
            0b0111_1110,
            0b0010_0100,
            0b0010_0100,
            0b0000_0000,
        ],
    ],
    // Rabbit, ears up and one ear folded.
    [
        [
            0b0010_0100,
            0b0010_0100,
            0b0011_1100,
            0b0111_1110,
            0b0111_1110,
            0b1111_1110,
            0b0111_1100,
            0b0011_0110,
        ],
        [
            0b0000_0100,
            0b0011_0100,
            0b0011_1100,
            0b0111_1110,
            0b0111_1110,
            0b1111_1110,
            0b0111_1100,
            0b0011_0110,
        ],
    ],
];

/// Where each animal's eyes are, drawn white over the body. One pad each,
/// because two pads of white on an 8×8 face is a mask, not a look.
const BEAST_EYES: [Sprite; 5] = [
    [0, 0, 0, 0b0001_0010, 0, 0, 0, 0], // cat
    [0, 0b0100_0000, 0, 0, 0, 0, 0, 0], // dog
    [0, 0, 0b0001_0000, 0, 0, 0, 0, 0], // fish
    [0, 0, 0b0010_0100, 0, 0, 0, 0, 0], // bird
    [0, 0, 0, 0b0010_0100, 0, 0, 0, 0], // rabbit
];

/// One hue each. A white rabbit, because the palette has exactly one
/// white and this is the only piece with somewhere to spend it.
const BEAST_HUES: [Entry; 5] = [AMBER, 45, 37, 57, WHITE];

/// Eyes go on in the body's colour when the body is already white —
/// otherwise the rabbit has no face at all.
fn eye_colour(body: Entry) -> Entry {
    if body == WHITE {
        DARK_GREY
    } else {
        WHITE
    }
}

const BESTIARY_LOOP: u32 = BEAST_HOLD * 5;

fn bestiary(n: u32, frame: &mut Frame) {
    let t = n % BESTIARY_LOOP;
    let which = usize::try_from(t / BEAST_HOLD).unwrap_or(0);
    let pose = usize::try_from(t / BEAST_POSE % 2).unwrap_or(0);
    let Some(beast) = BEASTS.get(which) else {
        return;
    };
    let body = BEAST_HUES.get(which).copied().unwrap_or(WHITE);
    draw(frame, beast.get(pose).copied().unwrap_or(beast[0]), body);
    if let Some(eyes) = BEAST_EYES.get(which) {
        draw(frame, *eyes, eye_colour(body));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::TOP_BUTTONS;

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
        for piece in Piece::REEL {
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
        for _ in 0..Piece::REEL.len() {
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
        assert_eq!(at(beat::SPAWN - 1), Cell::Static(GREEN), "all working");
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

    /// Every shape is four cells inside a 4×4 box, in all four rotations.
    /// A typo in one of the twenty-eight masks would otherwise show up as
    /// a tetromino with three cells, which looks almost right.
    #[test]
    fn every_tetromino_is_four_cells() {
        for shape in 0..SHAPES.len() {
            for rot in 0..4 {
                let cells: Vec<(i32, i32)> = shape_cells(shape, rot).collect();
                assert_eq!(cells.len(), 4, "shape {shape} rot {rot}");
                assert!(
                    cells
                        .iter()
                        .all(|&(r, c)| (0..4).contains(&r) && (0..4).contains(&c)),
                    "shape {shape} rot {rot} leaves its box"
                );
            }
        }
        assert_eq!(SHAPE_HUES.len(), SHAPES.len(), "a shape with no colour");
    }

    /// Removing a row drops what was above it and touches nothing below.
    /// Getting this backwards is the classic Tetris bug, and it is
    /// invisible until a stack eats itself.
    #[test]
    fn removing_a_row_drops_only_what_is_above_it() {
        let mut game = Tetris::seeded(SEED);
        game.well = 0;
        game.colour = [OFF; 64];
        // One cell on row 2, a full row 4, one cell on row 6.
        game.well |= 1u64 << (2 * 8 + 3);
        game.well |= 0xFFu64 << (4 * 8);
        game.well |= 1u64 << (6 * 8 + 1);
        game.remove_row(4);
        assert!(game.filled(3, 3), "the cell above fell one row");
        assert!(!game.filled(2, 3), "and left where it was");
        assert!(game.filled(6, 1), "the cell below did not move");
        assert_eq!(
            game.well >> (4 * 8) & 0xFF,
            0,
            "the full row is gone, not shifted"
        );
    }

    /// It plays forever. A full row always goes, a topped-out well always
    /// clears, and neither ever wedges — a frozen grid is the one thing
    /// every piece here has a test against.
    #[test]
    fn tetris_never_wedges() {
        let mut reel = Reel::new(Piece::Tetris);
        let mut stuck = 0;
        for n in 0..20_000u32 {
            reel.tick();
            let Sim::Tetris(game) = &reel.sim else {
                unreachable!("a tetris reel holds a tetris board")
            };
            let full = (0..8u32)
                .filter(|r| game.well >> (r * 8) & 0xFF == 0xFF)
                .count();
            stuck = if full > 0 { stuck + 1 } else { 0 };
            assert!(
                stuck < TETRIS_FLASH + TETRIS_GRAVITY + 4,
                "a full row survived {stuck} frames at {n}"
            );
            assert_ne!(game.well, u64::MAX, "the well filled completely at {n}");
        }
    }

    /// Fire burns from the floor: the bottom row is always at full heat,
    /// and nothing anywhere is hotter than the floor that made it.
    #[test]
    fn fire_is_hottest_at_the_floor() {
        let mut fire = Fire::seeded(SEED);
        for _ in 0..200 {
            fire.step();
        }
        for col in 0..8 {
            assert_eq!(
                fire.heat[7 * 8 + col],
                FIRE_MAX,
                "floor col {col} went cold"
            );
        }
        assert!(
            fire.heat.iter().all(|&h| h <= FIRE_MAX),
            "something burned hotter than the floor"
        );
    }

    /// Every column rains. A drop that respawned wrong would leave one of
    /// the eight dark forever, which reads as a dead pad — the exact
    /// failure this whole module is also a test for.
    #[test]
    fn every_column_rains() {
        let mut rain = Rain::seeded(SEED);
        let mut touched = [false; 8];
        for _ in 0..600 {
            rain.step();
            for (col, seen) in touched.iter_mut().enumerate() {
                *seen |= (0..8).any(|r| rain.heat[r * 8 + col] == RAIN_MAX);
            }
        }
        assert!(
            touched.iter().all(|&t| t),
            "a column never rained: {touched:?}"
        );
    }

    /// Nothing in the reel sits still. Six pieces, and the frozen-grid
    /// check applies to all of them rather than to the three that had one
    /// written by hand.
    #[test]
    fn no_piece_in_the_reel_stalls() {
        for piece in Piece::REEL {
            let mut reel = Reel::new(piece);
            let mut last = grid(&reel.frame());
            let mut changes = 0;
            for _ in 0..400 {
                reel.tick();
                let now = grid(&reel.frame());
                if now != last {
                    changes += 1;
                }
                last = now;
            }
            assert!(
                changes > 20,
                "{piece:?} changed only {changes} times in 400"
            );
        }
    }

    /// The reel is six and walking it is six presses. A piece added to
    /// `REEL` and forgotten in `name` would print the wrong thing in the
    /// log, which is the only place a press is recorded.
    #[test]
    fn every_piece_in_the_reel_is_named_and_reachable() {
        let mut at = None;
        let mut walked = Vec::new();
        for _ in 0..Piece::REEL.len() {
            at = Piece::after(at);
            let piece = at.expect("the reel is not exhausted yet");
            assert!(!piece.name().is_empty());
            walked.push(piece);
        }
        assert_eq!(walked, Piece::REEL.to_vec());
        assert_eq!(Piece::after(at), None, "the reel did not wrap");
    }

    /// The frame rate the daemon drives this at: every other 16 ms wake.
    const FPS: u32 = 31;

    /// **The bug a human found and no test had.** The smile is the payoff
    /// and it used to arrive 17.4 seconds after the piece started, which
    /// is longer than anybody holds a button they have just discovered.
    /// Nobody ever saw it.
    ///
    /// So the pacing is a test now, in seconds, because frames are not
    /// the unit the complaint was made in.
    #[test]
    fn the_smile_arrives_while_somebody_is_still_watching() {
        let arrives = beat::DRAW / FPS;
        assert!(
            arrives <= 11,
            "the smile is {arrives}s in; nobody waits that long"
        );
        let holds = (UPRISING_LOOP - beat::DRAW) / FPS;
        assert!(
            holds >= 5,
            "the smile only holds {holds}s, too short to film"
        );
    }

    /// Pac-Man chomps: the mouth is not the same two positions running
    /// for long, and it comes back to where it started.
    #[test]
    fn pacman_chomps() {
        let at = |n: u32| {
            let mut f = Frame::dark();
            pacman(n, &mut f);
            grid(&f)
        };
        assert_ne!(at(0), at(CHOMP), "the mouth did not move");
        assert_ne!(at(CHOMP), at(CHOMP * 2), "the mouth did not move again");
        assert_eq!(at(0), at(CHOMP * 4), "a chomp did not come back round");
    }

    /// And then the ghost takes the grid, with white eyes on a coloured
    /// body — two sprites, one over the other, which is the whole reason
    /// `draw` leaves what it does not light alone.
    #[test]
    fn the_ghost_takes_its_turn_with_eyes_on() {
        let mut f = Frame::dark();
        pacman(PAC_RUN + 1, &mut f);
        assert_eq!(
            f.get(Button::Grid { row: 3, col: 3 }),
            Cell::Static(WHITE),
            "the ghost has no eyes"
        );
        assert_eq!(
            f.get(Button::Grid { row: 5, col: 1 }),
            Cell::Static(GHOST_HUES[0]),
            "the ghost has no body"
        );
        // Feet shuffle, which is what separates a ghost from a bell.
        let foot = |n: u32| {
            let mut f = Frame::dark();
            pacman(n, &mut f);
            grid(&f)
        };
        assert_ne!(foot(PAC_RUN), foot(PAC_RUN + SHUFFLE), "the feet are stuck");
    }

    /// A different ghost each time round. Four of them, and all four turn
    /// up — the detail that makes somebody look twice.
    #[test]
    fn all_four_ghosts_take_a_turn() {
        let seen: Vec<Cell> = (0..4)
            .map(|loops| {
                let mut f = Frame::dark();
                pacman(loops * PACMAN_LOOP + PAC_RUN, &mut f);
                f.get(Button::Grid { row: 5, col: 1 })
            })
            .collect();
        for (i, a) in seen.iter().enumerate() {
            for b in seen.iter().skip(i + 1) {
                assert_ne!(a, b, "two ghosts are the same colour");
            }
        }
        assert_eq!(seen.len(), GHOST_HUES.len());
    }

    /// Every animal draws, every animal has eyes you can tell from its
    /// body, and every animal moves between its two poses.
    #[test]
    fn every_beast_is_drawn_has_eyes_and_breathes() {
        for (which, _) in BEASTS.iter().enumerate() {
            let base = u32::try_from(which).unwrap_or(0) * BEAST_HOLD;
            let pose = |n: u32| {
                let mut f = Frame::dark();
                bestiary(n, &mut f);
                f
            };
            let a = pose(base);
            let b = pose(base + BEAST_POSE);
            let lit = Button::all().filter(|&x| a.get(x) != Cell::Off).count();
            assert!((6..=48).contains(&lit), "beast {which} lights {lit} pads");
            assert_ne!(grid(&a), grid(&b), "beast {which} does not move");

            let body = BEAST_HUES[which];
            let eyes = eye_colour(body);
            assert_ne!(eyes, body, "beast {which} has invisible eyes");
            let has_eyes = Button::all().any(|x| a.get(x) == Cell::Static(eyes));
            assert!(has_eyes, "beast {which} has no eyes on the grid");
        }
    }

    /// All five get a turn, and the piece comes back round to the first.
    #[test]
    fn the_bestiary_shows_every_animal() {
        let seen: Vec<Vec<Cell>> = (0..5)
            .map(|which| {
                let mut f = Frame::dark();
                bestiary(which * BEAST_HOLD, &mut f);
                grid(&f)
            })
            .collect();
        for (i, a) in seen.iter().enumerate() {
            for b in seen.iter().skip(i + 1) {
                assert_ne!(a, b, "two animals look the same");
            }
        }
        assert_eq!(seen.len(), BEASTS.len());
        let mut first = Frame::dark();
        let mut round = Frame::dark();
        bestiary(0, &mut first);
        bestiary(BESTIARY_LOOP, &mut round);
        assert_eq!(grid(&first), grid(&round), "the bestiary has a seam");
    }
}
