//! Isolated matching-cost matrix: one entity per layout, sparse (1/16)
//! matches, query reconstruction included. Setup and five warmups are untimed.
use crate::Bench;
use ponko_ecs::{With, Without, World};
use std::hint::black_box;

struct Value(u64);
struct Marker<const N: usize>;

fn world(layouts: usize) -> World {
    let mut w = World::new();
    for bits in 0..layouts {
        let e = w.spawn((Value(bits as u64),));
        macro_rules! marker {
            ($n:literal) => {
                if bits & (1 << $n) != 0 {
                    w.insert(e, Marker::<$n>);
                }
            };
        }
        marker!(0);
        marker!(1);
        marker!(2);
        marker!(3);
        marker!(4);
        marker!(5);
        marker!(6);
        marker!(7);
        marker!(8);
        marker!(9);
    }
    w
}

fn hint<'w>(q: ponko_ecs::Query<'w, &'static Value>, strategy: &str) -> ponko_ecs::Query<'w, &'static Value> {
    match strategy {
        "scan" => q.uncached(),
        "auto" => q,
        _ => q.cached(),
    }
}

pub fn run(b: &mut Bench) {
    for layouts in [1, 8, 32, 64, 128, 256, 1024] {
        // `scan`: the index off for the query. `auto`: the world's default
        // policy decides. `cached`: the query asks for the index.
        for strategy in ["scan", "auto", "cached"] {
            b.run(
                &format!("query-cache/{layouts}/cold/{strategy}"),
                100,
                || world(layouts),
                |w| {
                    let q = w
                        .query::<&Value>()
                        .filter::<With<Marker<0>>>()
                        .filter::<With<Marker<1>>>()
                        .filter::<Without<Marker<2>>>()
                        .filter::<Without<Marker<3>>>();
                    let mut q = hint(q, strategy);
                    black_box(q.count());
                },
            );
            for frequency in [1, 100, 1000] {
                for mode in ["count", "each", "iter", "dense"] {
                    b.steady(
                        &format!("query-cache/{layouts}/{frequency}/{mode}/{strategy}"),
                        100,
                        || world(layouts),
                        |w| {
                            for _ in 0..frequency {
                                let q = w.query::<&Value>();
                                let q = if mode == "dense" {
                                    q
                                } else {
                                    q.filter::<With<Marker<0>>>()
                                        .filter::<With<Marker<1>>>()
                                        .filter::<Without<Marker<2>>>()
                                        .filter::<Without<Marker<3>>>()
                                };
                                let mut q = hint(q, strategy);
                                match mode {
                                    "count" | "dense" => {
                                        black_box(q.count());
                                    }
                                    "each" => q.for_each(|v| {
                                        black_box(v.0);
                                    }),
                                    _ => {
                                        black_box(q.iter().map(|v| v.0).sum::<u64>());
                                    }
                                }
                            }
                        },
                    );
                }
            }
        }
    }
}
