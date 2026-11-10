//! Parallel queries across few huge versus many tiny archetypes, with light
//! and heavy work per entity (`par-arch/...`). See `PAR_ARCHETYPES.md`.
//!
//! Every workload has `layouts` matching archetypes of `rows` entities each
//! (plus an optional huge one, `mixed`). `seq` is `for_each` on one core,
//! `par` is `par_for_each`, `collect` is `par_collect_into` into a reused `Vec`.
use crate::Bench;
use ponko_ecs::World;
use std::hint::black_box;

#[derive(Clone, Copy)]
struct Pos {
    x: f32,
    y: f32,
}
#[derive(Clone, Copy)]
struct Vel {
    x: f32,
    y: f32,
}
struct Marker<const N: usize>;

/// `layouts` archetypes (`layouts` must be a power of two up to 2^14) of `rows`
/// entities each, then `huge` more entities in the plain `(Pos, Vel)` layout.
pub(crate) fn world(layouts: usize, rows: usize, huge: usize) -> World {
    assert!(layouts.is_power_of_two() && layouts <= 1 << 14);
    let mut w = World::new();
    let n = layouts * rows;
    let entities: Vec<_> = (0..n)
        .map(|i| w.spawn((Pos { x: i as f32, y: 0.0 }, Vel { x: 1.0, y: 0.5 })))
        .collect();
    for (i, &e) in entities.iter().enumerate() {
        let bits = i % layouts;
        macro_rules! marker {
            ($($n:literal)*) => {$(
                if bits & (1 << $n) != 0 {
                    w.insert(e, Marker::<$n>);
                }
            )*};
        }
        marker!(0 1 2 3 4 5 6 7 8 9 10 11 12 13);
    }
    w.spawn_batch((0..huge).map(|i| (Pos { x: i as f32, y: 0.0 }, Vel { x: 1.0, y: 0.5 })));
    w
}

#[inline]
fn light(p: &mut Pos, v: &Vel, dt: f32) {
    p.x += v.x * dt;
    p.y += v.y * dt;
}

/// Roughly 20 ns per entity: a few transcendental calls.
#[inline]
fn heavy(p: &mut Pos, v: &Vel, dt: f32) {
    for _ in 0..4 {
        let (s, c) = (p.x * 0.001 + p.y).sin_cos();
        p.x += (v.x * c - v.y * s) * dt;
        p.y = (p.y + (v.y * c + v.x * s) * dt).rem_euclid(4096.0);
    }
}

pub fn run(b: &mut Bench) {
    // (layouts, rows each, extra rows in one huge archetype)
    let shapes: &[(usize, usize, usize)] = &[
        (1, 1_000_000, 0),
        (4, 250_000, 0),
        (1, 65_536, 0),
        (1, 16_384, 0),
        (1, 4096, 0),
        (1, 1024, 0),
        (4096, 256, 0),
        (4096, 16, 0),
        (1024, 64, 0),
        (2048, 16, 0),
        (1024, 16, 0),
        (512, 16, 0),
        (256, 16, 0),
        (64, 8, 0),
        (1024, 64, 500_000),
    ];
    for &(layouts, rows, huge) in shapes {
        let shape = if huge > 0 {
            format!("{layouts}x{rows}+{huge}")
        } else {
            format!("{layouts}x{rows}")
        };
        let total = layouts * rows + huge;
        let runs = if total >= 500_000 { 50 } else { 200 };
        for work in ["light", "heavy"] {
            let step = if work == "light" { light } else { heavy };
            b.steady(
                &format!("par-arch/{shape}/{work}/seq"),
                runs,
                || world(layouts, rows, huge),
                |w| {
                    let dt = black_box(1.0 / 60.0);
                    w.query::<(&mut Pos, &Vel)>().for_each(|(p, v)| step(p, v, dt));
                },
            );
            b.steady(
                &format!("par-arch/{shape}/{work}/par"),
                runs,
                || world(layouts, rows, huge),
                |w| {
                    let dt = black_box(1.0 / 60.0);
                    w.query::<(&mut Pos, &Vel)>().par_for_each(|(p, v)| step(p, v, dt));
                },
            );
        }
        b.steady(
            &format!("par-arch/{shape}/light/collect"),
            runs,
            || (world(layouts, rows, huge), Vec::with_capacity(total)),
            |(w, out)| {
                w.query::<&Pos>().par_collect_into(out, |p| p.x * black_box(2.0));
            },
        );
    }
    // The opt-in matching cache in front of the parallel path.
    b.steady(
        "par-arch/4096x16/light/par-cached",
        200,
        || world(4096, 16, 0),
        |w| {
            let dt = black_box(1.0 / 60.0);
            w.query::<(&mut Pos, &Vel)>()
                .cached()
                .par_for_each(|(p, v)| light(p, v, dt));
        },
    );
}
