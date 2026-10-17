use super::colour::paint_heat;
use super::rng::Rng;
use crate::surface::palette::{Entry, DARK_GREEN, DARK_GREY, GREEN, WHITE};
use crate::surface::Frame;

/// How fast a drop falls, in sixteenths of a pad per frame. A range, not
/// a constant: eight columns at one speed is a curtain, and what reads as
/// rain is eight columns disagreeing.
pub(super) const RAIN_SLOWEST: u32 = 2;
/// The fast end of the same range.
pub(super) const RAIN_FASTEST: u32 = 7;
/// Full heat under a drop, and so the length of the tail behind it.
pub(super) const RAIN_MAX: u8 = 20;
/// Head to tail: a white head on a green body, going out.
pub(super) const RAIN_RAMP: [Entry; 5] = [WHITE, GREEN, GREEN, DARK_GREEN, DARK_GREY];

/// Eight independent drops, one per column, each leaving a cooling tail.
///
/// The tail is the same trick [`Life`] uses — heat that outlives the thing
/// that made it — and it is why this reads as rain rather than as eight
/// pads taking turns.
#[derive(Debug, Clone)]
pub(super) struct Rain {
    pub(super) heat: [u8; 64],
    /// Each drop's position, in sixteenths of a pad from the top row.
    /// **Negative is above the grid**, which is where a drop waits out
    /// its gap before falling again.
    pub(super) head: [i32; 8],
    pub(super) speed: [i32; 8],
    pub(super) rng: Rng,
}

impl Rain {
    pub(super) fn seeded(seed: u32) -> Self {
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
    pub(super) fn step(&mut self) {
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

    pub(super) fn paint(&self, frame: &mut Frame) {
        paint_heat(frame, &self.heat, RAIN_MAX, &RAIN_RAMP);
    }
}
