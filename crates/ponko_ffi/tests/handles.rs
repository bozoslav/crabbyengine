//! One handle lifecycle through the C API: `ponko_ctx_handle_is_live`,
//! `ponko_ctx_handle_free` and `ponko_ctx_live_counts` for every kind.
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::null_mut;

use PonkoHandleKind::*;
use PonkoStatus::*;

unsafe fn is_live(ctx: *mut PonkoCtx, kind: PonkoHandleKind, id: u32) -> bool {
    let mut live = false;
    assert_eq!(
        unsafe { ponko_ctx_handle_is_live(ctx, kind as u32, id, &mut live) },
        PONKO_OK
    );
    live
}

unsafe fn counts(ctx: *mut PonkoCtx) -> PonkoLiveCounts {
    let mut c = PonkoLiveCounts::default();
    assert_eq!(unsafe { ponko_ctx_live_counts(ctx, &mut c) }, PONKO_OK);
    c
}

unsafe extern "C" fn lifecycle(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let errors = &mut *user.cast::<Vec<PonkoStatus>>();
        let before = counts(ctx);
        let white = PonkoColor {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        };
        let mut texture = std::mem::zeroed::<PonkoTexture>();
        assert_eq!(
            ponko_ctx_create_texture(ctx, 1, 1, [255u8; 4].as_ptr(), 4, &mut texture),
            PONKO_OK
        );
        let mut material = PonkoMaterial::default();
        assert_eq!(
            ponko_ctx_create_material(ctx, &ponko_material_desc(white), &mut material),
            PONKO_OK
        );
        let mut sound = std::mem::zeroed::<PonkoSound>();
        assert_eq!(
            ponko_ctx_create_sound(ctx, 8000, [0.0f32; 8].as_ptr(), 8, &mut sound),
            PONKO_OK
        );
        let made = [
            (PONKO_HANDLE_TEXTURE, texture.id),
            (PONKO_HANDLE_MATERIAL, material.id),
            (PONKO_HANDLE_SOUND, sound.id),
        ];
        let now = counts(ctx);
        assert_eq!(now.textures, before.textures + 1);
        assert_eq!(now.materials, before.materials + 1);
        assert_eq!(now.sounds, before.sounds + 1);
        for (kind, id) in made {
            assert!(is_live(ctx, kind, id), "{kind:?} live");
            assert_eq!(ponko_ctx_handle_free(ctx, kind as u32, id), PONKO_OK);
            assert_eq!(ponko_ctx_handle_free(ctx, kind as u32, id), PONKO_OK, "again: nothing");
            assert!(!is_live(ctx, kind, id), "{kind:?} dead");
        }
        assert_eq!(counts(ctx), before);
        // The C API's own checks see a freed material as gone.
        let data = ponko_mesh_data_cube(1.0);
        let mut mesh = PonkoMesh::default();
        errors.push(ponko_ctx_create_mesh(ctx, data, material, &mut mesh));
        ponko_mesh_data_free(data);
        // No handle of any kind has the NONE id.
        for kind in 0..=10 {
            let mut live = true;
            assert_eq!(
                ponko_ctx_handle_is_live(ctx, kind, PONKO_HANDLE_NONE, &mut live),
                PONKO_OK
            );
            assert!(!live);
            assert_eq!(ponko_ctx_handle_free(ctx, kind, PONKO_HANDLE_NONE), PONKO_OK);
        }
        // The built-in font is live and is never freed.
        assert!(is_live(ctx, PONKO_HANDLE_FONT, 0));
        assert_eq!(ponko_ctx_handle_free(ctx, PONKO_HANDLE_FONT as u32, 0), PONKO_OK);
        assert!(is_live(ctx, PONKO_HANDLE_FONT, 0));
        // Errors.
        errors.push(ponko_ctx_handle_free(ctx, 11, 0));
        errors.push(ponko_ctx_handle_is_live(ctx, 0, 0, null_mut()));
        errors.push(ponko_ctx_live_counts(ctx, null_mut()));
    }
}

#[test]
fn every_kind_frees_through_one_pair_of_calls() {
    let mut errors: Vec<PonkoStatus> = Vec::new();
    unsafe {
        let g = ponko_game_new(c"handles".as_ptr(), 64, 64);
        let mut world = null_mut();
        let user = (&raw mut errors).cast();
        assert_eq!(
            ponko_game_run_headless(g, 1, Some(lifecycle), user, &mut world),
            PONKO_OK
        );
        ponko_world_free(world);
    }
    assert_eq!(errors, vec![PONKO_INVALID_ARGUMENT; 4]);
}

unsafe extern "C" fn accounting(ctx: *mut PonkoCtx, user: *mut c_void) {
    use PonkoAssetClass::*;
    unsafe {
        let errors = &mut *user.cast::<Vec<PonkoStatus>>();
        let count = |class: PonkoAssetClass| {
            let mut c = PonkoAssetCount::default();
            assert_eq!(ponko_ctx_asset_count(ctx, class as u32, &mut c), PONKO_OK);
            c
        };
        let before = count(PONKO_ASSET_CLASS_TEXTURE);
        let mut texture = std::mem::zeroed::<PonkoTexture>();
        let pixels = [0u8; 2 * 2 * 4];
        assert_eq!(
            ponko_ctx_create_texture(ctx, 2, 2, pixels.as_ptr(), pixels.len(), &mut texture),
            PONKO_OK
        );
        let after = count(PONKO_ASSET_CLASS_TEXTURE);
        assert_eq!((after.live, after.bytes), (before.live + 1, before.bytes + 16));
        let mut n = 0;
        let class = PONKO_ASSET_CLASS_TEXTURE as u32;
        assert_eq!(ponko_ctx_asset_handles(ctx, class, null_mut(), 0, &mut n), PONKO_OK);
        assert_eq!(n, after.live);
        let mut records = vec![PonkoAssetRecord::default(); n];
        assert_eq!(
            ponko_ctx_asset_handles(ctx, class, records.as_mut_ptr(), n, &mut n),
            PONKO_OK
        );
        let made = records.iter().find(|r| r.id == texture.id).expect("listed");
        assert_eq!(made.owner, PonkoAssetOwner::PONKO_ASSET_OWNER_CODE as u32);
        assert_eq!((made.bytes, made.age_frames), (16, 0));
        // Made in code: no file name. (Loading and unloading a model file
        // is covered by the native tests, which have the asset folder.)
        let mut name = [1 as std::ffi::c_char; 8];
        let mut len = 9;
        assert_eq!(
            ponko_ctx_asset_owner(ctx, class, texture.id, name.as_mut_ptr(), name.len(), &mut len),
            PONKO_OK
        );
        assert_eq!((len, name[0]), (0, 0));
        let mut unloaded = true;
        assert_eq!(
            ponko_ctx_unload_mesh(ctx, c"models/none.glb".as_ptr(), &mut unloaded),
            PONKO_OK
        );
        assert!(!unloaded);
        // Errors.
        let mut c = PonkoAssetCount::default();
        errors.push(ponko_ctx_asset_count(ctx, 10, &mut c));
        errors.push(ponko_ctx_asset_handles(ctx, class, null_mut(), 1, &mut n));
        errors.push(ponko_ctx_asset_owner(ctx, class, 0, null_mut(), 1, &mut len));
        errors.push(ponko_ctx_unload_mesh(ctx, std::ptr::null(), &mut unloaded));
    }
}

#[test]
fn asset_counts_records_owners_and_unloading_work_from_c() {
    let mut errors: Vec<PonkoStatus> = Vec::new();
    unsafe {
        let g = ponko_game_new(c"accounting".as_ptr(), 64, 64);
        let mut world = null_mut();
        let user = (&raw mut errors).cast();
        assert_eq!(
            ponko_game_run_headless(g, 1, Some(accounting), user, &mut world),
            PONKO_OK
        );
        ponko_world_free(world);
    }
    assert_eq!(errors, vec![PONKO_INVALID_ARGUMENT; 4]);
}
