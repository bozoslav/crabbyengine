//! With the parallel system scheduler off (the default), running a game
//! must not start rayon's global thread pool: a game that never asked for
//! threads gets none from the scheduler, and can still configure the pool
//! itself.
//!
//! This is its own test binary, with one test: the global pool is one per
//! process and can be built once.

use ponko::access::SystemAccess;
use ponko::prelude::*;
use ponko::system_view::SystemView;

#[derive(Component)]
struct Pos(f32);
#[derive(Component)]
struct Vel(f32);

#[test]
fn a_game_with_the_scheduler_off_leaves_the_global_thread_pool_unbuilt() {
    let mut world = Game::new("off", 1, 1)
        // Off whatever `PONKO_PARALLEL_SYSTEMS` says in this test run.
        .parallel_systems(false)
        .parallel_systems_env(false)
        .sim_system(|ctx: &mut Ctx| {
            for p in ctx.world.query::<&mut Pos>().iter() {
                p.0 += 1.0;
            }
        })
        // View systems that would share a batch if the scheduler were on.
        .add(
            Phase::AfterPhysics,
            (|sys: &mut SystemView| sys.world.query::<&mut Pos>().for_each(|p| p.0 += 1.0))
                .access(SystemAccess::new().write::<Pos>()),
        )
        .add(
            Phase::AfterPhysics,
            (|sys: &mut SystemView| sys.world.query::<&mut Vel>().for_each(|v| v.0 += 1.0))
                .access(SystemAccess::new().write::<Vel>()),
        )
        .run_headless(5, |ctx| {
            ctx.world.spawn((Pos(0.0), Vel(0.0)));
        });
    let mut rows = world.query::<(&Pos, &Vel)>();
    let (p, v) = rows.iter().map(|(p, v)| (p.0, v.0)).next().unwrap();
    assert_eq!((p, v), (10.0, 5.0));

    // Building the global pool fails if anything already started it.
    let built = rayon::ThreadPoolBuilder::new().num_threads(2).build_global();
    assert!(built.is_ok(), "the game started rayon's global pool: {built:?}");
}
