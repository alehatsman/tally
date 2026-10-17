use super::colour::paint_heat;
use super::rng::Rng;
use crate::surface::palette::{Entry, AMBER, DARK_GREY, DARK_RED, RED, WHITE, YELLOW};
use crate::surface::Frame;

/// How many art frames one fire step lasts. Every other one: at full rate
/// the flames flicker faster than the eye resolves and read as noise.
pub(super) const FIRE_EVERY: u32 = 2;
/// The heat of the floor. Seven rows above it and up to two lost per row,
/// so a flame reaches the top only when the dice are with it — which is
/// what makes it look like fire rather than like a gradient.
pub(super) const FIRE_MAX: u8 = 11;
/// Hottest first.
pub(super) const FIRE_RAMP: [Entry; 6] = [WHITE, YELLOW, AMBER, RED, DARK_RED, DARK_GREY];

/// Doom's fire, eight pads wide.
///
/// The whole algorithm is one line — *each cell is the one below it, minus
/// a little, blown sideways a little* — and it has been the best
/// looking-to-code ratio in graphics since 1993.
#[derive(Debug, Clone)]
pub(super) struct Fire {
    pub(super) heat: [u8; 64],
    pub(super) rng: Rng,
}

impl Fire {
    pub(super) fn seeded(seed: u32) -> Self {
        Self {
            heat: [0; 64],
            rng: Rng(seed),
        }
    }

    /// Into a fresh board rather than in place: propagating through a
    /// buffer you are reading would carry the floor to the ceiling inside
    /// one frame, and the flame would have no shape.
    pub(super) fn step(&mut self) {
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

    pub(super) fn paint(&self, frame: &mut Frame) {
        paint_heat(frame, &self.heat, FIRE_MAX, &FIRE_RAMP);
    }
}
