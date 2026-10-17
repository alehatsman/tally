use super::sprite::draw;
use super::sprite::Sprite;
use crate::surface::palette::{Entry, AMBER, DARK_GREY, WHITE};
use crate::surface::Frame;

/// Frames per pose. Two poses an animal, so this is also how long one
/// breath lasts.
pub(super) const BEAST_POSE: u32 = 15;
/// How long each animal keeps the grid: six poses, about three seconds.
pub(super) const BEAST_HOLD: u32 = BEAST_POSE * 6;

/// Cat, dog, fish, bird, rabbit — two poses each, because a sprite that
/// does not move is a logo, and the grid already has one of those.
pub(super) const BEASTS: [[Sprite; 2]; 5] = [
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
pub(super) const BEAST_EYES: [Sprite; 5] = [
    [0, 0, 0, 0b0001_0010, 0, 0, 0, 0], // cat
    [0, 0b0100_0000, 0, 0, 0, 0, 0, 0], // dog
    [0, 0, 0b0001_0000, 0, 0, 0, 0, 0], // fish
    [0, 0, 0b0010_0100, 0, 0, 0, 0, 0], // bird
    [0, 0, 0, 0b0010_0100, 0, 0, 0, 0], // rabbit
];

/// One hue each. A white rabbit, because the palette has exactly one
/// white and this is the only piece with somewhere to spend it.
pub(super) const BEAST_HUES: [Entry; 5] = [AMBER, 45, 37, 57, WHITE];

/// Eyes go on in the body's colour when the body is already white —
/// otherwise the rabbit has no face at all.
pub(super) fn eye_colour(body: Entry) -> Entry {
    if body == WHITE {
        DARK_GREY
    } else {
        WHITE
    }
}

pub(super) const BESTIARY_LOOP: u32 = BEAST_HOLD * 5;

pub(super) fn paint(n: u32, frame: &mut Frame) {
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
