use ponko_ecs::{Component, World};

#[derive(Component, Debug, PartialEq)]
struct Small(u32);
#[derive(Component, Debug, PartialEq)]
struct Big(u64);
#[derive(Component)]
struct Marker;

#[test]
fn raw_views_share_storage_and_release_the_exclusive_borrow() {
    let mut world = World::new();
    let e = world.spawn((Small(7),));
    let id = world.component_id::<Small>().unwrap();
    assert!(world.raw_ref().get_dynamic(e, id).is_some());
    assert_eq!(
        world.raw_ref().component_by_name(std::any::type_name::<Small>()),
        Some(id)
    );
    {
        let mut raw = world.raw();
        let mut rows = 0;
        raw.for_each_chunk(&[id], |chunk| rows += chunk.len());
        assert_eq!(rows, 1);
        assert!(raw.remove_dynamic(e, id));
    }
    assert!(world.get::<Small>(e).is_none());
    assert!(!world.raw_ref().archetypes().is_empty());
}

#[test]
fn a_shared_raw_view_hands_out_borrows_of_the_world() {
    let mut world = World::new();
    world.spawn((Marker,));
    // The slice outlives the temporary view: it borrows the world itself.
    let tables = world.raw_ref().archetypes();
    assert!(!tables.is_empty());
}

#[test]
fn the_raw_view_spawns_and_inserts_through_pointers() {
    let mut world = World::new();
    world.spawn((Small(0), Big(0)));
    let small = world.component_id::<Small>().unwrap();
    let big = world.component_id::<Big>().unwrap();
    let (a, b, c) = (Small(5), Big(6), Small(7));
    // SAFETY: each pointer points to a valid value of its component, which
    // needs no drop, so the world may take a copy.
    let spawned = unsafe { world.raw().spawn_dynamic(&[(small, (&a as *const Small).cast())]) };
    assert_eq!(world.get::<Small>(spawned), Some(&Small(5)));
    // SAFETY: as above.
    assert!(unsafe { world.raw().insert_dynamic(spawned, big, (&b as *const Big).cast()) });
    assert_eq!(world.get::<Big>(spawned), Some(&Big(6)));
    let saved = ponko_ecs::Entity::from_bits(9 | 3 << 32);
    // SAFETY: as above.
    assert!(unsafe {
        world
            .raw()
            .spawn_at_dynamic(saved, &[(small, (&c as *const Small).cast())])
    });
    assert_eq!(world.get::<Small>(saved), Some(&Small(7)));
    // SAFETY: as above; the slot is taken now, so nothing is spawned.
    assert!(!unsafe {
        world
            .raw()
            .spawn_at_dynamic(saved, &[(small, (&c as *const Small).cast())])
    });
    // A dead entity takes nothing.
    world.despawn(spawned);
    // SAFETY: as above.
    assert!(!unsafe { world.raw().insert_dynamic(spawned, small, (&a as *const Small).cast()) });
    assert_eq!(world.len(), 2);
}
