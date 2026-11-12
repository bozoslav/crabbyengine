//! Action ids from C: names resolved once (Rust's `ActionId`), then
//! queried without a name lookup.
mod common;
use common::*;
use ponko_ffi::*;
use std::ptr::null;

use PonkoStatus::*;

fn key(k: PonkoKey, down: bool) -> (u32, u32, u32, f32) {
    (0, 0, k as u32, if down { 1.0 } else { 0.0 })
}
unsafe fn feed(ctx: *mut PonkoCtx, (kind, slot, input, value): (u32, u32, u32, f32)) {
    assert_eq!(unsafe { ponko_ctx_feed_input(ctx, kind, slot, input, value) }, PONKO_OK);
}
unsafe fn down(ctx: *mut PonkoCtx, id: u32) -> (bool, bool, bool) {
    let (mut d, mut p, mut r) = (false, false, false);
    unsafe {
        assert_eq!(ponko_ctx_action_down_id(ctx, id, &mut d), PONKO_OK);
        assert_eq!(ponko_ctx_action_pressed_id(ctx, id, &mut p), PONKO_OK);
        assert_eq!(ponko_ctx_action_released_id(ctx, id, &mut r), PONKO_OK);
    }
    (d, p, r)
}

#[test]
fn ids_read_the_same_as_names() {
    run_ctx(|ctx| unsafe {
        install_actions(ctx);
        let mut jump = PONKO_ACTION_NONE;
        assert_eq!(ponko_ctx_action_id(ctx, c"jump".as_ptr(), &mut jump), PONKO_OK);
        assert_ne!(jump, PONKO_ACTION_NONE);
        let names = [c"throttle".as_ptr(), c"move".as_ptr()];
        let mut ids = [0u32; 2];
        assert_eq!(ponko_ctx_action_ids(ctx, names.as_ptr(), 2, ids.as_mut_ptr()), PONKO_OK);
        let [throttle, walk] = ids;
        assert!(jump != throttle && throttle != walk);

        feed(ctx, key(PonkoKey::PONKO_KEY_SPACE, true));
        feed(ctx, key(PonkoKey::PONKO_KEY_W, true));
        assert_eq!(down(ctx, jump), (true, true, false));
        let mut by_name = false;
        assert_eq!(ponko_ctx_action_down(ctx, c"jump".as_ptr(), &mut by_name), PONKO_OK);
        assert!(by_name);
        let (mut axis, mut axis2) = (0.0, PonkoVec2 { x: 0.0, y: 0.0 });
        assert_eq!(ponko_ctx_action_axis_id(ctx, throttle, &mut axis), PONKO_OK);
        assert_eq!(ponko_ctx_action_axis2_id(ctx, walk, &mut axis2), PONKO_OK);
        assert_eq!(axis, 1.0);
        assert_eq!(axis2.y, -1.0, "W is up, y down");
        feed(ctx, key(PonkoKey::PONKO_KEY_SPACE, false));
        assert_eq!(down(ctx, jump), (false, true, true), "both edges of this frame");
    });
}

#[test]
fn unknown_names_give_none_and_not_found() {
    run_ctx(|ctx| unsafe {
        install_actions(ctx);
        let mut id = 0;
        assert_eq!(ponko_ctx_action_id(ctx, c"jupm".as_ptr(), &mut id), PONKO_NOT_FOUND);
        assert_eq!(id, PONKO_ACTION_NONE);
        assert_eq!(ponko_last_error_kind(), PonkoErrorKind::PONKO_ERROR_NOT_FOUND);
        assert!(
            last_error().contains("jump"),
            "names the known actions: {}",
            last_error()
        );
        assert_eq!(down(ctx, PONKO_ACTION_NONE), (false, false, false), "none reads false");
        let mut axis = 1.0;
        assert_eq!(ponko_ctx_action_axis_id(ctx, 12345, &mut axis), PONKO_OK);
        assert_eq!(axis, 0.0, "an id naming no action reads zero");

        let names = [c"jump".as_ptr(), c"nope".as_ptr()];
        let mut ids = [7u32; 2];
        assert_eq!(
            ponko_ctx_action_ids(ctx, names.as_ptr(), 2, ids.as_mut_ptr()),
            PONKO_NOT_FOUND
        );
        assert!(last_error().contains("nope"));
        assert_ne!(ids[0], PONKO_ACTION_NONE, "known names still resolve");
        assert_eq!(ids[1], PONKO_ACTION_NONE);
        assert_eq!(
            ponko_ctx_action_ids(ctx, null(), 1, ids.as_mut_ptr()),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(ponko_ctx_action_ids(ctx, null(), 0, std::ptr::null_mut()), PONKO_OK);
    });
}

#[test]
fn ids_stay_valid_when_bindings_change() {
    run_ctx(|ctx| unsafe {
        install_actions(ctx);
        let mut jump = 0;
        assert_eq!(ponko_ctx_action_id(ctx, c"jump".as_ptr(), &mut jump), PONKO_OK);
        let map = ponko_action_map_new();
        assert_eq!(ponko_ctx_actions(ctx, map), PONKO_OK);
        let enter = [PonkoActionBinding {
            kind: 0,
            pad: 0,
            first: PonkoKey::PONKO_KEY_ENTER as u32,
            second: 0,
            third: 0,
            fourth: 0,
            deadzone: 0.0,
        }];
        assert_eq!(
            ponko_action_map_set(map, c"jump".as_ptr(), 0, enter.as_ptr(), 1),
            PONKO_OK
        );
        assert_eq!(ponko_ctx_set_actions(ctx, map), PONKO_OK);
        ponko_action_map_free(map);
        feed(ctx, key(PonkoKey::PONKO_KEY_ENTER, true));
        assert!(down(ctx, jump).0, "the old id reads the rebound action");
    });
}
