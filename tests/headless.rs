//! Headless runs: simulation without a window or GPU (`Game::run_headless`).

use std::sync::atomic::{AtomicU32, Ordering};

use crabbyengine::*;

struct Pos(f32);
struct Ticks(Vec<u64>);

fn fall(ctx: &mut Ctx) {
    let dt = ctx.dt();
    ctx.world.query::<&mut Pos>().for_each(|p| p.0 -= 10.0 * dt);
}

fn record_tick(ctx: &mut Ctx) {
    let t = ctx.sim_tick();
    ctx.world.resource_mut::<Ticks>().0.push(t);
}

#[test]
fn runs_the_requested_number_of_ticks() {
    let world = Game::new("t", 800, 600).sim_system(record_tick).run_headless(5, |ctx| {
        ctx.world.insert_resource(Ticks(Vec::new()));
    });
    assert_eq!(world.resource::<Ticks>().0, [0, 1, 2, 3, 4]);
}

#[test]
fn dt_is_the_fixed_step() {
    let mut world = Game::new("t", 800, 600)
        .fixed_hz(100)
        .sim_system(fall)
        .run_headless(100, |ctx| {
            ctx.world.spawn((Pos(50.0),));
        });
    let y = world.query::<&Pos>().iter().next().unwrap().0;
    assert!((y - 40.0).abs() < 1e-3, "one simulated second at 10/s, got {y}");
}

#[test]
fn sim_time_counts_ticks() {
    struct Seen(f64);
    fn see(ctx: &mut Ctx) {
        let t = ctx.sim_time();
        ctx.world.resource_mut::<Seen>().0 = t;
    }
    let world = Game::new("t", 1, 1)
        .fixed_hz(50)
        .sim_system(see)
        .run_headless(50, |ctx| {
            ctx.world.insert_resource(Seen(0.0));
        });
    // The last tick sees 49 completed ticks.
    assert!((world.resource::<Seen>().0 - 49.0 / 50.0).abs() < 1e-9);
}

#[test]
fn quit_stops_early() {
    fn quit_at_3(ctx: &mut Ctx) {
        record_tick(ctx);
        if ctx.sim_tick() == 3 {
            ctx.quit();
        }
    }
    let world = Game::new("t", 1, 1).sim_system(quit_at_3).run_headless(1000, |ctx| {
        ctx.world.insert_resource(Ticks(Vec::new()));
    });
    assert_eq!(world.resource::<Ticks>().0, [0, 1, 2, 3]);
}

#[test]
fn frame_systems_and_setup_closure_do_not_run() {
    static FRAME_RUNS: AtomicU32 = AtomicU32::new(0);
    fn frame(_: &mut Ctx) {
        FRAME_RUNS.fetch_add(1, Ordering::Relaxed);
    }
    Game::new("t", 1, 1).system(frame).run_headless(10, |_| {
        |_: &mut Ctx| {
            FRAME_RUNS.fetch_add(1, Ordering::Relaxed);
        }
    });
    assert_eq!(FRAME_RUNS.load(Ordering::Relaxed), 0);
}

#[test]
fn commands_from_setup_and_systems_apply() {
    struct Spawner;
    fn spawn_one(ctx: &mut Ctx) {
        ctx.commands.spawn((Pos(0.0),));
    }
    let world = Game::new("t", 1, 1).sim_system(spawn_one).run_headless(7, |ctx| {
        ctx.commands.spawn((Spawner,));
    });
    assert_eq!(world.len(), 8);
}

#[test]
fn textures_load_without_a_gpu() {
    let world = Game::new("t", 1, 1).run_headless(1, |ctx| {
        let crab = ctx.load_texture("crab.png");
        assert!(crab.width() > 0 && crab.height() > 0);
        let made = ctx.create_texture(2, 3, &[255; 24]);
        assert_eq!((made.width(), made.height()), (2, 3));
        assert_ne!(crab, made);
        ctx.world.spawn((Transform2D::default(), Sprite::new(crab)));
    });
    assert_eq!(world.len(), 1);
}

#[test]
fn sim_speed_rejects_negative() {
    let result = std::panic::catch_unwind(|| {
        Game::new("t", 1, 1).run_headless(0, |ctx| ctx.set_sim_speed(-1.0));
    });
    assert!(result.is_err());
}

// ---------------------------------------------------------- reduced rates

#[derive(Default)]
struct Log(Vec<(&'static str, u64, f32)>);

fn logger(name: &'static str) -> impl FnMut(&mut Ctx) {
    move |ctx| {
        let (tick, dt) = (ctx.sim_tick(), ctx.dt());
        ctx.world.resource_mut::<Log>().0.push((name, tick, dt));
    }
}

#[test]
fn every_n_systems_run_every_nth_tick_with_n_steps_of_dt() {
    let world = Game::new("t", 1, 1)
        .fixed_hz(60)
        .sim_system_every(4, logger("slow"))
        .sim_system(logger("fast"))
        .run_headless(12, |ctx| ctx.world.insert_resource(Log::default()));
    let log = &world.resource::<Log>().0;
    let slow: Vec<u64> = log.iter().filter(|l| l.0 == "slow").map(|l| l.1).collect();
    assert_eq!(slow, [0, 4, 8]);
    assert_eq!(log.iter().filter(|l| l.0 == "fast").count(), 12);
    for &(name, _, dt) in log {
        let want = if name == "slow" { 4.0 / 60.0 } else { 1.0 / 60.0 };
        assert!((dt - want).abs() < 1e-6, "{name}: {dt}");
    }
}

#[test]
fn systems_with_the_same_interval_run_on_different_ticks() {
    let world = Game::new("t", 1, 1)
        .sim_system_every(3, logger("a"))
        .sim_system_every(3, logger("b"))
        .sim_system_every(3, logger("c"))
        .run_headless(6, |ctx| ctx.world.insert_resource(Log::default()));
    let ticks = |n| -> Vec<u64> {
        world
            .resource::<Log>()
            .0
            .iter()
            .filter(|l| l.0 == n)
            .map(|l| l.1)
            .collect()
    };
    assert_eq!(
        (ticks("a"), ticks("b"), ticks("c")),
        (vec![0, 3], vec![1, 4], vec![2, 5])
    );
}

#[test]
#[should_panic(expected = "interval must be at least 1 tick")]
fn every_zero_ticks_panics() {
    let _ = Game::new("t", 1, 1).sim_system_every(0, |_: &mut Ctx| {});
}

struct Turned(u32);

fn take_turns(ctx: &mut Ctx) {
    let turn = ctx.turns(5);
    ctx.world.query::<(Entity, &mut Turned)>().par_for_each(|(e, t)| {
        if turn.includes(e) {
            t.0 += 1;
        }
    });
}

#[test]
fn each_entity_takes_one_turn_per_interval() {
    let mut world = Game::new("t", 1, 1).sim_system(take_turns).run_headless(5, |ctx| {
        ctx.world.spawn_batch((0..1003).map(|_| (Turned(0),)));
    });
    assert!(world.query::<&Turned>().iter().all(|t| t.0 == 1));
}

#[test]
#[should_panic(expected = "`every` must be at least 1")]
fn turns_of_zero_panics() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.turns(0);
    });
}
