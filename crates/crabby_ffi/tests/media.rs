//! Shapes, UI widgets, fonts, sound, screenshots and turns through the C API.
use crabby_ffi::*;
use std::ffi::{CStr, c_void};
use std::ptr::{null, null_mut};

use CrabbyStatus::*;

/// Runs `system` as a sim system for `ticks` headless ticks.
fn run(ticks: u64, system: CrabbySystemFn, user: *mut c_void) {
    unsafe {
        let game = crabby_game_new(c"media".as_ptr(), 1, 1);
        assert_eq!(crabby_game_sim_system(game, c"s".as_ptr(), 1, system, user), CRABBY_OK);
        let mut world = null_mut();
        assert_eq!(
            crabby_game_run_headless(game, ticks, None, null_mut(), &mut world),
            CRABBY_OK
        );
        crabby_world_free(world);
    }
}
fn last_error() -> String {
    unsafe { CStr::from_ptr(crabby_last_error()) }
        .to_string_lossy()
        .into_owned()
}
const WHITE: CrabbyColor = CrabbyColor {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};

#[derive(Default)]
struct Drawn {
    label: CrabbyVec2,
    measured: CrabbyVec2,
    measured_ex: CrabbyVec2,
    clicked: bool,
    over_ui: bool,
    style: Option<CrabbyUiStyle>,
}

unsafe extern "C" fn draw_everything(ctx: *mut CrabbyCtx, user: *mut c_void) {
    let d = unsafe { &mut *user.cast::<Drawn>() };
    let rect = CrabbyRect {
        min: CrabbyVec2 { x: 0.0, y: 0.0 },
        max: CrabbyVec2 { x: 100.0, y: 40.0 },
    };
    let v = CrabbyVec2 { x: 5.0, y: 5.0 };
    unsafe {
        assert_eq!(crabby_ctx_draw_rect(ctx, rect, WHITE), CRABBY_OK);
        assert_eq!(crabby_ctx_draw_line(ctx, v, rect.max, 2.0, WHITE), CRABBY_OK);
        assert_eq!(crabby_ctx_draw_circle(ctx, v, 3.0, WHITE), CRABBY_OK);
        assert_eq!(crabby_ctx_ui_panel(ctx, rect), CRABBY_OK);
        assert_eq!(
            crabby_ctx_ui_label(ctx, v, c"Crabs: 12".as_ptr(), &mut d.label),
            CRABBY_OK
        );
        assert_eq!(
            crabby_ctx_ui_label(ctx, v, c"no size wanted".as_ptr(), null_mut()),
            CRABBY_OK
        );
        assert_eq!(
            crabby_ctx_ui_button(ctx, rect, c"Go".as_ptr(), &mut d.clicked),
            CRABBY_OK
        );
        assert_eq!(crabby_ctx_mouse_over_ui(ctx, &mut d.over_ui), CRABBY_OK);
        assert_eq!(
            crabby_ctx_measure_text(ctx, c"Crabs: 12".as_ptr(), 16.0, &mut d.measured),
            CRABBY_OK
        );
        assert_eq!(
            crabby_ctx_measure_text_ex(
                ctx,
                c"Crabs: 12".as_ptr(),
                CrabbyFont::default(),
                16.0,
                &mut d.measured_ex
            ),
            CRABBY_OK
        );
        let mut params = crabby_text_params_default();
        params.size = 32.0;
        assert_eq!(crabby_ctx_draw_text_ex(ctx, c"big".as_ptr(), v, &params), CRABBY_OK);
        assert_eq!(
            crabby_ctx_save_screenshot(ctx, c"never-written.png".as_ptr()),
            CRABBY_OK
        );

        let mut style = crabby_ui_style_default();
        style.text_size = 24.0;
        style.accent = WHITE;
        assert_eq!(crabby_ctx_set_ui_style(ctx, &style), CRABBY_OK);
        let mut read = crabby_ui_style_default();
        assert_eq!(crabby_ctx_ui_style(ctx, &mut read), CRABBY_OK);
        d.style = Some(read);

        assert_eq!(crabby_ctx_set_ui_style(ctx, null()), CRABBY_INVALID_ARGUMENT);
        assert_eq!(
            crabby_ctx_ui_button(ctx, rect, c"Go".as_ptr(), null_mut()),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_ctx_draw_text_ex(ctx, c"x".as_ptr(), v, null()),
            CRABBY_INVALID_ARGUMENT
        );
    }
}

#[test]
fn shapes_text_and_widgets_work_headless() {
    let mut d = Drawn::default();
    run(1, Some(draw_everything), (&mut d as *mut Drawn).cast());
    assert!(d.label.x > 0.0 && d.label.y > 0.0, "{:?}", d.label);
    // The UI label uses the style's text size, 16 by default, like measure_text.
    assert_eq!(d.label, d.measured);
    assert_eq!(d.measured, d.measured_ex);
    assert!(!d.clicked, "nobody clicked in a headless run");
    // The headless mouse rests at (0, 0), inside the panel just drawn.
    assert!(d.over_ui);
    let style = d.style.unwrap();
    assert_eq!(style.text_size, 24.0);
    assert_eq!(style.accent, WHITE);
}

unsafe extern "C" fn unknown_font(ctx: *mut CrabbyCtx, user: *mut c_void) {
    let mut size = CrabbyVec2::default();
    let status = unsafe { crabby_ctx_measure_text_ex(ctx, c"x".as_ptr(), CrabbyFont { id: 99 }, 16.0, &mut size) };
    unsafe { *user.cast::<Option<CrabbyStatus>>() = Some(status) };
}

#[test]
fn made_up_fonts_fail_instead_of_crashing() {
    let mut status: Option<CrabbyStatus> = None;
    run(1, Some(unknown_font), (&mut status as *mut Option<CrabbyStatus>).cast());
    assert_ne!(status, Some(CRABBY_OK));
    assert!(status.is_some());
}

unsafe extern "C" fn missing_files(ctx: *mut CrabbyCtx, user: *mut c_void) {
    let errors = unsafe { &mut *user.cast::<Vec<String>>() };
    unsafe {
        let mut font = CrabbyFont::default();
        assert_ne!(
            crabby_ctx_load_font(ctx, c"no-such-font.ttf".as_ptr(), &mut font),
            CRABBY_OK
        );
        errors.push(last_error());
        let mut sound = CrabbySound { id: 0, seconds: 0.0 };
        assert_ne!(
            crabby_ctx_load_sound(ctx, c"no-such-sound.ogg".as_ptr(), &mut sound),
            CRABBY_OK
        );
        errors.push(last_error());
    }
}

#[test]
#[cfg_attr(miri, ignore)] // the asset folder lookup reads the executable path
fn missing_fonts_and_sounds_fail_with_the_engine_message() {
    let mut errors: Vec<String> = Vec::new();
    run(1, Some(missing_files), (&mut errors as *mut Vec<String>).cast());
    assert_eq!(errors.len(), 2);
    assert!(errors[0].contains("no-such-font.ttf"), "{}", errors[0]);
    assert!(errors[1].contains("no-such-sound.ogg"), "{}", errors[1]);
}

#[derive(Default)]
struct Sounds {
    beep: Option<CrabbySound>,
    playing: Vec<CrabbyPlaying>,
    errors: Vec<CrabbyStatus>,
}

unsafe extern "C" fn make_sounds(ctx: *mut CrabbyCtx, user: *mut c_void) {
    let s = unsafe { &mut *user.cast::<Sounds>() };
    let samples: Vec<f32> = (0..22050).map(|i| (i as f32 * 0.06).sin() * 0.3).collect();
    unsafe {
        let mut beep = CrabbySound { id: 0, seconds: 0.0 };
        assert_eq!(
            crabby_ctx_create_sound(ctx, 44100, samples.as_ptr(), samples.len(), &mut beep),
            CRABBY_OK
        );
        s.beep = Some(beep);
        let mut p = CrabbyPlaying { id: 7 };
        assert_eq!(crabby_ctx_play_sound(ctx, beep, null(), &mut p), CRABBY_OK);
        s.playing.push(p);
        let mut params = crabby_sound_params_default();
        params.looping = true;
        params.volume = 0.5;
        assert_eq!(crabby_ctx_play_sound(ctx, beep, &params, &mut p), CRABBY_OK);
        s.playing.push(p);
        assert_eq!(crabby_ctx_play_sound(ctx, beep, null(), null_mut()), CRABBY_OK);
        assert_eq!(crabby_ctx_stop_sound(ctx, p), CRABBY_OK);

        let mut out = beep;
        s.errors.push(crabby_ctx_create_sound(
            ctx,
            0,
            samples.as_ptr(),
            samples.len(),
            &mut out,
        ));
        s.errors.push(crabby_ctx_create_sound(ctx, 44100, null(), 4, &mut out));
        s.errors
            .push(crabby_ctx_create_sound(ctx, 44100, samples.as_ptr(), 4, null_mut()));
        // An empty sound is fine.
        assert_eq!(crabby_ctx_create_sound(ctx, 44100, null(), 0, &mut out), CRABBY_OK);
        assert_eq!(out.seconds, 0.0);
    }
}

#[test]
fn sounds_are_made_from_samples_and_silent_headless() {
    let mut s = Sounds::default();
    run(1, Some(make_sounds), (&mut s as *mut Sounds).cast());
    let beep = s.beep.unwrap();
    assert!((beep.seconds - 0.5).abs() < 1e-3, "{}", beep.seconds);
    // Headless runs play nothing, and say so with id 0.
    assert!(s.playing.iter().all(|p| p.id == 0));
    assert_eq!(s.errors, [CRABBY_INVALID_ARGUMENT; 3]);
}

#[derive(Default)]
struct TurnLog {
    seen: Vec<CrabbyTurns>,
    zero: Option<CrabbyStatus>,
}

unsafe extern "C" fn log_turns(ctx: *mut CrabbyCtx, user: *mut c_void) {
    let log = unsafe { &mut *user.cast::<TurnLog>() };
    let mut t = CrabbyTurns {
        every: 0,
        now: 0,
        dt: 0.0,
    };
    unsafe {
        assert_eq!(crabby_ctx_turns(ctx, 4, &mut t), CRABBY_OK);
        log.zero = Some(crabby_ctx_turns(ctx, 0, &mut t));
    }
    log.seen.push(t);
}

#[test]
fn turns_cycle_through_the_round_with_its_whole_dt() {
    let mut log = TurnLog::default();
    run(8, Some(log_turns), (&mut log as *mut TurnLog).cast());
    assert_eq!(
        log.seen.iter().map(|t| t.now).collect::<Vec<_>>(),
        [0, 1, 2, 3, 0, 1, 2, 3]
    );
    assert!(
        log.seen
            .iter()
            .all(|t| t.every == 4 && (t.dt - 4.0 / 60.0).abs() < 1e-6)
    );
    assert_eq!(log.zero, Some(CRABBY_INVALID_ARGUMENT));
}
