//! Events between systems: `ctx.events.send` and `ctx.events.read`.

use ponko::*;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Ping(u64);

/// What each reader saw, by tick: (tick, ping).
#[derive(Default)]
struct Seen(Vec<(&'static str, u64, u64)>);

fn send_ping(ctx: &mut Ctx) {
    let t = ctx.time.sim_tick();
    ctx.events.send(Ping(t));
}

fn reader(name: &'static str) -> impl FnMut(&mut Ctx) {
    move |ctx: &mut Ctx| {
        let t = ctx.time.sim_tick();
        for p in ctx.events.read::<Ping>() {
            ctx.world.resource_mut::<Seen>().0.push((name, t, p.0));
        }
    }
}

fn seen_by(world: &World, name: &str) -> Vec<(u64, u64)> {
    let seen = world.resource::<Seen>();
    seen.0.iter().filter(|s| s.0 == name).map(|s| (s.1, s.2)).collect()
}

fn setup(ctx: &mut Ctx) {
    ctx.world.insert_resource(Seen::default());
}

#[test]
fn readers_after_the_sender_see_events_in_the_same_tick() {
    let world = Game::new("events", 320, 240)
        .sim_system(send_ping)
        .sim_system(reader("after"))
        .run_headless(3, setup);
    assert_eq!(seen_by(&world, "after"), [(0, 0), (1, 1), (2, 2)]);
}

#[test]
fn readers_before_the_sender_see_events_next_tick() {
    let world = Game::new("events", 320, 240)
        .sim_system(reader("before"))
        .sim_system(send_ping)
        .run_headless(3, setup);
    assert_eq!(seen_by(&world, "before"), [(1, 0), (2, 1)]);
}

#[test]
fn every_reader_sees_every_event_exactly_once() {
    let world = Game::new("events", 320, 240)
        .sim_system(reader("a"))
        .sim_system(send_ping)
        .sim_system(reader("b"))
        .sim_system(reader("c"))
        .run_headless(50, setup);
    let pings = |name| seen_by(&world, name).iter().map(|s| s.1).collect::<Vec<_>>();
    let all: Vec<u64> = (0..50).collect();
    assert_eq!(pings("a"), all[..49]);
    assert_eq!(pings("b"), all);
    assert_eq!(pings("c"), all);
}

#[test]
fn slow_systems_see_every_event_since_their_last_run() {
    let world = Game::new("events", 320, 240)
        .sim_system(send_ping)
        .sim_system_every(4, reader("slow"))
        .run_headless(12, setup);
    let got: Vec<u64> = seen_by(&world, "slow").iter().map(|s| s.1).collect();
    let all: Vec<u64> = (0..12).collect();
    assert_eq!(got, all[..got.len()], "none skipped, in order");
    assert!(got.len() >= 9, "read on every run: {got:?}");
}

#[test]
fn events_sent_in_setup_reach_sim_systems() {
    let world = Game::new("events", 320, 240)
        .sim_system(reader("r"))
        .run_headless(2, |ctx| {
            setup(ctx);
            ctx.events.send(Ping(99));
        });
    assert_eq!(seen_by(&world, "r"), [(0, 99)]);
}

// ANCHOR: events
struct Health(i32);
struct Died {
    who: Entity,
}

/// Sends one `Died` per entity out of health, from inside a query.
fn deaths(ctx: &mut Ctx) {
    ctx.world.query::<(Entity, &Health)>().for_each(|(e, h)| {
        if h.0 <= 0 {
            ctx.events.send(Died { who: e });
        }
    });
}

/// Any later system (or this one next tick) reads them, once each.
fn funerals(ctx: &mut Ctx) {
    for d in ctx.events.read::<Died>() {
        ctx.world.despawn(d.who);
    }
}
// ANCHOR_END: events

#[test]
fn events_can_be_sent_from_inside_a_query() {
    let world = Game::new("events", 320, 240)
        .sim_system(deaths)
        .sim_system(funerals)
        .run_headless(1, |ctx| {
            for hp in [0, 3, -1, 5] {
                ctx.world.spawn((Health(hp),));
            }
        });
    assert_eq!(world.len(), 2);
}

#[test]
fn old_events_are_dropped() {
    let world = Game::new("events", 320, 240)
        .sim_system(send_ping)
        .sim_system(|ctx| {
            let kept = ctx.events.peek::<Ping>().len();
            ctx.world.resource_mut::<Seen>().0.push(("kept", 0, kept as u64));
        })
        .run_headless(100, setup);
    let most = seen_by(&world, "kept").iter().map(|s| s.1).max().unwrap();
    assert!(most <= 3, "only the last couple of ticks are kept, saw {most}");
}

// ANCHOR: events_reflect
#[derive(serde::Serialize)]
struct Scored {
    points: u32,
}

fn setup_tools(ctx: &mut Ctx) {
    // Tools (the control endpoint, MCP) can now collect Scored events as JSON.
    ctx.events.reflect::<Scored>();
}
// ANCHOR_END: events_reflect

#[test]
fn reflecting_an_event_type_twice_is_fine() {
    Game::new("events", 320, 240).run_headless(0, |ctx| {
        setup_tools(ctx);
        setup_tools(ctx);
        ctx.events.send(Scored { points: 3 });
        assert_eq!(ctx.events.peek::<Scored>()[0].points, 3);
    });
}

mod other {
    #[derive(serde::Serialize)]
    pub struct Scored;
}

#[test]
#[should_panic(expected = "two event types would both be named `Scored`")]
fn two_event_types_with_one_name_are_refused() {
    Game::new("events", 320, 240).run_headless(0, |ctx| {
        ctx.events.reflect::<Scored>();
        ctx.events.reflect::<other::Scored>();
    });
}
