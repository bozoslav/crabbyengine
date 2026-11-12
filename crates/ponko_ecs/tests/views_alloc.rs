//! Running a reused `ViewState` doesn't allocate once its buffers have
//! grown, with change filters and removals too. Its own test binary with a
//! single test. The counter is per thread: the test harness's other threads
//! (its main thread waiting on the test) allocate at times of their own.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use ponko_ecs::*;

struct Counting;

thread_local! {
    // `const` init: reading it never allocates, so the allocator may use it.
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

fn allocations() -> usize {
    ALLOCATIONS.with(Cell::get)
}

fn count() {
    // `try_with`: a thread being torn down may still free or grow memory.
    let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
}

// SAFETY: forwards to the system allocator, only counting calls.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: the caller's contract, forwarded.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller's contract, forwarded.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        // SAFETY: the caller's contract, forwarded.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

struct Pos(f32);
struct Vel(f32);
struct Score(u32);

#[test]
fn reused_view_state_runs_without_allocating() {
    let mut world = World::new();
    world.insert_resource(Score(0));
    world.spawn_batch((0..256).map(|i| (Pos(i as f32), Vel(1.0))));
    let mut state = ViewState::new(ViewAccess::new().write::<Pos>().read::<Vel>().write_resource::<Score>());
    let mut run = |world: &mut World| {
        state.run(world, |mut view| {
            view.query::<(&mut Pos, &Vel)>().for_each(|(p, v)| p.0 += v.0);
            view.resource_mut::<Score>().0 += 1;
        });
    };
    run(&mut world); // warm up: the state's buffers grow once
    let before = allocations();
    for _ in 0..100 {
        run(&mut world);
    }
    assert_eq!(allocations(), before);
    assert_eq!(world.resource::<Score>().0, 101);
    change_filters_and_removals_allocate_nothing();
}

/// Change filters and removals through a reused view allocate nothing.
fn change_filters_and_removals_allocate_nothing() {
    let mut world = World::new();
    world.track_changes::<Pos>();
    world.spawn_batch((0..256).map(|i| (Pos(i as f32), Vel(1.0))));
    let gone = world.spawn((Pos(0.0), Vel(0.0)));
    world.despawn(gone);
    let mut state = ViewState::new(ViewAccess::new().write::<Pos>().read::<Vel>());
    let mut seen = 0;
    let mut run = |world: &mut World| {
        state.run(world, |mut view| {
            view.query::<(&mut Pos, &Vel)>()
                .filter::<(Changed<Pos>, Or<(With<Vel>, Added<Pos>)>)>()
                .since(Tick::default())
                .for_each(|(p, v)| p.0 += v.0);
            seen += view.query::<&Pos>().filter::<Added<Pos>>().count();
            seen += view.query::<&Pos>().filter::<Changed<Pos>>().iter().count();
            seen += view.removed::<Pos>().len();
        });
    };
    run(&mut world);
    let before = allocations();
    for _ in 0..100 {
        run(&mut world);
    }
    assert_eq!(allocations(), before);
    assert!(seen > 0);
}
