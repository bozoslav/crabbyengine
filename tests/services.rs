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

/// The flat spellings from before the services stay for one deprecation
/// cycle: each forwards to its service.
#[test]
#[allow(deprecated)]
fn the_flat_time_and_input_spellings_still_forward() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.set_sim_speed(2.0);
        assert_eq!(ctx.sim_speed(), ctx.time.speed());
        assert_eq!(ctx.time(), ctx.time.elapsed());
        assert_eq!(ctx.frame(), ctx.time.frame());
        assert_eq!(ctx.fps(), ctx.time.fps());
        assert_eq!(ctx.sim_tick(), ctx.time.sim_tick());
        assert_eq!(ctx.fixed_dt(), ctx.time.fixed_dt());
        assert_eq!(ctx.fixed_steps(), ctx.time.fixed_steps());
        assert_eq!(ctx.fixed_alpha(), ctx.time.fixed_alpha());
        ctx.pause();
        assert!(ctx.is_paused() && ctx.time.is_paused());
        ctx.resume();
        ctx.step();
        assert_eq!(ctx.turns(4), ctx.time.turns(4));

        ctx.feed_input(InputEvent::Key(Key::Space, true));
        assert!(ctx.input.key_pressed(Key::Space));
        ctx.set_cursor_locked(true);
        assert!(ctx.cursor_locked() && ctx.input.cursor_locked());
        ctx.ui_insert_text("x");
        assert_eq!(ctx.typed_text(), "x");
        assert_eq!(ctx.mouse_delta(), ctx.input.mouse_delta());
        assert_eq!(ctx.mouse_wheel(), ctx.input.mouse_wheel());
        assert_eq!(ctx.gamepads().len(), ctx.input.gamepads().len());
        ctx.set_actions(ActionMap::default());
        assert!(!ctx.action_down("jump"));
    });
}
