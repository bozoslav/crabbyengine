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

fn run(threshold: Duration) -> (u32, f64) {
    let world = Game::new("schedule allocations", 1, 1)
        .parallel_systems(true)
        .parallel_threshold(threshold)
        .parallel_systems_env(false)
        .sim_system(|ctx: &mut Ctx| {
            let now = ponko::perf::alloc_counts();
            ctx.world.insert_resource(Before(now));
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
            movement,
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
            if ctx.time.sim_tick() >= 50 && made > 0 {
                ctx.world.resource_mut::<Allocating>().0 += 1;
            }
        })
        .run_headless(300, |ctx| {
            ctx.world.insert_resource(Total(0.0));
            ctx.world.insert_resource(Allocating(0));
            ctx.world
                .spawn_batch((0..500).map(|i| (Pos(i as f32), Vel(1.0), i as u64)));
        });
    (world.resource::<Allocating>().0, world.resource::<Total>().0)
}

#[test]
fn a_batch_allocates_nothing_in_steady_state() {
    // In order on the game's thread: the batch's own work never allocates.
    let (allocating, total) = run(Duration::MAX);
    assert_eq!(total, 500.0 * 299.0, "the reader saw every tick's event one tick later");
    assert_eq!(allocating, 0, "ticks whose batch allocated, run in order");
    // On the thread pool: the engine's side doesn't either. (Run in the same
    // test: the counter is the whole process's.)
    let (allocating, threaded_total) = run(Duration::ZERO);
    assert_eq!(threaded_total, total);
    assert_eq!(allocating, 0, "ticks whose batch allocated, run on threads");
}
