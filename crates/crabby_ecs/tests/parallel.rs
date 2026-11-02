//! Multi-core queries (`par_for_each`).
#![cfg(feature = "parallel")]

mod common;

use common::*;
use crabby_ecs::*;

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
