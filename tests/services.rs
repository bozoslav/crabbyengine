//! The service fields of `Ctx` (`ctx.time`, `ctx.input`, ...): each is its
//! own field, so it borrows apart from the world and from the others, and
//! the flat `Ctx` methods from before them still forward.

use ponko::*;

struct Pos(f32);

#[test]
fn time_reads_while_a_world_query_is_alive() {
    fn fall(ctx: &mut Ctx) {
        // The query borrows `ctx.world`; the clock is another field.
        for p in ctx.world.query::<&mut Pos>().iter() {
            p.0 += ctx.time.dt();
        }
    }
    let mut world = Game::new("t", 1, 1)
        .fixed_hz(10)
        .sim_system(fall)
        .run_headless(10, |ctx| {
            ctx.world.spawn((Pos(0.0),));
        });
    let p = world.query::<&Pos>().iter().next().unwrap().0;
    assert!((p - 1.0).abs() < 1e-5, "ten ticks of 0.1 s, got {p}");
}

#[test]
fn input_reads_while_a_world_query_is_alive() {
    fn jump(ctx: &mut Ctx) {
        for p in ctx.world.query::<&mut Pos>().iter() {
            if ctx.input.key_down(Key::Space) {
                p.0 += 1.0;
            }
        }
    }
    let mut world = Game::new("t", 1, 1).sim_system(jump).run_headless(3, |ctx| {
        ctx.world.spawn((Pos(0.0),));
        ctx.input.feed(InputEvent::Key(Key::Space, true));
    });
    assert_eq!(world.query::<&Pos>().iter().next().unwrap().0, 3.0);
}

#[test]
fn dt_in_a_sim_system_is_the_sim_step_and_frame_dt_stays_the_frames() {
    fn check(ctx: &mut Ctx) {
        assert_eq!(ctx.time.dt(), ctx.time.sim_dt());
        assert_eq!(ctx.time.sim_dt(), ctx.time.fixed_dt());
        assert_eq!(ctx.dt(), ctx.time.dt(), "the shortcut is the same number");
        // A headless run has one frame per tick.
        assert_eq!(ctx.time.frame_dt(), ctx.time.fixed_dt());
    }
    Game::new("t", 1, 1)
        .fixed_hz(50)
        .sim_system(check)
        .run_headless(4, |ctx| {
            // Setup is outside the simulation: `dt` is the frame's, and no frame ran yet.
            assert_eq!(ctx.time.dt(), ctx.time.frame_dt());
            assert_eq!(ctx.time.frame_dt(), 0.0);
            assert_eq!(ctx.time.sim_dt(), 1.0 / 50.0);
        });
}

#[test]
fn sim_dt_covers_every_step_of_a_slower_system() {
    fn slow(ctx: &mut Ctx) {
        assert_eq!(ctx.time.sim_dt(), 4.0 * ctx.time.fixed_dt());
        assert_eq!(ctx.time.dt(), ctx.time.sim_dt());
        ctx.world.resource_mut::<Runs>().0 += 1;
    }
    struct Runs(u32);
    let world = Game::new("t", 1, 1)
        .sim_system_every(4, slow)
        .run_headless(8, |ctx| ctx.world.insert_resource(Runs(0)));
    assert_eq!(world.resource::<Runs>().0, 2);
}

#[test]
fn the_clock_counts_ticks_and_controls_the_simulation() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        assert_eq!(ctx.time.sim_tick(), 0);
        assert_eq!(ctx.time.speed(), 1.0);
        ctx.time.set_speed(4.0);
        assert_eq!(ctx.time.speed(), 4.0);
        assert!(!ctx.time.is_paused());
        ctx.time.pause();
        assert!(ctx.time.is_paused());
        ctx.time.resume();
        assert!(!ctx.time.is_paused());
    });
    fn count(ctx: &mut Ctx) {
        let tick = ctx.time.sim_tick();
        assert_eq!(ctx.time.sim_time(), tick as f64 * f64::from(ctx.time.fixed_dt()));
    }
    Game::new("t", 1, 1)
        .fixed_hz(64)
        .sim_system(count)
        .run_headless(5, |_| {});
}

#[test]
#[should_panic(expected = "Time::set_speed: speed must be a finite number >= 0")]
fn a_negative_sim_speed_panics_with_the_service_name() {
    Game::new("t", 1, 1).run_headless(0, |ctx| ctx.time.set_speed(-1.0));
}

#[test]
fn input_edges_and_the_cursor_lock_live_on_the_input_service() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.input.feed(InputEvent::Mouse(MouseButton::Left, true));
        assert!(ctx.input.mouse_pressed(MouseButton::Left));
        assert!(ctx.input.mouse_down(MouseButton::Left));
        assert!(!ctx.input.cursor_locked());
        ctx.input.set_cursor_locked(true);
        assert!(ctx.input.cursor_locked());
        ctx.input.insert_text("hi");
        assert_eq!(ctx.input.typed_text(), "hi");
        // The shortcuts read the same state.
        ctx.input.feed(InputEvent::Key(Key::A, true));
        assert!(ctx.key_down(Key::A) && ctx.key_pressed(Key::A));
        assert_eq!(ctx.mouse_pos(), ctx.input.mouse_pos());
    });
}

#[test]
fn perf_reads_while_a_world_query_is_alive_and_scopes_borrow_nothing() {
    fn work(ctx: &mut Ctx) {
        // The guard holds no borrow of `ctx`, so the timed code may use all of it.
        let _timing = ctx.perf.scope("services/work");
        for p in ctx.world.query::<&mut Pos>().iter() {
            p.0 += ctx.perf.system_timings().len() as f32;
        }
        ctx.commands.spawn((Pos(0.0),));
    }
    let mut world = Game::new("t", 1, 1).sim_system(work).run_headless(2, |ctx| {
        ctx.perf.set_profiling(true);
        ctx.world.spawn((Pos(0.0),));
    });
    assert_eq!(world.query::<&Pos>().count(), 3);
}

#[test]
fn the_report_counts_the_entities_left_by_the_last_system() {
    fn grow(ctx: &mut Ctx) {
        ctx.commands.spawn((Pos(0.0),));
    }
    struct Seen(Vec<usize>);
    fn look(ctx: &mut Ctx) {
        let entities = ctx.perf.report().entities;
        ctx.world.resource_mut::<Seen>().0.push(entities);
    }
    let world = Game::new("t", 1, 1)
        .sim_system(grow)
        .sim_system(look)
        .run_headless(3, |ctx| ctx.world.insert_resource(Seen(Vec::new())));
    assert_eq!(world.resource::<Seen>().0, [1, 2, 3]);
}

#[test]
fn the_hud_switch_turns_profiling_on_and_scopes_get_timed() {
    fn timed(ctx: &mut Ctx) {
        let _timing = ctx.perf.scope("services/timed");
        std::hint::black_box((0..1000).sum::<u64>());
    }
    struct Names(Vec<&'static str>);
    fn read(ctx: &mut Ctx) {
        let names: Vec<_> = ctx.perf.scope_timings().iter().map(|s| s.name).collect();
        ctx.world.resource_mut::<Names>().0 = names;
    }
    let world = Game::new("t", 1, 1)
        .sim_system(timed)
        .sim_system(read)
        .run_headless(3, |ctx| {
            ctx.world.insert_resource(Names(Vec::new()));
            assert!(!ctx.perf.profiling());
            ctx.perf.set_hud(true);
            assert!(ctx.perf.hud());
            assert!(ctx.perf.profiling(), "the overlay needs the deeper numbers");
        });
    assert!(world.resource::<Names>().0.contains(&"services/timed"));
}

struct Beep(Sound);

#[test]
fn sounds_play_while_a_world_query_is_alive() {
    fn beep_each(ctx: &mut Ctx) {
        let beep = ctx.world.resource::<Beep>().0;
        for p in ctx.world.query::<&mut Pos>().iter() {
            // `ctx.audio` is its own field: no need to collect first.
            let playing = ctx.audio.play(&beep);
            ctx.audio.stop(playing);
            p.0 += 1.0;
        }
    }
    let mut world = Game::new("t", 1, 1).sim_system(beep_each).run_headless(2, |ctx| {
        let beep = ctx.audio.create(8000, &[0.0; 80]);
        ctx.world.insert_resource(Beep(beep));
        ctx.world.spawn_batch([(Pos(0.0),), (Pos(0.0),)]);
    });
    assert!(world.query::<&Pos>().iter().all(|p| p.0 == 2.0));
}

#[test]
fn the_audio_service_owns_sounds_buses_and_the_shortcuts_reach_it() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let beep = ctx.audio.create(8000, &[0.0; 8000]);
        assert_eq!(beep.seconds(), 1.0);
        assert!(ctx.is_live(beep));
        // The shortcut plays through the same service.
        let playing = ctx.play_sound(&beep);
        ctx.audio.stop(playing);
        let music = ctx.audio.bus("music");
        assert_eq!(ctx.audio.bus("music"), music, "a name gives the same bus");
        ctx.audio.set_bus_volume(music, 0.25);
        ctx.audio.mute_bus(music, true);
        let on_bus = ctx
            .audio
            .play_on(&beep, SoundParams::default(), music, AudioEffects::default());
        ctx.audio.set_effects(on_bus, AudioEffects::default());
        ctx.audio.stop(on_bus);
        assert!(ctx.audio.try_load("missing.ogg").is_err());
        ctx.free(beep);
        assert!(!ctx.is_live(beep));
    });
}

#[cfg(feature = "3d")]
#[test]
fn the_default_listener_is_the_camera_as_the_running_system_found_it() {
    fn listen(ctx: &mut Ctx) {
        // The camera the system started with is where the ears are...
        assert_eq!(ctx.audio.listener().position, ctx.camera3d.position);
        // ...and a listener of the game's own replaces it at once.
        let ears = AudioListener3D {
            position: vec3(1.0, 2.0, 3.0),
            rotation: Quat::IDENTITY,
        };
        ctx.audio.set_listener(Some(ears));
        assert_eq!(ctx.audio.listener().position, ears.position);
        ctx.audio.set_listener(None);
        ctx.camera3d.position.x += 1.0;
    }
    Game::new("t", 1, 1).sim_system(listen).run_headless(3, |ctx| {
        ctx.camera3d.position = vec3(5.0, 0.0, 0.0);
    });
}
