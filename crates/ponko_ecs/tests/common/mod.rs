//! Components shared by the integration tests.
#![allow(dead_code)]

use ponko_ecs::Component;

#[derive(Debug, PartialEq, Clone, Copy, Component)]
pub struct Pos(pub f32, pub f32);
#[derive(Debug, PartialEq, Clone, Copy, Component)]
pub struct Vel(pub f32, pub f32);
#[derive(Debug, PartialEq, Component)]
pub struct Tag;
#[derive(Component)]
pub struct Frozen;

/// A tiny seeded PRNG (SplitMix64) for generated test cases: deterministic on
/// every platform, so a failing seed reproduces anywhere. Not for anything
/// but tests.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform-enough integer in `0..n` (`n > 0`); the modulo bias is
    /// irrelevant for choosing test operations.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    /// `true` with probability about `1 / n`.
    pub fn one_in(&mut self, n: usize) -> bool {
        self.below(n) == 0
    }
}
