//! Recording and applying commands into a reused `Commands` doesn't allocate
//! once its buffers have grown. Its own test binary with a single test: the
//! counter is process-wide (every thread), so no other test may run beside
//! it, or its allocations would count.

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

/// A sized component, so each command carries bytes, not just a type.
#[derive(Component)]
struct Marked(#[allow(dead_code)] u64);
#[derive(Component)]
struct Index(#[allow(dead_code)] u64);

#[test]
fn reused_commands_record_and_apply_without_allocating() {
    let mut world = World::new();
    let entities: Vec<_> = (0..256u64).map(|i| world.spawn((Index(i),))).collect();
    let mut commands = Commands::new();
    let frame = |world: &mut World, commands: &mut Commands| {
        for &e in &entities {
            commands.insert(e, Marked(7));
        }
        let total = 3u64;
        commands.add(move |w| *w.resource_mut::<u64>() += total);
        world.apply(commands);
        for &e in &entities {
            commands.remove::<Marked>(e);
        }
        world.apply(commands);
    };
    world.insert_resource(0u64);
    // The first frames grow the command buffers and the world's tables.
    for _ in 0..3 {
        frame(&mut world, &mut commands);
    }
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    for _ in 0..10 {
        frame(&mut world, &mut commands);
    }
    assert_eq!(ALLOCATIONS.load(Ordering::Relaxed) - before, 0);
    assert_eq!(*world.resource::<u64>(), 13 * 3);
}
