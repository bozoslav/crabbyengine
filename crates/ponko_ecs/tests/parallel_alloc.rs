//! Parallel queries reuse their work plan: once warm, the querying thread
//! allocates nothing. Its own test binary: it counts allocations through a
//! global allocator, per thread, so only the thread running the query is
//! counted.
#![cfg(feature = "parallel")]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use ponko_ecs::*;

struct Counting;

thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

fn count() {
    // `try_with`: the counter may be gone while a thread shuts down.
    let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
}

fn allocations() -> usize {
    ALLOCATIONS.with(Cell::get)
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

struct Value(u64);
struct Marker<const N: usize>;

/// 512 small archetypes (a plan of 512 entries) plus a big one, so the plan
/// is both batched and split.
fn world() -> World {
    let mut w = World::new();
    for bits in 0..512u64 {
        for _ in 0..1 + bits % 16 {
            let e = w.spawn((Value(bits),));
            macro_rules! m {
                ($($n:literal)*) => {$(
                    if bits & (1 << $n) != 0 {
                        w.insert(e, Marker::<$n>);
                    }
                )*};
            }
            m!(0 1 2 3 4 5 6 7 8);
        }
    }
    w.spawn_batch((0..20_000).map(|i| (Value(i),)));
    w
}

#[test]
fn warm_parallel_queries_do_not_allocate_on_the_querying_thread() {
    let mut w = world();
    let mut plain = Vec::new();
    let mut owned = Vec::new();
    let mut frame = |w: &mut World| {
        w.query::<&mut Value>().par_for_each(|v| v.0 = v.0.wrapping_add(1));
        w.query::<&Value>().par_for_each_indexed(|i, v| {
            std::hint::black_box((i, v.0));
        });
        w.query::<&Value>().without::<Marker<3>>().par_for_each(|v| {
            std::hint::black_box(v.0);
        });
        // Small archetypes only: the first chunk is timed on this thread.
        w.query::<&Value>().with::<Marker<0>>().par_for_each(|v| {
            std::hint::black_box(v.0);
        });
        w.query::<&Value>().par_collect_into(&mut plain, |v| v.0);
        // A destructor takes the other collection path.
        w.query::<&Value>().par_collect_into(&mut owned, |v| Owned(v.0));
    };
    struct Owned(u64);
    impl Drop for Owned {
        fn drop(&mut self) {
            std::hint::black_box(self.0);
        }
    }

    // One thread: the chunked rayon path still runs, but the querying thread
    // never parks waiting for a sibling. Parking allocates rayon's own sleep
    // state once per thread on some platforms, which isn't this crate's.
    let pool = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
    pool.install(|| {
        for _ in 0..3 {
            frame(&mut w);
        }
        let before = allocations();
        for _ in 0..20 {
            frame(&mut w);
        }
        assert_eq!(allocations() - before, 0, "allocations in 20 warm frames");
    });
    assert_eq!(plain.len(), w.query::<&Value>().count());
}
