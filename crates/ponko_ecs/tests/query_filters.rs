//! Query filters (`With`, `Without`, `Or`), larger tuples and aliasing edge cases.

mod common;

use common::*;
use ponko_ecs::*;

struct Player;
struct Enemy;
struct Dead;
struct Ghost;
#[derive(Debug, PartialEq, Clone, Copy)]
struct Health(i32);

/// One entity per combination of the four markers, with `Health(bits)`:
/// bit 0 `Player`, 1 `Enemy`, 2 `Dead`, 3 `Ghost`.
fn world_of_combinations() -> World {
    let mut w = World::new();
    for bits in 0..16 {
        let e = w.spawn((Health(bits),));
        if bits & 1 != 0 {
            w.insert(e, Player);
        }
        if bits & 2 != 0 {
            w.insert(e, Enemy);
        }
        if bits & 4 != 0 {
            w.insert(e, Dead);
        }
        if bits & 8 != 0 {
            w.insert(e, Ghost);
        }
    }
    w
}

fn matched(mut q: Query<'_, &Health>) -> Vec<i32> {
    let mut out: Vec<i32> = q.iter().map(|h| h.0).collect();
    out.sort();
    out
}

fn expected(pred: impl Fn(i32) -> bool) -> Vec<i32> {
    (0..16).filter(|&b| pred(b)).collect()
}

const PLAYER: i32 = 1;
const ENEMY: i32 = 2;
const DEAD: i32 = 4;
const GHOST: i32 = 8;

#[test]
fn typed_with_and_without_match_the_builder_methods() {
    let mut w = world_of_combinations();
    let typed = matched(w.query::<&Health>().filter::<(With<Player>, Without<Dead>)>());
    let builder = matched(w.query::<&Health>().filter::<With<Player>>().filter::<Without<Dead>>());
    assert_eq!(typed, builder);
    assert_eq!(typed, expected(|b| b & PLAYER != 0 && b & DEAD == 0));
}

#[test]
fn or_matches_any_of_its_terms() {
    let mut w = world_of_combinations();
    let got = matched(w.query::<&Health>().filter::<Or<(With<Player>, With<Enemy>)>>());
    assert_eq!(got, expected(|b| b & (PLAYER | ENEMY) != 0));
}

#[test]
fn or_mixes_with_and_without_terms() {
    let mut w = world_of_combinations();
    // Alive, or a ghost.
    let got = matched(w.query::<&Health>().filter::<Or<(Without<Dead>, With<Ghost>)>>());
    assert_eq!(got, expected(|b| b & DEAD == 0 || b & GHOST != 0));
}

#[test]
fn or_combines_with_other_filters_as_and() {
    let mut w = world_of_combinations();
    type Visible = (Or<(With<Player>, With<Enemy>)>, Or<(Without<Dead>, With<Ghost>)>);
    let got = matched(w.query::<&Health>().filter::<Visible>().filter::<Without<Ghost>>());
    assert_eq!(
        got,
        expected(|b| b & (PLAYER | ENEMY) != 0 && (b & DEAD == 0 || b & GHOST != 0) && b & GHOST == 0)
    );
}

#[test]
fn nested_or_flattens() {
    let mut w = world_of_combinations();
    let got = matched(
        w.query::<&Health>()
            .filter::<Or<(With<Player>, Or<(With<Enemy>, With<Ghost>)>)>>(),
    );
    assert_eq!(got, expected(|b| b & (PLAYER | ENEMY | GHOST) != 0));
}

#[test]
fn repeated_filter_calls_accumulate() {
    let mut w = world_of_combinations();
    let got = matched(
        w.query::<&Health>()
            .filter::<With<Player>>()
            .filter::<Or<(With<Enemy>, With<Dead>)>>()
            .filter::<Without<Ghost>>(),
    );
    assert_eq!(
        got,
        expected(|b| b & PLAYER != 0 && b & (ENEMY | DEAD) != 0 && b & GHOST == 0)
    );
}

#[test]
fn contradictory_filters_match_nothing() {
    let mut w = world_of_combinations();
    assert_eq!(
        w.query::<&Health>()
            .filter::<With<Player>>()
            .filter::<Without<Player>>()
            .count(),
        0
    );
}

#[test]
fn filters_apply_to_every_way_of_running_a_query() {
    let mut w = world_of_combinations();
    let want = expected(|b| b & (PLAYER | ENEMY) != 0);

    let mut q = w
        .query::<(Entity, &Health)>()
        .filter::<Or<(With<Player>, With<Enemy>)>>();
    assert_eq!(q.count(), want.len());
    let mut seen = Vec::new();
    q.for_each(|(_, h)| seen.push(h.0));
    seen.sort();
    assert_eq!(seen, want);
    let entities: Vec<Entity> = q.iter().map(|(e, _)| e).collect();

    let all: Vec<(Entity, i32)> = w.query::<(Entity, &Health)>().iter().map(|(e, h)| (e, h.0)).collect();
    let mut q = w.query::<&Health>().filter::<Or<(With<Player>, With<Enemy>)>>();
    for (e, bits) in all {
        assert_eq!(q.get(e).is_some(), want.contains(&bits), "get({bits})");
        assert_eq!(entities.contains(&e), want.contains(&bits));
    }
}

#[test]
fn filter_on_a_fetched_mutable_component_is_allowed() {
    let mut w = world_of_combinations();
    w.query::<&mut Health>()
        .filter::<Or<(With<Health>, With<Dead>)>>()
        .filter::<With<Health>>()
        .for_each(|h| h.0 += 100);
    assert!(w.query::<&Health>().iter().all(|h| h.0 >= 100));
}

#[test]
fn more_than_four_with_and_without_filters_fit() {
    struct C0;
    struct C1;
    struct C2;
    struct C3;
    struct C4;
    struct C5;
    let mut w = World::new();
    w.spawn((Pos(0.0, 0.0), C0, C1, C2, C3, C4, C5));
    w.spawn((Pos(0.0, 0.0), C0, C1, C2, C3, C4));
    w.spawn((Pos(0.0, 0.0), Vel(0.0, 0.0), Tag, Frozen));
    let n = w
        .query::<&Pos>()
        .filter::<With<C0>>()
        .filter::<With<C1>>()
        .filter::<With<C2>>()
        .filter::<With<C3>>()
        .filter::<With<C4>>()
        .filter::<With<C5>>()
        .count();
    assert_eq!(n, 1);
    let n = w
        .query::<&Pos>()
        .filter::<Without<C5>>()
        .filter::<Without<Vel>>()
        .filter::<Without<Tag>>()
        .filter::<Without<Frozen>>()
        .filter::<Without<Health>>()
        .count();
    assert_eq!(n, 1);
}

#[test]
fn repeated_with_filters_use_one_slot() {
    let mut w = World::new();
    w.spawn((Pos(0.0, 0.0), Tag));
    let mut q = w.query::<&Pos>();
    for _ in 0..20 {
        q = q.filter::<With<Tag>>().filter::<Without<Frozen>>();
    }
    assert_eq!(q.count(), 1);
}

#[test]
#[should_panic(expected = "at most 8 `with` filters per query")]
fn too_many_with_filters_panic() {
    struct C0;
    struct C1;
    struct C2;
    struct C3;
    struct C4;
    struct C5;
    struct C6;
    struct C7;
    struct C8;
    let mut w = World::new();
    let _ = w
        .query::<&Pos>()
        .filter::<(
            With<C0>,
            With<C1>,
            With<C2>,
            With<C3>,
            With<C4>,
            With<C5>,
            With<C6>,
            With<C7>,
        )>()
        .filter::<With<C8>>();
}

#[test]
#[should_panic(expected = "at most 4 `Or` filters per query")]
fn too_many_or_filters_panic() {
    let mut w = World::new();
    let _ = w
        .query::<&Pos>()
        .filter::<Or<(With<Player>,)>>()
        .filter::<Or<(With<Enemy>,)>>()
        .filter::<Or<(With<Dead>,)>>()
        .filter::<Or<(With<Ghost>,)>>()
        .filter::<Or<(With<Tag>,)>>();
}

#[test]
#[should_panic(expected = "at most 16 components across a query's `Or` filters")]
fn too_many_or_terms_panic() {
    type Four = Or<(With<Player>, With<Enemy>, With<Dead>, With<Ghost>)>;
    type Five = Or<(With<Player>, With<Enemy>, With<Dead>, With<Ghost>, With<Tag>)>;
    let mut w = World::new();
    let _ = w.query::<&Pos>().filter::<(Four, Four, Four, Five)>();
}

#[test]
fn twelve_element_tuples_work() {
    struct C(u8);
    struct D(u8);
    struct E(u8);
    struct F(u8);
    struct G(u8);
    struct H(u8);
    struct I(u8);
    struct J(u8);
    struct K(u8);
    let mut w = World::new();
    let e = w.spawn((
        Pos(1.0, 0.0),
        Vel(2.0, 0.0),
        Health(3),
        C(4),
        D(5),
        E(6),
        F(7),
        G(8),
        H(9),
        I(10),
        J(11),
    ));
    w.insert(e, K(12));
    let mut sum = 0;
    w.query::<(
        Entity,
        &mut Pos,
        &Vel,
        &Health,
        &C,
        &D,
        &E,
        &F,
        &G,
        &H,
        Option<&I>,
        (&J, &K),
    )>()
    .for_each(|(_, p, v, h, c, d, e, f, g, hh, i, (j, k))| {
        p.0 += v.0;
        sum = h.0 + (c.0 + d.0 + e.0 + f.0 + g.0 + hh.0 + i.unwrap().0 + j.0 + k.0) as i32;
    });
    assert_eq!(sum, 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12);
    assert_eq!(w.get::<Pos>(e), Some(&Pos(3.0, 0.0)));
}

#[test]
fn nested_tuples_go_beyond_twelve() {
    let mut w = World::new();
    w.spawn((Pos(1.0, 0.0), Vel(2.0, 0.0), Health(3)));
    type Twelve<'a> = (
        &'a Pos,
        &'a Pos,
        &'a Pos,
        &'a Pos,
        &'a Pos,
        &'a Pos,
        &'a Pos,
        &'a Pos,
        &'a Pos,
        &'a Pos,
        &'a Pos,
        &'a Pos,
    );
    let n = w.query::<(Twelve<'_>, (&Vel, &Health))>().count();
    assert_eq!(n, 1);
}

// Aliasing: optional mutable access counts as mutable access.

#[test]
#[should_panic(expected = "twice")]
fn optional_mutable_and_shared_alias() {
    let mut w = World::new();
    let _ = w.query::<(Option<&mut Pos>, &Pos)>();
}

#[test]
#[should_panic(expected = "twice")]
fn shared_and_optional_mutable_alias() {
    let mut w = World::new();
    let _ = w.query::<(&Pos, Option<&mut Pos>)>();
}

#[test]
#[should_panic(expected = "twice")]
fn two_optional_mutables_alias() {
    let mut w = World::new();
    let _ = w.query::<(Option<&mut Pos>, Option<&mut Pos>)>();
}

#[test]
#[should_panic(expected = "twice")]
fn aliasing_inside_nested_tuples_is_caught() {
    let mut w = World::new();
    let _ = w.query::<((&Vel, &mut Pos), (Entity, (Option<&Pos>,)))>();
}

#[test]
fn optional_shared_and_shared_do_not_alias() {
    let mut w = World::new();
    w.spawn((Pos(1.0, 0.0),));
    w.spawn((Vel(1.0, 0.0),));
    let mut seen = 0;
    w.query::<(Option<&Pos>, &Pos)>().for_each(|(a, b)| {
        assert_eq!(a, Some(b));
        seen += 1;
    });
    assert_eq!(seen, 1);
}

#[test]
fn mutable_access_to_different_components_is_fine() {
    let mut w = World::new();
    w.spawn((Pos(1.0, 0.0), Vel(2.0, 0.0)));
    w.query::<(&mut Pos, Option<&mut Vel>, Entity, Entity)>()
        .for_each(|(p, v, a, b)| {
            assert_eq!(a, b);
            p.0 += v.unwrap().0;
        });
    assert_eq!(w.query::<&Pos>().iter().next(), Some(&Pos(3.0, 0.0)));
}
