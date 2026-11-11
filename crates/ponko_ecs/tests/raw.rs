use ponko_ecs::World;

#[test]
fn raw_views_share_storage_and_release_the_exclusive_borrow() {
    let mut world = World::new();
    let e = world.spawn((7u32,));
    let id = world.component_id::<u32>().unwrap();
    {
        let mut raw = world.raw();
        assert!(raw.get_dynamic(e, id).is_some());
        assert_eq!(raw.component_by_name("u32"), Some(id));
        let mut rows = 0;
        raw.for_each_chunk(&[id], |chunk| rows += chunk.len());
        assert_eq!(rows, 1);
        assert!(raw.remove_dynamic(e, id));
    }
    assert!(world.get::<u32>(e).is_none());
    assert!(!world.raw_ref().archetypes().is_empty());
}

#[test]
fn a_shared_raw_view_hands_out_borrows_of_the_world() {
    let mut world = World::new();
    world.spawn((1u8,));
    // The slice outlives the temporary view: it borrows the world itself.
    let tables = world.raw_ref().archetypes();
    assert!(!tables.is_empty());
}

#[test]
fn the_raw_view_spawns_and_inserts_through_pointers() {
    let mut world = World::new();
    world.spawn((0u32, 0u64));
    let small = world.component_id::<u32>().unwrap();
    let big = world.component_id::<u64>().unwrap();
    let (a, b, c) = (5u32, 6u64, 7u32);
    // SAFETY: each pointer points to a valid value of its component, which
    // needs no drop, so the world may take a copy.
    let spawned = unsafe { world.raw().spawn_dynamic(&[(small, (&a as *const u32).cast())]) };
    assert_eq!(world.get::<u32>(spawned), Some(&5));
    // SAFETY: as above.
    assert!(unsafe { world.raw().insert_dynamic(spawned, big, (&b as *const u64).cast()) });
    assert_eq!(world.get::<u64>(spawned), Some(&6));
    let saved = ponko_ecs::Entity::from_bits(9 | 3 << 32);
    // SAFETY: as above.
    assert!(unsafe {
        world
            .raw()
            .spawn_at_dynamic(saved, &[(small, (&c as *const u32).cast())])
    });
    assert_eq!(world.get::<u32>(saved), Some(&7));
    // SAFETY: as above; the slot is taken now, so nothing is spawned.
    assert!(!unsafe {
        world
            .raw()
            .spawn_at_dynamic(saved, &[(small, (&c as *const u32).cast())])
    });
    // A dead entity takes nothing.
    world.despawn(spawned);
    // SAFETY: as above.
    assert!(!unsafe { world.raw().insert_dynamic(spawned, small, (&a as *const u32).cast()) });
    assert_eq!(world.len(), 2);
}
