use super::sprite::draw;
use super::sprite::Sprite;
use crate::surface::palette::{Entry, AMBER, RED, WHITE, YELLOW};
use crate::surface::Frame;

/// Frames per mouth position. Four positions, so a chomp is twenty
/// frames — about a chomp and a half a second, which is the speed the
/// arcade ran at and the speed it has to be to read as one.
pub(super) const CHOMP: u32 = 5;
/// How long Pac-Man has the grid before the ghost takes it.
pub(super) const PAC_RUN: u32 = 160;
/// And how long the ghost keeps it.
pub(super) const GHOST_RUN: u32 = 96;
/// Frames per foot position. The ghost's shuffle is the whole reason it
/// reads as a ghost and not as a bell.
pub(super) const SHUFFLE: u32 = 8;
pub(super) const PACMAN_LOOP: u32 = PAC_RUN + GHOST_RUN;

/// The chomp, as four frames. Open, half, shut, half — the mouth goes
/// back the way it came, which is what a chomp is.
pub(super) const PAC: [Sprite; 4] = [
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
pub(super) const GHOST: [Sprite; 2] = [
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
pub(super) const GHOST_EYES: Sprite = [
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
pub(super) const GHOST_HUES: [Entry; 4] = [RED, 57, 37, AMBER];

pub(super) fn paint(n: u32, frame: &mut Frame) {
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
