//! Change detection in the game loop: each system's `Added`/`Changed`
//! filters and `removed` report what happened since that system last ran,
//! whatever the order of systems, phases and intervals. Also saves and loads.
//! The storage side is covered in `crates/ponko_ecs/tests/changes.rs`.

use ponko::*;
use serde::{Deserialize, Serialize};

/// What systems observed, in order.
#[derive(Default)]
struct Log(Vec<String>);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Health(i32);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Shield(i32);

fn log(ctx: &mut Ctx, line: String) {
    ctx.world.resource_mut::<Log>().0.push(line);
}

fn lines(world: &World) -> Vec<String> {
    world.resource::<Log>().0.clone()
}

/// The `Health` values this system is shown as changed, sorted.
fn changed(ctx: &mut Ctx) -> Vec<i32> {
    let mut out: Vec<i32> = ctx
        .world
        .query::<&Health>()
        .filter::<Changed<Health>>()
        .iter()
        .map(|h| h.0)
        .collect();
    out.sort();
    out
}

fn setup(ctx: &mut Ctx) {
    ctx.world.insert_resource(Log::default());
    ctx.world.track_changes::<Health>();
    for hp in [10, 20, 30] {
        ctx.world.spawn((Health(hp),));
    }
}

/// Adds 1 to the `Health` whose tens digit is `tick + 1`, on ticks 0..3.
fn hurt_one(ctx: &mut Ctx) {
    let tens = ctx.time.sim_tick() as i32 + 1;
    let target = ctx
        .world
        .query::<(Entity, &Health)>()
        .iter()
        .find(|(_, h)| h.0 / 10 == tens)
        .map(|(e, _)| e);
    if let Some(e) = target {
        ctx.world.get_mut::<Health>(e).unwrap().0 += 1;
    }
}

#[test]
fn every_system_sees_each_change_once_before_or_after_the_writer() {
    let world = Game::new("changes", 1, 1)
        .sim_system(|ctx: &mut Ctx| {
            let seen = changed(ctx);
            log(ctx, format!("before {} {seen:?}", ctx.time.sim_tick()));
        })
        .sim_system(hurt_one)
        .sim_system(|ctx: &mut Ctx| {
            let seen = changed(ctx);
            log(ctx, format!("after {} {seen:?}", ctx.time.sim_tick()));
        })
        .run_headless(5, setup);
    assert_eq!(
        lines(&world),
        [
            // Setup's spawns reach every system on its first run.
            "before 0 [10, 20, 30]",
            // The first run after the writer covers the spawns and the write.
            "after 0 [11, 20, 30]",
            // A system before the writer sees a write on its next run.
            "before 1 [11]",
            "after 1 [21]",
            "before 2 [21]",
            "after 2 [31]",
            "before 3 [31]",
            "after 3 []",
            "before 4 []",
            "after 4 []",
        ]
    );
}

#[test]
fn a_system_is_not_shown_its_own_changes_or_commands() {
    let world = Game::new("changes", 1, 1)
        .sim_system(|ctx: &mut Ctx| {
            // Reacts to changes by writing and by recording commands: it
            // must not trigger itself on the next tick.
            let seen = changed(ctx);
            log(ctx, format!("reactor {} {seen:?}", ctx.time.sim_tick()));
            let hurt: Vec<Entity> = ctx.world.query::<Entity>().filter::<Changed<Health>>().iter().collect();
            for (i, e) in hurt.into_iter().enumerate() {
                if i == 0 {
                    ctx.world.get_mut::<Health>(e).unwrap().0 += 1;
                } else {
                    ctx.commands.insert(e, Health(0));
                }
            }
        })
        .sim_system(|ctx: &mut Ctx| {
            let seen = changed(ctx);
            log(ctx, format!("watcher {} {seen:?}", ctx.time.sim_tick()));
        })
        .run_headless(3, setup);
    assert_eq!(
        lines(&world),
        [
            "reactor 0 [10, 20, 30]",
            // The reactor's direct write and its applied commands, to the next system.
            "watcher 0 [0, 0, 11]",
            // Nothing new for the reactor: the changes were its own.
            "reactor 1 []",
            "watcher 1 []",
            "reactor 2 []",
            "watcher 2 []",
        ]
    );
}

#[test]
fn a_view_system_writes_are_changes_for_the_other_systems() {
    fn shield_up(sys: &mut SystemView) {
        // Only entities with a `Shield` are borrowed mutably.
        sys.world
            .query::<(&mut Health, &Shield)>()
            .for_each(|(h, s)| h.0 += s.0);
    }
    let world = Game::new("changes", 1, 1)
        .add_view(
            Phase::AfterPhysics,
            SystemAccess::new().write::<Health>().read::<Shield>(),
            shield_up,
        )
        .sim_system(|ctx: &mut Ctx| {
            let seen = changed(ctx);
            log(ctx, format!("{} {seen:?}", ctx.time.sim_tick()));
        })
        .run_headless(3, |ctx| {
            setup(ctx);
            ctx.world.spawn((Health(100), Shield(1)));
        });
    assert_eq!(lines(&world), ["0 [10, 20, 30, 101]", "1 [102]", "2 [103]"]);
}

#[test]
fn a_mutable_query_in_one_system_is_a_change_for_the_others() {
    let world = Game::new("changes", 1, 1)
        .sim_system(|ctx: &mut Ctx| {
            // Only entities with a `Shield` are borrowed mutably.
            ctx.world
                .query::<(&mut Health, &Shield)>()
                .for_each(|(h, s)| h.0 += s.0);
        })
        .sim_system(|ctx: &mut Ctx| {
            let seen = changed(ctx);
            log(ctx, format!("{} {seen:?}", ctx.time.sim_tick()));
        })
        .run_headless(3, |ctx| {
            setup(ctx);
            ctx.world.spawn((Health(100), Shield(1)));
        });
    assert_eq!(lines(&world), ["0 [10, 20, 30, 101]", "1 [102]", "2 [103]"]);
}

#[test]
fn a_slow_system_sees_everything_since_its_last_run() {
    let world = Game::new("changes", 1, 1)
        .sim_system(hurt_one)
        .sim_system_every(2, |ctx: &mut Ctx| {
            let seen = changed(ctx);
            log(ctx, format!("slow {} {seen:?}", ctx.time.sim_tick()));
        })
        .run_headless(6, setup);
    assert_eq!(lines(&world), ["slow 0 [11, 20, 30]", "slow 2 [21, 31]", "slow 4 []"]);
}

#[test]
fn systems_in_every_fixed_phase_see_changes_from_the_other_phases() {
    let reader = |name: &'static str| {
        move |ctx: &mut Ctx| {
            let seen = changed(ctx);
            log(ctx, format!("{name} {} {seen:?}", ctx.time.sim_tick()));
        }
    };
    let world = Game::new("changes", 1, 1)
        .add(Phase::SimLast, reader("last"))
        .add(Phase::AfterPhysics, |ctx: &mut Ctx| {
            if ctx.time.sim_tick() == 1 {
                hurt_one(ctx);
            }
        })
        .add(Phase::Input, reader("input"))
        .run_headless(3, setup);
    assert_eq!(
        lines(&world),
        [
            "input 0 [10, 20, 30]",
            "last 0 [10, 20, 30]",
            "input 1 []",
            "last 1 [21]",
            "input 2 [21]",
            "last 2 []",
        ]
    );
}

#[test]
fn every_system_is_shown_each_removal_once() {
    fn removed(ctx: &mut Ctx) -> usize {
        ctx.world.removed::<Health>().len()
    }
    let world = Game::new("changes", 1, 1)
        .sim_system(|ctx: &mut Ctx| {
            let n = removed(ctx);
            log(ctx, format!("before {} {n}", ctx.time.sim_tick()));
        })
        .sim_system(|ctx: &mut Ctx| {
            // Despawns one entity per tick: one directly, the rest deferred.
            let victim = ctx.world.query::<Entity>().with::<Health>().iter().next();
            match (victim, ctx.time.sim_tick()) {
                (Some(e), 0) => {
                    ctx.world.despawn(e);
                }
                (Some(e), _) => ctx.commands.despawn(e),
                (None, _) => {}
            }
        })
        .sim_system(|ctx: &mut Ctx| {
            let n = removed(ctx);
            log(ctx, format!("after {} {n}", ctx.time.sim_tick()));
        })
        .run_headless(5, setup);
    assert_eq!(
        lines(&world),
        [
            "before 0 0",
            "after 0 1",
            "before 1 1",
            "after 1 1",
            "before 2 1",
            "after 2 1",
            "before 3 1",
            "after 3 0",
            "before 4 0",
            "after 4 0",
        ]
    );
    // Removals every system has seen are dropped, so the log doesn't grow.
    assert!(world.removed_since::<Health>(Tick::default()).is_empty());
}

#[test]
fn removal_logs_stay_small_over_a_long_run() {
    #[derive(Default)]
    struct Longest(usize);
    let world = Game::new("changes", 1, 1)
        .sim_system(|ctx: &mut Ctx| {
            // 50 short-lived entities per tick.
            let old: Vec<Entity> = ctx.world.query::<Entity>().with::<Health>().iter().collect();
            for e in old {
                ctx.commands.despawn(e);
            }
            for hp in 0..50 {
                ctx.commands.spawn((Health(hp),));
            }
        })
        .sim_system_every(4, |ctx: &mut Ctx| {
            let _ = ctx.world.removed::<Health>();
        })
        .sim_system(|ctx: &mut Ctx| {
            let kept = ctx.world.removed_since::<Health>(Tick::default()).len();
            let longest = ctx.world.resource_mut::<Longest>();
            longest.0 = longest.0.max(kept);
        })
        .run_headless(400, |ctx| {
            ctx.world.insert_resource(Longest::default());
            ctx.world.track_changes::<Health>();
        });
    // Held back by the slowest system (every 4 ticks), not by the run's length.
    let longest = world.resource::<Longest>().0;
    assert!((50..=5 * 50).contains(&longest), "kept {longest} removals");
}

#[test]
fn a_system_that_starts_using_a_filter_late_sees_every_value() {
    let world = Game::new("changes", 1, 1)
        .sim_system(|ctx: &mut Ctx| {
            // Nothing asked about `Shield` before tick 2.
            if ctx.time.sim_tick() >= 2 {
                let n = ctx.world.query::<&Shield>().filter::<Changed<Shield>>().count();
                let added = ctx.world.query::<&Shield>().filter::<Added<Shield>>().count();
                log(ctx, format!("{} changed={n} added={added}", ctx.time.sim_tick()));
            }
        })
        .run_headless(4, |ctx| {
            ctx.world.insert_resource(Log::default());
            ctx.world.spawn((Shield(1),));
            ctx.world.spawn((Shield(2),));
        });
    assert_eq!(lines(&world), ["2 changed=2 added=2", "3 changed=0 added=0"]);
}

#[test]
fn loading_a_save_counts_as_removing_and_adding_everything() {
    let world = Game::new("changes", 1, 1)
        .sim_system(|ctx: &mut Ctx| {
            let (changed, added) = (
                changed(ctx),
                ctx.world.query::<&Health>().filter::<Added<Health>>().count(),
            );
            let removed = ctx.world.removed::<Health>().len();
            log(
                ctx,
                format!(
                    "{} changed={changed:?} added={added} removed={removed}",
                    ctx.time.sim_tick()
                ),
            );
        })
        .sim_system(|ctx: &mut Ctx| match ctx.time.sim_tick() {
            1 => {
                let save = ctx.world.save();
                ctx.world.insert_resource(save);
                hurt_one(ctx);
            }
            3 => {
                let save = ctx.world.remove_resource::<serde_json::Value>().unwrap();
                ctx.world.load(&save).unwrap();
            }
            _ => {}
        })
        .run_headless(6, |ctx| {
            setup(ctx);
            ctx.world.reflect::<Health>();
        });
    assert_eq!(
        lines(&world),
        [
            "0 changed=[10, 20, 30] added=3 removed=0",
            "1 changed=[] added=0 removed=0",
            // The write made after saving.
            "2 changed=[21] added=0 removed=0",
            "3 changed=[] added=0 removed=0",
            // The load: stamps aren't saved, so every loaded value is new,
            // and every entity it replaced is reported as removed.
            "4 changed=[10, 20, 30] added=3 removed=3",
            "5 changed=[] added=0 removed=0",
        ]
    );
}

#[test]
fn replacing_the_world_shows_every_reader_everything() {
    // A system's last-run tick outlives a replaced world; the new world's
    // ticks start over, and must not hide its contents from that system.
    let world = Game::new("changes", 1, 1)
        .sim_system(|ctx: &mut Ctx| {
            let seen = changed(ctx);
            log(ctx, format!("{} {seen:?}", ctx.time.sim_tick()));
        })
        .sim_system(|ctx: &mut Ctx| {
            if ctx.time.sim_tick() == 5 {
                let mut fresh = World::new();
                fresh.insert_resource(Log(lines(&ctx.world)));
                fresh.track_changes::<Health>();
                fresh.spawn((Health(7),));
                ctx.world = fresh;
            }
        })
        .run_headless(8, setup);
    assert_eq!(lines(&world)[5..], ["5 []", "6 [7]", "7 []"]);
}

#[test]
fn a_rare_system_sees_everything_in_a_world_that_replaced_another() {
    // The reader runs every 20th tick. In between, the world is replaced
    // and the new one's ticks start over: by the reader's next run they
    // have passed the tick it remembers from the old world. That tick means
    // nothing in the new world, so the reader starts from the beginning.
    let world = Game::new("changes", 1, 1)
        .sim_system_every(20, |ctx: &mut Ctx| {
            let seen = changed(ctx);
            let removed = ctx.world.removed::<Health>().len();
            log(ctx, format!("{seen:?}, {removed} removed"));
        })
        .sim_system(|ctx: &mut Ctx| {
            if ctx.time.sim_tick() == 5 {
                let mut fresh = World::new();
                fresh.insert_resource(Log(lines(&ctx.world)));
                fresh.track_changes::<Health>();
                fresh.spawn((Health(7),));
                let gone = fresh.spawn((Health(8),));
                ctx.world = fresh;
                // Removed in the new world's first tick.
                ctx.world.despawn(gone);
            }
        })
        .run_headless(60, setup);
    assert_eq!(
        lines(&world),
        ["[10, 20, 30], 0 removed", "[7], 1 removed", "[], 0 removed"]
    );
    // Once the reader has seen the removal, it is dropped.
    assert!(world.removed_since::<Health>(Tick::default()).is_empty());
}

#[test]
fn removals_in_a_new_world_wait_for_systems_that_last_ran_in_the_old_one() {
    // Two rare readers and a writer that replaces the world. Before either
    // reader has run in the new world, its removal log must keep everything,
    // whatever ticks the readers remember from the old world.
    let world = Game::new("changes", 1, 1)
        .sim_system_every(20, |ctx: &mut Ctx| {
            let n = ctx.world.removed::<Health>().len();
            log(ctx, format!("first {n}"));
        })
        .sim_system_every(30, |ctx: &mut Ctx| {
            let n = ctx.world.removed::<Health>().len();
            log(ctx, format!("second {n}"));
        })
        .sim_system(|ctx: &mut Ctx| match ctx.time.sim_tick() {
            // Many ticks in the old world, so the readers' old ticks are
            // far past the new world's first ones.
            35 => {
                let mut fresh = World::new();
                fresh.insert_resource(Log(Vec::new()));
                fresh.track_changes::<Health>();
                for hp in [1, 2, 3] {
                    fresh.spawn((Health(hp),));
                }
                ctx.world = fresh;
            }
            36..=38 => {
                let victim = ctx.world.query::<Entity>().with::<Health>().iter().next();
                ctx.world.despawn(victim.unwrap());
            }
            _ => {}
        })
        .run_headless(100, setup);
    // Each reader is shown the three removals once, on its first run in
    // the new world, and nothing after that.
    let seen = lines(&world);
    assert_eq!(seen.iter().filter(|l| *l == "first 3").count(), 1, "{seen:?}");
    assert_eq!(seen.iter().filter(|l| *l == "second 3").count(), 1, "{seen:?}");
    assert!(seen.iter().all(|l| l.ends_with(" 3") || l.ends_with(" 0")), "{seen:?}");
    assert!(world.removed_since::<Health>(Tick::default()).is_empty());
}

#[test]
fn code_outside_systems_sees_everything() {
    let world = Game::new("changes", 1, 1)
        .sim_system(|_: &mut Ctx| {})
        .run_headless(3, setup);
    let mut world = world;
    assert_eq!(world.changes_since(), Tick::default());
    assert_eq!(world.query::<&Health>().filter::<Changed<Health>>().count(), 3);
}

#[test]
fn games_that_track_nothing_keep_no_stamps() {
    let world = Game::new("changes", 1, 1)
        .sim_system(|ctx: &mut Ctx| {
            ctx.world.query::<&mut Health>().for_each(|h| h.0 += 1);
        })
        .run_headless(3, |ctx| {
            ctx.world.spawn((Health(1), Transform2D::default()));
        });
    assert!(world.components().iter().all(|c| !c.tracks_changes()));
}
