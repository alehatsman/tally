/// Where every board starts.
///
/// A constant, so each piece opens the same way every time and a test can
/// say what its first generation is. It lived in `life` until the pieces
/// were given files of their own, which is when it became visible that
/// all four stateful boards seed from it and only one of them was named
/// after it (`docs/08-daemon.md`).
pub(super) const SEED: u32 = 0x7a11;

/// Xorshift. Not cryptography and not uniform to a statistician — it
/// decides which pads light up.
#[derive(Debug, Clone)]
pub(super) struct Rng(pub(super) u32);

impl Rng {
    pub(super) fn bits(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    /// `0..n`. The modulo bias at these sizes is a rounding error on a
    /// light show.
    pub(super) fn below(&mut self, n: u32) -> u32 {
        self.bits() % n.max(1)
    }
}
