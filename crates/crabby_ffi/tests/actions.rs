use CrabbyStatus::*;
use ponko_ffi::*;
use std::ffi::{CStr, CString, c_void};
use std::ptr::{null, null_mut};

fn binding(kind: u32, first: u32) -> CrabbyActionBinding {
    CrabbyActionBinding {
        kind,
        pad: 0,
        first,
        second: 0,
        third: 0,
        fourth: 0,
        deadzone: 0.2,
    }
}
unsafe fn save(map: *mut CrabbyActionMap) -> String {
    unsafe {
        let mut len = 0;
        assert_eq!(crabby_action_map_save(map, null_mut(), 0, &mut len), CRABBY_OK);
        let mut buffer = vec![0i8; len + 1];
        assert_eq!(
            crabby_action_map_save(map, buffer.as_mut_ptr(), buffer.len(), &mut len),
            CRABBY_OK
        );
        CStr::from_ptr(buffer.as_ptr()).to_str().unwrap().to_owned()
    }
}

#[test]
fn all_sources_round_trip_and_invalid_updates_are_atomic() {
    unsafe {
        let map = crabby_action_map_new();
        let buttons = [
            binding(0, CrabbyKey::CRABBY_KEY_SPACE as u32),
            binding(1, 0),
            binding(2, 0),
        ];
        assert_eq!(
            crabby_action_map_set(map, c"jump".as_ptr(), 0, buttons.as_ptr(), buttons.len()),
            CRABBY_OK
        );
        let mut keys = binding(3, CrabbyKey::CRABBY_KEY_S as u32);
        keys.second = CrabbyKey::CRABBY_KEY_W as u32;
        let axes = [
            keys,
            binding(4, CrabbyGamepadAxis::CRABBY_GAMEPAD_RIGHT_TRIGGER_AXIS as u32),
        ];
        assert_eq!(
            crabby_action_map_set(map, c"throttle".as_ptr(), 1, axes.as_ptr(), axes.len()),
            CRABBY_OK
        );
        let mut keys2 = binding(5, CrabbyKey::CRABBY_KEY_A as u32);
        keys2.second = CrabbyKey::CRABBY_KEY_D as u32;
        keys2.third = CrabbyKey::CRABBY_KEY_W as u32;
        keys2.fourth = CrabbyKey::CRABBY_KEY_S as u32;
        let vectors = [keys2, binding(6, 0)];
        assert_eq!(
            crabby_action_map_set(map, c"move".as_ptr(), 2, vectors.as_ptr(), vectors.len()),
            CRABBY_OK
        );
        let before = save(map);
        for (kind, source) in [
            (0, binding(0, u32::MAX)),
            (0, binding(1, 999)),
            (0, binding(2, 999)),
            (1, binding(4, 999)),
            (2, binding(6, 5)),
            (0, binding(6, 0)),
        ] {
            assert_eq!(
                crabby_action_map_set(map, c"jump".as_ptr(), kind, &source, 1),
                CRABBY_INVALID_ARGUMENT
            );
        }
        let mut bad = binding(2, 0);
        bad.pad = 8;
        assert_eq!(
            crabby_action_map_set(map, c"jump".as_ptr(), 0, &bad, 1),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_action_map_set(map, c"jump".as_ptr(), 99, null(), 0),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_action_map_set(map, c"jump".as_ptr(), 0, null(), 1),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_action_map_set(map, null(), 0, null(), 0),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(crabby_action_map_load(map, c"bad".as_ptr()), CRABBY_INVALID_ARGUMENT);
        assert_eq!(save(map), before);
        let restored = crabby_action_map_new();
        let json = CString::new(before.clone()).unwrap();
        assert_eq!(crabby_action_map_load(restored, json.as_ptr()), CRABBY_OK);
        assert_eq!(save(restored), before);
        let mut len = 0;
        let mut small = [42i8; 1];
        assert_eq!(crabby_action_map_save(map, small.as_mut_ptr(), 1, &mut len), CRABBY_OK);
        assert_eq!(small, [0]);
        assert_eq!(len, before.len());
        assert_eq!(
            crabby_action_map_save(map, null_mut(), 1, &mut len),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_action_map_save(map, null_mut(), 0, null_mut()),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(crabby_action_map_remove(restored, c"jump".as_ptr()), CRABBY_OK);
        assert!(!save(restored).contains("jump"));
        crabby_action_map_free(restored);
        crabby_action_map_free(map);
        crabby_action_map_free(null_mut());
    }
}

unsafe extern "C" fn setup(ctx: *mut CrabbyCtx, _: *mut c_void) {
    unsafe {
        let map = crabby_action_map_new();
        let b = binding(0, CrabbyKey::CRABBY_KEY_SPACE as u32);
        assert_eq!(crabby_action_map_set(map, c"jump".as_ptr(), 0, &b, 1), CRABBY_OK);
        let axis = binding(4, CrabbyGamepadAxis::CRABBY_GAMEPAD_RIGHT_TRIGGER_AXIS as u32);
        assert_eq!(crabby_action_map_set(map, c"throttle".as_ptr(), 1, &axis, 1), CRABBY_OK);
        let stick = binding(6, 0);
        assert_eq!(crabby_action_map_set(map, c"move".as_ptr(), 2, &stick, 1), CRABBY_OK);
        assert_eq!(crabby_ctx_set_actions(ctx, map), CRABBY_OK);
        assert_eq!(crabby_ctx_actions(ctx, map), CRABBY_OK);
        assert!(save(map).contains("jump"));
        crabby_action_map_free(map);
        assert_eq!(crabby_ctx_feed_input(ctx, 0, 0, b.first, 1.0), CRABBY_OK);
        let mut state = false;
        assert_eq!(crabby_ctx_action_down(ctx, c"jump".as_ptr(), &mut state), CRABBY_OK);
        assert!(state);
        assert_eq!(crabby_ctx_action_pressed(ctx, c"jump".as_ptr(), &mut state), CRABBY_OK);
        assert!(state);
        assert_eq!(crabby_ctx_feed_input(ctx, 0, 0, b.first, 0.0), CRABBY_OK);
        assert_eq!(crabby_ctx_action_released(ctx, c"jump".as_ptr(), &mut state), CRABBY_OK);
        assert!(state);
        assert_eq!(crabby_ctx_feed_input(ctx, 3, 0, axis.first, 0.6), CRABBY_OK);
        let mut scalar = 0.0;
        assert_eq!(
            crabby_ctx_action_axis(ctx, c"throttle".as_ptr(), &mut scalar),
            CRABBY_OK
        );
        assert!((scalar - 0.5).abs() < 1e-6);
        assert_eq!(crabby_ctx_feed_input(ctx, 3, 0, 0, 1.0), CRABBY_OK);
        let mut vector = CrabbyVec2 { x: 0.0, y: 0.0 };
        assert_eq!(crabby_ctx_action_axis2(ctx, c"move".as_ptr(), &mut vector), CRABBY_OK);
        assert_eq!(vector.x, 1.0);
        assert_eq!(crabby_ctx_feed_input(ctx, 1, 0, 0, 1.0), CRABBY_OK);
        assert_eq!(crabby_ctx_feed_input(ctx, 2, 0, 0, 1.0), CRABBY_OK);
        assert_eq!(crabby_ctx_feed_input(ctx, 4, 0, 0, 0.0), CRABBY_OK);
        assert_eq!(crabby_ctx_feed_input(ctx, 5, 0, 0, 0.0), CRABBY_OK);
        assert_eq!(crabby_ctx_feed_input(ctx, 99, 0, 0, 0.0), CRABBY_INVALID_ARGUMENT);
        assert_eq!(crabby_ctx_action_down(ctx, null(), &mut state), CRABBY_INVALID_ARGUMENT);
        assert_eq!(
            crabby_ctx_action_down(ctx, c"jump".as_ptr(), null_mut()),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(crabby_ctx_set_actions(ctx, null_mut()), CRABBY_INVALID_ARGUMENT);
    }
}
#[test]
fn context_calls_use_the_same_actions_as_rust() {
    unsafe {
        let game = crabby_game_new(c"actions".as_ptr(), 1, 1);
        let mut world = null_mut();
        assert_eq!(
            crabby_game_run_headless(game, 0, Some(setup), null_mut(), &mut world),
            CRABBY_OK
        );
        assert_eq!(crabby_world_free(world), CRABBY_OK);
    }
}
