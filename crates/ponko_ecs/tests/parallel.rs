//! Multi-core queries (`par_for_each`).
#![cfg(feature = "parallel")]

mod common;

use common::*;
use ponko_ecs::*;

/// Extra marker for a third matching archetype.
struct Vel2;

#[test]
fn par_for_each_visits_every_entity_once() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    // Sizes relative to the largest parallel chunk (8192 rows, 64 under Miri):
    // several archetypes bigger than one chunk (with a partial last chunk),
    // one smaller than a chunk, a filtered-out one, and a non-matching one.
    let chunk = if cfg!(miri) { 64 } else { 8192 };
    let (big, mid, small, other) = (6 * chunk + 7, 2 * chunk + 3, chunk / 2, chunk / 8);
    let mut w = World::new();
    w.spawn_batch((0..big).map(|i| (Pos(i as f32, 0.0), Vel(1.0, 0.0))));
    w.spawn_batch((0..mid).map(|i| (Pos(i as f32, 0.0), Vel(1.0, 0.0), Tag)));
    w.spawn_batch((0..small).map(|i| (Pos(i as f32, 0.0), Vel(1.0, 0.0), Tag, Vel2)));
    w.spawn_batch((0..100).map(|i| (Pos(i as f32, 0.0), Vel(1.0, 0.0), Frozen)));
    w.spawn_batch((0..other).map(|i| (Pos(i as f32, 0.0),)));
    let matching = big + mid + small;
    let visits = AtomicUsize::new(0);
    w.query::<(&mut Pos, &Vel)>()
        .without::<Frozen>()
        .par_for_each(|(p, v)| {
            p.1 += v.0;
            visits.fetch_add(1, Ordering::Relaxed);
        });
    assert_eq!(visits.into_inner(), matching);
    assert_eq!(w.query::<&Pos>().iter().filter(|p| p.1 == 1.0).count(), matching);
    assert_eq!(w.query::<&Pos>().iter().filter(|p| p.1 == 0.0).count(), 100 + other);
}

#[test]
fn par_for_each_indexed_matches_the_sequential_order() {
    use std::sync::atomic::{AtomicU32, Ordering};
    // Same mix of archetype sizes as above, so indices have to carry over
    // between archetypes and between chunks.
    let chunk = if cfg!(miri) { 64 } else { 8192 };
    let mut w = World::new();
    w.spawn_batch((0..3 * chunk + 5).map(|i| (Pos(i as f32, 0.0), Vel(1.0, 0.0))));
    w.spawn_batch((0..chunk / 2).map(|i| (Pos(i as f32 + 1e5, 0.0), Vel(1.0, 0.0), Tag)));
    w.spawn_batch((0..100).map(|i| (Pos(i as f32, 0.0), Vel(1.0, 0.0), Frozen)));
    w.spawn_batch((0..2 * chunk + 1).map(|i| (Pos(i as f32 + 2e5, 0.0), Vel(1.0, 0.0), Vel2)));
    let expected: Vec<u32> = w
        .query::<(&Pos, &Vel)>()
        .without::<Frozen>()
        .iter()
        .map(|(p, _)| p.0 as u32)
        .collect();
    let out: Vec<AtomicU32> = (0..expected.len()).map(|_| AtomicU32::new(u32::MAX)).collect();
    w.query::<(&Pos, &Vel)>()
        .without::<Frozen>()
        .par_for_each_indexed(|i, (p, _)| {
            let previous = out[i].swap(p.0 as u32, Ordering::Relaxed);
            assert_eq!(previous, u32::MAX, "index {i} visited twice");
        });
    let got: Vec<u32> = out.into_iter().map(AtomicU32::into_inner).collect();
    assert_eq!(got, expected);
}

#[test]
fn par_collect_into_matches_the_sequential_order_across_archetypes() {
    let mut world = World::new();
    world.spawn_batch((0..3000).map(|i| (Pos(i as f32, 0.0),)));
    world.spawn_batch((3000..5000).map(|i| (Pos(i as f32, 0.0), Tag)));
    let mut expected = Vec::new();
    world.query::<&Pos>().for_each(|p| expected.push(p.0));
    // Strings own memory: written once each, dropped once each.
    let mut out = vec!["stale".to_string(); 10];
    world.query::<&Pos>().par_collect_into(&mut out, |p| p.0.to_string());
    let want: Vec<String> = expected.iter().map(f32::to_string).collect();
    assert_eq!(out, want);
    // Reused: replaced, not appended to; filters apply.
    world
        .query::<&Pos>()
        .with::<Tag>()
        .par_collect_into(&mut out, |p| p.0.to_string());
    assert_eq!(out.len(), 2000);
    assert_eq!(out[0], "3000");
}

#[test]
fn par_collect_into_an_empty_query_empties_the_vec() {
    let mut world = World::new();
    let mut out = vec![1, 2, 3];
    world.query::<&Pos>().par_collect_into(&mut out, |p| p.0 as i32);
    assert!(out.is_empty());
}

fn assert_panicking_collect_drops_each_written_value(rows: usize, panic_at: usize) {
    use std::cell::Cell;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    struct Id(usize);
    // Send but not Sync: cleanup must require only the API's R: Send bound.
    struct Counted {
        id: usize,
        drops: Arc<[AtomicUsize]>,
        _not_sync: Cell<()>,
    }
    impl Drop for Counted {
        fn drop(&mut self) {
            self.drops[self.id].fetch_add(1, Ordering::Relaxed);
        }
    }

    let mut world = World::new();
    // Finish one archetype before starting the possibly parallel one.
    world.spawn_batch((0..3).map(|i| (Id(i), Tag)));
    world.spawn_batch((3..rows).map(|i| (Id(i),)));
    let created: Vec<_> = (0..rows).map(|_| AtomicBool::new(false)).collect();
    let drops: Arc<[AtomicUsize]> = (0..=rows).map(|_| AtomicUsize::new(0)).collect();
    let make = |id| Counted {
        id,
        drops: drops.clone(),
        _not_sync: Cell::new(()),
    };
    let mut out = Vec::with_capacity(rows);
    out.push(make(rows));
    let capacity = out.capacity();
    let buffer = out.as_ptr();
    let result = catch_unwind(AssertUnwindSafe(|| {
        world.query::<&Id>().par_collect_into(&mut out, |id| {
            assert_ne!(id.0, panic_at, "mapper panic");
            created[id.0].store(true, Ordering::Relaxed);
            make(id.0)
        });
    }));
    assert!(result.is_err());
    assert!(out.is_empty());
    assert_eq!(drops[rows].load(Ordering::Relaxed), 1, "old output is dropped once");
    for i in 0..rows {
        assert_eq!(
            drops[i].load(Ordering::Relaxed),
            usize::from(created[i].load(Ordering::Relaxed)),
            "written result {i} is dropped exactly once"
        );
    }
    assert!(created[..3].iter().all(|c| c.load(Ordering::Relaxed)));

    // The same allocation remains usable after unwinding, including its length.
    world.query::<&Id>().par_collect_into(&mut out, |id| make(id.0));
    assert_eq!(out.len(), rows);
    assert_eq!(out.capacity(), capacity);
    assert_eq!(out.as_ptr(), buffer);
    assert_eq!(
        out.iter().map(|v| v.id).collect::<Vec<_>>(),
        (0..rows).collect::<Vec<_>>()
    );
    drop(out);
    for i in 0..rows {
        assert_eq!(
            drops[i].load(Ordering::Relaxed),
            1 + usize::from(created[i].load(Ordering::Relaxed))
        );
    }
}

#[test]
fn par_collect_into_drops_written_values_on_small_mapper_panic_and_reuses_the_vec() {
    assert_panicking_collect_drops_each_written_value(7, 5);
}

#[test]
fn par_collect_into_drops_each_written_value_across_chunks_and_archetypes_on_mapper_panic() {
    let chunk = if cfg!(miri) { 64 } else { 512 };
    rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap()
        .install(|| assert_panicking_collect_drops_each_written_value(3 + 2 * chunk + 7, 3 + chunk + 1));
}

#[test]
fn par_collect_into_drops_each_written_zero_sized_value_on_mapper_panic() {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static DROPS: AtomicUsize = AtomicUsize::new(0);
    struct Counted;
    impl Drop for Counted {
        fn drop(&mut self) {
            DROPS.fetch_add(1, Ordering::Relaxed);
        }
    }
    let chunk = if cfg!(miri) { 64 } else { 512 };
    let mut world = World::new();
    world.spawn_batch((0..2 * chunk + 7).map(|i| (Pos(i as f32, 0.0),)));
    let created = AtomicUsize::new(0);
    let mut out = Vec::new();
    let result = catch_unwind(AssertUnwindSafe(|| {
        world.query::<&Pos>().par_collect_into(&mut out, |p| {
            assert_ne!(p.0 as usize, chunk + 1, "mapper panic");
            created.fetch_add(1, Ordering::Relaxed);
            Counted
        });
    }));
    assert!(result.is_err());
    assert!(out.is_empty());
    assert_eq!(DROPS.load(Ordering::Relaxed), created.load(Ordering::Relaxed));
    drop(out);
    assert_eq!(DROPS.load(Ordering::Relaxed), created.load(Ordering::Relaxed));
}
