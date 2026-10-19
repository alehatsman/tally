use super::colour::{paint_heat, EMBER};
use super::rng::Rng;
use crate::surface::Frame;

/// Full heat, and the length of the wake: a dead pad is dark five
/// generations later. Six, which is also [`EMBER`]'s length, so a
/// generation of cooling is exactly one step down the ramp.
pub(super) const HEAT_MAX: u8 = 6;

/// How many art frames one generation lasts. Six a second: fast enough to
/// read as motion, slow enough to follow a glider.
pub(super) const LIFE_EVERY: u32 = 5;
/// How many generations one soup gets before it is replaced regardless.
///
/// The cycle detector below catches a board that *stops*. It cannot catch
/// one that is merely finished — a lone glider on an 8×8 torus returns to
/// where it started every 32 generations and will cruise there until the
/// cable comes out, which is pretty for twenty seconds and wallpaper after
/// a minute. Three hundred generations is about fifty seconds.
pub(super) const LIFE_SPAN: u32 = 300;
/// Chance a cell is alive in a fresh soup, in percent. Forty is the band
/// that reliably produces gliders rather than a dense blob that settles.
pub(super) const DENSITY: u32 = 40;

/// Conway on an 8×8 torus, drawn as heat.
///
/// The torus is what keeps a board this small interesting — a glider that
/// would walk off a 64-pad edge in eight generations instead comes back.
#[derive(Debug, Clone)]
pub(super) struct Life {
    /// One bit per pad, `row * 8 + col`, both 0-based.
    pub(super) cells: u64,
    /// How recently each pad was alive. Full while it is, counting down
    /// after it dies — the wake, which is the half worth watching.
    pub(super) heat: [u8; 64],
    /// The last four generations, so a still life or a two-cycle is caught
    /// the generation it appears. A frozen grid is indistinguishable from
    /// a crashed daemon, which is the most expensive thing this surface
    /// can imply.
    pub(super) recent: [u64; 4],
    /// Generations since the last soup, against [`LIFE_SPAN`].
    pub(super) age: u32,
    pub(super) rng: Rng,
}

impl Life {
    pub(super) fn seeded(seed: u32) -> Self {
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
    pub(super) fn step(&mut self) {
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

    /// The wake, through the same ramp [`Rain`] and [`Fire`] use. It had
    /// its own copy of the walk and the arithmetic until the two were
    /// shown to agree on all seven heats — see `life_and_paint_heat_agree`.
    pub(super) fn paint(&self, f: &mut Frame) {
        paint_heat(f, &self.heat, HEAT_MAX, &EMBER);
    }
}
