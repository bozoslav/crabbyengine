//! Headless tests: the game runs without a window or GPU, as fast as the
//! CPU allows. `cargo test` runs them.

use ponko::d3::Transform;
use ponko::prelude::*;
use {{crate_name}}::{Settings, Spin, game};

#[test]
fn the_cube_turns_at_the_configured_speed() {
    // One second at the default 60 ticks per second.
    let mut world = game().run_headless(60);
    let speed = world.resource::<Settings>().spin_speed;
    let mut angles = Vec::new();
    world
        .query::<(&Transform, &Spin)>()
        .for_each(|(t, _)| angles.push(t.rotation.angle_between(Quat::IDENTITY)));
    assert_eq!(angles.len(), 1, "one spinning cube");
    assert!((angles[0] - speed).abs() < 1e-3, "turned {} rad, expected {speed}", angles[0]);
}
