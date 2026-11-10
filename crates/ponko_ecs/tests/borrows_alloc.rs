//! Borrowing several resources, carrying resources into a query, looking up
//! other entities and `get_many_mut` do not allocate. Its own test binary:
//! it counts every allocation through a global allocator.

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

struct Speed(u64);
struct Score(u64);
struct Total(u64);
struct Target(Entity);

#[test]
fn the_combined_borrows_do_not_allocate() {
    let mut w = World::new();
    w.insert_resource(Speed(2));
    w.insert_resource(Score(0));
    w.insert_resource(Total(0));
    let hub = w.spawn((0u64,));
    let other = w.spawn((1u64,));
    w.spawn_batch((0..1000).map(|_| (Target(hub), 1u32)));

    let frame = |w: &mut World| {
        let (speed, score, total) = w.resources_mut::<(Speed, Score, Total)>();
        score.0 += speed.0;
        total.0 += score.0;

        w.query::<&mut u32>()
            .with_resources::<(&Speed, &mut Score)>()
            .for_each(|n, (speed, score)| {
                *n = n.wrapping_add(speed.0 as u32);
                score.0 += 1;
            });

        w.query::<(&Target, &u32)>()
            .with_lookup::<&mut u64>()
            .for_each(|(target, n), others| {
                if let Some(sum) = others.get_mut(target.0) {
                    *sum += u64::from(*n);
                }
            });

        if let Some([a, b]) = w.get_many_mut::<u64, 2>([hub, other]) {
            std::mem::swap(a, b);
        }
    };
    // The first frame registers what it touches.
    frame(&mut w);
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    for _ in 0..100 {
        frame(&mut w);
    }
    assert_eq!(ALLOCATIONS.load(Ordering::Relaxed) - before, 0);
    assert_eq!(w.resource::<Score>().0, 101 * 2 + 101 * 1000);
}
