//! Packs the point and spot lights' shadow views into one depth atlas.
//!
//! Every view is a square tile whose side is a power of two, from
//! [`MIN_TILE`] up to the atlas's side. Placed largest first along a
//! Z-order (Morton) curve over [`MIN_TILE`]² cells, such tiles leave no
//! gaps: each starts where the curve has covered a whole aligned square
//! of its size, so a set fits exactly when its area does. That makes
//! fitting simple ([`fit`]: shrink tiles until the area fits) and packing
//! one pass ([`pack`]).

/// The smallest tile a light shadow view gets, in texels.
pub(crate) const MIN_TILE: u32 = 64;
/// The smallest and largest light shadow atlas, in texels a side.
pub(crate) const MIN_LIGHT_SHADOW_ATLAS: u32 = 1024;
pub(crate) const MAX_LIGHT_SHADOW_ATLAS: u32 = 8192;

/// A view's place in the atlas: x, y (from the top left) and side, in texels.
pub(crate) type Tile = [u32; 3];

/// The largest power of two at most `v` (1 for 0).
pub(crate) fn floor_pow2(v: u32) -> u32 {
    1 << (31 - v.max(1).leading_zeros())
}

/// The power of two nearest `v` (by ratio), at least 1.
pub(crate) fn round_pow2(v: f32) -> u32 {
    if v.is_nan() || v <= 1.0 {
        return 1;
    }
    1 << (v.log2().round() as u32).min(31)
}

/// Shrinks the lights' tiles until they all fit an `atlas`² atlas.
/// `lights` holds (tile side, views) per light, most important first; sides
/// are made powers of two from [`MIN_TILE`] to `atlas` first. While the
/// area is too big, the largest tile is halved (the least important one
/// among equals); once every tile is [`MIN_TILE`], the least important
/// light is dropped. Returns how many lights (from the front) are kept.
pub(crate) fn fit(lights: &mut [(u32, u32)], atlas: u32) -> usize {
    let atlas = floor_pow2(atlas.max(MIN_TILE));
    for light in lights.iter_mut() {
        light.0 = floor_pow2(light.0.clamp(MIN_TILE, atlas));
    }
    let area = |lights: &[(u32, u32)]| -> u64 {
        lights
            .iter()
            .map(|&(side, views)| u64::from(side) * u64::from(side) * u64::from(views))
            .sum()
    };
    let capacity = u64::from(atlas) * u64::from(atlas);
    let mut kept = lights.len();
    while kept > 0 && area(&lights[..kept]) > capacity {
        let (largest, side) = lights[..kept]
            .iter()
            .enumerate()
            .map(|(i, l)| (i, l.0))
            .max_by_key(|&(i, side)| (side, i))
            .unwrap_or((0, 0));
        if side > MIN_TILE {
            lights[largest].0 = side / 2;
        } else {
            kept -= 1;
        }
    }
    kept
}

/// Places a tile of each side in `sides` (powers of two from [`MIN_TILE`]
/// to `atlas`, together no bigger than the atlas: see [`fit`]) and writes
/// them to `tiles`, in the same order. `order` is scratch space.
pub(crate) fn pack(sides: &[u32], atlas: u32, order: &mut Vec<u32>, tiles: &mut Vec<Tile>) {
    order.clear();
    order.extend(0..sides.len() as u32);
    order.sort_unstable_by_key(|&i| (std::cmp::Reverse(sides[i as usize]), i));
    tiles.clear();
    tiles.resize(sides.len(), [0; 3]);
    // The next free cell along the curve, in MIN_TILE² cells.
    let mut cell = 0u32;
    for &i in order.iter() {
        let side = sides[i as usize];
        let (x, y) = demorton(cell);
        tiles[i as usize] = [x * MIN_TILE, y * MIN_TILE, side];
        cell += (side / MIN_TILE) * (side / MIN_TILE);
    }
    debug_assert!(u64::from(cell) * u64::from(MIN_TILE * MIN_TILE) <= u64::from(atlas) * u64::from(atlas));
}

/// The cell at Morton index `v`: its even bits are x, its odd bits y.
fn demorton(v: u32) -> (u32, u32) {
    let compact = |mut v: u32| {
        v &= 0x5555_5555;
        v = (v | (v >> 1)) & 0x3333_3333;
        v = (v | (v >> 2)) & 0x0F0F_0F0F;
        v = (v | (v >> 4)) & 0x00FF_00FF;
        (v | (v >> 8)) & 0x0000_FFFF
    };
    (compact(v), compact(v >> 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overlaps(a: Tile, b: Tile) -> bool {
        a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[2] && b[1] < a[1] + a[2]
    }

    /// Every tile is inside the atlas, aligned to its own size, has the
    /// side asked for and overlaps no other.
    fn check(sides: &[u32], atlas: u32, tiles: &[Tile]) {
        assert_eq!(sides.len(), tiles.len());
        for (i, (&side, &t)) in sides.iter().zip(tiles).enumerate() {
            assert_eq!(t[2], side, "tile {i} keeps its side");
            assert!(t[0] + side <= atlas && t[1] + side <= atlas, "tile {i} {t:?} is inside");
            assert!(t[0] % side == 0 && t[1] % side == 0, "tile {i} {t:?} is aligned");
            for (j, &u) in tiles.iter().enumerate().skip(i + 1) {
                assert!(!overlaps(t, u), "tiles {i} {t:?} and {j} {u:?} overlap");
            }
        }
    }

    #[test]
    fn powers_of_two_round_as_documented() {
        assert_eq!(
            [0, 1, 2, 3, 1000, 1024, 1025].map(floor_pow2),
            [1, 1, 2, 2, 512, 1024, 1024]
        );
        assert_eq!(
            [0.0, 0.5, 2.9, 3.0, 700.0, 760.0, f32::NAN].map(round_pow2),
            [1, 1, 4, 4, 512, 1024, 1]
        );
    }

    #[test]
    fn a_full_atlas_of_mixed_tiles_packs_without_gaps() {
        // 2 x 2048² + 7 x 1024² + 15 x 256² + 16 x 64² = 4096² exactly.
        let mut sides = vec![256; 15];
        sides.extend([64; 16]);
        sides.extend([1024, 2048, 1024, 1024, 2048, 1024, 1024, 1024, 1024]);
        let (mut order, mut tiles) = (Vec::new(), Vec::new());
        pack(&sides, 4096, &mut order, &mut tiles);
        check(&sides, 4096, &tiles);
        let area: u32 = sides.iter().map(|s| s * s).sum();
        assert_eq!(area, 4096 * 4096);
        // The largest goes first, into the corner.
        assert_eq!(tiles[32], [0, 0, 2048]);
    }

    #[test]
    fn many_small_tiles_fit_beside_a_few_large_ones() {
        // Two near point lights at 1024 and twenty far spot lights at 128.
        let mut sides = vec![1024; 12];
        sides.extend([128; 20]);
        let (mut order, mut tiles) = (Vec::new(), Vec::new());
        pack(&sides, 4096, &mut order, &mut tiles);
        check(&sides, 4096, &tiles);
    }

    #[test]
    fn tiles_that_fit_keep_their_sizes() {
        let mut lights = [(1024, 6), (512, 1), (100, 1), (3000, 1)];
        assert_eq!(fit(&mut lights, 4096), 4);
        // Rounded down to powers of two, at least the smallest tile and at
        // most the atlas.
        assert_eq!(lights, [(1024, 6), (512, 1), (64, 1), (2048, 1)]);
        let mut small = [(10, 1)];
        assert_eq!(fit(&mut small, 1024), 1);
        assert_eq!(small, [(MIN_TILE, 1)]);
    }

    #[test]
    fn overflowing_tiles_shrink_largest_first_then_lights_drop() {
        // 3 point lights at 1024: 18 tiles of 1024² don't fit 16.
        let mut lights = [(1024, 6), (1024, 6), (1024, 6)];
        assert_eq!(fit(&mut lights, 4096), 3);
        // The least important light gives way first.
        assert_eq!(lights, [(1024, 6), (1024, 6), (512, 6)]);
        // A large hinted tile is halved before smaller ones.
        let mut lights = [(512, 1), (2048, 1), (2048, 1), (2048, 1), (2048, 1), (512, 1)];
        assert_eq!(fit(&mut lights, 4096), 6);
        assert_eq!(lights, [(512, 1), (2048, 1), (2048, 1), (2048, 1), (1024, 1), (512, 1)]);
        // When even the smallest tiles don't fit, the last lights go.
        let mut lights = [(64, 6); 50];
        let kept = fit(&mut lights, 1024);
        assert_eq!(kept, 42, "1024² holds 256 tiles of 64²: 42 point lights");
        assert!(lights.iter().all(|l| l.0 == MIN_TILE));
    }

    #[test]
    fn whatever_fit_keeps_always_packs() {
        // A spread of sets: whatever `fit` keeps packs without overlap.
        let mut seed = 0x2545_F491u32;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        let (mut order, mut tiles, mut sides) = (Vec::new(), Vec::new(), Vec::new());
        for round in 0..200 {
            let atlas = [1024, 2048, 4096, 8192][round % 4];
            let count = 1 + next() as usize % 24;
            let mut lights: Vec<(u32, u32)> = (0..count)
                .map(|_| (16 << (next() % 9), if next() % 3 == 0 { 6 } else { 1 }))
                .collect();
            let kept = fit(&mut lights, atlas);
            assert!(kept > 0);
            sides.clear();
            for &(side, views) in &lights[..kept] {
                sides.extend(std::iter::repeat_n(side, views as usize));
            }
            pack(&sides, atlas, &mut order, &mut tiles);
            check(&sides, atlas, &tiles);
        }
    }
}
