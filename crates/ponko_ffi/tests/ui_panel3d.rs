//! World-space UI panels through the C API: making a panel on an entity,
//! building its UI with the `ponko_ctx_ui_*` calls and a target, input,
//! the pointer ray, and errors, in headless runs (no window).
use ponko_ffi::*;
use std::ffi::{CStr, c_void};
use std::ptr::{null, null_mut};

use PonkoStatus::*;

type Ctx = *mut PonkoCtx;

fn last_error() -> String {
    unsafe { CStr::from_ptr(ponko_last_error()) }
        .to_string_lossy()
        .into_owned()
}
fn v3(x: f32, y: f32, z: f32) -> PonkoVec3 {
    PonkoVec3 { x, y, z }
}
fn pct(value: f32) -> PonkoUiLength {
    PonkoUiLength {
        kind: PonkoUiLengthKind::PONKO_UI_LENGTH_PERCENT as u32,
        value,
    }
}

/// Spawns an entity with a 3D transform at `at`.
unsafe fn spawn_at(ctx: Ctx, at: PonkoVec3) -> u64 {
    unsafe {
        let w = ponko_ctx_world(ctx);
        let mut component = 0;
        assert_eq!(ponko_transform3d_component(w, &mut component), PONKO_OK);
        let t = ponko_transform3d_at(at);
        let value = PonkoValue {
            component,
            data: (&t as *const PonkoTransform3D).cast(),
            size: size_of::<PonkoTransform3D>(),
        };
        let mut e = 0;
        assert_eq!(ponko_spawn(w, &value, 1, &mut e), PONKO_OK);
        e
    }
}

/// A panel 2 x 1 m, 200 x 100 pixels, 3 m in front of the origin.
unsafe fn make_panel(ctx: Ctx) -> u64 {
    unsafe {
        let e = spawn_at(ctx, v3(0.0, 0.0, -3.0));
        let mut texture = PonkoTexture {
            id: 0,
            width: 0,
            height: 0,
        };
        let size = PonkoVec2 { x: 2.0, y: 1.0 };
        assert_eq!(
            ponko_ctx_create_ui_panel(ctx, e, 200, 100, size, &mut texture),
            PONKO_OK,
            "{}",
            last_error()
        );
        assert_eq!((texture.width, texture.height), (200, 100));
        e
    }
}

/// A full-size button on the panel's UI, pushed as its screen.
unsafe fn full_button(ctx: Ctx, panel: u64) -> u64 {
    unsafe {
        let layout = PonkoUiLayout {
            width: pct(1.0),
            height: pct(1.0),
            ..ponko_ui_layout_default()
        };
        let mut button = PONKO_UI_NONE;
        assert_eq!(
            ponko_ctx_ui_add_button(ctx, panel, PONKO_UI_NONE, c"Buy".as_ptr(), &layout, &mut button),
            PONKO_OK,
            "{}",
            last_error()
        );
        assert_eq!(ponko_ctx_ui_push_screen(ctx, panel, button, false), PONKO_OK);
        button
    }
}

/// Runs `body` once as a headless game's setup.
fn run_setup(body: unsafe extern "C" fn(Ctx, *mut c_void)) {
    unsafe {
        let game = ponko_game_new(c"panels".as_ptr(), 1, 1);
        let mut world = null_mut();
        assert_eq!(
            ponko_game_run_headless(game, 0, Some(body), null_mut(), &mut world),
            PONKO_OK
        );
        ponko_world_free(world);
    }
}

/// Headless runs don't process input for panels (the engine does that before a
/// window's frames), so the pointer stays off; the engine's own tests cover
/// where a ray lands. These check what C can do: set and clear the ray and
/// the flag, and read the pointer.
#[test]
fn the_pointer_ray_and_interactive_flag_are_accepted() {
    unsafe extern "C" fn body(ctx: Ctx, _: *mut c_void) {
        unsafe {
            let panel = make_panel(ctx);
            // Aim straight at the panel's center.
            assert_eq!(
                ponko_ctx_set_ui_pointer_ray(ctx, v3(0.0, 0.0, 0.0), v3(0.0, 0.0, -1.0)),
                PONKO_OK
            );
            let (mut over, mut at) = (true, PonkoVec2 { x: 1.0, y: 1.0 });
            assert_eq!(ponko_ctx_ui_panel_pointer(ctx, panel, &mut over, &mut at), PONKO_OK);
            assert!(!over);
            assert_eq!((at.x, at.y), (0.0, 0.0));
            assert_eq!(ponko_ctx_set_ui_panel_interactive(ctx, panel, false), PONKO_OK);
            assert_eq!(ponko_ctx_set_ui_panel_interactive(ctx, panel, true), PONKO_OK);
            assert_eq!(ponko_ctx_clear_ui_pointer_ray(ctx), PONKO_OK);
        }
    }
    run_setup(body);
}

#[test]
fn a_panels_ui_is_built_and_clicked_with_the_same_calls() {
    unsafe extern "C" fn body(ctx: Ctx, _: *mut c_void) {
        unsafe {
            let panel = make_panel(ctx);
            let button = full_button(ctx, panel);
            // The screen's own UI is a different tree: nothing is on its stack.
            let mut top = 0;
            assert_eq!(ponko_ctx_ui_top_screen(ctx, PONKO_UI_SCREEN, &mut top), PONKO_OK);
            assert_eq!(top, PONKO_UI_NONE);
            assert_eq!(ponko_ctx_ui_top_screen(ctx, panel, &mut top), PONKO_OK);
            assert_eq!(top, button);
            // A rejected pop must leave the stack intact.
            assert_eq!(ponko_ctx_ui_pop_screen(ctx, panel, null_mut()), PONKO_INVALID_ARGUMENT);
            assert_eq!(ponko_ctx_ui_top_screen(ctx, panel, &mut top), PONKO_OK);
            assert_eq!(top, button);
            // Laid out in the panel's pixels.
            let viewport = PonkoVec2 { x: 200.0, y: 100.0 };
            assert_eq!(ponko_ctx_ui_layout(ctx, panel, viewport), PONKO_OK);
            let mut r = PonkoRect::default();
            assert_eq!(ponko_ctx_ui_rect(ctx, panel, button, &mut r), PONKO_OK);
            assert_eq!((r.max.x, r.max.y), (200.0, 100.0));
            assert_eq!(ponko_ctx_ui_set_text(ctx, panel, button, c"Sell".as_ptr()), PONKO_OK);
            let mut len = 0;
            let mut text = [0 as std::ffi::c_char; 8];
            assert_eq!(
                ponko_ctx_ui_text(ctx, panel, button, text.as_mut_ptr(), text.len(), &mut len),
                PONKO_OK
            );
            assert_eq!(len, 4);
            // Click it by hand, as a headless test would.
            let mut input = ponko_ui_input_default();
            input.pointer = PonkoVec2 { x: 100.0, y: 50.0 };
            input.pressed = true;
            input.held = true;
            assert_eq!(ponko_ctx_ui_frame(ctx, panel, viewport, &input), PONKO_OK);
            input.pressed = false;
            input.held = false;
            input.released = true;
            assert_eq!(ponko_ctx_ui_frame(ctx, panel, viewport, &input), PONKO_OK);
            let mut event = PonkoUiEvent {
                kind: PonkoUiEventKind::PONKO_UI_EVENT_CLICKED,
                id: 0,
            };
            assert_eq!(ponko_ctx_ui_next_event(ctx, panel, &mut event), PONKO_OK);
            assert_eq!(
                (event.kind, event.id),
                (PonkoUiEventKind::PONKO_UI_EVENT_CLICKED, button)
            );
            // The screen's UI got no event.
            assert_eq!(
                ponko_ctx_ui_next_event(ctx, PONKO_UI_SCREEN, &mut event),
                PONKO_NOT_FOUND
            );
            assert_eq!(ponko_ctx_ui_next_event(ctx, panel, &mut event), PONKO_NOT_FOUND);
            // Hit testing and the scale work on the panel too.
            let mut hit = 0;
            assert_eq!(ponko_ctx_ui_hit_test(ctx, panel, viewport, &mut hit), PONKO_OK);
            assert_eq!(ponko_ctx_ui_set_scale(ctx, panel, 2.0), PONKO_OK);
            // Removing the panel makes its UI unreachable but leaves the entity.
            assert_eq!(ponko_ctx_remove_ui_panel(ctx, panel), PONKO_OK);
            assert_eq!(ponko_ctx_ui_top_screen(ctx, panel, &mut top), PONKO_NOT_FOUND);
        }
    }
    run_setup(body);
}

#[test]
fn panel_calls_fail_cleanly() {
    unsafe extern "C" fn body(ctx: Ctx, _: *mut c_void) {
        unsafe {
            let plain = spawn_at(ctx, v3(0.0, 0.0, -3.0));
            let size = PonkoVec2 { x: 1.0, y: 1.0 };
            // No panel on the entity.
            let mut id = 0;
            assert_eq!(
                ponko_ctx_ui_add_panel(ctx, plain, PONKO_UI_NONE, null(), &mut id),
                PONKO_NOT_FOUND
            );
            assert_eq!(ponko_ctx_set_ui_panel_interactive(ctx, plain, false), PONKO_NOT_FOUND);
            let (mut over, mut at) = (false, PonkoVec2::default());
            assert_eq!(
                ponko_ctx_ui_panel_pointer(ctx, plain, &mut over, &mut at),
                PONKO_NOT_FOUND
            );
            // Not an entity at all.
            let ghost = plain | (99 << 32);
            assert_eq!(
                ponko_ctx_ui_add_panel(ctx, ghost, PONKO_UI_NONE, null(), &mut id),
                PONKO_INVALID_ARGUMENT
            );
            assert_eq!(
                ponko_ctx_create_ui_panel(ctx, ghost, 64, 64, size, null_mut()),
                PONKO_INVALID_ARGUMENT
            );
            assert_eq!(ponko_ctx_remove_ui_panel(ctx, ghost), PONKO_INVALID_ARGUMENT);
            assert_eq!(
                ponko_ctx_set_ui_panel_interactive(ctx, ghost, true),
                PONKO_INVALID_ARGUMENT
            );
            assert_eq!(
                ponko_ctx_ui_panel_pointer(ctx, ghost, &mut over, &mut at),
                PONKO_INVALID_ARGUMENT
            );
            // A despawned entity must not be accepted even after its slot is reused.
            let stale = spawn_at(ctx, v3(0.0, 0.0, 0.0));
            assert_eq!(ponko_despawn(ponko_ctx_world(ctx), stale), PONKO_OK);
            let _replacement = spawn_at(ctx, v3(0.0, 0.0, 0.0));
            assert_eq!(
                ponko_ctx_create_ui_panel(ctx, stale, 64, 64, size, null_mut()),
                PONKO_INVALID_ARGUMENT
            );
            // Bad sizes and rays.
            let bad = [
                (0, 64, size),
                (64, 0, size),
                (64, 64, PonkoVec2 { x: 0.0, y: 1.0 }),
                (64, 64, PonkoVec2 { x: 1.0, y: f32::NAN }),
            ];
            for (w, h, s) in bad {
                assert_eq!(
                    ponko_ctx_create_ui_panel(ctx, plain, w, h, s, null_mut()),
                    PONKO_INVALID_ARGUMENT,
                    "{w} x {h} {s:?}"
                );
            }
            let zero = v3(0.0, 0.0, 0.0);
            assert_eq!(ponko_ctx_set_ui_pointer_ray(ctx, zero, zero), PONKO_INVALID_ARGUMENT);
            assert_eq!(
                ponko_ctx_set_ui_pointer_ray(ctx, v3(f32::NAN, 0.0, 0.0), v3(0.0, 0.0, 1.0)),
                PONKO_INVALID_ARGUMENT
            );
            // The texture output is optional.
            assert_eq!(
                ponko_ctx_create_ui_panel(ctx, plain, 64, 64, size, null_mut()),
                PONKO_OK
            );
            assert_eq!(
                ponko_ctx_ui_panel_pointer(ctx, plain, null_mut(), &mut at),
                PONKO_INVALID_ARGUMENT
            );
            assert_eq!(
                ponko_ctx_ui_panel_pointer(ctx, plain, &mut over, null_mut()),
                PONKO_INVALID_ARGUMENT
            );
            assert_eq!(ponko_ctx_ui_panel_pointer(ctx, plain, &mut over, &mut at), PONKO_OK);
            assert!(!over);
            // A null ctx is an error, not a crash.
            assert_eq!(
                ponko_ctx_ui_top_screen(null_mut(), plain, &mut id),
                PONKO_INVALID_ARGUMENT
            );
        }
    }
    run_setup(body);
}
