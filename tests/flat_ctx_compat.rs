//! The flat `Ctx` methods from before the services (`ctx.time`, `ctx.input`,
//! ...) stay for one deprecation cycle: each forwards to its service.
#![allow(deprecated)]

use ponko::*;

#[test]
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

#[test]
fn the_flat_perf_spellings_still_forward() {
    Game::new("t", 1, 1).run_headless(1, |ctx| {
        ctx.set_perf_hud(true);
        assert!(ctx.perf_hud() && ctx.perf.hud());
        ctx.set_profiling(true);
        assert!(ctx.profiling() && ctx.perf.profiling());
        assert_eq!(ctx.profile("compat/closure", |ctx| ctx.world.len()), 0);
        assert_eq!(ctx.system_timings().len(), ctx.perf.system_timings().len());
        assert_eq!(ctx.render_stats(), ctx.perf.render_stats());
        assert_eq!(ctx.gpu_passes().len(), ctx.perf.gpu_passes().len());
        assert_eq!(ctx.scope_timings().len(), ctx.perf.scope_timings().len());
        assert_eq!(ctx.frame_stats().fps, ctx.perf.frame_stats().fps);
        assert_eq!(ctx.perf_report().entities, ctx.perf.report().entities);
        ctx.record_trace("unused.json", 1); // headless: does nothing
    });
}

#[test]
fn the_flat_audio_spellings_still_forward() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        let beep = ctx.create_sound(8000, &[0.0; 80]);
        let playing = ctx.play_sound_ex(&beep, SoundParams::default());
        ctx.stop_sound(playing);
        let music = ctx.audio_bus("music");
        assert_eq!(music, ctx.audio.bus("music"));
        ctx.set_audio_bus_volume(music, 0.5);
        ctx.mute_audio_bus(music, false);
        let on_bus = ctx.play_sound_on(&beep, SoundParams::default(), music, AudioEffects::default());
        ctx.set_sound_effects(on_bus, AudioEffects::default());
        assert!(ctx.try_load_sound("missing.ogg").is_err());
    });
}

#[cfg(feature = "3d")]
#[test]
fn the_flat_listener_reads_the_camera_live() {
    Game::new("t", 1, 1).run_headless(0, |ctx| {
        ctx.camera3d.position = vec3(9.0, 0.0, 0.0);
        // The flat spelling had all of `Ctx`, so it saw the camera at once.
        assert_eq!(ctx.audio_listener().position, vec3(9.0, 0.0, 0.0));
        let beep = ctx.create_sound(8000, &[0.0; 80]);
        let playing = ctx.play_sound_at(&beep, vec3(1.0, 0.0, 0.0), AudioSettings3D::default());
        ctx.stop_sound(playing);
        ctx.set_audio_listener(Some(AudioListener3D::default()));
        assert_eq!(ctx.audio_listener().position, Vec3::ZERO);
    });
}
