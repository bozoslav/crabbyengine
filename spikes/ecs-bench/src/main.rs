//! M3 ECS spike: own archetype ECS vs bevy_ecs vs hecs.
//!
//! Run with: cargo run --release -p ecs-bench [-- filter]
//! Each benchmark builds its state untimed, then times one operation; we report
//! the median and minimum over several runs.

#[allow(dead_code)] // spike: API surface kept for the M3 port
mod own;

use bevy_ecs::component::Component;
use std::hint::black_box;
use std::time::{Duration, Instant};

const N: usize = 1_000_000;
const ADD_REMOVE_N: usize = 100_000;
const DT: f32 = 1.0 / 60.0;

#[derive(Component, Clone, Copy, Default)]
struct Pos {
    x: f32,
    y: f32,
}
#[derive(Component, Clone, Copy, Default)]
struct Vel {
    x: f32,
    y: f32,
}
/// Stand-in for a sprite component: texture id + tint.
#[derive(Component, Clone, Copy, Default)]
struct Spr {
    _tex: u32,
    _tint: [f32; 4],
}
#[derive(Component, Clone, Copy, Default)]
struct Tag;
#[derive(Component, Clone, Copy, Default)]
struct M0;
#[derive(Component, Clone, Copy, Default)]
struct M1;
#[derive(Component, Clone, Copy, Default)]
struct M2;
#[derive(Component, Clone, Copy, Default)]
struct M3;

fn ent(i: usize) -> (Pos, Vel, Spr) {
    (Pos { x: i as f32, y: 0.0 }, Vel { x: 1.0, y: 0.5 }, Spr::default())
}

/// Deterministic shuffled order for random-access benchmarks.
fn shuffled(n: usize) -> Vec<usize> {
    let mut v: Vec<usize> = (0..n).collect();
    let mut s = 0x9E3779B97F4A7C15u64;
    for i in (1..n).rev() {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        v.swap(i, (s % (i as u64 + 1)) as usize);
    }
    v
}

struct Bench {
    filter: Option<String>,
}

impl Bench {
    /// Builds the state once, then times `op` repeatedly on it (like a system
    /// running every frame). Use for iteration benchmarks.
    fn steady<S>(&mut self, name: &str, runs: usize, setup: impl FnOnce() -> S, mut op: impl FnMut(&mut S)) {
        if let Some(f) = &self.filter
            && !name.contains(f.as_str())
        {
            return;
        }
        let mut s = setup();
        for _ in 0..5 {
            op(&mut s);
        }
        let mut times: Vec<_> = (0..runs)
            .map(|_| {
                let t = Instant::now();
                op(&mut s);
                t.elapsed()
            })
            .collect();
        black_box(&s);
        self.report(name, &mut times);
    }

    fn report(&mut self, name: &str, times: &mut [Duration]) {
        times.sort();
        let (med, min) = (times[times.len() / 2], times[0]);
        println!("{name:<28} median {:>9.3} ms   min {:>9.3} ms", ms(med), ms(min));
    }

    fn run<S>(&mut self, name: &str, runs: usize, mut setup: impl FnMut() -> S, mut op: impl FnMut(&mut S)) {
        if let Some(f) = &self.filter
            && !name.contains(f.as_str())
        {
            return;
        }
        let mut times = Vec::with_capacity(runs);
        // One warmup run, then measured runs.
        for i in 0..runs + 1 {
            let mut s = setup();
            let t = Instant::now();
            op(&mut s);
            let el = t.elapsed();
            if i > 0 {
                times.push(el);
            }
            drop(black_box(s));
        }
        self.report(name, &mut times);
    }
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1e3
}

fn main() {
    let filter = std::env::args().nth(1);
    let mut b = Bench { filter };
    println!("N = {N}, add/remove N = {ADD_REMOVE_N}\n");

    // ---- baseline: plain Vecs (upper bound for iteration) -------------------
    b.steady(
        "iter/raw-vec",
        200,
        // Non-zero data: zeroed vecs get lazily mapped pages that fault on first touch.
        || {
            (
                (0..N).map(|i| ent(i).0).collect::<Vec<_>>(),
                vec![Vel { x: 1.0, y: 0.5 }; N],
            )
        },
        |(p, v)| {
            let dt = black_box(DT);
            for (p, v) in p.iter_mut().zip(v.iter()) {
                p.x += v.x * dt;
                p.y += v.y * dt;
            }
        },
    );

    b.steady(
        "iter/raw-vec-grown",
        200,
        // Same data, but grown by push like ECS columns (realloc'd buffers).
        || {
            let (mut p, mut v) = (Vec::new(), Vec::new());
            for i in 0..N {
                p.push(ent(i).0);
                v.push(ent(i).1);
            }
            (p, v)
        },
        |(p, v)| {
            let dt = black_box(DT);
            for (p, v) in p.iter_mut().zip(v.iter()) {
                p.x += v.x * dt;
                p.y += v.y * dt;
            }
        },
    );

    // ---- simulation at scale: single core vs all cores (crabby_ecs) ----------
    /// Agent with heading; "heavy" work = steering with trig, like a real sim tick.
    #[derive(Clone, Copy)]
    struct Agent {
        heading: f32,
        speed: f32,
    }
    fn heavy(p: &mut Pos, a: &mut Agent, dt: f32) {
        let wander = (p.x * 0.013 + p.y * 0.007 + a.heading).sin() * 0.8;
        a.heading += wander * dt;
        let (s, c) = a.heading.sin_cos();
        p.x = (p.x + c * a.speed * dt).rem_euclid(4096.0);
        p.y = (p.y + s * a.speed * dt).rem_euclid(4096.0);
    }
    for &n in &[1_000_000usize, 10_000_000] {
        let world = |n: usize| {
            let mut w = crabby_ecs::World::new();
            w.spawn_batch((0..n).map(|i| {
                (
                    Pos {
                        x: (i % 4096) as f32,
                        y: (i / 4096 % 4096) as f32,
                    },
                    Vel { x: 1.0, y: 0.5 },
                    Agent {
                        heading: i as f32,
                        speed: 30.0,
                    },
                )
            }));
            w
        };
        let tag = if n >= 10_000_000 { "10M" } else { "1M" };
        let runs = if n >= 10_000_000 { 30 } else { 100 };
        b.steady(
            &format!("sim-light-{tag}/1-core"),
            runs,
            || world(n),
            |w| {
                let dt = black_box(DT);
                w.query::<(&mut Pos, &Vel)>().for_each(|(p, v)| {
                    p.x += v.x * dt;
                    p.y += v.y * dt;
                });
            },
        );
        b.steady(
            &format!("sim-light-{tag}/all-cores"),
            runs,
            || world(n),
            |w| {
                let dt = black_box(DT);
                w.query::<(&mut Pos, &Vel)>().par_for_each(|(p, v)| {
                    p.x += v.x * dt;
                    p.y += v.y * dt;
                });
            },
        );
        b.steady(
            &format!("sim-heavy-{tag}/1-core"),
            runs,
            || world(n),
            |w| {
                let dt = black_box(DT);
                w.query::<(&mut Pos, &mut Agent)>().for_each(|(p, a)| heavy(p, a, dt));
            },
        );
        b.steady(
            &format!("sim-heavy-{tag}/all-cores"),
            runs,
            || world(n),
            |w| {
                let dt = black_box(DT);
                w.query::<(&mut Pos, &mut Agent)>()
                    .par_for_each(|(p, a)| heavy(p, a, dt));
            },
        );
    }

    // ---- spawn ------------------------------------------------------------------
    b.run("spawn/own", 10, own::World::new, |w| {
        for i in 0..N {
            w.spawn(ent(i));
        }
    });
    b.run("spawn/own-batch", 10, own::World::new, |w| {
        w.spawn_batch((0..N).map(ent))
    });
    b.run("spawn/crabby", 10, crabby_ecs::World::new, |w| {
        for i in 0..N {
            w.spawn(ent(i));
        }
    });
    b.run("spawn/crabby-batch", 10, crabby_ecs::World::new, |w| {
        w.spawn_batch((0..N).map(ent))
    });
    b.run("spawn/hecs", 10, hecs::World::new, |w| {
        for i in 0..N {
            w.spawn(ent(i));
        }
    });
    b.run("spawn/hecs-batch", 10, hecs::World::new, |w| {
        w.spawn_batch((0..N).map(ent));
    });
    b.run("spawn/bevy", 10, bevy_ecs::world::World::new, |w| {
        for i in 0..N {
            w.spawn(ent(i));
        }
    });
    b.run("spawn/bevy-batch", 10, bevy_ecs::world::World::new, |w| {
        w.spawn_batch((0..N).map(ent));
    });

    // ---- iterate: pos += vel * dt over 1M entities, one archetype -------------
    b.steady(
        "iter/own",
        200,
        || {
            let mut w = own::World::new();
            w.spawn_batch((0..N).map(ent));
            w
        },
        |w| {
            let dt = black_box(DT);
            w.for_each::<(&mut Pos, &Vel)>(|(p, v)| {
                p.x += v.x * dt;
                p.y += v.y * dt;
            });
        },
    );
    b.steady(
        "iter/crabby",
        200,
        || {
            let mut w = crabby_ecs::World::new();
            w.spawn_batch((0..N).map(ent));
            w
        },
        |w| {
            let dt = black_box(DT);
            w.query::<(&mut Pos, &Vel)>().for_each(|(p, v)| {
                p.x += v.x * dt;
                p.y += v.y * dt;
            });
        },
    );
    b.steady(
        "iter/crabby-iter",
        200,
        || {
            let mut w = crabby_ecs::World::new();
            w.spawn_batch((0..N).map(ent));
            w
        },
        |w| {
            let dt = black_box(DT);
            for (p, v) in w.query::<(&mut Pos, &Vel)>().iter() {
                p.x += v.x * dt;
                p.y += v.y * dt;
            }
        },
    );
    b.steady(
        "iter/own-dynamic-chunk",
        200,
        || {
            // Components registered by name/size/align only, as the C API would.
            let mut w = own::World::new();
            let p = w.register_dynamic("Pos", 8, 4, None);
            let v = w.register_dynamic("Vel", 8, 4, None);
            let s = w.register_dynamic("Spr", 20, 4, None);
            for i in 0..N {
                let (pp, vv, ss) = ent(i);
                unsafe {
                    w.spawn_dynamic(&[
                        (p, (&pp as *const Pos).cast()),
                        (v, (&vv as *const Vel).cast()),
                        (s, (&ss as *const Spr).cast()),
                    ]);
                }
            }
            (w, [p, v])
        },
        |(w, ids)| {
            let dt = black_box(DT);
            w.for_each_chunk(ids, |len, cols| {
                // What a C++ system does: cast the columns, run a plain loop.
                let pos = unsafe { std::slice::from_raw_parts_mut(cols[0].cast::<[f32; 2]>(), len) };
                let vel = unsafe { std::slice::from_raw_parts(cols[1].cast::<[f32; 2]>(), len) };
                for (p, v) in pos.iter_mut().zip(vel) {
                    p[0] += v[0] * dt;
                    p[1] += v[1] * dt;
                }
            });
        },
    );
    b.steady(
        "iter/hecs",
        200,
        || {
            let mut w = hecs::World::new();
            w.spawn_batch((0..N).map(ent));
            w
        },
        |w| {
            let dt = black_box(DT);
            for (p, v) in w.query_mut::<(&mut Pos, &Vel)>() {
                p.x += v.x * dt;
                p.y += v.y * dt;
            }
        },
    );
    b.steady(
        "iter/bevy",
        200,
        || {
            let mut w = bevy_ecs::world::World::new();
            w.spawn_batch((0..N).map(ent));
            let q = w.query::<(&mut Pos, &Vel)>();
            (w, q)
        },
        |(w, q)| {
            let dt = black_box(DT);
            q.iter_mut(w).for_each(|(mut p, v)| {
                p.x += v.x * dt;
                p.y += v.y * dt;
            });
        },
    );

    // ---- fragmented iterate: same 1M spread over 16 archetypes ----------------
    fn markers(i: usize) -> [bool; 4] {
        [i & 1 != 0, i & 2 != 0, i & 4 != 0, i & 8 != 0]
    }
    b.steady(
        "iter-frag16/own",
        200,
        || {
            let mut w = own::World::new();
            for i in 0..N {
                let e = w.spawn(ent(i));
                let m = markers(i);
                if m[0] {
                    w.insert(e, M0);
                }
                if m[1] {
                    w.insert(e, M1);
                }
                if m[2] {
                    w.insert(e, M2);
                }
                if m[3] {
                    w.insert(e, M3);
                }
            }
            w
        },
        |w| {
            let dt = black_box(DT);
            w.for_each::<(&mut Pos, &Vel)>(|(p, v)| {
                p.x += v.x * dt;
                p.y += v.y * dt;
            });
        },
    );
    b.steady(
        "iter-frag16/hecs",
        200,
        || {
            let mut w = hecs::World::new();
            for i in 0..N {
                let e = w.spawn(ent(i));
                let m = markers(i);
                if m[0] {
                    w.insert_one(e, M0).unwrap();
                }
                if m[1] {
                    w.insert_one(e, M1).unwrap();
                }
                if m[2] {
                    w.insert_one(e, M2).unwrap();
                }
                if m[3] {
                    w.insert_one(e, M3).unwrap();
                }
            }
            w
        },
        |w| {
            let dt = black_box(DT);
            for (p, v) in w.query_mut::<(&mut Pos, &Vel)>() {
                p.x += v.x * dt;
                p.y += v.y * dt;
            }
        },
    );
    b.steady(
        "iter-frag16/bevy",
        200,
        || {
            let mut w = bevy_ecs::world::World::new();
            for i in 0..N {
                let mut e = w.spawn(ent(i));
                let m = markers(i);
                if m[0] {
                    e.insert(M0);
                }
                if m[1] {
                    e.insert(M1);
                }
                if m[2] {
                    e.insert(M2);
                }
                if m[3] {
                    e.insert(M3);
                }
            }
            let q = w.query::<(&mut Pos, &Vel)>();
            (w, q)
        },
        |(w, q)| {
            let dt = black_box(DT);
            q.iter_mut(w).for_each(|(mut p, v)| {
                p.x += v.x * dt;
                p.y += v.y * dt;
            });
        },
    );

    // ---- random access: get<Pos> for 1M entities in shuffled order ----------------
    let order = shuffled(N);
    b.run(
        "get-random/own",
        10,
        || {
            let mut w = own::World::new();
            let es: Vec<_> = (0..N).map(|i| w.spawn(ent(i))).collect();
            let es: Vec<_> = order.iter().map(|&i| es[i]).collect();
            (w, es)
        },
        |(w, es)| {
            let mut s = 0.0;
            for &e in es.iter() {
                s += w.get::<Pos>(e).unwrap().x;
            }
            black_box(s);
        },
    );
    b.run(
        "get-random/crabby",
        10,
        || {
            let mut w = crabby_ecs::World::new();
            let es: Vec<_> = (0..N).map(|i| w.spawn(ent(i))).collect();
            let es: Vec<_> = order.iter().map(|&i| es[i]).collect();
            (w, es)
        },
        |(w, es)| {
            let mut s = 0.0;
            for &e in es.iter() {
                s += w.get::<Pos>(e).unwrap().x;
            }
            black_box(s);
        },
    );
    b.run(
        "get-random/hecs",
        10,
        || {
            let mut w = hecs::World::new();
            let es: Vec<_> = (0..N).map(|i| w.spawn(ent(i))).collect();
            let es: Vec<_> = order.iter().map(|&i| es[i]).collect();
            (w, es)
        },
        |(w, es)| {
            let mut s = 0.0;
            for &e in es.iter() {
                s += w.get::<&Pos>(e).unwrap().x;
            }
            black_box(s);
        },
    );
    b.run(
        "get-random/bevy",
        10,
        || {
            let mut w = bevy_ecs::world::World::new();
            let es: Vec<_> = (0..N).map(|i| w.spawn(ent(i)).id()).collect();
            let es: Vec<_> = order.iter().map(|&i| es[i]).collect();
            (w, es)
        },
        |(w, es)| {
            let mut s = 0.0;
            for &e in es.iter() {
                s += w.get::<Pos>(e).unwrap().x;
            }
            black_box(s);
        },
    );

    // ---- add + remove a component on 100k entities ----------------------------
    b.run(
        "add-remove/own",
        10,
        || {
            let mut w = own::World::new();
            let es: Vec<_> = (0..ADD_REMOVE_N).map(|i| w.spawn(ent(i))).collect();
            (w, es)
        },
        |(w, es)| {
            for &e in es.iter() {
                w.insert(e, Tag);
            }
            for &e in es.iter() {
                w.remove::<Tag>(e);
            }
        },
    );
    b.run(
        "add-remove/crabby",
        10,
        || {
            let mut w = crabby_ecs::World::new();
            let es: Vec<_> = (0..ADD_REMOVE_N).map(|i| w.spawn(ent(i))).collect();
            (w, es)
        },
        |(w, es)| {
            for &e in es.iter() {
                w.insert(e, Tag);
            }
            for &e in es.iter() {
                w.remove::<Tag>(e);
            }
        },
    );
    b.run(
        "add-remove/hecs",
        10,
        || {
            let mut w = hecs::World::new();
            let es: Vec<_> = (0..ADD_REMOVE_N).map(|i| w.spawn(ent(i))).collect();
            (w, es)
        },
        |(w, es)| {
            for &e in es.iter() {
                w.insert_one(e, Tag).unwrap();
            }
            for &e in es.iter() {
                w.remove_one::<Tag>(e).unwrap();
            }
        },
    );
    b.run(
        "add-remove/bevy",
        10,
        || {
            let mut w = bevy_ecs::world::World::new();
            let es: Vec<_> = (0..ADD_REMOVE_N).map(|i| w.spawn(ent(i)).id()).collect();
            (w, es)
        },
        |(w, es)| {
            for &e in es.iter() {
                w.entity_mut(e).insert(Tag);
            }
            for &e in es.iter() {
                w.entity_mut(e).remove::<Tag>();
            }
        },
    );

    // ---- despawn 1M -----------------------------------------------------------------
    b.run(
        "despawn/own",
        10,
        || {
            let mut w = own::World::new();
            let es: Vec<_> = (0..N).map(|i| w.spawn(ent(i))).collect();
            (w, es)
        },
        |(w, es)| {
            for &e in es.iter() {
                w.despawn(e);
            }
        },
    );
    b.run(
        "despawn/crabby",
        10,
        || {
            let mut w = crabby_ecs::World::new();
            let es: Vec<_> = (0..N).map(|i| w.spawn(ent(i))).collect();
            (w, es)
        },
        |(w, es)| {
            for &e in es.iter() {
                w.despawn(e);
            }
        },
    );
    b.run(
        "despawn/hecs",
        10,
        || {
            let mut w = hecs::World::new();
            let es: Vec<_> = (0..N).map(|i| w.spawn(ent(i))).collect();
            (w, es)
        },
        |(w, es)| {
            for &e in es.iter() {
                w.despawn(e).unwrap();
            }
        },
    );
    b.run(
        "despawn/bevy",
        10,
        || {
            let mut w = bevy_ecs::world::World::new();
            let es: Vec<_> = (0..N).map(|i| w.spawn(ent(i)).id()).collect();
            (w, es)
        },
        |(w, es)| {
            for &e in es.iter() {
                w.despawn(e);
            }
        },
    );
}
