//! The sprite stress workload of the `bunnymark_ecs` example without a
//! window (`sprites/...`): bouncing sprite entities drawn by the built-in
//! sprite drawing, then ordered and batched as the renderer does. The time
//! is that CPU side of one frame (collecting, ordering, batching); the
//! movement between frames is not timed.
//!
//! - `draw-10k`: 10,000 sprites, one thread.
//! - `draw-100k`: 100,000 sprites, collected on all cores.
//! - `draw-100k-z`: the same with half of them one layer up, so the frame
//!   is sorted by draw order.
use std::hint::black_box;
use std::time::{Duration, Instant};

use ponko::d2::{Sprite, Transform as Transform2D};
use ponko::prelude::*;

use crate::{Bench, report};

const WARMUP: usize = 10;
const FRAMES: usize = 200;

#[derive(Component)]
struct Vel(Vec2);

/// Every timed frame.
struct Times(Vec<Duration>);

fn rand(seed: &mut u64) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 40) as f32 / (1u64 << 24) as f32
}

fn spawn(ctx: &mut Ctx, n: usize, layered: bool) {
    let crab = ctx.gfx.create_texture(32, 32, &[255; 32 * 32 * 4]);
    let mut seed = 0x9E37_79B9_7F4A_7C15;
    let batch: Vec<_> = (0..n)
        .map(|i| {
            let vel = vec2(rand(&mut seed) * 400.0 - 200.0, rand(&mut seed) * 400.0 - 300.0);
            let color = Color::rgb(
                0.6 + rand(&mut seed) * 0.4,
                0.6 + rand(&mut seed) * 0.4,
                0.6 + rand(&mut seed) * 0.4,
            );
            let z = if layered && i % 2 == 1 { 1.0 } else { 0.0 };
            (
                Transform2D::at(vec2(rand(&mut seed) * 1280.0, rand(&mut seed) * 720.0)),
                Sprite::new(crab).with_color(color).with_z(z),
                Vel(vel),
            )
        })
        .collect();
    ctx.world.spawn_batch(batch);
}

/// `bunnymark_ecs`'s movement.
fn bounce(ctx: &mut Ctx) {
    let dt = 1.0 / 60.0;
    let bounds = vec2(1280.0, 720.0);
    ctx.world.query::<(&mut Transform2D, &mut Vel)>().for_each(|(t, v)| {
        let (p, v) = (&mut t.position, &mut v.0);
        v.y += 600.0 * dt;
        *p += *v * dt;
        if p.x < 0.0 || p.x > bounds.x {
            p.x = p.x.clamp(0.0, bounds.x);
            v.x = -v.x;
        }
        if p.y > bounds.y {
            p.y = bounds.y;
            v.y = -v.y.abs() * 0.85;
        }
        if p.y < 0.0 {
            p.y = 0.0;
            v.y = 0.0;
        }
    });
}

pub fn run(b: &mut Bench) {
    for (name, n, layered) in [
        ("sprites/draw-10k", 10_000, false),
        ("sprites/draw-100k", 100_000, false),
        ("sprites/draw-100k-z", 100_000, true),
    ] {
        if b.skips(name) {
            continue;
        }
        let mut world = Game::new("sprites bench", 1280, 720).run_headless(0, move |ctx| {
            spawn(ctx, n, layered);
            let mut times = Vec::with_capacity(FRAMES);
            for frame in 0..WARMUP + FRAMES {
                bounce(ctx);
                let start = Instant::now();
                let drawn = ctx.bench_sprite_frame();
                let took = start.elapsed();
                assert_eq!(drawn.0, n, "every sprite is drawn");
                black_box(drawn);
                if frame >= WARMUP {
                    times.push(took);
                }
            }
            ctx.world.insert_resource(Times(times));
        });
        let mut times = world.remove_resource::<Times>().expect("the times").0;
        b.report(report::Record::new(name, report::Kind::Steady, WARMUP, &mut times));
    }
}
