//! Repeated queries do not allocate after matching-cache warmup. Its own test binary: it counts every
//! allocation through a global allocator.

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

struct Marker<const N: usize>;

#[test]
fn warmed_matching_and_reconstructed_queries_do_not_allocate() {
    let mut w = World::new();
    for bits in 0..256 {
        let e = w.spawn((bits as u64,));
        macro_rules! m {
            ($n:literal) => {
                if bits & (1 << $n) != 0 {
                    w.insert(e, Marker::<$n>);
                }
            };
        }
        m!(0);
        m!(1);
        m!(2);
        m!(3);
        m!(4);
        m!(5);
        m!(6);
        m!(7);
    }
    let frame = |w: &mut World| {
        let mut q = w
            .query::<&mut u64>()
            .cached()
            .with::<Marker<0>>()
            .without::<Marker<1>>();
        assert_eq!(q.count(), 64);
        q.for_each(|v| *v += 1);
        assert_eq!(q.iter().count(), 64);
    };
    frame(&mut w);
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    for _ in 0..10 {
        frame(&mut w);
    }
    assert_eq!(ALLOCATIONS.load(Ordering::Relaxed) - before, 0);
}
