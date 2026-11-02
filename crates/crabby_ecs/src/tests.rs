use std::sync::Arc;

use crate::*;

#[derive(Debug, PartialEq, Clone, Copy)]
struct Pos(f32, f32);
#[derive(Debug, PartialEq, Clone, Copy)]
struct Vel(f32, f32);
#[derive(Debug, PartialEq)]
struct Tag;
struct Frozen;

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
    w.query::<(&mut Pos, &Vel)>().without::<Frozen>().for_each(|(p, v)| p.0 += v.0);
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
    w.query::<(&Pos, Option<&Vel>)>().for_each(|(p, v)| seen.push((p.0, v.map(|v| v.0))));
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
    w.query::<(&mut Pos, &Pos)>();
}

#[test]
fn same_component_twice_read_only_is_fine() {
    let mut w = World::new();
    w.spawn((Pos(1.0, 0.0),));
    w.query::<(&Pos, &Pos)>().for_each(|(a, b)| assert_eq!(a, b));
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
fn resources() {
    struct Score(u32);
    let mut w = World::new();
    assert!(w.get_resource::<Score>().is_none());
    w.insert_resource(Score(1));
    w.resource_mut::<Score>().0 += 1;
    assert_eq!(w.resource::<Score>().0, 2);
    w.insert_resource(Score(10));
    assert_eq!(w.remove_resource::<Score>().map(|s| s.0), Some(10));
    assert!(w.get_resource::<Score>().is_none());
}

#[test]
#[should_panic(expected = "insert_resource")]
fn missing_resource_panics_helpfully() {
    struct Missing;
    World::new().resource::<Missing>();
}

#[test]
fn commands_apply_in_order() {
    let mut w = World::new();
    let es: Vec<_> = (0..10).map(|i| w.spawn((Pos(i as f32, 0.0),))).collect();
    let mut cmd = Commands::new();
    w.query::<(Entity, &Pos)>().for_each(|(e, p)| {
        if p.0 < 5.0 {
            cmd.despawn(e);
        } else {
            cmd.insert(e, Tag);
        }
    });
    cmd.spawn((Pos(100.0, 0.0),));
    cmd.remove::<Tag>(es[9]);
    cmd.add(|w| w.insert_resource(42u32));
    assert_eq!(cmd.len(), 13);
    w.apply(&mut cmd);
    assert!(cmd.is_empty());
    assert_eq!(w.len(), 6);
    assert_eq!(w.query::<&Tag>().count(), 4);
    assert_eq!(*w.resource::<u32>(), 42);
}

#[test]
fn dynamic_components_and_chunks() {
    let mut w = World::new();
    let p = unsafe { w.register_component("Pos", 8, 4, None) };
    let v = unsafe { w.register_component("Vel", 8, 4, None) };
    assert_eq!(unsafe { w.register_component("Pos", 8, 4, None) }, p);
    assert_eq!(w.component_by_name("Vel"), Some(v));
    let mut es = Vec::new();
    for i in 0..100 {
        let pos = [i as f32, 0.0f32];
        let vel = [1.0f32, 2.0f32];
        es.push(unsafe { w.spawn_dynamic(&[(p, pos.as_ptr().cast()), (v, vel.as_ptr().cast())]) });
    }
    let mut chunks = 0;
    w.for_each_chunk(&[p, v], |c| {
        chunks += 1;
        let pos = unsafe { std::slice::from_raw_parts_mut(c.columns[0].cast::<[f32; 2]>(), c.len()) };
        let vel = unsafe { std::slice::from_raw_parts(c.columns[1].cast::<[f32; 2]>(), c.len()) };
        for (p, v) in pos.iter_mut().zip(vel) {
            p[0] += v[0];
            p[1] += v[1];
        }
    });
    assert_eq!(chunks, 1);
    let ptr = w.get_dynamic(es[10], p).unwrap();
    assert_eq!(unsafe { *ptr.cast::<[f32; 2]>() }, [11.0, 2.0]);

    // Add/remove a dynamic component.
    let hp = unsafe { w.register_component("Hp", 4, 4, None) };
    let val = 7i32;
    assert!(unsafe { w.insert_dynamic(es[0], hp, (&val as *const i32).cast()) });
    assert_eq!(unsafe { *w.get_dynamic(es[0], hp).unwrap().cast::<i32>() }, 7);
    assert!(w.remove_dynamic(es[0], hp));
    assert!(w.get_dynamic(es[0], hp).is_none());
    assert!(w.get_dynamic(es[0], p).is_some());
}

#[test]
#[should_panic(expected = "different layout")]
fn dynamic_name_clash_panics() {
    let mut w = World::new();
    unsafe {
        w.register_component("Pos", 8, 4, None);
        w.register_component("Pos", 4, 4, None);
    }
}

#[test]
fn rust_and_dynamic_views_share_storage() {
    let mut w = World::new();
    let e = w.spawn((Pos(1.0, 2.0),));
    let id = w.component_by_name(std::any::type_name::<Pos>()).unwrap();
    assert_eq!(Some(id), w.component_id::<Pos>());
    let mut n = 0;
    w.for_each_chunk(&[id], |c| {
        n += c.len();
        assert_eq!(c.entities, &[e]);
    });
    assert_eq!(n, 1);
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

#[cfg(feature = "parallel")]
#[test]
fn par_for_each_visits_every_entity_once() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let mut w = World::new();
    // Several archetypes, some bigger than one chunk, some smaller.
    w.spawn_batch((0..50_000).map(|i| (Pos(i as f32, 0.0), Vel(1.0, 0.0))));
    w.spawn_batch((0..20_000).map(|i| (Pos(i as f32, 0.0), Vel(1.0, 0.0), Tag)));
    w.spawn_batch((0..100).map(|i| (Pos(i as f32, 0.0), Vel(1.0, 0.0), Frozen)));
    w.spawn_batch((0..1000).map(|i| (Pos(i as f32, 0.0),)));
    let visits = AtomicUsize::new(0);
    w.query::<(&mut Pos, &Vel)>().without::<Frozen>().par_for_each(|(p, v)| {
        p.1 += v.0;
        visits.fetch_add(1, Ordering::Relaxed);
    });
    assert_eq!(visits.into_inner(), 70_000);
    assert_eq!(w.query::<&Pos>().iter().filter(|p| p.1 == 1.0).count(), 70_000);
    assert_eq!(w.query::<&Pos>().iter().filter(|p| p.1 == 0.0).count(), 1100);
}
