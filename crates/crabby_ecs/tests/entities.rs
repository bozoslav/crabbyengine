//! Spawning, despawning, handles, single-entity access, archetype moves, drops.

mod common;

use std::sync::Arc;

use common::*;
use crabby_ecs::*;

#[test]
fn spawn_get_despawn() {
    let mut w = World::new();
    let a = w.spawn((Pos(1.0, 2.0), Vel(3.0, 4.0)));
    let b = w.spawn((Pos(5.0, 6.0),));
    assert_eq!(w.len(), 2);
    assert_eq!(w.get::<Pos>(a), Some(&Pos(1.0, 2.0)));
    assert_eq!(w.get::<Vel>(b), None);
    assert!(w.has::<Vel>(a) && !w.has::<Vel>(b));
    assert!(w.despawn(a));
    assert!(!w.despawn(a));
    assert!(!w.is_alive(a));
    assert_eq!(w.get::<Pos>(a), None);
    assert_eq!(w.len(), 1);
}

#[test]
fn stale_handles_stay_dead() {
    let mut w = World::new();
    let a = w.spawn((Pos(0.0, 0.0),));
    w.despawn(a);
    let c = w.spawn((Pos(7.0, 8.0),));
    assert_eq!(c.index(), a.index(), "slot is reused");
    assert_ne!(a, c);
    assert_eq!(w.get::<Pos>(a), None, "old handle must not see the new entity");
    assert_eq!(w.get::<Pos>(c), Some(&Pos(7.0, 8.0)));
    assert_eq!(Entity::from_bits(c.to_bits()), c);
}

#[test]
fn swap_remove_keeps_rows_valid() {
    let mut w = World::new();
    let es: Vec<_> = (0..10).map(|i| w.spawn((Pos(i as f32, 0.0),))).collect();
    w.despawn(es[2]);
    w.despawn(es[0]);
    w.despawn(es[9]);
    for (i, &e) in es.iter().enumerate() {
        if [0, 2, 9].contains(&i) {
            assert!(!w.is_alive(e));
        } else {
            assert_eq!(w.get::<Pos>(e), Some(&Pos(i as f32, 0.0)));
        }
    }
}

#[test]
fn insert_and_remove_move_between_archetypes() {
    let mut w = World::new();
    let es: Vec<_> = (0..5).map(|i| w.spawn((Pos(i as f32, 0.0),))).collect();
    assert!(w.insert(es[1], Vel(1.0, 1.0)));
    assert!(w.insert(es[3], Tag));
    assert_eq!(w.get::<Vel>(es[1]), Some(&Vel(1.0, 1.0)));
    for (i, &e) in es.iter().enumerate() {
        assert_eq!(w.get::<Pos>(e), Some(&Pos(i as f32, 0.0)));
    }
    assert_eq!(w.remove::<Vel>(es[1]), Some(Vel(1.0, 1.0)));
    assert_eq!(w.remove::<Vel>(es[1]), None);
    assert_eq!(w.get::<Pos>(es[1]), Some(&Pos(1.0, 0.0)));
    w.insert(es[0], Vel(9.0, 9.0));
    w.insert(es[0], Vel(2.0, 2.0)); // replaces
    assert_eq!(w.get::<Vel>(es[0]), Some(&Vel(2.0, 2.0)));
    // Removing the last component leaves an empty entity that is still alive.
    let e = w.spawn((Tag,));
    assert_eq!(w.remove::<Tag>(e), Some(Tag));
    assert!(w.is_alive(e));
    // Dead entities.
    w.despawn(es[4]);
    assert!(!w.insert(es[4], Tag));
    assert_eq!(w.remove::<Pos>(es[4]), None);
}

#[test]
fn get_mut_changes_value() {
    let mut w = World::new();
    let e = w.spawn((Pos(0.0, 0.0),));
    w.get_mut::<Pos>(e).unwrap().0 = 5.0;
    assert_eq!(w.get::<Pos>(e), Some(&Pos(5.0, 0.0)));
}

#[test]
#[should_panic(expected = "same component twice")]
fn duplicate_bundle_panics() {
    World::new().spawn((Pos(0.0, 0.0), Pos(1.0, 1.0)));
}

#[test]
fn drops_run_exactly_once() {
    let rc = Arc::new(());
    {
        let mut w = World::new();
        let a = w.spawn((rc.clone(), Pos(0.0, 0.0)));
        let b = w.spawn((rc.clone(), Pos(0.0, 0.0)));
        let c = w.spawn((rc.clone(),));
        assert_eq!(Arc::strong_count(&rc), 4);
        w.despawn(a);
        assert_eq!(Arc::strong_count(&rc), 3);
        w.insert(b, Vel(0.0, 0.0)); // moves the Arc to another archetype
        assert_eq!(Arc::strong_count(&rc), 3);
        let taken = w.remove::<Arc<()>>(b).unwrap();
        assert_eq!(Arc::strong_count(&rc), 3);
        drop(taken);
        assert_eq!(Arc::strong_count(&rc), 2);
        w.insert(c, rc.clone()); // replace drops the old one
        assert_eq!(Arc::strong_count(&rc), 2);
        let id = w.component_id::<Arc<()>>().unwrap();
        assert!(w.remove_dynamic(c, id));
        assert_eq!(Arc::strong_count(&rc), 1);
        w.spawn((rc.clone(),));
    }
    assert_eq!(Arc::strong_count(&rc), 1, "dropping the world drops its components");
}

#[test]
fn zero_sized_components() {
    let mut w = World::new();
    let es: Vec<_> = (0..20).map(|_| w.spawn((Tag, Pos(1.0, 1.0)))).collect();
    w.despawn(es[3]);
    assert_eq!(w.query::<(&Tag, &Pos)>().count(), 19);
    let e = w.spawn((Tag,));
    assert_eq!(w.get::<Tag>(e), Some(&Tag));
}

#[test]
fn spawn_batch_and_many_archetypes() {
    let mut w = World::new();
    w.spawn_batch((0..1000).map(|i| (Pos(i as f32, 0.0), Vel(1.0, 0.0))));
    assert_eq!(w.len(), 1000);
    let es: Vec<_> = w.query::<Entity>().iter().collect();
    for (i, &e) in es.iter().enumerate() {
        if i % 2 == 0 {
            w.insert(e, Tag);
        }
        if i % 7 == 0 {
            w.despawn(e);
        }
    }
    assert_eq!(w.query::<&Pos>().count(), 1000 - 143);
    assert_eq!(w.query::<&Pos>().with::<Tag>().count(), 500 - 72);
}

#[test]
fn world_is_send_and_sync() {
    fn check<T: Send + Sync>() {}
    fn send<T: Send>() {}
    check::<World>();
    send::<Commands>();
}
