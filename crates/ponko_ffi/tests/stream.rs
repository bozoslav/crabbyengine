//! Streaming through the C API: the cell streamer, tile data built on
//! another thread, and freeing meshes and terrain tiles.
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

use PonkoStatus::*;

fn at(x: f32, z: f32) -> PonkoVec3 {
    PonkoVec3 { x, y: 0.0, z }
}

fn list(
    s: *mut PonkoCellStreamer,
    f: unsafe extern "C" fn(*mut PonkoCellStreamer, *mut *const PonkoCell, *mut usize) -> PonkoStatus,
) -> Vec<PonkoCell> {
    let (mut p, mut n) = (null(), 0);
    unsafe {
        assert_eq!(f(s, &mut p, &mut n), PONKO_OK);
        std::slice::from_raw_parts(p, n).to_vec()
    }
}

#[test]
fn cells_stream_in_nearest_first_and_out_past_the_keep_radius() {
    let s = ponko_cells_new(10.0, 25.0, 40.0);
    unsafe {
        assert_eq!(ponko_cells_update(s, at(5.0, 5.0), 3), PONKO_OK);
        let first = list(s, ponko_cells_to_load);
        assert_eq!(first.len(), 3);
        assert_eq!(first[0], PonkoCell { x: 0, z: 0 });
        assert_eq!(ponko_cells_update(s, at(5.0, 5.0), usize::MAX), PONKO_OK);
        let (mut loaded, mut count) = (false, 0);
        assert_eq!(
            ponko_cells_is_loaded(s, PonkoCell { x: 2, z: 0 }, &mut loaded, &mut count),
            PONKO_OK
        );
        assert!(loaded);
        let all = count;
        assert_eq!(all, first.len() + list(s, ponko_cells_to_load).len());
        // Far off: everything drops.
        assert_eq!(ponko_cells_update(s, at(9000.0, 5.0), 0), PONKO_OK);
        assert_eq!(list(s, ponko_cells_to_drop).len(), all);
        assert_eq!(ponko_cells_clear(s), PONKO_OK);
        assert_eq!(
            ponko_cells_is_loaded(s, PonkoCell::default(), &mut loaded, null_mut()),
            PONKO_OK
        );
        assert!(!loaded);
        assert_eq!(
            ponko_cells_is_loaded(s, PonkoCell::default(), null_mut(), null_mut()),
            PONKO_INVALID_ARGUMENT
        );
        ponko_cells_free(s);
    }
    assert!(ponko_cells_new(f32::NAN, 1.0, 1.0).is_null());
    assert!(ponko_cells_new(1.0, -1.0, 1.0).is_null());
}

unsafe extern "C" fn stream(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let errors = &mut *user.cast::<Vec<PonkoStatus>>();
        let mut ground = PonkoMaterial::default();
        let white = PonkoColor {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        };
        assert_eq!(
            ponko_ctx_create_material(ctx, &ponko_material_desc(white), &mut ground),
            PONKO_OK
        );
        // Built on another thread, uploaded on the game's.
        let data = std::thread::spawn(|| {
            let heights = vec![2.0f32; 17 * 17];
            ponko_terrain_tile_data_new(heights.as_ptr(), heights.len(), 32.0) as usize
        })
        .join()
        .unwrap() as *mut PonkoTerrainTileData;
        assert!(!data.is_null());
        let mut tile = std::mem::zeroed::<PonkoTerrainTile>();
        assert_eq!(ponko_ctx_upload_terrain_tile(ctx, data, ground, &mut tile), PONKO_OK);
        ponko_terrain_tile_data_free(data);
        assert_eq!(tile.lod_distance, 32.0);
        let mut live = false;
        assert_eq!(ponko_ctx_is_mesh_live(ctx, tile.lods[2], &mut live), PONKO_OK);
        assert!(live);
        let mut casts = false;
        assert_eq!(ponko_ctx_mesh_shadows(ctx, tile.lods[2], &mut casts), PONKO_OK);
        assert!(casts, "meshes cast shadows at first");
        assert_eq!(ponko_ctx_set_mesh_shadows(ctx, tile.lods[2], false), PONKO_OK);
        assert_eq!(ponko_ctx_mesh_shadows(ctx, tile.lods[2], &mut casts), PONKO_OK);
        assert!(!casts);
        assert_eq!(ponko_ctx_free_terrain_tile(ctx, tile), PONKO_OK);
        for mesh in tile.lods {
            assert_eq!(ponko_ctx_is_mesh_live(ctx, mesh, &mut live), PONKO_OK);
            assert!(!live);
        }
        assert_eq!(
            ponko_ctx_free_mesh(ctx, tile.lods[0]),
            PONKO_OK,
            "freeing twice is fine"
        );
        // Errors.
        errors.push(ponko_ctx_upload_terrain_tile(ctx, null(), ground, &mut tile));
        let data = ponko_terrain_tile_data_new([0.0f32; 4].as_ptr(), 4, 1.0);
        errors.push(ponko_ctx_upload_terrain_tile(
            ctx,
            data,
            PonkoMaterial { id: 999 },
            &mut tile,
        ));
        ponko_terrain_tile_data_free(data);
        errors.push(ponko_ctx_is_mesh_live(ctx, tile.lods[0], null_mut()));
    }
}

#[test]
fn tiles_upload_and_free_from_c() {
    let mut errors: Vec<PonkoStatus> = Vec::new();
    unsafe {
        let g = ponko_game_new(c"stream".as_ptr(), 64, 64);
        let mut world = null_mut();
        let user = (&raw mut errors).cast();
        assert_eq!(ponko_game_run_headless(g, 1, Some(stream), user, &mut world), PONKO_OK);
        ponko_world_free(world);
    }
    assert_eq!(errors, vec![PONKO_INVALID_ARGUMENT; 3]);
    assert!(unsafe { ponko_terrain_tile_data_new([0.0f32; 3].as_ptr(), 3, 1.0) }.is_null());
}

unsafe extern "C" fn far_ground(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let errors = &mut *user.cast::<Vec<PonkoStatus>>();
        let mut ground = PonkoMaterial::default();
        let green = PonkoColor {
            r: 0.2,
            g: 0.6,
            b: 0.2,
            a: 1.0,
        };
        assert_eq!(
            ponko_ctx_create_material(ctx, &ponko_material_desc(green), &mut ground),
            PONKO_OK
        );
        // 2 km sampled every 16 m (128 squares, 2 x 2 meshes), cells of 128 m.
        let heights = vec![1.0f32; 129 * 129];
        let corner = PonkoVec2 { x: -1024.0, y: -1024.0 };
        let far = ponko_far_terrain_new(heights.as_ptr(), heights.len(), 129, corner, 16.0, 128.0, ground);
        assert!(!far.is_null());
        assert_eq!(ponko_far_terrain_update(far, ctx), PONKO_OK);
        let (mut cut, mut meshes) = (true, 0);
        let cell = PonkoCell { x: 0, z: 0 };
        assert_eq!(ponko_far_terrain_is_cut(far, cell, &mut cut, &mut meshes), PONKO_OK);
        assert_eq!((cut, meshes), (false, 4));
        assert_eq!(ponko_far_terrain_cut(far, cell, true), PONKO_OK);
        assert_eq!(ponko_far_terrain_update(far, ctx), PONKO_OK);
        assert_eq!(ponko_far_terrain_is_cut(far, cell, &mut cut, null_mut()), PONKO_OK);
        assert!(cut);
        assert_eq!(ponko_far_terrain_cut(far, cell, false), PONKO_OK);
        assert_eq!(ponko_far_terrain_is_cut(far, cell, &mut cut, null_mut()), PONKO_OK);
        assert!(!cut);
        assert_eq!(ponko_far_terrain_clear(far, ctx), PONKO_OK);
        assert_eq!(ponko_far_terrain_is_cut(far, cell, &mut cut, &mut meshes), PONKO_OK);
        assert_eq!(meshes, 0);
        ponko_far_terrain_free(far);
        // Errors: a material from elsewhere, no output, no far terrain.
        let wrong = ponko_far_terrain_new(
            heights.as_ptr(),
            heights.len(),
            129,
            corner,
            16.0,
            128.0,
            PonkoMaterial { id: 999 },
        );
        errors.push(ponko_far_terrain_update(wrong, ctx));
        errors.push(ponko_far_terrain_is_cut(wrong, cell, null_mut(), null_mut()));
        ponko_far_terrain_free(wrong);
        errors.push(ponko_far_terrain_update(null_mut(), ctx));
    }
}

#[test]
fn far_terrain_is_built_cut_and_cleared_from_c() {
    let mut errors: Vec<PonkoStatus> = Vec::new();
    unsafe {
        let g = ponko_game_new(c"far".as_ptr(), 64, 64);
        let mut world = null_mut();
        let user = (&raw mut errors).cast();
        assert_eq!(
            ponko_game_run_headless(g, 1, Some(far_ground), user, &mut world),
            PONKO_OK
        );
        ponko_world_free(world);
    }
    assert_eq!(errors, vec![PONKO_INVALID_ARGUMENT; 3]);
    let h = [0.0f32; 16];
    let at = PonkoVec2 { x: 5.0, y: 0.0 };
    let off_grid = unsafe { ponko_far_terrain_new(h.as_ptr(), 16, 4, at, 10.0, 10.0, PonkoMaterial::default()) };
    assert!(off_grid.is_null(), "the corner must be on the grid");
    assert!(
        unsafe { ponko_far_terrain_new(null(), 16, 4, PonkoVec2::default(), 1.0, 1.0, PonkoMaterial::default(),) }
            .is_null()
    );
    assert!(
        unsafe {
            ponko_far_terrain_new(
                h.as_ptr(),
                15,
                4,
                PonkoVec2::default(),
                1.0,
                1.0,
                PonkoMaterial::default(),
            )
        }
        .is_null()
    );
}
