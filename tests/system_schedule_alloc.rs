//! The steady-state allocations of a batch of view systems.
//!
//! A single test in its own binary: `CountingAllocator` counts every thread
//! in the process, so nothing else may run while this one measures.
use std::time::Duration;

use ponko::access::SystemAccess;
#[allow(unused_imports)]
use ponko::alloc_count::CountingAllocator;
use ponko::prelude::*;
use ponko::system_view::SystemView;

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

const WARMUP: u64 = 50;
const TICKS: u64 = 250;

struct Pos(f32);
struct Vel(f32);
struct Total(f64);
#[derive(Clone, Copy)]
struct Moved(u32);

/// The allocation count when the batch was about to start.
struct Before(ponko::perf::AllocCounts);

/// How many ticks' batches allocated, after warm-up.
struct Allocating(u32);

fn movement(sys: &mut SystemView) {
    let mut n = 0;
    sys.world.query::<(&mut Pos, &Vel)>().for_each(|(p, v)| {
        p.0 += v.0;
        n += 1;
    });
    sys.events().send(Moved(n));
}

fn total(sys: &mut SystemView) {
    let moved: u32 = sys.events().read::<Moved>().iter().map(|m| m.0).sum();
    sys.world.resource_mut::<Total>().0 += f64::from(moved);
}

fn run(threshold: Duration, allocate: bool) -> (u32, f64) {
    let (world, report) = Game::new("schedule allocations", 1, 1)
        .parallel_systems(true)
        .parallel_threshold(threshold)
        .parallel_systems_env(false)
        .sim_system(|ctx: &mut Ctx| {
            let now = ponko::perf::alloc_counts();
            ctx.world.resource_mut::<Before>().0 = now;
        })
        // One batch: a reader before the sender, and a writer of its own.
        .add_view(
            Phase::AfterPhysics,
            SystemAccess::new().read_events::<Moved>().write_resource::<Total>(),
            total,
        )
        .add_view(
            Phase::AfterPhysics,
            SystemAccess::new().write::<Pos>().read::<Vel>().send_events::<Moved>(),
            move |sys: &mut SystemView| {
                // Positive control inside the same measured batch.
                if allocate {
                    std::hint::black_box(Box::new([0u8; 64]));
                }
                movement(sys);
            },
        )
        .add_view(
            Phase::AfterPhysics,
            SystemAccess::new().write::<u64>(),
            |sys: &mut SystemView| {
                sys.world.query::<&mut u64>().for_each(|n| *n += 1);
            },
        )
        // Runs right after the batch's barrier.
        .sim_system(|ctx: &mut Ctx| {
            let made = ponko::perf::alloc_counts().count - ctx.world.resource::<Before>().0.count;
            // Buffers grow and the thread pool starts in the first ticks.
            if ctx.time.sim_tick() >= WARMUP && made > 0 {
                ctx.world.resource_mut::<Allocating>().0 += 1;
            }
        })
        .profile_headless(WARMUP, TICKS, |ctx| {
            ctx.world.insert_resource(Before(ponko::perf::AllocCounts::default()));
            ctx.world.insert_resource(Total(0.0));
            ctx.world.insert_resource(Allocating(0));
            ctx.world
                .spawn_batch((0..500).map(|i| (Pos(i as f32), Vel(1.0), i as u64)));
        });
    assert_eq!(report.frames, TICKS as usize);
    assert!(report.allocs.is_some(), "allocation counting must be enabled");
    (world.resource::<Allocating>().0, world.resource::<Total>().0)
}

#[test]
fn a_batch_allocates_nothing_in_steady_state() {
    // Keep controls and measurements in one test: the counter is process-wide.
    // The thread pool's workers allocate while they start, on their own
    // threads and in their own time: the whole test takes milliseconds, so
    // on a slow machine that landed in a measured tick. Start every worker
    // and wait for it before anything is counted.
    rayon::broadcast(|_| ());
    // Profiling runs batches sequentially, including when parallelism is requested.
    for (threshold, mode) in [(Duration::MAX, "in order"), (Duration::ZERO, "parallel requested")] {
        let (allocating, control_total) = run(threshold, true);
        assert_eq!(
            allocating, TICKS as u32,
            "positive control must count every measured tick, {mode}"
        );

        let (allocating, total) = run(threshold, false);
        assert_eq!(
            total,
            500.0 * (WARMUP + TICKS - 1) as f64,
            "the reader saw every tick's event one tick later"
        );
        assert_eq!(control_total, total);
        assert_eq!(allocating, 0, "ticks whose batch allocated, {mode}");
    }
}
