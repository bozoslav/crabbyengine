//! Running a reused `ViewState` doesn't allocate once its buffers have
//! grown. Its own test binary with a single test: the counter is
//! process-wide, so no other test may run beside it.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use ponko_ecs::*;

struct Counting;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

// SAFETY: forwards to the system allocator, only counting calls.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: the caller's contract, forwarded.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the caller's contract, forwarded.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
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
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    for _ in 0..100 {
        run(&mut world);
    }
    assert_eq!(ALLOCATIONS.load(Ordering::Relaxed), before);
    assert_eq!(world.resource::<Score>().0, 101);
}
