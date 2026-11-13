//! Wide archetypes: queries over them and warm structural changes on them do
//! not allocate, and removing a component with a destructor drops it once.
//! Its own test binary: it replaces the global allocator. Only allocations on
//! the thread inside `allocations_in` are counted, so libtest's other threads
//! (its slow-test warning under Miri, for one) can't fail it.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

use ponko_ecs::*;

struct Counting;

thread_local! {
    /// Allocations on this thread while `allocations_in` runs here; `None` otherwise.
    static ALLOCATIONS: Cell<Option<usize>> = const { Cell::new(None) };
}

/// Counts an allocation if this thread is inside `allocations_in`.
fn note() {
    // `try_with`: the allocator may run while this thread's locals are torn down.
    let _ = ALLOCATIONS.try_with(|n| n.set(n.get().map(|n| n + 1)));
}

// SAFETY: forwards to the system allocator, only counting calls.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note();
        // SAFETY: the caller's contract, forwarded.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller's contract, forwarded.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        note();
        // SAFETY: the caller's contract, forwarded.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// Heap allocations (and reallocations) `f` makes on this thread.
fn allocations_in(f: impl FnOnce()) -> usize {
    ALLOCATIONS.with(|n| n.set(Some(0)));
    f();
    ALLOCATIONS.with(|n| n.take()).unwrap()
}

#[derive(Clone, Copy)]
struct P(f32);
#[derive(Clone, Copy)]
struct V(f32);
struct Tag;
/// A Rust component with a destructor.
struct Name(#[allow(dead_code)] String);

const WIDTH: usize = 40;
const ARCHETYPES: usize = 64;

/// Drops of the runtime-registered components below, by kind.
static SMALL_DROPS: AtomicUsize = AtomicUsize::new(0);
static LARGE_DROPS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C-unwind" fn drop_small(_: *mut u8) {
    SMALL_DROPS.fetch_add(1, Ordering::Relaxed);
}

/// Checks that the value arrives intact: every byte is its index.
unsafe extern "C-unwind" fn drop_large(p: *mut u8) {
    // SAFETY: called on a 256-byte value of this component.
    let bytes = unsafe { std::slice::from_raw_parts(p, 256) };
    assert!(bytes.iter().enumerate().all(|(i, &b)| b == i as u8));
    LARGE_DROPS.fetch_add(1, Ordering::Relaxed);
}

/// `ARCHETYPES` archetypes of `WIDTH` components each (`P`, `V` and 38 of 64
/// runtime components), two entities in each.
fn wide_world() -> (World, Vec<ComponentId>, ComponentId, ComponentId) {
    let mut w = World::new();
    // SAFETY: plain 4-byte values without a destructor.
    let pool: Vec<ComponentId> = (0..64)
        .map(|i| unsafe { w.raw().register_component(&format!("c{i}"), 4, 4, None) })
        .collect();
    let (p, v) = (w.register::<P>(), w.register::<V>());
    w.register::<Tag>();
    let (pv, vv, zero) = (P(1.0), V(0.5), 0u32);
    for a in 0..ARCHETYPES {
        // Archetype `a` leaves out pool ids a..a+26 (mod 64): 38 of 64, all different.
        let mut parts: Vec<(ComponentId, *const u8)> = vec![(p, (&raw const pv).cast()), (v, (&raw const vv).cast())];
        parts.extend(
            (0..64)
                .filter(|i| (i + 64 - a) % 64 >= 26)
                .map(|i| (pool[i], (&raw const zero).cast::<u8>())),
        );
        assert_eq!(parts.len(), WIDTH);
        for _ in 0..2 {
            // SAFETY: every pointer is to a valid `Copy` value of its component.
            unsafe { w.raw().spawn_dynamic(&parts) };
        }
    }
    (w, pool, p, v)
}

fn queries_over_wide_archetypes_do_not_allocate() {
    let (mut w, pool, p, v) = wide_world();
    let frame = |w: &mut World| {
        let mut n = 0;
        w.query::<(&mut P, &V)>().for_each(|(a, b)| {
            a.0 += b.0;
            n += 1;
        });
        assert_eq!(n, 2 * ARCHETYPES);
        assert_eq!(
            w.query::<(&P, &V)>().filter::<Without<Tag>>().iter().count(),
            2 * ARCHETYPES
        );
        assert_eq!(w.query::<&P>().filter::<With<Tag>>().count(), 0);
        let mut rows = 0;
        w.raw().for_each_chunk(&[v, p, pool[63]], |c| rows += c.len());
        assert!(rows > 0 && rows < 2 * ARCHETYPES);
        let arch = w.raw_ref().archetypes().iter().find(|a| a.len() == 2).unwrap();
        assert!(arch.has(p) && arch.column_ptr(v).is_some() && arch.component_ids().len() == WIDTH);
    };
    frame(&mut w);
    let allocs = allocations_in(|| {
        for _ in 0..10 {
            frame(&mut w);
        }
    });
    assert_eq!(allocs, 0, "queries over {WIDTH}-component archetypes allocated");
}

fn warm_transitions_on_a_wide_entity_do_not_allocate() {
    let (mut w, pool, _, _) = wide_world();
    // SAFETY: 8-byte and 256-byte values; the drop functions only read them.
    let small = unsafe { w.raw().register_component("small-drop", 8, 8, Some(drop_small)) };
    let large = unsafe { w.raw().register_component("large-drop", 256, 16, Some(drop_large)) };
    let e = w.query::<Entity>().iter().next().unwrap();
    let plain = *pool
        .iter()
        .find(|&&id| w.raw_ref().get_dynamic(e, id).is_none())
        .unwrap();
    let small_value = [7u8; 8];
    let large_value: [u8; 256] = std::array::from_fn(|i| i as u8);
    let toggle = |w: &mut World| {
        let value = 1u32;
        // SAFETY: each value matches its component's layout; the world owns it after.
        unsafe {
            assert!(w.raw().insert_dynamic(e, plain, (&raw const value).cast()));
            assert!(w.raw().insert_dynamic(e, small, small_value.as_ptr()));
        }
        assert!(w.raw().remove_dynamic(e, plain));
        assert!(w.raw().remove_dynamic(e, small));
        assert!(w.insert(e, Name(String::new())));
        let name = w.remove::<Name>(e).unwrap();
        drop(name);
    };
    // Warm-up: creates the archetypes and edges, and grows their columns.
    toggle(&mut w);
    toggle(&mut w);
    let drops = SMALL_DROPS.load(Ordering::Relaxed);
    let allocs = allocations_in(|| {
        for _ in 0..100 {
            toggle(&mut w);
        }
    });
    // This also shows that every transition-edge lookup hit: a miss copies the
    // id list (`to_vec`/`collect` in world.rs), which allocates for any
    // archetype of more than one component, even when the archetype it finds
    // already exists (so the archetype count can't tell a miss from a hit).
    assert_eq!(allocs, 0, "warm add/remove on a {WIDTH}-component entity allocated");
    assert_eq!(
        SMALL_DROPS.load(Ordering::Relaxed) - drops,
        100,
        "each removed value is dropped once"
    );

    // A value too large for the stack slot still arrives intact and is dropped once.
    // SAFETY: a 256-byte value of the 256-byte component.
    assert!(unsafe { w.raw().insert_dynamic(e, large, large_value.as_ptr()) });
    assert!(w.raw().remove_dynamic(e, large));
    assert_eq!(LARGE_DROPS.load(Ordering::Relaxed), 1);
    assert!(w.raw_ref().get_dynamic(e, large).is_none());
}

#[test]
fn wide_archetypes_query_and_change_without_allocating() {
    queries_over_wide_archetypes_do_not_allocate();
    warm_transitions_on_a_wide_entity_do_not_allocate();
}
