//! Equal-work typed-filter overhead: 1M rows over 16 archetypes.
//! Every filter matches every row, keeping iteration work identical.

use crate::{Bench, DT, M0, M1, M2, M3, N, Pos, Vel, ent};
use ponko_ecs::{Or, With, Without, World};
use std::hint::black_box;

fn world() -> World {
    let mut w = World::new();
    for i in 0..N {
        let e = w.spawn(ent(i));
        if i & 1 != 0 {
            w.insert(e, M0);
        }
        if i & 2 != 0 {
            w.insert(e, M1);
        }
        if i & 4 != 0 {
            w.insert(e, M2);
        }
        if i & 8 != 0 {
            w.insert(e, M3);
        }
    }
    w
}

pub fn run(b: &mut Bench) {
    b.steady("query-filter/plain", 200, world, |w| {
        let dt = black_box(DT);
        w.query::<(&mut Pos, &Vel)>().for_each(|(p, v)| {
            p.x += v.x * dt;
            p.y += v.y * dt;
        });
    });
    b.steady("query-filter/typed", 200, world, |w| {
        let dt = black_box(DT);
        w.query::<(&mut Pos, &Vel)>().filter::<With<Vel>>().for_each(|(p, v)| {
            p.x += v.x * dt;
            p.y += v.y * dt;
        });
    });
    b.steady("query-filter/nested-or-and", 200, world, |w| {
        let dt = black_box(DT);
        w.query::<(&mut Pos, &Vel)>()
            .filter::<Or<((With<Pos>, With<Vel>), Without<M0>)>>()
            .for_each(|(p, v)| {
                p.x += v.x * dt;
                p.y += v.y * dt;
            });
    });
}
