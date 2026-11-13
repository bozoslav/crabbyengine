//! Tests of the simulation. `cargo test` runs them.

use ponko::prelude::*;
use {{crate_name}}::{Bounces, Settings, Walker, simulation};

#[test]
fn every_walker_stays_inside_the_world() {
    let mut world = simulation().run_headless(60 * 20);
    let size = world.resource::<Settings>().size;
    let mut outside = 0;
    world.query::<&Walker>().for_each(|w| {
        if w.position.cmplt(Vec2::ZERO).any() || w.position.cmpgt(Vec2::splat(size)).any() {
            outside += 1;
        }
    });
    assert_eq!(outside, 0);
    assert!(world.resource::<Bounces>().0 > 0, "walkers reached an edge");
}

#[test]
fn the_same_start_gives_the_same_result() {
    let positions = |mut world: World| {
        let mut out = Vec::new();
        world.query::<&Walker>().for_each(|w| out.push(w.position));
        out
    };
    let a = positions(simulation().run_headless(300));
    let b = positions(simulation().run_headless(300));
    assert!(!a.is_empty());
    assert_eq!(a, b);
}
