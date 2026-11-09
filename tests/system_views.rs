//! View systems (`Game::add_view`): what a `SystemView` offers, that
//! anything undeclared panics in every build, service declarations, the
//! ordered barrier rules and the report. Headless.

use ponko::*;

#[derive(Debug, PartialEq)]
struct Pos(f32);
struct Vel(f32);
struct Health(i32);
struct Gravity(f32);
#[derive(Debug, PartialEq)]
struct Score(u32);
#[derive(Debug, PartialEq)]
struct Hit(u32);

/// What systems observed, in order.
#[derive(Default)]
struct Log(Vec<String>);

fn setup(ctx: &mut Ctx) {
    ctx.world.insert_resource(Gravity(-1.0));
    ctx.world.insert_resource(Score(0));
    ctx.world.insert_resource(Log::default());
    ctx.world.spawn((Pos(0.0), Vel(1.0), Health(3)));
}

fn run(game: Game, ticks: u64) -> World {
    game.run_headless(ticks, setup)
}

fn one(access: SystemAccess, system: impl FnMut(&mut SystemView) + 'static) -> World {
    run(
        Game::new("views", 1, 1).add_view(Phase::AfterPhysics, access, system),
        1,
    )
}

// ---- the world part ------------------------------------------------------------------

#[test]
fn a_view_system_moves_declared_components_each_tick() {
    let world = run(
        Game::new("views", 1, 1).add_view(
            Phase::AfterPhysics,
            SystemAccess::new()
                .write::<Pos>()
                .read::<Vel>()
                .read_service(Service::Time),
            |sys| {
                let dt = sys.dt();
                sys.world
                    .query::<(&mut Pos, &Vel)>()
                    .for_each(|(p, v)| p.0 += v.0 * dt * 60.0);
            },
        ),
        3,
    );
    let mut q = world;
    let x = q.query::<&Pos>().iter().next().unwrap().0;
    assert!((x - 3.0).abs() < 1e-4, "{x}");
}

#[test]
fn written_resources_are_back_in_the_world_after_the_system() {
    let world = run(
        Game::new("views", 1, 1)
            .add_view(
                Phase::AfterPhysics,
                SystemAccess::new().write_resource::<Score>().read_resource::<Gravity>(),
                |sys| {
                    let g = sys.world.resource::<Gravity>().0;
                    sys.world.resource_mut::<Score>().0 += (-g) as u32;
                },
            )
            // A plain Ctx system after it sees the resource where it belongs.
            .sim_system(|ctx: &mut Ctx| {
                let s = ctx.world.resource::<Score>().0;
                ctx.world.resource_mut::<Log>().0.push(format!("score {s}"));
            }),
        2,
    );
    assert_eq!(world.resource::<Score>(), &Score(2));
    assert_eq!(world.resource::<Log>().0, ["score 1", "score 2"]);
}

#[test]
fn commands_from_a_view_apply_before_the_next_system() {
    let world = run(
        Game::new("views", 1, 1)
            .add_view(Phase::AfterPhysics, SystemAccess::new().commands(), |sys| {
                sys.commands().spawn((Health(9),));
            })
            .sim_system(|ctx: &mut Ctx| {
                let n = ctx.world.query::<&Health>().count();
                ctx.world.resource_mut::<Log>().0.push(format!("{n}"));
            }),
        2,
    );
    assert_eq!(world.resource::<Log>().0, ["2", "3"]);
}

#[test]
#[should_panic(expected = "doesn't declare that it may write component")]
fn writing_an_undeclared_component_panics_in_every_build() {
    one(SystemAccess::new().read::<Pos>(), |sys| {
        sys.world.query::<&mut Pos>().for_each(|p| p.0 = 1.0);
    });
}

#[test]
#[should_panic(expected = "doesn't declare that it may read resource")]
fn reading_an_undeclared_resource_panics() {
    one(SystemAccess::new(), |sys| {
        let _ = sys.world.resource::<Score>();
    });
}

#[test]
#[should_panic(expected = "doesn't declare that it records commands")]
fn undeclared_commands_panic() {
    one(SystemAccess::new(), |sys| {
        sys.commands().spawn((Health(1),));
    });
}

#[test]
#[should_panic(expected = "system `system_views::the_message_names_the_system::a_named_system` doesn't declare")]
fn the_message_names_the_system() {
    fn a_named_system(sys: &mut SystemView) {
        let _ = sys.world.resource::<Score>();
    }
    one(SystemAccess::new(), a_named_system);
}

// ---- events ----------------------------------------------------------------------------

#[test]
fn view_events_follow_the_declaration_and_the_usual_visibility() {
    let world = run(
        Game::new("views", 1, 1)
            .add_view(Phase::AfterPhysics, SystemAccess::new().send_events::<Hit>(), |sys| {
                sys.events().send(Hit(1));
            })
            .add_view(
                Phase::AfterPhysics,
                SystemAccess::new().read_events::<Hit>().write_resource::<Log>(),
                |sys| {
                    let mut events = sys.events();
                    let n = events.read::<Hit>().len();
                    let kept = events.peek::<Hit>().len();
                    sys.world.resource_mut::<Log>().0.push(format!("read {n} kept {kept}"));
                },
            ),
        2,
    );
    assert_eq!(world.resource::<Log>().0, ["read 1 kept 1", "read 1 kept 2"]);
}

#[test]
#[should_panic(expected = "doesn't declare that it may send events `system_views::Hit`")]
fn sending_undeclared_events_panics() {
    one(SystemAccess::new().read_events::<Hit>(), |sys| {
        sys.events().send(Hit(0))
    });
}

#[test]
#[should_panic(expected = "doesn't declare that it may read events")]
fn reading_events_declared_only_as_sent_panics() {
    one(SystemAccess::new().send_events::<Hit>(), |sys| {
        let _ = sys.events().read::<Hit>().len();
    });
}

#[test]
#[should_panic(expected = "doesn't declare that it may read events")]
fn peeking_undeclared_events_panics() {
    one(SystemAccess::new(), |sys| {
        let _ = sys.events().peek::<Hit>().len();
    });
}

// ---- services --------------------------------------------------------------------------

#[test]
fn declared_services_are_offered() {
    let world = one(
        SystemAccess::new()
            .read_service(Service::Input)
            .read_service(Service::Time)
            .read_service(Service::Random)
            .read_service(Service::Spatial)
            .write_service(Service::Camera)
            .write_service(Service::Net)
            .write_service(Service::Audio)
            .write_resource::<Log>(),
        |sys| {
            let line = format!(
                "key {} mouse {} {} {} pos {} tick {} frame {} time {} fixed {} spatial {} player {}",
                sys.key_down(Key::Space) || sys.key_pressed(Key::Space) || sys.key_released(Key::Space),
                sys.mouse_down(MouseButton::Left),
                sys.mouse_pressed(MouseButton::Left),
                sys.mouse_released(MouseButton::Left),
                sys.mouse_pos(),
                sys.sim_tick(),
                sys.frame(),
                sys.time() >= 0.0,
                sys.fixed_dt() > 0.0,
                sys.spatial().len(),
                sys.net().player(),
            );
            let r = sys.random().stream(0).u64();
            let same = sys.random().stream(0).u64();
            assert_eq!(r, same, "one stream per system and tick");
            sys.camera_mut().zoom = 2.0;
            assert_eq!(sys.camera().zoom, 2.0);
            let _ = sys.net_mut().players();
            assert_eq!(sys.name(), sys.name());
            assert!(sys.access().service(Service::Audio).unwrap());
            sys.world.resource_mut::<Log>().0.push(line);
        },
    );
    let log = &world.resource::<Log>().0;
    assert_eq!(log.len(), 1);
    assert!(log[0].starts_with("key false mouse false false false"), "{}", log[0]);
}

#[test]
fn view_random_matches_ctx_random_for_the_same_slot() {
    // The view's stream is the one `ctx.random()` gives that system.
    let world = run(
        Game::new("views", 1, 1).add_view(
            Phase::AfterPhysics,
            SystemAccess::new()
                .read_service(Service::Random)
                .write_resource::<Log>(),
            |sys| {
                let v = sys.random().stream(3).u64();
                sys.world.resource_mut::<Log>().0.push(v.to_string());
            },
        ),
        2,
    );
    let log = &world.resource::<Log>().0;
    assert_eq!(log.len(), 2);
    assert_ne!(log[0], log[1], "a new stream every tick");
}

#[test]
#[should_panic(expected = "doesn't declare that it may read service `input`")]
fn undeclared_input_panics() {
    one(SystemAccess::new(), |sys| {
        let _ = sys.key_down(Key::Space);
    });
}

#[test]
#[should_panic(expected = "doesn't declare that it may read service `time`")]
fn undeclared_time_panics() {
    one(SystemAccess::new(), |sys| {
        let _ = sys.dt();
    });
}

#[test]
#[should_panic(expected = "doesn't declare that it may write service `camera`")]
fn reading_a_service_does_not_allow_writing_it() {
    one(SystemAccess::new().read_service(Service::Camera), |sys| {
        sys.camera_mut().zoom = 3.0;
    });
}

struct Click(Sound);

fn with_sound(access: SystemAccess, system: impl FnMut(&mut SystemView) + 'static) -> World {
    Game::new("views", 1, 1)
        .add_view(Phase::AfterPhysics, access, system)
        .run_headless(1, |ctx| {
            setup(ctx);
            let sound = ctx.create_sound(8000, &[0.0; 80]);
            ctx.world.insert_resource(Click(sound));
        })
}

#[test]
fn declared_audio_plays_sounds_and_is_silent_headless() {
    let world = with_sound(
        SystemAccess::new()
            .write_service(Service::Audio)
            .read_resource::<Click>()
            .write_resource::<Log>(),
        |sys| {
            let sound = sys.world.resource::<Click>().0;
            let a = sys.play_sound(&sound);
            let b = sys.play_sound_ex(&sound, SoundParams::default());
            sys.world.resource_mut::<Log>().0.push(format!("{}", a == b));
        },
    );
    assert_eq!(world.resource::<Log>().0, ["true"]);
}

#[test]
#[should_panic(expected = "doesn't declare that it may write service `audio`")]
fn playing_sound_needs_audio() {
    with_sound(SystemAccess::new().read_resource::<Click>(), |sys| {
        let sound = sys.world.resource::<Click>().0;
        let _ = sys.play_sound(&sound);
    });
}

#[test]
#[should_panic(expected = "declares service `gfx`, which a view doesn't offer")]
fn services_a_view_cannot_offer_are_rejected_at_registration() {
    let _ = Game::new("views", 1, 1).add_view(Phase::Frame, SystemAccess::new().write_service(Service::Gfx), |_| {});
}

#[test]
#[should_panic(expected = "declares writing service `time`, which a view only reads")]
fn writing_a_read_only_service_is_rejected_at_registration() {
    let _ = Game::new("views", 1, 1).add_view(
        Phase::AfterPhysics,
        SystemAccess::new().write_service(Service::Time),
        |_| {},
    );
}

#[test]
#[should_panic(expected = "is exclusive")]
fn an_exclusive_view_is_rejected() {
    let _ = Game::new("views", 1, 1).add_view(Phase::AfterPhysics, SystemAccess::exclusive(), |_| {});
}

#[test]
#[should_panic(expected = "which has no Rust type")]
fn named_items_are_rejected_for_views() {
    let _ = Game::new("views", 1, 1).add_view(Phase::AfterPhysics, SystemAccess::new().read_named("pos"), |_| {});
}

#[test]
fn service_declarations_conflict_like_data() {
    let read = SystemAccess::new().read_service(Service::Input);
    let write = SystemAccess::new().write_service(Service::Input);
    assert!(read.conflicts_with(&read.clone()).is_empty());
    assert_eq!(read.conflicts_with(&write), [AccessConflict::Service(Service::Input)]);
    assert_eq!(write.conflicts_before(&read), [AccessConflict::Service(Service::Input)]);
    assert!(
        write
            .conflicts_with(&SystemAccess::new().write_service(Service::Audio))
            .is_empty()
    );
    assert_eq!(read.service(Service::Input), Some(false));
    assert_eq!(
        read.clone().write_service(Service::Input).service(Service::Input),
        Some(true)
    );
    assert_eq!(read.service(Service::Audio), None);
    assert_eq!(
        AccessConflict::Service(Service::Physics3D).to_string(),
        "service `physics3d`"
    );
    assert_eq!(Service::ALL.len(), 13);
    let names: Vec<&str> = Service::ALL.iter().map(|s| s.name()).collect();
    assert_eq!(names[0], "input");
    assert_eq!(Service::Gfx.to_string(), "gfx");
}

// ---- ordered barrier rules -------------------------------------------------------------

#[test]
fn ordered_rules_relax_sends_and_commands_only_where_order_hides_them() {
    let send = SystemAccess::new().send_events::<Hit>();
    let read = SystemAccess::new().read_events::<Hit>();
    // Symmetric rules: any sender clashes.
    assert_eq!(send.conflicts_with(&read).len(), 1);
    assert_eq!(send.conflicts_with(&send.clone()).len(), 1);
    // Ordered: a reader before a sender, and two senders, are fine.
    assert!(read.conflicts_before(&send).is_empty());
    assert!(send.conflicts_before(&send.clone()).is_empty());
    assert_eq!(
        send.conflicts_before(&read),
        [AccessConflict::Event(std::any::type_name::<Hit>())]
    );

    let spawner = SystemAccess::new().commands();
    let reader = SystemAccess::new().read::<Pos>();
    assert_eq!(spawner.conflicts_with(&reader), [AccessConflict::Commands]);
    assert_eq!(reader.conflicts_with(&spawner), [AccessConflict::Commands]);
    // An earlier recorder's commands would be seen by a later world user...
    assert_eq!(spawner.conflicts_before(&reader), [AccessConflict::Commands]);
    // ...but an earlier world user never sees a later recorder's commands,
    assert!(reader.conflicts_before(&spawner).is_empty());
    // and two recorders apply in order at the barrier.
    assert!(spawner.conflicts_before(&spawner.clone()).is_empty());
    // Data and exclusivity are the same in both rule sets.
    let w = SystemAccess::new().write::<Pos>();
    assert_eq!(reader.conflicts_before(&w), reader.conflicts_with(&w));
    assert_eq!(
        SystemAccess::exclusive().conflicts_before(&reader),
        [AccessConflict::Exclusive]
    );
}

/// The sequential behaviour each ordered rule relies on, observed: a reader
/// before a sender sees the events on its next run; two senders' events
/// arrive in registration order; a world user before a command recorder
/// doesn't see its commands until the next tick.
#[test]
fn sequential_runs_show_what_the_ordered_rules_rely_on() {
    fn log(sys: &mut SystemView, line: String) {
        sys.world.resource_mut::<Log>().0.push(line);
    }
    let reader = SystemAccess::new().read_events::<Hit>().write_resource::<Log>();
    let world = run(
        Game::new("views", 1, 1)
            .add_view(Phase::AfterPhysics, reader.clone().read::<Health>(), |sys| {
                let hits: Vec<u32> = sys.events().read::<Hit>().iter().map(|h| h.0).collect();
                let n = sys.world.query::<&Health>().count();
                log(sys, format!("early {hits:?} health {n}"));
            })
            .add_view(
                Phase::AfterPhysics,
                SystemAccess::new()
                    .send_events::<Hit>()
                    .commands()
                    .read_service(Service::Time),
                |sys| {
                    let t = sys.sim_tick() as u32;
                    sys.events().send(Hit(t * 10 + 1));
                    sys.commands().spawn((Health(1),));
                },
            )
            .add_view(
                Phase::AfterPhysics,
                SystemAccess::new().send_events::<Hit>().read_service(Service::Time),
                |sys| {
                    let t = sys.sim_tick() as u32;
                    sys.events().send(Hit(t * 10 + 2));
                },
            )
            .add_view(Phase::AfterPhysics, reader, |sys| {
                let hits: Vec<u32> = sys.events().read::<Hit>().iter().map(|h| h.0).collect();
                log(sys, format!("late {hits:?}"));
            }),
        2,
    );
    assert_eq!(
        world.resource::<Log>().0,
        [
            "early [] health 1",
            "late [1, 2]",
            "early [1, 2] health 2",
            "late [11, 12]"
        ]
    );
}

// ---- the report --------------------------------------------------------------------------

fn fall(sys: &mut SystemView) {
    let g = sys.world.resource::<Gravity>().0;
    sys.world.query::<&mut Vel>().for_each(|v| v.0 += g);
}

fn decay(sys: &mut SystemView) {
    sys.world.query::<&mut Health>().for_each(|h| h.0 -= 1);
}

fn ctx_mover(ctx: &mut Ctx) {
    ctx.world.query::<&mut Pos>().for_each(|p| p.0 += 1.0);
}

#[test]
fn only_view_systems_are_parallel_candidates() {
    let game = Game::new("views", 1, 1)
        .add_view(
            Phase::AfterPhysics,
            SystemAccess::new().write::<Vel>().read_resource::<Gravity>(),
            fall,
        )
        .add_view(Phase::AfterPhysics, SystemAccess::new().write::<Health>(), decay)
        .add_with_access(Phase::AfterPhysics, SystemAccess::new().write::<Pos>(), ctx_mover);
    let report = game.access_report();
    let short = |n: &str| n.rsplit("::").next().unwrap().to_string();
    let enforced: Vec<String> = report
        .systems
        .iter()
        .filter(|s| s.enforced)
        .map(|s| short(s.name))
        .collect();
    assert_eq!(enforced, ["fall", "decay"]);
    assert_eq!(report.compatible_pairs().count(), 3, "no declared conflicts");
    let parallel: Vec<(String, String)> = report.parallel_pairs().map(|(a, b)| (short(a), short(b))).collect();
    assert_eq!(parallel, [("fall".to_string(), "decay".to_string())]);
    let text = report.to_string();
    assert!(
        text.contains("fall + system_views::decay: could run together"),
        "{text}"
    );
    assert!(text.contains("a `Ctx` system stays sequential"), "{text}");
    // And they run, in order, like any other system.
    let world = game.run_headless(1, setup);
    let mut world = world;
    let (v, h) = world
        .query::<(&Vel, &Health)>()
        .iter()
        .map(|(v, h)| (v.0, h.0))
        .next()
        .unwrap();
    assert_eq!((v, h), (0.0, 2));
}

#[test]
fn the_report_uses_the_ordered_rules() {
    let report = Game::new("views", 1, 1)
        .add_view(Phase::AfterPhysics, SystemAccess::new().read_events::<Hit>(), |_| {})
        .add_view(Phase::AfterPhysics, SystemAccess::new().send_events::<Hit>(), |_| {})
        .add_view(Phase::AfterPhysics, SystemAccess::new().read_events::<Hit>(), |_| {})
        .access_report();
    let conflicts: Vec<(usize, usize, usize)> = report
        .pairs
        .iter()
        .map(|p| (p.first, p.second, p.conflicts.len()))
        .filter(|p| p.2 > 0)
        .collect();
    // Only the sender before the second reader conflicts.
    assert_eq!(conflicts.len(), 1);
    let p = conflicts[0];
    assert!(
        report.systems[p.0]
            .access
            .as_ref()
            .unwrap()
            .conflicts_before(report.systems[p.1].access.as_ref().unwrap())
            .len()
            == 1
    );
}
