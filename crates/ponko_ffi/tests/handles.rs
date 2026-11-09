//! One handle lifecycle through the C API: `crabby_ctx_handle_is_live`,
//! `crabby_ctx_handle_free` and `crabby_ctx_live_counts` for every kind.
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::null_mut;

use CrabbyHandleKind::*;
use CrabbyStatus::*;

unsafe fn is_live(ctx: *mut CrabbyCtx, kind: CrabbyHandleKind, id: u32) -> bool {
    let mut live = false;
    assert_eq!(
        unsafe { crabby_ctx_handle_is_live(ctx, kind as u32, id, &mut live) },
        CRABBY_OK
    );
    live
}

unsafe fn counts(ctx: *mut CrabbyCtx) -> CrabbyLiveCounts {
    let mut c = CrabbyLiveCounts::default();
    assert_eq!(unsafe { crabby_ctx_live_counts(ctx, &mut c) }, CRABBY_OK);
    c
}

unsafe extern "C" fn lifecycle(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let errors = &mut *user.cast::<Vec<CrabbyStatus>>();
        let before = counts(ctx);
        let white = CrabbyColor {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        };
        let mut texture = std::mem::zeroed::<CrabbyTexture>();
        assert_eq!(
            crabby_ctx_create_texture(ctx, 1, 1, [255u8; 4].as_ptr(), 4, &mut texture),
            CRABBY_OK
        );
        let mut material = CrabbyMaterial::default();
        assert_eq!(
            crabby_ctx_create_material(ctx, &crabby_material_desc(white), &mut material),
            CRABBY_OK
        );
        let mut sound = std::mem::zeroed::<CrabbySound>();
        assert_eq!(
            crabby_ctx_create_sound(ctx, 8000, [0.0f32; 8].as_ptr(), 8, &mut sound),
            CRABBY_OK
        );
        let made = [
            (CRABBY_HANDLE_TEXTURE, texture.id),
            (CRABBY_HANDLE_MATERIAL, material.id),
            (CRABBY_HANDLE_SOUND, sound.id),
        ];
        let now = counts(ctx);
        assert_eq!(now.textures, before.textures + 1);
        assert_eq!(now.materials, before.materials + 1);
        assert_eq!(now.sounds, before.sounds + 1);
        for (kind, id) in made {
            assert!(is_live(ctx, kind, id), "{kind:?} live");
            assert_eq!(crabby_ctx_handle_free(ctx, kind as u32, id), CRABBY_OK);
            assert_eq!(
                crabby_ctx_handle_free(ctx, kind as u32, id),
                CRABBY_OK,
                "again: nothing"
            );
            assert!(!is_live(ctx, kind, id), "{kind:?} dead");
        }
        assert_eq!(counts(ctx), before);
        // The C API's own checks see a freed material as gone.
        let data = crabby_mesh_data_cube(1.0);
        let mut mesh = CrabbyMesh::default();
        errors.push(crabby_ctx_create_mesh(ctx, data, material, &mut mesh));
        crabby_mesh_data_free(data);
        // No handle of any kind has the NONE id.
        for kind in 0..=10 {
            let mut live = true;
            assert_eq!(
                crabby_ctx_handle_is_live(ctx, kind, CRABBY_HANDLE_NONE, &mut live),
                CRABBY_OK
            );
            assert!(!live);
            assert_eq!(crabby_ctx_handle_free(ctx, kind, CRABBY_HANDLE_NONE), CRABBY_OK);
        }
        // The built-in font is live and is never freed.
        assert!(is_live(ctx, CRABBY_HANDLE_FONT, 0));
        assert_eq!(crabby_ctx_handle_free(ctx, CRABBY_HANDLE_FONT as u32, 0), CRABBY_OK);
        assert!(is_live(ctx, CRABBY_HANDLE_FONT, 0));
        // Errors.
        errors.push(crabby_ctx_handle_free(ctx, 11, 0));
        errors.push(crabby_ctx_handle_is_live(ctx, 0, 0, null_mut()));
        errors.push(crabby_ctx_live_counts(ctx, null_mut()));
    }
}

#[test]
fn every_kind_frees_through_one_pair_of_calls() {
    let mut errors: Vec<CrabbyStatus> = Vec::new();
    unsafe {
        let g = crabby_game_new(c"handles".as_ptr(), 64, 64);
        let mut world = null_mut();
        let user = (&raw mut errors).cast();
        assert_eq!(
            crabby_game_run_headless(g, 1, Some(lifecycle), user, &mut world),
            CRABBY_OK
        );
        crabby_world_free(world);
    }
    assert_eq!(errors, vec![CRABBY_INVALID_ARGUMENT; 4]);
}
