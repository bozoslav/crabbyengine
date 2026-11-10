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
    assert_eq!(w.raw_ref().component_by_name("Vel"), Some(v));
    let mut es = Vec::new();
    for i in 0..100 {
        let pos = [i as f32, 0.0f32];
        let vel = [1.0f32, 2.0f32];
        es.push(unsafe {
            w.raw()
                .spawn_dynamic(&[(p, pos.as_ptr().cast()), (v, vel.as_ptr().cast())])
        });
    }
    let mut chunks = 0;
    w.raw().for_each_chunk(&[p, v], |c| {
        chunks += 1;
        let pos = unsafe { std::slice::from_raw_parts_mut(c.columns[0].cast::<[f32; 2]>(), c.len()) };
        let vel = unsafe { std::slice::from_raw_parts(c.columns[1].cast::<[f32; 2]>(), c.len()) };
        for (p, v) in pos.iter_mut().zip(vel) {
            p[0] += v[0];
            p[1] += v[1];
        }
    });
    assert_eq!(chunks, 1);
    let ptr = w.raw_ref().get_dynamic(es[10], p).unwrap();
    assert_eq!(unsafe { *ptr.cast::<[f32; 2]>() }, [11.0, 2.0]);

    // Add/remove a dynamic component.
    let hp = unsafe { w.register_component("Hp", 4, 4, None) };
    let val = 7i32;
    assert!(unsafe { w.raw().insert_dynamic(es[0], hp, (&val as *const i32).cast()) });
    assert_eq!(unsafe { *w.raw_ref().get_dynamic(es[0], hp).unwrap().cast::<i32>() }, 7);
    assert!(w.raw().remove_dynamic(es[0], hp));
    assert!(w.raw_ref().get_dynamic(es[0], hp).is_none());
    assert!(w.raw_ref().get_dynamic(es[0], p).is_some());
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
    let id = w.raw_ref().component_by_name(std::any::type_name::<Pos>()).unwrap();
    assert_eq!(Some(id), w.component_id::<Pos>());
    let mut n = 0;
    w.raw().for_each_chunk(&[id], |c| {
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
    assert!(unsafe {
        w.raw()
            .spawn_at_dynamic(saved, &[(pos, p.as_ptr().cast()), (name, n.as_ptr().cast())])
    });
    assert!(w.is_alive(saved));
    assert_eq!(
        unsafe { *w.raw_ref().get_dynamic(saved, pos).unwrap().cast::<[f32; 2]>() },
        p
    );
    assert_eq!(
        unsafe { *w.raw_ref().get_dynamic(saved, name).unwrap().cast::<[u64; 2]>() },
        n
    );

    // Taken: nothing changes.
    assert!(!unsafe { w.raw().spawn_at_dynamic(saved, &[(pos, [0.0f32, 0.0].as_ptr().cast())]) });
    assert_eq!(w.len(), 1);
    assert_eq!(
        unsafe { *w.raw_ref().get_dynamic(saved, pos).unwrap().cast::<[f32; 2]>() },
        p
    );

    // Fresh spawns never collide with the restored id, and queries see both.
    let fresh = unsafe { w.raw().spawn_dynamic(&[(pos, [1.0f32, 1.0].as_ptr().cast())]) };
    assert_ne!(fresh, saved);
    let mut rows = 0;
    w.raw().for_each_chunk(&[pos], |c| rows += c.len());
    assert_eq!(rows, 2);
}

#[test]
#[should_panic(expected = "spawn_at_dynamic: the same component was given twice")]
fn spawn_at_dynamic_rejects_duplicate_components() {
    let mut w = World::new();
    let hp = unsafe { w.register_component("Hp", 4, 4, None) };
    let v = 1i32;
    let v = (&v as *const i32).cast();
    unsafe { w.raw().spawn_at_dynamic(Entity::from_bits(3), &[(hp, v), (hp, v)]) };
}

/// Structural changes map every source column to its place in the destination
/// archetype; with many components, adding or removing one at the start, in the
/// middle or at the end must leave every other value where it belongs.
#[test]
fn structural_changes_keep_values_in_wide_archetypes() {
    const WIDE: u32 = 40;
    let mut w = World::new();
    let ids: Vec<ComponentId> = (0..WIDE)
        .map(|i| unsafe { w.register_component(&format!("wide{i}"), 4, 4, None) })
        .collect();
    // The value of component `c` on entity `n` is `c * 100 + n`.
    let value = |c: u32, n: u32| c * 100 + n;
    let set = |w: &mut World, n: u32, e: Entity, c: u32| {
        let v = value(c, n);
        assert!(unsafe { w.raw().insert_dynamic(e, ids[c as usize], (&raw const v).cast()) });
    };
    let check = |w: &World, es: &[Entity], has: &dyn Fn(u32) -> bool| {
        for (n, &e) in es.iter().enumerate() {
            for c in 0..WIDE {
                let got = w
                    .raw_ref()
                    .get_dynamic(e, ids[c as usize])
                    .map(|p| unsafe { *p.cast::<u32>() });
                assert_eq!(got, has(c).then(|| value(c, n as u32)), "entity {n} component {c}");
            }
        }
    };

    // Three entities share the archetype, so rows are swapped when one leaves.
    let es: Vec<Entity> = (0..3).map(|_| w.spawn((Pos(0.0, 0.0),))).collect();
    // Insert in a scrambled order; the archetype's columns are sorted by id regardless.
    let initial = |c: u32| c.is_multiple_of(2) && c != 20;
    let order: Vec<u32> = (0..WIDE).filter(|&c| initial(c)).rev().collect();
    for (n, &e) in es.iter().enumerate() {
        for &c in &order {
            set(&mut w, n as u32, e, c);
        }
    }
    check(&w, &es, &initial);

    // Add at the start, in the middle (a gap) and at the end, then remove them again.
    for extra in [1, 20, WIDE - 1] {
        for (n, &e) in es.iter().enumerate() {
            set(&mut w, n as u32, e, extra);
        }
        check(&w, &es, &|c| initial(c) || c == extra);
        for &e in &es {
            assert!(w.raw().remove_dynamic(e, ids[extra as usize]));
        }
        check(&w, &es, &initial);
    }

    // Remove from the start, the middle and the end of the original set, middle entity first.
    for gone in [0, 22, 38] {
        for &i in &[1, 0, 2] {
            assert!(w.raw().remove_dynamic(es[i], ids[gone as usize]));
        }
    }
    check(&w, &es, &|c| initial(c) && ![0, 22, 38].contains(&c));
}
