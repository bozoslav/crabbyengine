//! Residency budgets and asset streaming through the C API: textures and
//! meshes streamed without waiting, their states and errors, budgets,
//! upload stats, and a LOD group whose level streams in.
use std::ffi::{CString, c_void};
use std::ptr::null_mut;

use ponko_ffi::*;

use CrabbyAssetState::*;
use CrabbyResidencyKind::*;
use CrabbyStatus::*;

const HOUSE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/models/house.glb");
const CRAB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/crab.png");

unsafe fn state(ctx: *mut CrabbyCtx, kind: CrabbyResidencyKind, name: &CString) -> u32 {
    let mut s = 99;
    assert_eq!(
        unsafe { crabby_ctx_asset_state(ctx, kind as u32, name.as_ptr(), &mut s) },
        CRABBY_OK
    );
    s
}

#[derive(Default)]
struct Seen {
    entity: u64,
    tick: u32,
    house_resident_at: Option<u32>,
}

unsafe extern "C" fn setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let seen = &mut *user.cast::<Seen>();
        let (house, crab) = (CString::new(HOUSE).unwrap(), CString::new(CRAB).unwrap());
        assert_eq!(crabby_ctx_set_deterministic_uploads(ctx, true), CRABBY_OK);
        assert_eq!(crabby_ctx_set_upload_budget(ctx, true, 1 << 20), CRABBY_OK);

        // A texture: not ready at once, then resident under one handle.
        let mut texture = std::mem::zeroed::<CrabbyTexture>();
        let mut ready = true;
        assert_eq!(
            crabby_ctx_stream_texture(ctx, crab.as_ptr(), &mut texture, &mut ready),
            CRABBY_OK
        );
        assert!(!ready);
        assert_eq!(state(ctx, CRABBY_RESIDENCY_TEXTURE, &crab), CRABBY_ASSET_LOADING as u32);
        assert_eq!(crabby_ctx_finish_uploads(ctx), CRABBY_OK);
        assert_eq!(
            crabby_ctx_stream_texture(ctx, crab.as_ptr(), &mut texture, &mut ready),
            CRABBY_OK
        );
        assert!(ready && texture.width == 32);
        assert_eq!(
            state(ctx, CRABBY_RESIDENCY_TEXTURE, &crab),
            CRABBY_ASSET_RESIDENT as u32
        );
        let mut again = std::mem::zeroed::<CrabbyTexture>();
        assert_eq!(
            crabby_ctx_resident_texture(ctx, crab.as_ptr(), true, &mut again),
            CRABBY_OK
        );
        assert_eq!(again.id, texture.id);
        let mut stats = CrabbyResidencyStats::default();
        assert_eq!(
            crabby_ctx_residency(ctx, CRABBY_RESIDENCY_TEXTURE as u32, &mut stats),
            CRABBY_OK
        );
        assert_eq!(
            (stats.resident, stats.pinned, stats.resident_bytes),
            (1, 1, 32 * 32 * 4)
        );
        assert_eq!(
            crabby_ctx_unpin_resident(ctx, CRABBY_RESIDENCY_TEXTURE as u32, crab.as_ptr()),
            CRABBY_OK
        );

        // Budgets.
        assert_eq!(
            crabby_ctx_set_residency_budget(ctx, CRABBY_RESIDENCY_MESH as u32, true, 1 << 30),
            CRABBY_OK
        );
        assert_eq!(
            crabby_ctx_residency(ctx, CRABBY_RESIDENCY_MESH as u32, &mut stats),
            CRABBY_OK
        );
        assert!(stats.has_budget && stats.budget == 1 << 30 && stats.resident == 0);
        assert_eq!(
            crabby_ctx_set_residency_budget(ctx, 9, true, 0),
            CRABBY_INVALID_ARGUMENT
        );

        // A missing file: failed, with its message.
        let nope = c"nope.png";
        assert_eq!(
            crabby_ctx_stream_texture(ctx, nope.as_ptr(), &mut texture, &mut ready),
            CRABBY_OK
        );
        assert_eq!(crabby_ctx_finish_uploads(ctx), CRABBY_OK);
        assert_eq!(
            state(ctx, CRABBY_RESIDENCY_TEXTURE, &CString::from(nope)),
            CRABBY_ASSET_FAILED as u32
        );
        let mut buffer = [0 as std::ffi::c_char; 8];
        let mut len = 0;
        assert_eq!(
            crabby_ctx_asset_error(
                ctx,
                CRABBY_RESIDENCY_TEXTURE as u32,
                nope.as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len(),
                &mut len
            ),
            CRABBY_OK
        );
        assert!(len > buffer.len(), "the full length, cut short in the buffer");
        assert_eq!(buffer[7], 0);
        let mut uploads = CrabbyUploadStats::default();
        assert_eq!(crabby_ctx_upload_stats(ctx, &mut uploads), CRABBY_OK);
        assert_eq!((uploads.failures, uploads.pending), (1, 0));
        assert!(uploads.has_budget && uploads.budget == 1 << 20);

        // A LOD group whose one level streams in from a file.
        let world = crabby_ctx_world(ctx);
        let mut transform = 0;
        assert_eq!(crabby_transform3d_component(world, &mut transform), CRABBY_OK);
        let at = crabby_transform3d_at(CrabbyVec3 { x: 0.0, y: 0.0, z: 0.0 });
        let value = CrabbyValue {
            component: transform,
            data: (&raw const at).cast(),
            size: size_of::<CrabbyTransform3D>(),
        };
        assert_eq!(crabby_spawn(world, &value, 1, &mut seen.entity), CRABBY_OK);
        let renderer = crabby_lod_renderer(CrabbyMesh { id: CRABBY_MESH_NONE });
        let level = CrabbyLodLevel {
            screen_height: 0.0,
            fade_width: 0.0,
            renderers: &renderer,
            renderer_count: 1,
            custom_shadow: false,
            shadow_casters: std::ptr::null(),
            shadow_caster_count: 0,
        };
        assert_eq!(
            crabby_ctx_set_lod_group(ctx, seen.entity, 4.0, CrabbyVec3 { x: 0.0, y: 0.0, z: 0.0 }, &level, 1),
            CRABBY_OK
        );
        let files = [house.as_ptr()];
        assert_eq!(
            crabby_ctx_set_streamed_lod(ctx, seen.entity, files.as_ptr(), 1),
            CRABBY_OK
        );
        assert_eq!(
            crabby_ctx_set_streamed_lod(ctx, seen.entity, std::ptr::null(), 1),
            CRABBY_INVALID_ARGUMENT
        );
    }
}

unsafe extern "C" fn watch(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let seen = &mut *user.cast::<Seen>();
        seen.tick += 1;
        let house = CString::new(HOUSE).unwrap();
        if seen.house_resident_at.is_none() && state(ctx, CRABBY_RESIDENCY_MESH, &house) == CRABBY_ASSET_RESIDENT as u32
        {
            seen.house_resident_at = Some(seen.tick);
            let mut mesh = CrabbyMesh::default();
            let mut ready = false;
            assert_eq!(
                crabby_ctx_stream_mesh(ctx, house.as_ptr(), &mut mesh, &mut ready),
                CRABBY_OK
            );
            assert!(ready);
            let mut resident = false;
            assert_eq!(
                crabby_ctx_is_resident(ctx, CRABBY_RESIDENCY_MESH as u32, house.as_ptr(), &mut resident),
                CRABBY_OK
            );
            assert!(resident);
        }
    }
}

#[test]
fn games_stream_assets_and_lod_levels_through_the_c_api() {
    let mut seen = Seen::default();
    let user = (&raw mut seen).cast();
    unsafe {
        let g = crabby_game_new(c"streaming".as_ptr(), 64, 64);
        assert_eq!(
            crabby_game_sim_system(g, c"watch".as_ptr(), 1, Some(watch), user),
            CRABBY_OK
        );
        let mut world = null_mut();
        assert_eq!(crabby_game_run_headless(g, 4, Some(setup), user, &mut world), CRABBY_OK);
        assert_eq!(crabby_world_free(world), CRABBY_OK);
    }
    // Asked for by the group in the first frame's streaming step and, with
    // deterministic uploads, uploaded in the same step.
    assert_eq!(seen.house_resident_at, Some(1));
}
