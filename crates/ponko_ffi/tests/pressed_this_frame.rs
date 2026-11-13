//! `ponko_ctx_pressed_this_frame`: what was pressed this frame, as bindings
//! that go straight into `ponko_action_map_set`.
mod common;
use common::*;
use ponko_ffi::*;
use std::ptr::null_mut;

use PonkoStatus::*;

unsafe fn feed(ctx: *mut PonkoCtx, kind: u32, slot: u32, input: u32, value: f32) {
    assert_eq!(unsafe { ponko_ctx_feed_input(ctx, kind, slot, input, value) }, PONKO_OK);
}

unsafe fn pressed(ctx: *mut PonkoCtx, threshold: f32) -> Vec<PonkoActionBinding> {
    let mut all = [PonkoActionBinding {
        kind: 99,
        pad: 0,
        first: 0,
        second: 0,
        third: 0,
        fourth: 0,
        deadzone: 0.0,
    }; 16];
    let mut count = 0;
    assert_eq!(
        unsafe { ponko_ctx_pressed_this_frame(ctx, threshold, all.as_mut_ptr(), all.len(), &mut count) },
        PONKO_OK,
        "{}",
        last_error()
    );
    all[..count].to_vec()
}

fn kinds(list: &[PonkoActionBinding]) -> Vec<(u32, u32, u32)> {
    list.iter().map(|b| (b.kind, b.pad, b.first)).collect()
}

#[test]
fn every_device_is_described_as_an_action_binding() {
    run_ctx(|ctx| unsafe {
        feed(ctx, 3, 1, PonkoGamepadAxis::PONKO_GAMEPAD_RIGHT_TRIGGER_AXIS as u32, 0.9);
        feed(ctx, 2, 1, PonkoGamepadButton::PONKO_GAMEPAD_WEST as u32, 1.0);
        feed(ctx, 1, 0, PonkoMouseButton::PONKO_MOUSE_RIGHT as u32, 1.0);
        feed(ctx, 0, 0, PonkoKey::PONKO_KEY_J as u32, 1.0);
        let list = pressed(ctx, 0.0);
        assert_eq!(
            kinds(&list),
            [
                (PonkoActionSource::PONKO_ACTION_KEY as u32, 0, PonkoKey::PONKO_KEY_J as u32),
                (
                    PonkoActionSource::PONKO_ACTION_MOUSE as u32,
                    0,
                    PonkoMouseButton::PONKO_MOUSE_RIGHT as u32
                ),
                (
                    PonkoActionSource::PONKO_ACTION_GAMEPAD_BUTTON as u32,
                    1,
                    PonkoGamepadButton::PONKO_GAMEPAD_WEST as u32
                ),
                (
                    PonkoActionSource::PONKO_ACTION_GAMEPAD_AXIS as u32,
                    1,
                    PonkoGamepadAxis::PONKO_GAMEPAD_RIGHT_TRIGGER_AXIS as u32
                ),
            ]
        );
        assert!(list[3].deadzone > 0.0 && list[3].deadzone < PONKO_PRESS_THRESHOLD);
    });
}

#[test]
fn the_answer_binds_directly_and_works() {
    run_ctx(|ctx| unsafe {
        feed(ctx, 0, 0, PonkoKey::PONKO_KEY_J as u32, 1.0);
        let list = pressed(ctx, 0.0);
        let map = ponko_action_map_new();
        assert_eq!(
            ponko_action_map_set(map, c"jump".as_ptr(), 0, list.as_ptr(), list.len()),
            PONKO_OK
        );
        assert_eq!(ponko_ctx_set_actions(ctx, map), PONKO_OK);
        ponko_action_map_free(map);
        let mut down = false;
        assert_eq!(ponko_ctx_action_down(ctx, c"jump".as_ptr(), &mut down), PONKO_OK);
        assert!(down);
    });
}

#[test]
fn an_axis_answer_binds_as_a_scalar_action() {
    run_ctx(|ctx| unsafe {
        feed(ctx, 3, 0, PonkoGamepadAxis::PONKO_GAMEPAD_LEFT_STICK_X as u32, -1.0);
        let list = pressed(ctx, 0.0);
        assert_eq!(list.len(), 1);
        let map = ponko_action_map_new();
        assert_eq!(
            ponko_action_map_set(map, c"steer".as_ptr(), 1, list.as_ptr(), list.len()),
            PONKO_OK
        );
        assert_eq!(ponko_ctx_set_actions(ctx, map), PONKO_OK);
        ponko_action_map_free(map);
        let mut axis = 0.0;
        assert_eq!(ponko_ctx_action_axis(ctx, c"steer".as_ptr(), &mut axis), PONKO_OK);
        assert!(axis < -0.9, "{axis}");
    });
}

#[test]
fn the_axis_threshold_is_chosen_and_zero_means_the_default() {
    run_ctx(|ctx| unsafe {
        feed(ctx, 3, 0, PonkoGamepadAxis::PONKO_GAMEPAD_LEFT_TRIGGER_AXIS as u32, 0.3);
        assert!(pressed(ctx, 0.0).is_empty(), "0.3 is under the default");
        assert_eq!(pressed(ctx, 0.25).len(), 1);
        assert!(pressed(ctx, 0.5).is_empty());
    });
}

#[test]
fn a_short_buffer_keeps_the_first_and_reports_the_full_count() {
    run_ctx(|ctx| unsafe {
        for key in [PonkoKey::PONKO_KEY_A, PonkoKey::PONKO_KEY_B, PonkoKey::PONKO_KEY_C] {
            feed(ctx, 0, 0, key as u32, 1.0);
        }
        let mut count = 0;
        let mut two = [PonkoActionBinding {
            kind: 99,
            pad: 0,
            first: 99,
            second: 0,
            third: 0,
            fourth: 0,
            deadzone: 0.0,
        }; 3];
        assert_eq!(ponko_ctx_pressed_this_frame(ctx, 0.0, two.as_mut_ptr(), 2, &mut count), PONKO_OK);
        assert_eq!(count, 3);
        assert_eq!((two[0].first, two[1].first), (PonkoKey::PONKO_KEY_A as u32, PonkoKey::PONKO_KEY_B as u32));
        assert_eq!(two[2].kind, 99, "nothing written past the capacity");
        // Counting only.
        assert_eq!(ponko_ctx_pressed_this_frame(ctx, 0.0, null_mut(), 0, &mut count), PONKO_OK);
        assert_eq!(count, 3);
    });
}

#[test]
fn bad_arguments_are_errors() {
    run_ctx(|ctx| unsafe {
        let mut count = 0;
        let mut one = [PonkoActionBinding {
            kind: 0,
            pad: 0,
            first: 0,
            second: 0,
            third: 0,
            fourth: 0,
            deadzone: 0.0,
        }];
        assert_eq!(ponko_ctx_pressed_this_frame(null_mut(), 0.0, one.as_mut_ptr(), 1, &mut count), PONKO_INVALID_ARGUMENT);
        assert_eq!(ponko_ctx_pressed_this_frame(ctx, 0.0, one.as_mut_ptr(), 1, null_mut()), PONKO_INVALID_ARGUMENT);
        assert_eq!(ponko_ctx_pressed_this_frame(ctx, 0.0, null_mut(), 1, &mut count), PONKO_INVALID_ARGUMENT);
        assert_eq!(ponko_ctx_pressed_this_frame(ctx, -1.0, one.as_mut_ptr(), 1, &mut count), PONKO_INVALID_ARGUMENT);
        assert_eq!(ponko_ctx_pressed_this_frame(ctx, f32::NAN, one.as_mut_ptr(), 1, &mut count), PONKO_INVALID_ARGUMENT);
    });
}
