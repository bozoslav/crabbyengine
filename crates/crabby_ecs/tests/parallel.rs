//! Multi-core queries (`par_for_each`).
#![cfg(feature = "parallel")]

mod common;

use common::*;
use crabby_ecs::*;

#[test]
fn par_for_each_visits_every_entity_once() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let mut w = World::new();
    // Several archetypes, some bigger than one chunk, some smaller.
    w.spawn_batch((0..50_000).map(|i| (Pos(i as f32, 0.0), Vel(1.0, 0.0))));
    w.spawn_batch((0..20_000).map(|i| (Pos(i as f32, 0.0), Vel(1.0, 0.0), Tag)));
    w.spawn_batch((0..100).map(|i| (Pos(i as f32, 0.0), Vel(1.0, 0.0), Frozen)));
    w.spawn_batch((0..1000).map(|i| (Pos(i as f32, 0.0),)));
    let visits = AtomicUsize::new(0);
    w.query::<(&mut Pos, &Vel)>()
        .without::<Frozen>()
        .par_for_each(|(p, v)| {
            p.1 += v.0;
            visits.fetch_add(1, Ordering::Relaxed);
        });
    assert_eq!(visits.into_inner(), 70_000);
    assert_eq!(w.query::<&Pos>().iter().filter(|p| p.1 == 1.0).count(), 70_000);
    assert_eq!(w.query::<&Pos>().iter().filter(|p| p.1 == 0.0).count(), 1100);
}
