//! Queries: fetch kinds, filters, iterators, aliasing rules.

mod common;

use common::*;
use ponko_ecs::*;

#[test]
fn query_matches_across_archetypes() {
    let mut w = World::new();
    w.spawn((Pos(0.0, 0.0), Vel(1.0, 0.0)));
    w.spawn((Pos(0.0, 0.0), Vel(1.0, 0.0), Tag));
    w.spawn((Pos(0.0, 0.0),));
    w.query::<(&mut Pos, &Vel)>().for_each(|(p, v)| p.0 += v.0);
    let sum: f32 = w.query::<&Pos>().iter().map(|p| p.0).sum();
    assert_eq!(sum, 2.0);
    assert_eq!(w.query::<&Pos>().count(), 3);
    assert_eq!(w.query::<(&Pos, &Vel)>().count(), 2);
}

#[test]
fn iter_and_for_each_agree() {
    let mut w = World::new();
    for i in 0..100 {
        let e = w.spawn((Pos(i as f32, 0.0),));
        if i % 3 == 0 {
            w.insert(e, Tag);
        }
        if i % 5 == 0 {
            w.insert(e, Vel(0.0, 0.0));
        }
    }
    let mut a = Vec::new();
    w.query::<(Entity, &Pos)>().for_each(|(e, p)| a.push((e, p.0)));
    let b: Vec<_> = w.query::<(Entity, &Pos)>().iter().map(|(e, p)| (e, p.0)).collect();
    assert_eq!(a, b);
    assert_eq!(a.len(), 100);
    for (e, x) in a {
        assert_eq!(w.get::<Pos>(e).unwrap().0, x);
    }
}

#[test]
fn filters() {
    let mut w = World::new();
    w.spawn((Pos(0.0, 0.0), Vel(1.0, 0.0)));
    w.spawn((Pos(0.0, 0.0), Vel(1.0, 0.0), Frozen));
    w.spawn((Pos(0.0, 0.0), Tag));
    w.query::<(&mut Pos, &Vel)>()
        .without::<Frozen>()
        .for_each(|(p, v)| p.0 += v.0);
    assert_eq!(w.query::<&Pos>().iter().filter(|p| p.0 == 1.0).count(), 1);
    assert_eq!(w.query::<&Pos>().with::<Tag>().count(), 1);
    assert_eq!(w.query::<&Pos>().with::<Vel>().without::<Frozen>().count(), 1);
    assert_eq!(w.query::<&Pos>().without::<Vel>().iter().count(), 1);
}

#[test]
fn optional_components() {
    let mut w = World::new();
    w.spawn((Pos(1.0, 0.0), Vel(5.0, 0.0)));
    w.spawn((Pos(2.0, 0.0),));
    let mut seen = Vec::new();
    w.query::<(&Pos, Option<&Vel>)>()
        .for_each(|(p, v)| seen.push((p.0, v.map(|v| v.0))));
    seen.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(seen, vec![(1.0, Some(5.0)), (2.0, None)]);
    w.query::<Option<&mut Vel>>().for_each(|v| {
        if let Some(v) = v {
            v.0 = 0.0;
        }
    });
    assert_eq!(w.query::<&Vel>().iter().next(), Some(&Vel(0.0, 0.0)));
}

#[test]
fn query_get_single_entity() {
    let mut w = World::new();
    let a = w.spawn((Pos(1.0, 0.0), Vel(2.0, 0.0)));
    let b = w.spawn((Pos(3.0, 0.0),));
    let mut q = w.query::<(&mut Pos, &Vel)>();
    let (p, v) = q.get(a).unwrap();
    p.0 += v.0;
    assert!(q.get(b).is_none());
    assert_eq!(w.get::<Pos>(a), Some(&Pos(3.0, 0.0)));
}

#[test]
#[should_panic(expected = "twice")]
fn aliasing_query_panics() {
    let mut w = World::new();
    let _ = w.query::<(&mut Pos, &Pos)>();
}

#[test]
fn same_component_twice_read_only_is_fine() {
    let mut w = World::new();
    w.spawn((Pos(1.0, 0.0),));
    w.query::<(&Pos, &Pos)>().for_each(|(a, b)| assert_eq!(a, b));
}
