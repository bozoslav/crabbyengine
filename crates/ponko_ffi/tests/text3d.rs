//! Owned SDF text: round trips, validation, buffer boundaries and removal.
use ponko_ffi::{PonkoStatus::*, *};
use std::{ffi::c_void, ptr::null_mut};
unsafe extern "C" fn setup(ctx: *mut PonkoCtx, _: *mut c_void) {
    // SAFETY: live callback context and local, correctly sized outputs.
    unsafe {
        let w = ponko_ctx_world(ctx);
        let mut e = 0;
        assert_eq!(ponko_spawn(w, std::ptr::null(), 0, &mut e), PONKO_OK);
        let mut v = ponko_text3d_default();
        assert_eq!(ponko_ctx_text3d(ctx, e, &mut v), PONKO_NOT_FOUND);
        v.size = 0.7;
        v.align = 2;
        v.billboard = true;
        v.outline_width = 0.06;
        v.softness = 0.03;
        v.outline = PonkoColor {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };
        assert_eq!(ponko_ctx_set_text3d(ctx, e, c"Hé\n世界".as_ptr(), &v), PONKO_OK);
        let mut got = ponko_text3d_default();
        assert_eq!(ponko_ctx_text3d(ctx, e, &mut got), PONKO_OK);
        assert_eq!(
            (got.size, got.align, got.billboard, got.outline_width, got.softness),
            (0.7, 2, true, 0.06, 0.03)
        );
        assert_eq!(got.outline.g, 0.2);
        let mut n = 0;
        assert_eq!(ponko_ctx_text3d_text(ctx, e, null_mut(), 0, &mut n), PONKO_OK);
        assert_eq!(n, "Hé\n世界".len());
        let mut bytes = vec![9u8; n + 1];
        assert_eq!(
            ponko_ctx_text3d_text(ctx, e, bytes.as_mut_ptr().cast(), n, &mut n),
            PONKO_OK
        );
        assert_eq!(bytes[0], 0);
        assert_eq!(
            ponko_ctx_text3d_text(ctx, e, bytes.as_mut_ptr().cast(), bytes.len(), &mut n),
            PONKO_OK
        );
        assert_eq!(&bytes[..n], "Hé\n世界".as_bytes());
        assert_eq!(bytes[n], 0);
        for bad in [
            PonkoText3D { align: 3, ..v },
            PonkoText3D {
                font: PonkoFont { id: u32::MAX },
                ..v
            },
            PonkoText3D { size: f32::NAN, ..v },
            PonkoText3D { softness: -1.0, ..v },
        ] {
            assert_eq!(
                ponko_ctx_set_text3d(ctx, e, c"bad".as_ptr(), &bad),
                PONKO_INVALID_ARGUMENT
            );
        }
        assert_eq!(
            ponko_ctx_set_text3d(ctx, e, c"\u{ff}".as_ptr(), std::ptr::null()),
            PONKO_INVALID_ARGUMENT
        );
        let invalid_utf8 = [255u8, 0];
        assert_eq!(
            ponko_ctx_set_text3d(ctx, e, invalid_utf8.as_ptr().cast(), &v),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(
            ponko_ctx_set_text3d(ctx, e, std::ptr::null(), &v),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(ponko_ctx_text3d(ctx, e, null_mut()), PONKO_INVALID_ARGUMENT);
        assert_eq!(
            ponko_ctx_text3d_text(ctx, e, null_mut(), 1, &mut n),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(
            ponko_ctx_text3d_text(ctx, e, null_mut(), 0, null_mut()),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(
            ponko_ctx_text3d_text(ctx, e, bytes.as_mut_ptr().cast(), bytes.len(), &mut n),
            PONKO_OK
        );
        assert_eq!(&bytes[..n], "Hé\n世界".as_bytes());
        assert_eq!(ponko_ctx_set_text3d(ctx, e, c"".as_ptr(), &v), PONKO_OK);
        assert_eq!(
            ponko_ctx_text3d_text(ctx, e, bytes.as_mut_ptr().cast(), 1, &mut n),
            PONKO_OK
        );
        assert_eq!((n, bytes[0]), (0, 0));
        assert_eq!(ponko_ctx_remove_text3d(ctx, e), PONKO_OK);
        assert_eq!(ponko_ctx_remove_text3d(ctx, e), PONKO_OK);
        assert_eq!(ponko_ctx_text3d_text(ctx, e, null_mut(), 0, &mut n), PONKO_NOT_FOUND);
        assert_eq!(ponko_despawn(w, e), PONKO_OK);
        assert_eq!(ponko_ctx_set_text3d(ctx, e, c"x".as_ptr(), &v), PONKO_INVALID_ARGUMENT);
        assert_eq!(ponko_ctx_text3d(ctx, e, &mut got), PONKO_INVALID_ARGUMENT);
        assert_eq!(
            ponko_ctx_text3d_text(ctx, e, null_mut(), 0, &mut n),
            PONKO_INVALID_ARGUMENT
        );
        assert_eq!(ponko_ctx_remove_text3d(ctx, e), PONKO_INVALID_ARGUMENT);
    }
}
#[test]
fn world_text_round_trips_and_rejects_bad_input() {
    // SAFETY: callback obeys the ABI contract, world freed exactly once.
    unsafe {
        let g = ponko_game_new(c"world text".as_ptr(), 1, 1);
        let mut w = null_mut();
        assert_eq!(ponko_game_run_headless(g, 0, Some(setup), null_mut(), &mut w), PONKO_OK);
        assert_eq!(ponko_world_free(w), PONKO_OK);
    }
}
