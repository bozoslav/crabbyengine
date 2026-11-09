//! The C/C++ path: runtime-registered components and raw column chunks.

mod common;

use common::*;
use ponko_ecs::*;

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
fn spawn_at_dynamic_restores_ids_and_refuses_taken_slots() {
    let mut w = World::new();
    let pos = unsafe { w.register_component("Pos", 8, 4, None) };
    let name = unsafe { w.register_component("Name", 16, 8, None) };
    let saved = Entity::from_bits(9 | 4 << 32);
    let p = [3.0f32, 4.0];
    let n = [7u64, 8];
    assert!(unsafe { w.spawn_at_dynamic(saved, &[(pos, p.as_ptr().cast()), (name, n.as_ptr().cast())]) });
    assert!(w.is_alive(saved));
    assert_eq!(unsafe { *w.get_dynamic(saved, pos).unwrap().cast::<[f32; 2]>() }, p);
    assert_eq!(unsafe { *w.get_dynamic(saved, name).unwrap().cast::<[u64; 2]>() }, n);

    // Taken: nothing changes.
    assert!(!unsafe { w.spawn_at_dynamic(saved, &[(pos, [0.0f32, 0.0].as_ptr().cast())]) });
    assert_eq!(w.len(), 1);
    assert_eq!(unsafe { *w.get_dynamic(saved, pos).unwrap().cast::<[f32; 2]>() }, p);

    // Fresh spawns never collide with the restored id, and queries see both.
    let fresh = unsafe { w.spawn_dynamic(&[(pos, [1.0f32, 1.0].as_ptr().cast())]) };
    assert_ne!(fresh, saved);
    let mut rows = 0;
    w.for_each_chunk(&[pos], |c| rows += c.len());
    assert_eq!(rows, 2);
}

#[test]
#[should_panic(expected = "spawn_at_dynamic: the same component was given twice")]
fn spawn_at_dynamic_rejects_duplicate_components() {
    let mut w = World::new();
    let hp = unsafe { w.register_component("Hp", 4, 4, None) };
    let v = 1i32;
    let v = (&v as *const i32).cast();
    unsafe { w.spawn_at_dynamic(Entity::from_bits(3), &[(hp, v), (hp, v)]) };
}
