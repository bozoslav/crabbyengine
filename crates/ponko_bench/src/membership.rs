//! Archetype component membership and structural-transition costs (issue #144).
//!
//! Everything here goes through ponko_ecs's public API, so the group needs no
//! change to the engine. Group name: `membership/`; run it with
//! `cargo run --release -p ponko_bench -- membership`.
//!
//! What is measured, for archetypes of 2, 8, 16, 32 and 64 components:
//!
//! - `membership/has/*`: `Archetype::has` (a linear scan of the sorted id
//!   list) against candidate layouts built from the same ids: binary search,
//!   a bitset, and a flat id→index table. Hits and misses, with the
//!   archetype's ids packed at the bottom of the id space ("dense") or
//!   scattered over 256 ids ("sparse"). Misses compare ids that are not in the
//!   set, which scans the whole list.
//! - `membership/index/*`: the same for a component's column index
//!   (`column_index` in storage.rs): a linear `position`, the public
//!   `column_ptr` wrapper, a bitset with popcount rank, the flat table and
//!   binary search.
//! - `membership/colmap/*`: the column mapping `move_row` computes for every
//!   structural change (for each column of the source, where is it in the
//!   destination?). The engine does `n` linear scans; both id lists are
//!   sorted, so a single merge pass is the obvious alternative.
//! - `membership/transition/*`: real `insert_dynamic`/`remove_dynamic` pairs
//!   on a world: `warm` (edge cached, one component toggled), `cycle32` (32
//!   different components toggled in turn, so 64 edges are in use) and `cold`
//!   (fresh world: the first add and remove, which resolve the edge and
//!   create the archetype).
//! - `membership/edge-lookup/*`: the transition-edge cache lookup itself
//!   (`HashMap<ComponentId, usize>` with the default hasher, as in storage.rs)
//!   against an identity-hash map and a short linear list, for 1 to 64 edges.
//!
//! Candidates are interleaved round by round, so slow drift on a busy machine
//! affects all of them alike. Each line shows nanoseconds per operation.
//! Lookups are independent of each other, so they measure throughput, not
//! latency.

use ponko_ecs::{Archetype, ComponentId, Entity, World};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::hint::black_box;
use std::time::Instant;

/// Rust component that every benchmark entity starts with (id 0).
struct Seed(#[allow(dead_code)] u32);

/// Components registered in the world: ids `0..REGISTERED`.
const REGISTERED: usize = 320;
/// Archetypes draw their components from `0..SPREAD`; ids `SPREAD..REGISTERED`
/// are never part of an archetype's set (misses, and the components that
/// transitions add and remove).
const SPREAD: usize = 256;
const WIDTHS: [usize; 5] = [2, 8, 16, 32, 64];
const LOOKUPS: usize = 16_384;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Layout {
    /// The archetype's ids are `0..n`.
    Dense,
    /// `0` and `n - 1` pseudo-random ids below [`SPREAD`].
    Sparse,
}

impl Layout {
    fn name(self) -> &'static str {
        match self {
            Layout::Dense => "dense",
            Layout::Sparse => "sparse",
        }
    }
}

// ---- deterministic helpers --------------------------------------------------

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = (self.next() % (i as u64 + 1)) as usize;
            v.swap(i, j);
        }
    }
}

/// The sorted component indexes of an `n`-component archetype.
fn pick(layout: Layout, n: usize) -> Vec<usize> {
    match layout {
        Layout::Dense => (0..n).collect(),
        Layout::Sparse => {
            let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ n as u64);
            let mut pool: Vec<usize> = (1..SPREAD).collect();
            rng.shuffle(&mut pool);
            let mut set: Vec<usize> = std::iter::once(0).chain(pool.into_iter().take(n - 1)).collect();
            set.sort_unstable();
            set
        }
    }
}

/// A world with `REGISTERED` components; `ids[i].index() == i`.
fn registered_world() -> (World, Vec<ComponentId>) {
    let mut w = World::new();
    let mut ids = vec![w.register::<Seed>()];
    for i in 1..REGISTERED {
        // SAFETY: plain 4-byte values, no destructor.
        ids.push(unsafe { w.register_component(&format!("c{i}"), 4, 4, None) });
    }
    assert!(ids.iter().enumerate().all(|(i, id)| id.index() == i));
    (w, ids)
}

fn add(w: &mut World, e: Entity, id: ComponentId) {
    let v = 0u32;
    // SAFETY: `id` is a registered 4-byte component and `v` is 4 bytes.
    assert!(unsafe { w.raw().insert_dynamic(e, id, (&raw const v).cast()) });
}

/// A world with one entity whose archetype has exactly the components `set`
/// (sorted indexes, starting with 0). Ascending inserts: the intermediate
/// archetypes are the prefixes of `set`.
fn world_with(set: &[usize]) -> (World, Vec<ComponentId>, Entity) {
    let (mut w, ids) = registered_world();
    let e = w.spawn((Seed(0),));
    for &i in &set[1..] {
        add(&mut w, e, ids[i]);
    }
    (w, ids, e)
}

fn archetype_of<'a>(w: &'a World, set: &[ComponentId]) -> &'a Archetype {
    w.raw_ref()
        .archetypes()
        .iter()
        .find(|a| a.component_ids() == set)
        .expect("archetype exists")
}

// ---- candidate layouts ------------------------------------------------------

/// One bit per component id; the column index of a present id is the number of
/// set bits below it (per-word prefix counts plus a popcount in the word).
struct Bits {
    words: Box<[u64]>,
    prefix: Box<[u16]>,
}

impl Bits {
    fn new(ids: &[ComponentId]) -> Self {
        let mut words = vec![0u64; REGISTERED.div_ceil(64)];
        for id in ids {
            words[id.index() >> 6] |= 1 << (id.index() & 63);
        }
        let mut prefix = Vec::with_capacity(words.len());
        let mut sum = 0u16;
        for w in &words {
            prefix.push(sum);
            sum += w.count_ones() as u16;
        }
        Bits {
            words: words.into(),
            prefix: prefix.into(),
        }
    }

    #[inline]
    fn has(&self, id: ComponentId) -> bool {
        let i = id.index();
        self.words.get(i >> 6).is_some_and(|w| w >> (i & 63) & 1 != 0)
    }

    #[inline]
    fn index(&self, id: ComponentId) -> Option<usize> {
        let i = id.index();
        let w = *self.words.get(i >> 6)?;
        let bit = 1u64 << (i & 63);
        (w & bit != 0).then(|| self.prefix[i >> 6] as usize + (w & (bit - 1)).count_ones() as usize)
    }

    fn bytes(&self) -> usize {
        self.words.len() * 8 + self.prefix.len() * 2
    }
}

/// `table[id]` is the column index, or `u16::MAX`.
struct Table(Box<[u16]>);

impl Table {
    fn new(ids: &[ComponentId]) -> Self {
        let mut t = vec![u16::MAX; REGISTERED];
        for (col, id) in ids.iter().enumerate() {
            t[id.index()] = col as u16;
        }
        Table(t.into())
    }

    #[inline]
    fn index(&self, id: ComponentId) -> Option<usize> {
        match self.0.get(id.index()) {
            Some(&c) if c != u16::MAX => Some(c as usize),
            _ => None,
        }
    }

    fn bytes(&self) -> usize {
        self.0.len() * 2
    }
}

#[inline]
fn linear_index(ids: &[ComponentId], id: ComponentId) -> Option<usize> {
    ids.iter().position(|&c| c == id)
}

/// Column mapping with one merge pass over two sorted id lists. Returns the
/// sum of the matched destination indexes (so the work cannot be optimized away).
fn merge_map(src: &[ComponentId], dst: &[ComponentId]) -> usize {
    let (mut j, mut sum) = (0, 0);
    for &s in src {
        while j < dst.len() && dst[j] < s {
            j += 1;
        }
        if j < dst.len() && dst[j] == s {
            sum += j;
        }
    }
    sum
}

fn linear_map(src: &[ComponentId], dst: &[ComponentId]) -> usize {
    src.iter().filter_map(|&s| linear_index(dst, s)).sum()
}

fn bits_map(src: &[ComponentId], dst: &Bits) -> usize {
    src.iter().filter_map(|&s| dst.index(s)).sum()
}

fn table_map(src: &[ComponentId], dst: &Table) -> usize {
    src.iter().filter_map(|&s| dst.index(s)).sum()
}

// ---- timing harness ---------------------------------------------------------

type Case<'a> = (&'static str, Box<dyn FnMut() -> usize + 'a>);

fn skips(filter: Option<&str>, name: &str) -> bool {
    filter.is_some_and(|f| !name.contains(f))
}

/// Times every case once per round, `rounds` rounds, so the cases share the
/// same noise. Prints `ns per op` (median and minimum over the rounds).
fn compare(filter: Option<&str>, group: &str, per_op: usize, rounds: usize, mut cases: Vec<Case<'_>>) {
    if cases
        .iter()
        .all(|(name, _)| skips(filter, &format!("membership/{group}/{name}")))
    {
        return;
    }
    let mut samples: Vec<Vec<f64>> = vec![Vec::with_capacity(rounds); cases.len()];
    for round in 0..rounds + 3 {
        for (i, (_, f)) in cases.iter_mut().enumerate() {
            let t = Instant::now();
            black_box(f());
            let ns = t.elapsed().as_secs_f64() * 1e9 / per_op as f64;
            if round >= 3 {
                samples[i].push(ns);
            }
        }
    }
    for ((name, _), s) in cases.iter().zip(samples.iter_mut()) {
        let full = format!("membership/{group}/{name}");
        if skips(filter, &full) {
            continue;
        }
        s.sort_by(f64::total_cmp);
        println!("{full:<48} median {:>9.2} ns   min {:>9.2} ns", s[s.len() / 2], s[0]);
    }
}

// ---- the benchmarks ---------------------------------------------------------

/// `LOOKUPS` ids from `pool`, in shuffled order, repeating `pool` as needed.
fn queries(pool: &[ComponentId], seed: u64) -> Vec<ComponentId> {
    let mut rng = Rng(seed);
    let mut q: Vec<ComponentId> = pool.iter().copied().cycle().take(LOOKUPS).collect();
    rng.shuffle(&mut q);
    q
}

fn lookups(filter: Option<&str>) {
    for layout in [Layout::Dense, Layout::Sparse] {
        for n in WIDTHS {
            let tag = format!("n{n}-{}", layout.name());
            let set = pick(layout, n);
            let (w, ids, _) = world_with(&set);
            let mine: Vec<ComponentId> = set.iter().map(|&i| ids[i]).collect();
            let arch = archetype_of(&w, &mine);
            let missing: Vec<ComponentId> = (0..REGISTERED).filter(|i| !set.contains(i)).map(|i| ids[i]).collect();
            let hit = queries(&mine, 1);
            let miss = queries(&missing, 2);
            let bits = Bits::new(&mine);
            let table = Table::new(&mine);

            for (kind, q) in [("hit", &hit), ("miss", &miss)] {
                let (arch, bits, mine) = (arch, &bits, &mine);
                let table = &table;
                compare(
                    filter,
                    "has",
                    LOOKUPS,
                    300,
                    vec![
                        (
                            leak(format!("{tag}/{kind}/linear(Archetype::has)")),
                            Box::new(move || q.iter().filter(|&&id| arch.has(id)).count()),
                        ),
                        (
                            leak(format!("{tag}/{kind}/binary-search")),
                            Box::new(move || q.iter().filter(|&&id| mine.binary_search(&id).is_ok()).count()),
                        ),
                        (
                            leak(format!("{tag}/{kind}/bitset")),
                            Box::new(move || q.iter().filter(|&&id| bits.has(id)).count()),
                        ),
                        (
                            leak(format!("{tag}/{kind}/table")),
                            Box::new(move || q.iter().filter(|&&id| table.index(id).is_some()).count()),
                        ),
                    ],
                );
                compare(
                    filter,
                    "index",
                    LOOKUPS,
                    300,
                    vec![
                        (
                            leak(format!("{tag}/{kind}/linear(position)")),
                            Box::new(move || q.iter().filter_map(|&id| linear_index(mine, id)).sum()),
                        ),
                        (
                            leak(format!("{tag}/{kind}/Archetype::column_ptr")),
                            Box::new(move || q.iter().filter_map(|&id| arch.column_ptr(id)).count()),
                        ),
                        (
                            leak(format!("{tag}/{kind}/bitset-rank")),
                            Box::new(move || q.iter().filter_map(|&id| bits.index(id)).sum()),
                        ),
                        (
                            leak(format!("{tag}/{kind}/table")),
                            Box::new(move || q.iter().filter_map(|&id| table.index(id)).sum()),
                        ),
                        (
                            leak(format!("{tag}/{kind}/binary-search")),
                            Box::new(move || q.iter().filter_map(|&id| mine.binary_search(&id).ok()).sum()),
                        ),
                    ],
                );
            }
        }
    }
}

/// Case names must outlive the closures; the benchmark process is short-lived.
fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// The column mapping of an add (`n` → `n + 1` components) and of a remove.
fn colmap(filter: Option<&str>) {
    for layout in [Layout::Dense, Layout::Sparse] {
        for n in WIDTHS {
            let tag = format!("n{n}-{}", layout.name());
            let set = pick(layout, n);
            let (_, ids) = registered_world();
            let small: Vec<ComponentId> = set.iter().map(|&i| ids[i]).collect();
            // One extra component from the never-in-a-set range.
            let mut big = small.clone();
            big.push(ids[SPREAD + 7]);
            big.sort_unstable();
            for (dir, src, dst) in [("add", &small, &big), ("remove", &big, &small)] {
                let bits = Bits::new(dst);
                let table = Table::new(dst);
                let (bits, table) = (&bits, &table);
                compare(
                    filter,
                    "colmap",
                    COLMAP_REPS,
                    1000,
                    vec![
                        (
                            leak(format!("{tag}/{dir}/linear(engine)")),
                            Box::new(move || {
                                let mut s = 0;
                                for _ in 0..COLMAP_REPS {
                                    s += linear_map(black_box(src), black_box(dst));
                                }
                                s
                            }),
                        ),
                        (
                            leak(format!("{tag}/{dir}/merge")),
                            Box::new(move || {
                                let mut s = 0;
                                for _ in 0..COLMAP_REPS {
                                    s += merge_map(black_box(src), black_box(dst));
                                }
                                s
                            }),
                        ),
                        (
                            leak(format!("{tag}/{dir}/bitset-rank")),
                            Box::new(move || {
                                let mut s = 0;
                                for _ in 0..COLMAP_REPS {
                                    s += bits_map(black_box(src), black_box(bits));
                                }
                                s
                            }),
                        ),
                        (
                            leak(format!("{tag}/{dir}/table")),
                            Box::new(move || {
                                let mut s = 0;
                                for _ in 0..COLMAP_REPS {
                                    s += table_map(black_box(src), black_box(table));
                                }
                                s
                            }),
                        ),
                    ],
                );
            }
        }
    }
}

const TOGGLES: usize = 512;
/// Column mappings per timed round (one mapping is only a few ns for small archetypes).
const COLMAP_REPS: usize = 256;

/// Real structural changes on a world, per transition (one insert or one remove).
fn transitions(filter: Option<&str>) {
    for n in WIDTHS {
        let set = pick(Layout::Sparse, n);
        let name = format!("n{n}-sparse");
        if skips(filter, &format!("membership/transition/{name}")) {
            continue;
        }
        let (mut w, ids, e) = world_with(&set);
        let extra0 = ids[SPREAD];
        let extras: Vec<ComponentId> = ids[SPREAD..SPREAD + 32].to_vec();
        let archetypes_before = {
            // Visit every edge once, so each case below starts with the edges cached.
            for &x in &extras {
                add(&mut w, e, x);
                assert!(w.raw().remove_dynamic(e, x));
            }
            w.raw_ref().archetypes().len()
        };
        let w = std::cell::RefCell::new(w);
        compare(
            filter,
            "transition",
            2 * TOGGLES,
            300,
            vec![
                (
                    leak(format!("{name}/warm-1-edge")),
                    Box::new(|| {
                        let mut w = w.borrow_mut();
                        for _ in 0..TOGGLES {
                            add(&mut w, e, extra0);
                            w.raw().remove_dynamic(e, extra0);
                        }
                        0
                    }),
                ),
                (
                    leak(format!("{name}/cycle32-64-edges")),
                    Box::new(|| {
                        let mut w = w.borrow_mut();
                        for k in 0..TOGGLES {
                            let x = extras[k % extras.len()];
                            add(&mut w, e, x);
                            w.raw().remove_dynamic(e, x);
                        }
                        0
                    }),
                ),
            ],
        );
        // The edge cache never missed after the first visit: no archetype was created.
        assert_eq!(w.borrow().raw_ref().archetypes().len(), archetypes_before);

        // Cold: a fresh world, timing the first add and remove of one component.
        let full = format!("membership/transition/{name}/cold-first-add+remove");
        if !skips(filter, &full) {
            let mut times = Vec::new();
            for round in 0..203 {
                let (mut w, ids, e) = world_with(&set);
                let t = Instant::now();
                add(&mut w, e, ids[SPREAD]);
                w.raw().remove_dynamic(e, ids[SPREAD]);
                let ns = t.elapsed().as_secs_f64() * 1e9 / 2.0;
                black_box(&w);
                if round >= 3 {
                    times.push(ns);
                }
            }
            times.sort_by(f64::total_cmp);
            println!(
                "{full:<48} median {:>9.2} ns   min {:>9.2} ns",
                times[times.len() / 2],
                times[0]
            );
        }
    }
}

/// Multiplicative hash of the id, as an identity-like hasher.
#[derive(Default)]
struct IdHasher(u64);

impl Hasher for IdHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 << 8) | u64::from(b);
        }
    }
    fn write_u32(&mut self, n: u32) {
        self.0 = u64::from(n).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

/// The edge cache lookup alone: the engine's `HashMap` (default hasher)
/// against an identity-style hasher and a short list.
fn edge_lookup(filter: Option<&str>) {
    let (_, ids) = registered_world();
    for edges in [1, 4, 16, 64] {
        let keys: Vec<ComponentId> = ids[SPREAD..SPREAD + edges].to_vec();
        let sip: HashMap<ComponentId, usize> = keys.iter().enumerate().map(|(i, &k)| (k, i)).collect();
        let fast: HashMap<ComponentId, usize, BuildHasherDefault<IdHasher>> =
            keys.iter().enumerate().map(|(i, &k)| (k, i)).collect();
        let list: Vec<(ComponentId, usize)> = keys.iter().enumerate().map(|(i, &k)| (k, i)).collect();
        let q = queries(&keys, 5);
        let (sip, fast, list, q) = (&sip, &fast, &list, &q);
        compare(
            filter,
            "edge-lookup",
            LOOKUPS,
            300,
            vec![
                (
                    leak(format!("e{edges}/hashmap-siphash(engine)")),
                    Box::new(move || q.iter().filter_map(|id| sip.get(id)).sum()),
                ),
                (
                    leak(format!("e{edges}/hashmap-identity-hash")),
                    Box::new(move || q.iter().filter_map(|id| fast.get(id)).sum()),
                ),
                (
                    leak(format!("e{edges}/linear-list")),
                    Box::new(move || {
                        q.iter()
                            .filter_map(|id| list.iter().find(|(k, _)| k == id).map(|&(_, v)| v))
                            .sum()
                    }),
                ),
            ],
        );
    }
}

/// Bytes per archetype the candidate layouts would add, for a world of
/// `REGISTERED` component types.
fn memory(filter: Option<&str>) {
    if skips(filter, "membership/memory") {
        return;
    }
    let (_, ids) = registered_world();
    let set = pick(Layout::Sparse, 16);
    let mine: Vec<ComponentId> = set.iter().map(|&i| ids[i]).collect();
    println!(
        "membership/memory: per archetype, {REGISTERED} registered components: \
         bitset+rank {} B, flat table {} B; the id list it supplements is {} B at 16 components \
         (4 B each), and a column is 24 B each.",
        Bits::new(&mine).bytes(),
        Table::new(&mine).bytes(),
        16 * 4,
    );
}

pub fn run(filter: Option<&str>) {
    memory(filter);
    lookups(filter);
    colmap(filter);
    edge_lookup(filter);
    transitions(filter);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_agree_with_the_linear_scan() {
        let (_, ids) = registered_world();
        for layout in [Layout::Dense, Layout::Sparse] {
            for n in WIDTHS {
                let set = pick(layout, n);
                assert_eq!(set.len(), n);
                assert!(set.windows(2).all(|p| p[0] < p[1]), "sorted and unique");
                let mine: Vec<ComponentId> = set.iter().map(|&i| ids[i]).collect();
                let (bits, table) = (Bits::new(&mine), Table::new(&mine));
                for &id in &ids {
                    let want = linear_index(&mine, id);
                    assert_eq!(bits.index(id), want);
                    assert_eq!(table.index(id), want);
                    assert_eq!(bits.has(id), want.is_some());
                    assert_eq!(mine.binary_search(&id).ok(), want);
                }
            }
        }
    }

    #[test]
    fn column_maps_agree() {
        let (_, ids) = registered_world();
        for layout in [Layout::Dense, Layout::Sparse] {
            for n in WIDTHS {
                let small: Vec<ComponentId> = pick(layout, n).iter().map(|&i| ids[i]).collect();
                let mut big = small.clone();
                big.push(ids[SPREAD + 7]);
                big.sort_unstable();
                for (src, dst) in [(&small, &big), (&big, &small)] {
                    let want = linear_map(src, dst);
                    assert_eq!(merge_map(src, dst), want);
                    assert_eq!(bits_map(src, &Bits::new(dst)), want);
                    assert_eq!(table_map(src, &Table::new(dst)), want);
                }
            }
        }
    }

    #[test]
    fn archetype_has_the_requested_components() {
        for n in WIDTHS {
            let set = pick(Layout::Sparse, n);
            let (w, ids, e) = world_with(&set);
            let mine: Vec<ComponentId> = set.iter().map(|&i| ids[i]).collect();
            let a = archetype_of(&w, &mine);
            assert_eq!(a.len(), 1);
            assert_eq!(a.entities(), &[e]);
            assert!(mine.iter().all(|&id| a.has(id)));
            assert!(!a.has(ids[SPREAD]));
        }
    }

    #[test]
    fn warm_transitions_create_no_archetypes() {
        let set = pick(Layout::Sparse, 8);
        let (mut w, ids, e) = world_with(&set);
        for &x in &ids[SPREAD..SPREAD + 4] {
            add(&mut w, e, x);
            assert!(w.raw().remove_dynamic(e, x));
        }
        let before = w.raw_ref().archetypes().len();
        for k in 0..64 {
            let x = ids[SPREAD + k % 4];
            add(&mut w, e, x);
            assert!(w.raw().remove_dynamic(e, x));
        }
        assert_eq!(w.raw_ref().archetypes().len(), before);
    }

    #[test]
    fn edge_lookup_maps_agree() {
        let (_, ids) = registered_world();
        let keys = &ids[SPREAD..SPREAD + 16];
        let fast: HashMap<ComponentId, usize, BuildHasherDefault<IdHasher>> =
            keys.iter().enumerate().map(|(i, &k)| (k, i)).collect();
        assert!(keys.iter().enumerate().all(|(i, k)| fast.get(k) == Some(&i)));
    }
}
