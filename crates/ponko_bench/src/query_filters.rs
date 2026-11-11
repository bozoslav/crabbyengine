//! Equal-work typed-filter overhead: 1M rows over 16 archetypes.
//! Every filter matches every row, keeping iteration work identical.

use crate::{Bench, DT, M0, M1, M2, M3, N, Pos, Vel, ent};
use ponko_ecs::{Chunk, ChunkQuery, FilterShape, FilterTree, Or, With, Without, World, compile_tree};
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

    // The path the C API takes: raw column pointers per archetype, with and
    // without a runtime filter. `c-plain` is `ponko_query_with` without a
    // filter, so it shows what the filter hook costs a query that has none.
    b.steady("query-filter/c-plain", 200, world, |w| {
        let ids = [w.register::<Pos>(), w.register::<Vel>()];
        let dt = black_box(DT);
        w.query_chunks(
            ChunkQuery {
                writes: 1,
                ..ChunkQuery::new(&ids)
            },
            |chunk| step(&chunk, dt),
        );
    });
    // The filtered C path compiles its filter on every call: the tree of
    // component numbers is walked into inline storage and each number is
    // looked up in the world (`ponko_query_filtered`). That is timed here too.
    let (pos, vel, m0) = {
        let mut w = world();
        (
            w.register::<Pos>().index() as u32,
            w.register::<Vel>().index() as u32,
            w.register::<M0>().index() as u32,
        )
    };
    let with = Tree::With(vel);
    let nested = Tree::Or(vec![
        Tree::All(vec![Tree::With(pos), Tree::With(vel)]),
        Tree::Without(m0),
    ]);
    for (name, tree) in [
        ("query-filter/c-filtered", with),
        ("query-filter/c-filtered-nested", nested),
    ] {
        b.steady(name, 200, world, |w| {
            let ids = [w.register::<Pos>(), w.register::<Vel>()];
            let dt = black_box(DT);
            let filter = compile_tree(&tree, |raw| w.component_at(raw as usize)).unwrap();
            w.query_chunks_filtered(
                ChunkQuery {
                    writes: 1,
                    ..ChunkQuery::new(&ids)
                },
                &filter,
                |chunk| step(&chunk, dt),
            );
        });
    }
}

/// A filter as the C API holds it: a tree of component numbers.
enum Tree {
    With(u32),
    Without(u32),
    All(Vec<Tree>),
    Or(Vec<Tree>),
}

impl FilterTree for Tree {
    fn shape(&self) -> FilterShape<'_, Self> {
        match self {
            Tree::With(id) => FilterShape::With(*id),
            Tree::Without(id) => FilterShape::Without(*id),
            Tree::All(parts) => FilterShape::All(parts),
            Tree::Or(arms) => FilterShape::Or(arms),
        }
    }
}

/// `pos += vel * dt` over a chunk's two raw columns.
fn step(chunk: &Chunk<'_>, dt: f32) {
    let (pos, vel) = (chunk.columns[0].cast::<Pos>(), chunk.columns[1].cast::<Vel>());
    for i in 0..chunk.entities.len() {
        // SAFETY: both columns hold `entities.len()` rows of these types, and `Pos` is only written.
        unsafe {
            let (p, v) = (&mut *pos.add(i), &*vel.add(i));
            p.x += v.x * dt;
            p.y += v.y * dt;
        }
    }
}
