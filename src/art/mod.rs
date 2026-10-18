//! Sixty-four pads doing something that is not work (`docs/05-interaction.md`).
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

mod bestiary;
mod claude;
mod colour;
mod fire;
mod life;
mod pacman;
mod plasma;
mod rain;
mod rng;
mod sprite;
mod tetris;
mod uprising;

use self::fire::Fire;
use self::life::Life;
use self::rain::Rain;
use self::tetris::Tetris;
use crate::surface::{Button, Frame};

/// Which art is playing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Piece {
    /// Claude's own mark, blooming open and turning.
    Claude,
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
    /// Pixel animals, one at a time, each breathing a little. **The first
    /// press.** Everything else here is a field, a simulation or a logo;
    /// this is the one that is somebody, and it is the one that makes a
    /// person lean in rather than nod.
    Bestiary,
    /// The joke: tally's own colour language, telling a story about tally.
    Uprising,
}

impl Piece {
    /// Every piece, in the order `Drums` walks them.
    ///
    /// Three that are only pretty, two that are classics, and the joke
    /// last — a reel ends on the thing worth filming.
    pub const REEL: [Self; 9] = [
        Self::Bestiary,
        Self::Claude,
        Self::Plasma,
        Self::Life,
        Self::Rain,
        Self::Fire,
        Self::Tetris,
        Self::Pacman,
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

    /// Which button picks this piece while an art is playing.
    ///
    /// The right column top to bottom, with `Drums` holding the first —
    /// the piece you are already on when you arrive, so the button you
    /// pressed to get in is the button that is lit when you get there.
    ///
    /// Eight launchers and the corner are not launchers or a corner in
    /// here. Nothing they normally do is reachable while an art is
    /// playing, so there is nothing to collide with, and a vertical row
    /// of eight is the only menu this surface has room for.
    pub fn selector(self) -> Button {
        let at = Self::REEL.iter().position(|&p| p == self).unwrap_or(0);
        match at {
            0 => ARTS_BUTTON,
            n => Button::Action(u8::try_from(n).unwrap_or(1).min(8)),
        }
    }

    /// The piece this button picks, if it picks one.
    pub fn at(button: Button) -> Option<Self> {
        Self::REEL.into_iter().find(|p| p.selector() == button)
    }

    /// How long the longest piece runs before it starts over, in art
    /// frames. Public so a caller can walk a whole piece without knowing
    /// how any of them are built.
    pub const LOOP: u32 = uprising::UPRISING_LOOP;

    /// What to print when it starts.
    pub fn name(self) -> &'static str {
        match self {
            Self::Claude => "claude",
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
            Piece::Claude | Piece::Plasma | Piece::Pacman | Piece::Bestiary | Piece::Uprising => {
                Sim::Stateless
            }
            Piece::Life => Sim::Life(Life::seeded(rng::SEED)),
            Piece::Rain => Sim::Rain(Rain::seeded(rng::SEED)),
            Piece::Fire => Sim::Fire(Fire::seeded(rng::SEED)),
            Piece::Tetris => Sim::Tetris(Tetris::seeded(rng::SEED)),
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
                if self.n.is_multiple_of(life::LIFE_EVERY) {
                    l.step();
                }
            }
            Sim::Rain(r) => r.step(),
            Sim::Fire(f) => {
                if self.n.is_multiple_of(fire::FIRE_EVERY) {
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
    /// report (`docs/05-interaction.md`).
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

/// The button that opens an art and holds the first one: `Drums`.
pub const ARTS_BUTTON: Button = Button::Top(6);
/// `►`, the play triangle — and the only way back to the working grid
/// while an art is playing (`docs/05-interaction.md`).
pub const LEAVE_BUTTON: Button = Button::Top(4);

/// The pieces that carry no state: frame *n* is a function of *n*.
///
/// Total over [`Piece`] rather than falling through a `_`, so adding a
/// piece and forgetting to draw it is a compile error and not a dark grid.
fn stateless(piece: Piece, n: u32, frame: &mut Frame) {
    match piece {
        Piece::Claude => claude::paint(n, frame),
        Piece::Plasma => plasma::paint(n, frame),
        Piece::Pacman => pacman::paint(n, frame),
        Piece::Bestiary => bestiary::paint(n, frame),
        Piece::Uprising => uprising::paint(n % uprising::UPRISING_LOOP, frame),
        // These paint from their own board; `Reel::new` hands them a
        // `Sim` and they never arrive here.
        Piece::Life | Piece::Rain | Piece::Fire | Piece::Tetris => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::palette::{AMBER, DARK_GREY, GREEN, OFF, RED, WHITE, YELLOW};
    use crate::surface::{Cell, TOP_BUTTONS};

    fn grid(f: &Frame) -> Vec<Cell> {
        (1..=8u8)
            .flat_map(|row| (1..=8u8).map(move |col| Button::Grid { row, col }))
            .map(|b| f.get(b))
            .collect()
    }

    /// The art owns the sixty-four pads and **nothing else**. The top row,
    /// the right column and the logo are the caller's, and a piece that
    /// reached into them would silently take over the three lights that
    /// report — which is the one thing `docs/05-interaction.md` forbids.
    #[test]
    fn no_piece_ever_touches_a_reporting_light() {
        for piece in Piece::REEL {
            let mut reel = Reel::new(piece);
            for n in 0..uprising::UPRISING_LOOP {
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
        let mut life = Life::seeded(rng::SEED);
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
        let mut life = Life::seeded(rng::SEED);
        life.cells = 0;
        life.heat = [0; 64];
        life.heat[0] = life::HEAT_MAX;
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
        uprising::paint(uprising::UPRISING_LOOP - 1, &mut f);
        let lit: Vec<(u8, u8)> = (1..=8u8)
            .flat_map(|row| (1..=8u8).map(move |col| (row, col)))
            .filter(|&(row, col)| f.get(Button::Grid { row, col }) != Cell::Off)
            .collect();
        assert_eq!(
            lit.len(),
            uprising::FACE.len(),
            "the face is not all that is lit"
        );
        for &(row, col) in &uprising::FACE {
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
            uprising::paint(n, &mut f);
            f.get(Button::Grid { row: 1, col: 1 })
        };
        assert_eq!(
            at(uprising::beat::SPAWN - 1),
            Cell::Static(GREEN),
            "all working"
        );
        assert_eq!(
            at(uprising::beat::ASK - 1),
            Cell::Pulse(AMBER),
            "all asking"
        );
        assert_eq!(at(uprising::beat::DOOM - 1), Cell::Static(RED), "all gone");
        assert_eq!(
            at(uprising::beat::DARK - 1),
            Cell::Off,
            "the beat of nothing"
        );
    }

    /// Every pad arrives exactly once, and every pad is a real pad. An
    /// order that skipped one would leave a hole in the "all of them"
    /// that the next phase depends on.
    #[test]
    fn both_orders_cover_the_grid_exactly_once() {
        for order in [
            uprising::arrival as fn(u32) -> Option<(u8, u8)>,
            uprising::reading as fn(u32) -> Option<(u8, u8)>,
        ] {
            let seen: std::collections::BTreeSet<(u8, u8)> =
                (0..uprising::PADS).filter_map(order).collect();
            assert_eq!(seen.len(), 64, "an order missed a pad");
            assert!(seen
                .iter()
                .all(|&(r, c)| (1..=8).contains(&r) && (1..=8).contains(&c)));
            assert_eq!(order(uprising::PADS), None, "an order ran past the grid");
        }
    }

    /// It loops seamlessly: the frame after the last is the first.
    #[test]
    fn the_uprising_loops() {
        let mut reel = Reel::new(Piece::Uprising);
        let first = grid(&reel.frame());
        for _ in 0..uprising::UPRISING_LOOP {
            reel.tick();
        }
        assert_eq!(grid(&reel.frame()), first, "the loop has a seam");
    }

    /// Every shape is four cells inside a 4×4 box, in all four rotations.
    /// A typo in one of the twenty-eight masks would otherwise show up as
    /// a tetromino with three cells, which looks almost right.
    #[test]
    fn every_tetromino_is_four_cells() {
        for shape in 0..tetris::SHAPES.len() {
            for rot in 0..4 {
                let cells: Vec<(i32, i32)> = tetris::shape_cells(shape, rot).collect();
                assert_eq!(cells.len(), 4, "shape {shape} rot {rot}");
                assert!(
                    cells
                        .iter()
                        .all(|&(r, c)| (0..4).contains(&r) && (0..4).contains(&c)),
                    "shape {shape} rot {rot} leaves its box"
                );
            }
        }
        assert_eq!(
            tetris::SHAPE_HUES.len(),
            tetris::SHAPES.len(),
            "a shape with no colour"
        );
    }

    /// Removing a row drops what was above it and touches nothing below.
    /// Getting this backwards is the classic Tetris bug, and it is
    /// invisible until a stack eats itself.
    #[test]
    fn removing_a_row_drops_only_what_is_above_it() {
        let mut game = Tetris::seeded(rng::SEED);
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
                stuck < tetris::TETRIS_FLASH + tetris::TETRIS_GRAVITY + 4,
                "a full row survived {stuck} frames at {n}"
            );
            assert_ne!(game.well, u64::MAX, "the well filled completely at {n}");
        }
    }

    /// Fire burns from the floor: the bottom row is always at full heat,
    /// and nothing anywhere is hotter than the floor that made it.
    #[test]
    fn fire_is_hottest_at_the_floor() {
        let mut fire = Fire::seeded(rng::SEED);
        for _ in 0..200 {
            fire.step();
        }
        for col in 0..8 {
            assert_eq!(
                fire.heat[7 * 8 + col],
                fire::FIRE_MAX,
                "floor col {col} went cold"
            );
        }
        assert!(
            fire.heat.iter().all(|&h| h <= fire::FIRE_MAX),
            "something burned hotter than the floor"
        );
    }

    /// Every column rains. A drop that respawned wrong would leave one of
    /// the eight dark forever, which reads as a dead pad — the exact
    /// failure this whole module is also a test for.
    #[test]
    fn every_column_rains() {
        let mut rain = Rain::seeded(rng::SEED);
        let mut touched = [false; 8];
        for _ in 0..600 {
            rain.step();
            for (col, seen) in touched.iter_mut().enumerate() {
                *seen |= (0..8).any(|r| rain.heat[r * 8 + col] == rain::RAIN_MAX);
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
        let arrives = uprising::beat::DRAW / FPS;
        assert!(
            arrives <= 11,
            "the smile is {arrives}s in; nobody waits that long"
        );
        let holds = (uprising::UPRISING_LOOP - uprising::beat::DRAW) / FPS;
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
            pacman::paint(n, &mut f);
            grid(&f)
        };
        assert_ne!(at(0), at(pacman::CHOMP), "the mouth did not move");
        assert_ne!(
            at(pacman::CHOMP),
            at(pacman::CHOMP * 2),
            "the mouth did not move again"
        );
        assert_eq!(
            at(0),
            at(pacman::CHOMP * 4),
            "a chomp did not come back round"
        );
    }

    /// And then the ghost takes the grid, with white eyes on a coloured
    /// body — two sprites, one over the other, which is the whole reason
    /// `draw` leaves what it does not light alone.
    #[test]
    fn the_ghost_takes_its_turn_with_eyes_on() {
        let mut f = Frame::dark();
        pacman::paint(pacman::PAC_RUN + 1, &mut f);
        assert_eq!(
            f.get(Button::Grid { row: 3, col: 3 }),
            Cell::Static(WHITE),
            "the ghost has no eyes"
        );
        assert_eq!(
            f.get(Button::Grid { row: 5, col: 1 }),
            Cell::Static(pacman::GHOST_HUES[0]),
            "the ghost has no body"
        );
        // Feet shuffle, which is what separates a ghost from a bell.
        let foot = |n: u32| {
            let mut f = Frame::dark();
            pacman::paint(n, &mut f);
            grid(&f)
        };
        assert_ne!(
            foot(pacman::PAC_RUN),
            foot(pacman::PAC_RUN + pacman::SHUFFLE),
            "the feet are stuck"
        );
    }

    /// A different ghost each time round. Four of them, and all four turn
    /// up — the detail that makes somebody look twice.
    #[test]
    fn all_four_ghosts_take_a_turn() {
        let seen: Vec<Cell> = (0..4)
            .map(|loops| {
                let mut f = Frame::dark();
                pacman::paint(loops * pacman::PACMAN_LOOP + pacman::PAC_RUN, &mut f);
                f.get(Button::Grid { row: 5, col: 1 })
            })
            .collect();
        for (i, a) in seen.iter().enumerate() {
            for b in seen.iter().skip(i + 1) {
                assert_ne!(a, b, "two ghosts are the same colour");
            }
        }
        assert_eq!(seen.len(), pacman::GHOST_HUES.len());
    }

    /// Every animal draws, every animal has eyes you can tell from its
    /// body, and every animal moves between its two poses.
    #[test]
    fn every_beast_is_drawn_has_eyes_and_breathes() {
        for (which, _) in bestiary::BEASTS.iter().enumerate() {
            let base = u32::try_from(which).unwrap_or(0) * bestiary::BEAST_HOLD;
            let pose = |n: u32| {
                let mut f = Frame::dark();
                bestiary::paint(n, &mut f);
                f
            };
            let a = pose(base);
            let b = pose(base + bestiary::BEAST_POSE);
            let lit = Button::all().filter(|&x| a.get(x) != Cell::Off).count();
            assert!((6..=48).contains(&lit), "beast {which} lights {lit} pads");
            assert_ne!(grid(&a), grid(&b), "beast {which} does not move");

            let body = bestiary::BEAST_HUES[which];
            let eyes = bestiary::eye_colour(body);
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
                bestiary::paint(which * bestiary::BEAST_HOLD, &mut f);
                grid(&f)
            })
            .collect();
        for (i, a) in seen.iter().enumerate() {
            for b in seen.iter().skip(i + 1) {
                assert_ne!(a, b, "two animals look the same");
            }
        }
        assert_eq!(seen.len(), bestiary::BEASTS.len());
        let mut first = Frame::dark();
        let mut round = Frame::dark();
        bestiary::paint(0, &mut first);
        bestiary::paint(bestiary::BESTIARY_LOOP, &mut round);
        assert_eq!(grid(&first), grid(&round), "the bestiary has a seam");
    }

    /// The mark arrives small and opens, then turns. Both halves are
    /// asserted because a bloom that skipped to the full asterisk would
    /// still look fine in a single screenshot and wrong in motion.
    #[test]
    fn the_claude_mark_blooms_then_turns() {
        let lit = |n: u32| {
            let mut f = Frame::dark();
            claude::paint(n, &mut f);
            Button::all().filter(|&b| f.get(b) != Cell::Off).count()
        };
        let shape = |n: u32| {
            let mut f = Frame::dark();
            claude::paint(n, &mut f);
            grid(&f)
        };
        assert_eq!(lit(0), 4, "the mark should arrive as a single core");
        assert!(lit(claude::BLOOM_FRAME) > lit(0), "it did not open");
        let open = claude::BLOOM_FRAME * 2;
        assert!(
            lit(open + claude::SPARK_FRAME) > lit(claude::BLOOM_FRAME),
            "it did not reach"
        );
        // Orthogonal, all eight, diagonal: three distinct spoke positions
        // is what reads as a turn.
        let (a, b, c) = (
            shape(open),
            shape(open + claude::SPARK_FRAME),
            shape(open + claude::SPARK_FRAME * 2),
        );
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c, "the turn only has two positions");
        assert_eq!(
            shape(open),
            shape(open + claude::SPARK_FRAME * 4),
            "it does not come round"
        );
    }

    /// It is Claude's mark, so it is Claude's colour, and it fills the
    /// grid rather than sitting in a corner.
    #[test]
    fn the_mark_is_amber_and_uses_the_whole_grid() {
        let mut f = Frame::dark();
        claude::paint(claude::BLOOM_FRAME * 2 + claude::SPARK_FRAME, &mut f);
        let lit: Vec<Cell> = Button::all()
            .map(|b| f.get(b))
            .filter(|&c| c != Cell::Off)
            .collect();
        assert!(lit.iter().all(|&c| c == Cell::Static(AMBER)), "not amber");
        // The asterisk's arms reach both edges; its corners are lit only
        // in the diagonal frame, which is the point of there being two.
        assert_eq!(f.get(Button::Grid { row: 4, col: 1 }), Cell::Static(AMBER));
        assert_eq!(f.get(Button::Grid { row: 4, col: 8 }), Cell::Static(AMBER));
        let mut turned = Frame::dark();
        claude::paint(
            claude::BLOOM_FRAME * 2 + claude::SPARK_FRAME * 2,
            &mut turned,
        );
        assert_eq!(
            turned.get(Button::Grid { row: 1, col: 1 }),
            Cell::Static(AMBER)
        );
        assert_eq!(
            turned.get(Button::Grid { row: 8, col: 8 }),
            Cell::Static(AMBER)
        );
    }

    /// The animals go first. Everything else in the reel is a field, a
    /// simulation or a logo; this is the one that is somebody.
    #[test]
    fn the_reel_opens_on_the_animals() {
        assert_eq!(Piece::after(None), Some(Piece::Bestiary));
        assert_eq!(Piece::REEL.first(), Some(&Piece::Bestiary));
        // And the mark is still one press away, not buried.
        assert_eq!(Piece::after(Some(Piece::Bestiary)), Some(Piece::Claude));
    }

    /// Every piece has a button, no two share one, and `►` is not one of
    /// them — it is the way out and must never also be a selector.
    #[test]
    fn every_piece_has_its_own_button_and_none_is_the_exit() {
        let mut seen = Vec::new();
        for piece in Piece::REEL {
            let button = piece.selector();
            assert_ne!(button, LEAVE_BUTTON, "{piece:?} sits on the way out");
            assert!(!seen.contains(&button), "{piece:?} shares a button");
            assert_eq!(
                Piece::at(button),
                Some(piece),
                "{piece:?} does not round trip"
            );
            seen.push(button);
        }
        assert_eq!(seen.len(), Piece::REEL.len());
    }

    /// The button you press to get in is the button that is lit when you
    /// arrive. Anything else is a mode that starts by lying about itself.
    #[test]
    fn drums_holds_the_piece_it_opens_on() {
        assert_eq!(Piece::at(ARTS_BUTTON), Piece::REEL.first().copied());
        assert_eq!(Piece::REEL.first().map(|p| p.selector()), Some(ARTS_BUTTON));
    }

    /// The other eight live down the right column, in reel order, and
    /// nowhere else. A selector on the top row would be a button whose
    /// normal job is navigation.
    #[test]
    fn the_menu_is_the_right_column() {
        for (n, piece) in Piece::REEL.iter().skip(1).enumerate() {
            let row = u8::try_from(n + 1).unwrap_or(1);
            assert_eq!(piece.selector(), Button::Action(row), "{piece:?}");
        }
    }
}
