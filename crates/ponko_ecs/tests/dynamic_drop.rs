//! Removing a runtime-registered component that has a drop function
//! (`RawWorld::remove_dynamic`, the C/C++ path): the value is moved out of
//! the world before it is dropped, into a stack slot when it fits (64 bytes
//! at up to 16-byte alignment) and onto the heap otherwise. Each layout case
//! below takes a different path; all of them must hand the drop function the
//! intact value, correctly aligned, exactly once.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicUsize, Ordering};

use ponko_ecs::*;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pos(f32, f32);

/// Fills a value of `n` bytes with a pattern the drop functions check.
fn pattern<const N: usize>() -> [u8; N] {
    std::array::from_fn(|i| (i * 7 + 3) as u8)
}

/// Checks the value at `p`: `n` bytes of `pattern`, aligned to `align`.
///
/// # Safety
/// `p` must point to `n` readable bytes.
unsafe fn check_value(p: *mut u8, n: usize, align: usize) {
    assert!((p as usize).is_multiple_of(align), "the value moved out is misaligned");
    // SAFETY: the caller's contract.
    let bytes = unsafe { std::slice::from_raw_parts(p, n) };
    assert!(
        bytes.iter().enumerate().all(|(i, &b)| b == (i * 7 + 3) as u8),
        "the value moved out is not intact"
    );
}

/// A world with one entity holding `Pos` and component `id` set to `value`.
fn entity_with(w: &mut World, id: ComponentId, value: *const u8) -> Entity {
    let e = w.spawn((Pos(1.0, 2.0),));
    // SAFETY: every caller passes a valid value of `id`'s layout.
    assert!(unsafe { w.raw().insert_dynamic(e, id, value) });
    e
}

/// Removes `id` from `e` and checks the world afterwards.
fn remove_and_check(w: &mut World, e: Entity, id: ComponentId) {
    assert!(w.raw().remove_dynamic(e, id));
    assert!(w.raw_ref().get_dynamic(e, id).is_none());
    assert_eq!(w.get::<Pos>(e), Some(&Pos(1.0, 2.0)));
    assert!(!w.raw().remove_dynamic(e, id), "removed twice");
}

static ALIGNED_32_DROPS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C-unwind" fn drop_aligned_32(p: *mut u8) {
    // SAFETY: called on a 32-byte value of this component.
    unsafe { check_value(p, 32, 32) };
    ALIGNED_32_DROPS.fetch_add(1, Ordering::Relaxed);
}

#[test]
fn a_small_value_aligned_past_the_stack_slot_is_dropped_intact_once() {
    // 32 bytes fit the 64-byte slot, but 32-byte alignment doesn't: the heap path.
    #[repr(C, align(32))]
    struct Aligned([u8; 32]);
    let mut w = World::new();
    // SAFETY: a 32-byte value at 32-byte alignment; the drop function only reads it.
    let id = unsafe { w.raw().register_component("aligned-32", 32, 32, Some(drop_aligned_32)) };
    let value = Aligned(pattern::<32>());
    let e = entity_with(&mut w, id, (&raw const value).cast());
    remove_and_check(&mut w, e, id);
    assert_eq!(ALIGNED_32_DROPS.load(Ordering::Relaxed), 1);
}

static EXACT_64_DROPS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C-unwind" fn drop_exact_64(p: *mut u8) {
    // SAFETY: called on a 64-byte value of this component.
    unsafe { check_value(p, 64, 16) };
    EXACT_64_DROPS.fetch_add(1, Ordering::Relaxed);
}

#[test]
fn a_value_that_exactly_fills_the_stack_slot_is_dropped_intact_once() {
    #[repr(C, align(16))]
    struct Exact([u8; 64]);
    let mut w = World::new();
    // SAFETY: a 64-byte value at 16-byte alignment; the drop function only reads it.
    let id = unsafe { w.raw().register_component("exact-64", 64, 16, Some(drop_exact_64)) };
    let value = Exact(pattern::<64>());
    let e = entity_with(&mut w, id, (&raw const value).cast());
    remove_and_check(&mut w, e, id);
    assert_eq!(EXACT_64_DROPS.load(Ordering::Relaxed), 1);
}

static ZERO_SIZED_DROPS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C-unwind" fn drop_zero_sized(p: *mut u8) {
    assert!(
        !p.is_null() && (p as usize).is_multiple_of(8),
        "a zero-sized value needs an aligned pointer"
    );
    ZERO_SIZED_DROPS.fetch_add(1, Ordering::Relaxed);
}

#[test]
fn a_zero_sized_value_with_a_drop_function_is_dropped_once() {
    let mut w = World::new();
    // SAFETY: a zero-sized value; the drop function reads nothing.
    let id = unsafe {
        w.raw()
            .register_component("zero-sized-drop", 0, 8, Some(drop_zero_sized))
    };
    let value = std::ptr::without_provenance::<u8>(8);
    let e = entity_with(&mut w, id, value);
    remove_and_check(&mut w, e, id);
    assert_eq!(ZERO_SIZED_DROPS.load(Ordering::Relaxed), 1);
}

static PANICKING_DROPS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C-unwind" fn drop_panicking(p: *mut u8) {
    // SAFETY: called on a 16-byte value of this component.
    unsafe { check_value(p, 16, 8) };
    PANICKING_DROPS.fetch_add(1, Ordering::Relaxed);
    panic!("drop panics");
}

#[test]
fn a_panicking_drop_in_remove_dynamic_runs_once_and_leaves_the_world_consistent() {
    let mut w = World::new();
    // SAFETY: a 16-byte value; the drop function only reads it, then panics.
    let id = unsafe {
        w.raw()
            .register_component("panicking-drop", 16, 8, Some(drop_panicking))
    };
    let value = pattern::<16>();
    let e = entity_with(&mut w, id, value.as_ptr());
    let other = w.spawn((Pos(3.0, 4.0),));

    let result = catch_unwind(AssertUnwindSafe(|| w.raw().remove_dynamic(e, id)));
    assert!(result.is_err(), "the drop's panic reaches the caller");
    assert_eq!(PANICKING_DROPS.load(Ordering::Relaxed), 1);

    // The value was moved out before the drop ran: the entity no longer has it,
    // and nothing drops it again.
    assert!(w.raw_ref().get_dynamic(e, id).is_none());
    assert!(!w.raw().remove_dynamic(e, id));
    assert_eq!(w.get::<Pos>(e), Some(&Pos(1.0, 2.0)));
    assert_eq!(w.get::<Pos>(other), Some(&Pos(3.0, 4.0)));
    assert_eq!(w.query::<&Pos>().count(), 2);

    // The world still works: the component can be added back, and removing it
    // again drops it (panicking again) exactly once more.
    // SAFETY: a valid 16-byte value of the component.
    assert!(unsafe { w.raw().insert_dynamic(e, id, value.as_ptr()) });
    assert!(w.raw_ref().get_dynamic(e, id).is_some());
    let result = catch_unwind(AssertUnwindSafe(|| w.raw().remove_dynamic(e, id)));
    assert!(result.is_err());
    assert_eq!(PANICKING_DROPS.load(Ordering::Relaxed), 2);
    drop(w);
    assert_eq!(
        PANICKING_DROPS.load(Ordering::Relaxed),
        2,
        "dropping the world dropped it again"
    );
}
