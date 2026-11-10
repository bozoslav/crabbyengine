//! Many small archetypes, and building queries over them, in ponko_ecs and
//! bevy_ecs.
//!
//! The world: 1024 archetypes of 16 entities each (16384 in total). Every
//! entity has `(Pos, Vel, Spr)` plus a different subset of 10 marker
//! components. Games with many component combinations look like this: lots of
//! archetypes, few entities in each, so per-archetype costs dominate.
//!
//! - `archetypes-1024/*`: `pos += vel * dt` over all of them, with the query
//!   built once (bevy keeps a `QueryState`; ponko builds a `Query` per run,
//!   which is how systems use it).
//! - `query-build-x1000/*`: build the `(&mut Pos, &Vel)` query 1000 times.
//!   bevy matches archetypes when it builds a `QueryState`; ponko resolves
//!   component ids only and matches while iterating, so the two numbers measure
//!   different work. Compare them together with `query-build+count`.
//! - `query-build+count-x1000/*`: build the query and count its matches, 1000
//!   times. This is the same work in both: resolve, match, read lengths.

use bevy_ecs::component::Component;
use std::hint::black_box;

use crate::{Bench, DT, Pos, Vel, ent};

/// Marker components: entity `i` has `Mk<k>` when bit `k` of its archetype index is set.
#[derive(Component, Clone, Copy, Default)]
struct Mk<const K: usize>;

pub const MARKERS: usize = 10;
pub const ARCHETYPES: usize = 1 << MARKERS;
pub const PER_ARCHETYPE: usize = 16;
pub const ENTITIES: usize = ARCHETYPES * PER_ARCHETYPE;
const QUERY_BUILDS: usize = 1000;

/// Runs `$body` with `$m` naming the marker type for bit `$k`.
macro_rules! with_marker {
    ($k:expr, |$m:ident| $body:expr) => {
        match $k {
            0 => {
                type $m = Mk<0>;
                $body
            }
            1 => {
                type $m = Mk<1>;
                $body
            }
            2 => {
                type $m = Mk<2>;
                $body
            }
            3 => {
                type $m = Mk<3>;
                $body
            }
            4 => {
                type $m = Mk<4>;
                $body
            }
            5 => {
                type $m = Mk<5>;
                $body
            }
            6 => {
                type $m = Mk<6>;
                $body
            }
            7 => {
                type $m = Mk<7>;
                $body
            }
            8 => {
                type $m = Mk<8>;
                $body
            }
            9 => {
                type $m = Mk<9>;
                $body
            }
            _ => unreachable!("only {MARKERS} markers"),
        }
    };
}

/// The marker bits of entity `i`. Consecutive entities land in different archetypes.
fn bits(i: usize) -> impl Iterator<Item = usize> {
    (0..MARKERS).filter(move |k| (i % ARCHETYPES) & (1 << k) != 0)
}

// Markers are added in ascending bit order, so every intermediate component
// set is itself one of the 1024 final sets: no extra archetypes appear.

fn ponko_world() -> ponko_ecs::World {
    let mut w = ponko_ecs::World::new();
    for i in 0..ENTITIES {
        let e = w.spawn(ent(i));
        for k in bits(i) {
            with_marker!(k, |M| w.insert(e, M::default()));
        }
    }
    w
}

fn bevy_world() -> bevy_ecs::world::World {
    let mut w = bevy_ecs::world::World::new();
    for i in 0..ENTITIES {
        let mut e = w.spawn(ent(i));
        for k in bits(i) {
            with_marker!(k, |M| e.insert(M::default()));
        }
    }
    w
}

fn step_ponko(w: &mut ponko_ecs::World, dt: f32) {
    w.query::<(&mut Pos, &Vel)>().for_each(|(p, v)| {
        p.x += v.x * dt;
        p.y += v.y * dt;
    });
}

type BevyMove = bevy_ecs::query::QueryState<(&'static mut Pos, &'static Vel)>;

fn step_bevy(w: &mut bevy_ecs::world::World, q: &mut BevyMove, dt: f32) {
    q.iter_mut(w).for_each(|(mut p, v)| {
        p.x += v.x * dt;
        p.y += v.y * dt;
    });
}

fn count_ponko(w: &mut ponko_ecs::World) -> usize {
    w.query::<(&mut Pos, &Vel)>().count()
}

fn count_bevy(w: &mut bevy_ecs::world::World) -> usize {
    w.query::<(&mut Pos, &Vel)>().iter_mut(w).len()
}

pub fn run(b: &mut Bench) {
    b.steady_with_bytes(
        "archetypes-1024/ponko",
        200,
        ponko_world,
        |w| step_ponko(w, black_box(DT)),
        |w| Some(w.allocated_bytes()),
    );
    b.steady(
        "archetypes-1024/bevy",
        200,
        || {
            let mut w = bevy_world();
            let q = w.query::<(&mut Pos, &Vel)>();
            (w, q)
        },
        |(w, q)| step_bevy(w, q, black_box(DT)),
    );

    b.steady(&format!("query-build-x{QUERY_BUILDS}/ponko"), 100, ponko_world, |w| {
        for _ in 0..QUERY_BUILDS {
            drop(black_box(w.query::<(&mut Pos, &Vel)>()));
        }
    });
    b.steady(&format!("query-build-x{QUERY_BUILDS}/bevy"), 100, bevy_world, |w| {
        for _ in 0..QUERY_BUILDS {
            black_box(w.query::<(&mut Pos, &Vel)>());
        }
    });

    b.steady(
        &format!("query-build+count-x{QUERY_BUILDS}/ponko"),
        100,
        ponko_world,
        |w| {
            for _ in 0..QUERY_BUILDS {
                black_box(count_ponko(w));
            }
        },
    );
    b.steady(
        &format!("query-build+count-x{QUERY_BUILDS}/bevy"),
        100,
        bevy_world,
        |w| {
            for _ in 0..QUERY_BUILDS {
                black_box(count_bevy(w));
            }
        },
    );
}

// Correctness of the workloads, checked apart from their timing: both
// libraries must build the same world and do the same work on it.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worlds_have_1024_archetypes_of_16_entities() {
        let w = ponko_world();
        let sizes: Vec<_> = w
            .raw_ref()
            .archetypes()
            .iter()
            .map(|a| a.len())
            .filter(|&n| n > 0)
            .collect();
        assert_eq!(sizes.len(), ARCHETYPES);
        assert!(sizes.iter().all(|&n| n == PER_ARCHETYPE));
        assert_eq!(w.len(), ENTITIES);

        // bevy also keeps entities of its own, so only count archetypes with `Pos`.
        let w = bevy_world();
        let pos = w.component_id::<Pos>().unwrap();
        let sizes: Vec<_> = w
            .archetypes()
            .iter()
            .filter(|a| a.contains(pos))
            .map(|a| a.len())
            .filter(|&n| n > 0)
            .collect();
        assert_eq!(sizes.len(), ARCHETYPES);
        assert!(sizes.iter().all(|&n| n == PER_ARCHETYPE as u32));
    }

    #[test]
    fn queries_count_every_entity_in_both() {
        assert_eq!(count_ponko(&mut ponko_world()), ENTITIES);
        assert_eq!(count_bevy(&mut bevy_world()), ENTITIES);
    }

    #[test]
    fn steps_move_every_entity_the_same() {
        let mut pw = ponko_world();
        let mut bw = bevy_world();
        let mut q = bw.query::<(&mut Pos, &Vel)>();
        for _ in 0..3 {
            step_ponko(&mut pw, DT);
            step_bevy(&mut bw, &mut q, DT);
        }
        let mut a: Vec<_> = pw
            .query::<&Pos>()
            .iter()
            .map(|p| (p.x.to_bits(), p.y.to_bits()))
            .collect();
        let mut b: Vec<_> = bw
            .query::<&Pos>()
            .iter(&bw)
            .map(|p| (p.x.to_bits(), p.y.to_bits()))
            .collect();
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(a, b);
        // Moved by 3 steps of (1.0, 0.5) * dt from (i, 0).
        let y = (0.0f32 + 0.5 * DT + 0.5 * DT + 0.5 * DT).to_bits();
        assert!(a.iter().all(|&(_, py)| py == y));
        assert_eq!(a.len(), ENTITIES);
    }
}
