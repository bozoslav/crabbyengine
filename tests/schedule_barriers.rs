//! When deferred commands apply and when events become visible or expire,
//! relative to systems, phases and setup. These pin the sequential contract
//! documented on `Ctx::commands`, `Events` and the guide's "Systems and the
//! frame" chapter; a future parallel scheduler must keep it or document the
//! change.

use ponko::*;

/// What systems observed, in order.
#[derive(Default)]
struct Log(Vec<String>);

#[derive(Clone, Copy, Debug, PartialEq)]
struct Marker(u32);

fn log(ctx: &mut Ctx, line: String) {
    ctx.world.resource_mut::<Log>().0.push(line);
}

fn lines(world: &World) -> Vec<String> {
    world.resource::<Log>().0.clone()
}

fn markers(ctx: &mut Ctx) -> usize {
    ctx.world.query::<&Marker>().count()
}

fn setup(ctx: &mut Ctx) {
    ctx.world.insert_resource(Log::default());
}

// ---- deferred commands ---------------------------------------------------------

#[test]
fn commands_are_not_visible_inside_the_recording_system() {
    let world = Game::new("barriers", 1, 1)
        .sim_system(|ctx: &mut Ctx| {
            let before = markers(ctx);
            ctx.commands.spawn((Marker(0),));
            let after = markers(ctx);
            log(
                ctx,
                format!("recorder {before}->{after} pending={}", ctx.commands.len()),
            );
        })
        .sim_system(|ctx: &mut Ctx| {
            let n = markers(ctx);
            log(ctx, format!("next {n} pending={}", ctx.commands.len()));
        })
        .run_headless(1, setup);
    assert_eq!(lines(&world), ["recorder 0->0 pending=1", "next 1 pending=0"]);
}

#[test]
fn every_system_boundary_is_a_barrier_across_all_fixed_phases() {
    // Registered in reverse: phases order them, and each one still sees
    // the spawns of every system that ran before it in the same tick.
    let spawn_and_count = |name: &'static str| {
        move |ctx: &mut Ctx| {
            let n = markers(ctx);
            log(ctx, format!("{name} sees {n}"));
            ctx.commands.spawn((Marker(0),));
        }
    };
    let world = Game::new("barriers", 1, 1)
        .add(Phase::SimLast, spawn_and_count("sim_last"))
        .add(Phase::AfterPhysics, spawn_and_count("after"))
        .add(Phase::Physics, spawn_and_count("physics"))
        .add(Phase::BeforePhysics, spawn_and_count("before"))
        .add(Phase::Input, spawn_and_count("input"))
        .add(Phase::Input, spawn_and_count("input2"))
        .run_headless(1, setup);
    assert_eq!(
        lines(&world),
        [
            "input sees 0",
            "input2 sees 1",
            "before sees 2",
            "physics sees 3",
            "after sees 4",
            "sim_last sees 5"
        ]
    );
    assert_eq!(world.len(), 6, "the last system's commands apply before the tick ends");
}

#[test]
fn commands_from_setup_apply_before_the_first_tick() {
    let world = Game::new("barriers", 1, 1)
        .add(Phase::Input, |ctx: &mut Ctx| {
            let n = markers(ctx);
            log(ctx, format!("tick {} sees {n}", ctx.time.sim_tick()));
        })
        .run_headless(1, |ctx| {
            setup(ctx);
            ctx.commands.spawn((Marker(1),));
            ctx.commands.spawn((Marker(2),));
            assert_eq!(ctx.world.query::<&Marker>().count(), 0, "still deferred in setup");
        });
    assert_eq!(lines(&world), ["tick 0 sees 2"]);
}

#[test]
fn commands_apply_in_recorded_order() {
    let mut world = Game::new("barriers", 1, 1)
        .sim_system(|ctx: &mut Ctx| {
            if ctx.time.sim_tick() != 0 {
                return;
            }
            let kept = ctx.world.spawn((Marker(1),));
            let gone = ctx.world.spawn((Marker(2),));
            // insert, then remove: removed. remove, then insert: present.
            ctx.commands.insert(kept, 10u8);
            ctx.commands.remove::<u8>(kept);
            ctx.commands.remove::<u16>(kept);
            ctx.commands.insert(kept, 20u16);
            // despawn, then insert: the insert finds a dead entity and does nothing.
            ctx.commands.despawn(gone);
            ctx.commands.insert(gone, 30u8);
            // Spawns get ids in recorded order.
            ctx.commands.spawn((Marker(3),));
            ctx.commands.spawn((Marker(4),));
            ctx.commands.add(|world| {
                let order: Vec<u32> = world.query::<&Marker>().iter().map(|m| m.0).collect();
                world.insert_resource(order);
            });
        })
        .run_headless(1, setup);
    let mut rows: Vec<(u32, Option<u8>, Option<u16>)> = (world.query::<(&Marker, Option<&u8>, Option<&u16>)>().iter())
        .map(|(m, a, b)| (m.0, a.copied(), b.copied()))
        .collect();
    rows.sort_by_key(|r| r.0);
    assert_eq!(rows, [(1, None, Some(20)), (3, None, None), (4, None, None)]);
    let mut seen = world.resource::<Vec<u32>>().clone();
    seen.sort_unstable();
    assert_eq!(seen, [1, 3, 4], "a closure command sees the commands before it");
}

#[test]
fn systems_skipped_by_every_neither_run_nor_flush_anything() {
    let world = Game::new("barriers", 1, 1)
        .sim_system_every(3, |ctx: &mut Ctx| ctx.commands.spawn((Marker(0),)))
        .sim_system(|ctx: &mut Ctx| {
            let n = markers(ctx);
            log(ctx, format!("{}:{n}", ctx.time.sim_tick()));
        })
        .run_headless(6, setup);
    assert_eq!(lines(&world), ["0:1", "1:1", "2:1", "3:2", "4:2", "5:2"]);
}

// ---- events and commands together ----------------------------------------------

struct Spawned;
struct Died(Entity);

#[test]
fn events_and_commands_from_one_system_are_visible_together_to_later_systems() {
    let world = Game::new("barriers", 1, 1)
        .sim_system(|ctx: &mut Ctx| {
            ctx.commands.spawn((Marker(0),));
            ctx.events.send(Spawned);
        })
        .sim_system(|ctx: &mut Ctx| {
            let events = ctx.events.read::<Spawned>().len();
            let n = markers(ctx);
            log(ctx, format!("tick {} events={events} markers={n}", ctx.time.sim_tick()));
        })
        .run_headless(2, setup);
    assert_eq!(
        lines(&world),
        ["tick 0 events=1 markers=1", "tick 1 events=1 markers=2"]
    );
}

#[test]
fn an_event_about_a_despawn_command_arrives_after_the_despawn() {
    let world = Game::new("barriers", 1, 1)
        .sim_system(|ctx: &mut Ctx| {
            if let Some(e) = ctx.world.query::<(Entity, &Marker)>().iter().map(|(e, _)| e).next() {
                ctx.commands.despawn(e);
                ctx.events.send(Died(e));
            }
        })
        .sim_system(|ctx: &mut Ctx| {
            let died: Vec<Entity> = ctx.events.read::<Died>().iter().map(|d| d.0).collect();
            for e in died {
                let alive = ctx.world.is_alive(e);
                log(ctx, format!("died alive={alive}"));
            }
        })
        .run_headless(2, |ctx| {
            setup(ctx);
            ctx.world.spawn((Marker(0),));
        });
    assert_eq!(lines(&world), ["died alive=false"]);
}

#[test]
fn readers_in_earlier_phases_see_later_phase_events_next_tick() {
    let world = Game::new("barriers", 1, 1)
        .add(Phase::SimLast, |ctx: &mut Ctx| {
            let t = ctx.time.sim_tick() as u32;
            ctx.events.send(Marker(t));
        })
        .add(Phase::Input, |ctx: &mut Ctx| {
            let got: Vec<u32> = ctx.events.read::<Marker>().iter().map(|m| m.0).collect();
            log(ctx, format!("tick {} read {got:?}", ctx.time.sim_tick()));
        })
        .run_headless(3, setup);
    assert_eq!(lines(&world), ["tick 0 read []", "tick 1 read [0]", "tick 2 read [1]"]);
}

#[test]
fn a_sender_reading_its_own_type_sees_its_own_events_once() {
    let world = Game::new("barriers", 1, 1)
        .sim_system(|ctx: &mut Ctx| {
            let t = ctx.time.sim_tick() as u32;
            ctx.events.send(Marker(t));
            let got: Vec<u32> = ctx.events.read::<Marker>().iter().map(|m| m.0).collect();
            log(ctx, format!("{got:?}"));
        })
        .run_headless(3, setup);
    assert_eq!(lines(&world), ["[0]", "[1]", "[2]"]);
}

#[test]
fn events_stay_readable_for_the_slowest_every_interval() {
    // A reader that runs every 4th tick still gets every event once:
    // kept events last at least `max(every)` ticks.
    let world = Game::new("barriers", 1, 1)
        .sim_system(|ctx: &mut Ctx| {
            let t = ctx.time.sim_tick() as u32;
            ctx.events.send(Marker(t));
            let kept = ctx.events.peek::<Marker>().len();
            log(ctx, format!("kept {kept}"));
        })
        .sim_system_every(4, |ctx: &mut Ctx| {
            let got: Vec<u32> = ctx.events.read::<Marker>().iter().map(|m| m.0).collect();
            log(ctx, format!("slow {got:?}"));
        })
        .run_headless(12, setup);
    let slow: Vec<u32> = lines(&world)
        .iter()
        .filter_map(|l| l.strip_prefix("slow "))
        .flat_map(|l| {
            l.trim_matches(['[', ']'])
                .split(", ")
                .filter(|s| !s.is_empty())
                .map(|s| s.parse::<u32>().unwrap())
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(slow, (0..9).collect::<Vec<_>>(), "every event once, in order");
    let most_kept = lines(&world)
        .iter()
        .filter_map(|l| l.strip_prefix("kept "))
        .map(|n| n.parse::<usize>().unwrap())
        .max()
        .unwrap();
    assert!(most_kept <= 6, "old events still expire, kept up to {most_kept}");
}

#[test]
fn headless_events_expire_two_ticks_after_they_were_sent() {
    let world = Game::new("barriers", 1, 1)
        .sim_system(|ctx: &mut Ctx| {
            if ctx.time.sim_tick() == 0 {
                ctx.events.send(Marker(0));
            }
            let kept = ctx.events.peek::<Marker>().len();
            log(ctx, format!("{}:{kept}", ctx.time.sim_tick()));
        })
        .run_headless(4, setup);
    assert_eq!(lines(&world), ["0:1", "1:1", "2:0", "3:0"]);
}
