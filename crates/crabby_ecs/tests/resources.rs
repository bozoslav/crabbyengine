//! Global resources.

use crabby_ecs::*;

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
    assert!(w.get_resource_dynamic("Score").is_none());
    let layout = Layout::new::<[u32; 2]>();
    let a: [u32; 2] = [3, 1];
    unsafe { w.insert_resource_dynamic("Score", layout, a.as_ptr().cast()) };
    assert_eq!(w.resource_layout_dynamic("Score"), Some(layout));
    let p = w.get_resource_dynamic("Score").unwrap().cast::<[u32; 2]>();
    unsafe {
        assert_eq!(*p, [3, 1]);
        (*p)[1] = 7; // written through the pointer
        assert_eq!(*w.get_resource_dynamic("Score").unwrap().cast::<[u32; 2]>(), [3, 7]);
    }
    let b: [u32; 2] = [9, 9];
    unsafe { w.insert_resource_dynamic("Score", layout, b.as_ptr().cast()) };
    assert_eq!(
        unsafe { *w.get_resource_dynamic("Score").unwrap().cast::<[u32; 2]>() },
        [9, 9]
    );
    // Zero-sized resources work too.
    unsafe { w.insert_resource_dynamic("Flag", Layout::new::<()>(), std::ptr::null()) };
    assert!(w.get_resource_dynamic("Flag").is_some());
    assert!(w.remove_resource_dynamic("Score") && w.remove_resource_dynamic("Flag"));
    assert!(!w.remove_resource_dynamic("Score") && w.get_resource_dynamic("Score").is_none());
}

#[test]
#[should_panic(expected = "exists with a different size or alignment")]
fn dynamic_resources_keep_their_layout() {
    use std::alloc::Layout;
    let mut w = World::new();
    let x = 1u32;
    unsafe {
        w.insert_resource_dynamic("Score", Layout::new::<u32>(), (&x as *const u32).cast());
        w.insert_resource_dynamic("Score", Layout::new::<u64>(), (&x as *const u32).cast());
    }
}
