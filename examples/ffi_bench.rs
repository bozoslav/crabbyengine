//! Rust reference for examples/cpp/benchmark.cpp; release builds only.
//! One million positions, 10 warmups, 200 measured full-column updates.
use ponko::Component;
use ponko_ecs::World;
use std::hint::black_box;
use std::time::Instant;

#[derive(Component)]
struct Pos {
    x: f32,
    y: f32,
}
#[derive(Component)]
struct Vel {
    x: f32,
    y: f32,
}

fn main() {
    let mut world = World::new();
    world.spawn_batch((0..1_000_000).map(|_| (Pos { x: 0.0, y: 0.0 }, Vel { x: 1.0, y: 2.0 })));
    let mut samples = Vec::with_capacity(200);
    for i in 0..210 {
        let start = Instant::now();
        world.query::<(&mut Pos, &Vel)>().for_each(|(p, v)| {
            p.x += v.x;
            p.y += v.y;
        });
        black_box(&world);
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        if i >= 10 {
            samples.push(elapsed);
        }
    }
    samples.sort_by(f64::total_cmp);
    let checksum: f64 = world
        .query::<&Pos>()
        .iter()
        .map(|p| f64::from(p.x) + f64::from(p.y))
        .sum();
    assert_eq!(checksum, 630_000_000.0);
    println!(
        "Rust: 1000000 entities, median {:.6} ms/update, checksum {checksum:.0}",
        (samples[99] + samples[100]) / 2.0
    );
}
