//! Physics: the same pile of boxes stepped through Ponko (`Game::physics`,
//! components synced every tick) and through rapier directly. The gap is
//! what Ponko's sync costs.
use ponko::d2::Collider;
use ponko::d2::RigidBody;
use ponko::d2::Transform as Transform2D;
use ponko::{Game, vec2};
use rapier2d::prelude as rp;

use crate::Bench;

pub const TICKS: u64 = 300;

/// `n` 10 px boxes in rows above a floor between two walls.
fn scene(n: usize) -> (Vec<(f32, f32)>, f32) {
    let per_row = (n as f32).sqrt().ceil() as usize * 2;
    let width = per_row as f32 * 12.0;
    let boxes = (0..n)
        .map(|i| {
            (
                (i % per_row) as f32 * 12.0 - width / 2.0 + 6.0,
                -((i / per_row) as f32) * 12.0,
            )
        })
        .collect();
    (boxes, width)
}

pub fn run(b: &mut Bench) {
    for n in [1_000, 10_000] {
        let (boxes, width) = scene(n);
        b.run(
            &format!("physics/ponko-{n}"),
            3,
            || (),
            |_| {
                let world = Game::new("bench", 1, 1)
                    .physics(vec2(0.0, 980.0))
                    .run_headless(TICKS, |ctx| {
                        let wall = |x: f32, y: f32, w: f32, h: f32| {
                            (
                                Transform2D::at(vec2(x, y)),
                                RigidBody::fixed(),
                                Collider::rect(vec2(w, h)),
                            )
                        };
                        ctx.world.spawn(wall(0.0, 100.0, width + 40.0, 20.0));
                        ctx.world.spawn(wall(-width / 2.0 - 10.0, -500.0, 20.0, 1200.0));
                        ctx.world.spawn(wall(width / 2.0 + 10.0, -500.0, 20.0, 1200.0));
                        for &(x, y) in &boxes {
                            ctx.world.spawn((
                                Transform2D::at(vec2(x, y)),
                                RigidBody::dynamic(),
                                Collider::rect(vec2(10.0, 10.0)),
                            ));
                        }
                    });
                std::hint::black_box(world);
            },
        );
        b.run(
            &format!("physics/rapier-{n}"),
            3,
            || (),
            |_| {
                let mut w = rp::PhysicsWorld::new();
                w.integration_parameters.length_unit = 50.0;
                w.integration_parameters.dt = 1.0 / 60.0;
                w.gravity = rp::Vector::new(0.0, 980.0);
                let mut wall = |x: f32, y: f32, hw: f32, hh: f32| {
                    let body = w.insert_body(rp::RigidBodyBuilder::fixed().translation(rp::Vector::new(x, y)));
                    w.insert_collider(rp::ColliderBuilder::cuboid(hw, hh), Some(body));
                };
                wall(0.0, 100.0, width / 2.0 + 20.0, 10.0);
                wall(-width / 2.0 - 10.0, -500.0, 10.0, 600.0);
                wall(width / 2.0 + 10.0, -500.0, 10.0, 600.0);
                for &(x, y) in &boxes {
                    let body = w.insert_body(rp::RigidBodyBuilder::dynamic().translation(rp::Vector::new(x, y)));
                    w.insert_collider(
                        rp::ColliderBuilder::cuboid(5.0, 5.0).active_events(rp::ActiveEvents::COLLISION_EVENTS),
                        Some(body),
                    );
                }
                let (a, ra) = std::sync::mpsc::channel();
                let (bb, rb) = std::sync::mpsc::channel();
                let (c, rc) = std::sync::mpsc::channel();
                let collector = rp::ChannelEventCollector::new(a, bb, c);
                for _ in 0..TICKS {
                    w.step_with_events(&(), &collector);
                    while ra.try_recv().is_ok() {}
                }
                drop((rb, rc));
                std::hint::black_box(w);
            },
        );
    }
}
