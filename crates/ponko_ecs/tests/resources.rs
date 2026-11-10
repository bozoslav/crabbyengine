//! Global resources.

use ponko_ecs::*;

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
    let _ = World::new().resource::<Missing>();
}

#[test]
fn dynamic_resources_are_stored_by_name_and_replaced_in_place() {
    use std::alloc::Layout;
    let mut w = World::new();
    assert!(w.raw_ref().get_resource_dynamic("Score").is_none());
    let layout = Layout::new::<[u32; 2]>();
    let a: [u32; 2] = [3, 1];
    unsafe { w.raw().insert_resource_dynamic("Score", layout, a.as_ptr().cast()) };
    assert_eq!(w.raw_ref().resource_layout_dynamic("Score"), Some(layout));
    let p = w.raw_ref().get_resource_dynamic("Score").unwrap().cast::<[u32; 2]>();
    unsafe {
        assert_eq!(*p, [3, 1]);
        (*p)[1] = 7; // written through the pointer
        assert_eq!(
            *w.raw_ref().get_resource_dynamic("Score").unwrap().cast::<[u32; 2]>(),
            [3, 7]
        );
    }
    let b: [u32; 2] = [9, 9];
    unsafe { w.raw().insert_resource_dynamic("Score", layout, b.as_ptr().cast()) };
    assert_eq!(
        unsafe { *w.raw_ref().get_resource_dynamic("Score").unwrap().cast::<[u32; 2]>() },
        [9, 9]
    );
    // Zero-sized resources work too.
    unsafe {
        w.raw()
            .insert_resource_dynamic("Flag", Layout::new::<()>(), std::ptr::null())
    };
    assert!(w.raw_ref().get_resource_dynamic("Flag").is_some());
    assert!(w.raw().remove_resource_dynamic("Score") && w.raw().remove_resource_dynamic("Flag"));
    assert!(!w.raw().remove_resource_dynamic("Score") && w.raw_ref().get_resource_dynamic("Score").is_none());
}

#[test]
#[should_panic(expected = "exists with a different size or alignment")]
fn dynamic_resources_keep_their_layout() {
    use std::alloc::Layout;
    let mut w = World::new();
    let x = 1u32;
    unsafe {
        w.raw()
            .insert_resource_dynamic("Score", Layout::new::<u32>(), (&x as *const u32).cast());
        w.raw()
            .insert_resource_dynamic("Score", Layout::new::<u64>(), (&x as *const u32).cast());
    }
}

#[test]
fn resource_scope_queries_and_restores_non_copy_resource_on_early_return() {
    struct Score(String);
    let mut w = World::new();
    let e = w.spawn((2u32,));
    w.insert_resource(Score("score:".into()));
    w.insert_resource(3u64);
    let address = w.resource::<Score>() as *const Score;
    let result = w.resource_scope::<Score, _>(|world, score| {
        assert!(world.get_resource::<Score>().is_none());
        let multiplier = *world.resource::<u64>() as u32;
        world.query::<&mut u32>().for_each(|n| *n *= multiplier);
        score.0.push('6');
        if world.get::<u32>(e) == Some(&6) {
            return 42;
        }
        0
    });
    assert_eq!(result, 42);
    assert_eq!(w.resource::<Score>().0, "score:6");
    assert_eq!(w.resource::<Score>() as *const Score, address);
}

#[test]
fn resource_scope_restores_after_panic_and_drops_replacement_once() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct Tracked(Arc<AtomicUsize>, usize);
    impl Drop for Tracked {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let drops = Arc::new(AtomicUsize::new(0));
    let mut w = World::new();
    w.insert_resource(Tracked(drops.clone(), 1));
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        w.resource_scope::<Tracked, _>(|world, value| {
            value.1 = 2;
            world.insert_resource(Tracked(drops.clone(), 99));
            panic!("callback failed");
        });
    }));
    assert!(panic.is_err());
    assert_eq!(w.resource::<Tracked>().1, 2);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    drop(w);
    assert_eq!(drops.load(Ordering::SeqCst), 2);
}

#[test]
fn resource_scopes_nest_for_distinct_types() {
    let mut w = World::new();
    w.insert_resource(1u32);
    w.insert_resource(2u64);
    w.resource_scope::<u32, _>(|world, a| {
        world.resource_scope::<u64, _>(|world, b| {
            assert!(world.get_resource::<u32>().is_none());
            assert!(world.get_resource::<u64>().is_none());
            *a += *b as u32;
            *b = 4;
        });
    });
    assert_eq!(*w.resource::<u32>(), 3);
    assert_eq!(*w.resource::<u64>(), 4);
}

#[test]
#[should_panic(expected = "insert_resource")]
fn resource_scope_missing_resource_panics_helpfully() {
    World::new().resource_scope::<u32, _>(|_, _| {});
}

#[test]
fn nesting_same_resource_panics_and_restores_outer_resource() {
    let mut w = World::new();
    w.insert_resource(1u32);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        w.resource_scope::<u32, _>(|world, value| {
            *value = 2;
            world.resource_scope::<u32, _>(|_, _| {});
        });
    }));
    assert!(panic.is_err());
    assert_eq!(*w.resource::<u32>(), 2);
}
