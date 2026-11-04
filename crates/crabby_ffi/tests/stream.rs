//! Streaming through the C API: the cell streamer, tile data built on
//! another thread, and freeing meshes and terrain tiles.
use crabby_ffi::*;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

use CrabbyStatus::*;

fn at(x: f32, z: f32) -> CrabbyVec3 {
    CrabbyVec3 { x, y: 0.0, z }
}

fn list(
    s: *mut CrabbyCellStreamer,
    f: unsafe extern "C" fn(*mut CrabbyCellStreamer, *mut *const CrabbyCell, *mut usize) -> CrabbyStatus,
) -> Vec<CrabbyCell> {
    let (mut p, mut n) = (null(), 0);
    unsafe {
        assert_eq!(f(s, &mut p, &mut n), CRABBY_OK);
        std::slice::from_raw_parts(p, n).to_vec()
    }
}

#[test]
fn cells_stream_in_nearest_first_and_out_past_the_keep_radius() {
    let s = crabby_cells_new(10.0, 25.0, 40.0);
    unsafe {
        assert_eq!(crabby_cells_update(s, at(5.0, 5.0), 3), CRABBY_OK);
        let first = list(s, crabby_cells_to_load);
        assert_eq!(first.len(), 3);
        assert_eq!(first[0], CrabbyCell { x: 0, z: 0 });
        assert_eq!(crabby_cells_update(s, at(5.0, 5.0), usize::MAX), CRABBY_OK);
        let (mut loaded, mut count) = (false, 0);
        assert_eq!(
            crabby_cells_is_loaded(s, CrabbyCell { x: 2, z: 0 }, &mut loaded, &mut count),
            CRABBY_OK
        );
        assert!(loaded);
        let all = count;
        assert_eq!(all, first.len() + list(s, crabby_cells_to_load).len());
        // Far off: everything drops.
        assert_eq!(crabby_cells_update(s, at(9000.0, 5.0), 0), CRABBY_OK);
        assert_eq!(list(s, crabby_cells_to_drop).len(), all);
        assert_eq!(crabby_cells_clear(s), CRABBY_OK);
        assert_eq!(
            crabby_cells_is_loaded(s, CrabbyCell::default(), &mut loaded, null_mut()),
            CRABBY_OK
        );
        assert!(!loaded);
        assert_eq!(
            crabby_cells_is_loaded(s, CrabbyCell::default(), null_mut(), null_mut()),
            CRABBY_INVALID_ARGUMENT
        );
        crabby_cells_free(s);
    }
    assert!(crabby_cells_new(f32::NAN, 1.0, 1.0).is_null());
    assert!(crabby_cells_new(1.0, -1.0, 1.0).is_null());
}

unsafe extern "C" fn stream(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let errors = &mut *user.cast::<Vec<CrabbyStatus>>();
        let mut ground = CrabbyMaterial::default();
        let white = CrabbyColor {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        };
        assert_eq!(
            crabby_ctx_create_material(ctx, &crabby_material_desc(white), &mut ground),
            CRABBY_OK
        );
        // Built on another thread, uploaded on the game's.
        let data = std::thread::spawn(|| {
            let heights = vec![2.0f32; 17 * 17];
            crabby_terrain_tile_data_new(heights.as_ptr(), heights.len(), 32.0) as usize
        })
        .join()
        .unwrap() as *mut CrabbyTerrainTileData;
        assert!(!data.is_null());
        let mut tile = std::mem::zeroed::<CrabbyTerrainTile>();
        assert_eq!(crabby_ctx_upload_terrain_tile(ctx, data, ground, &mut tile), CRABBY_OK);
        crabby_terrain_tile_data_free(data);
        assert_eq!(tile.lod_distance, 32.0);
        let mut live = false;
        assert_eq!(crabby_ctx_is_mesh_live(ctx, tile.lods[2], &mut live), CRABBY_OK);
        assert!(live);
        assert_eq!(crabby_ctx_free_terrain_tile(ctx, tile), CRABBY_OK);
        for mesh in tile.lods {
            assert_eq!(crabby_ctx_is_mesh_live(ctx, mesh, &mut live), CRABBY_OK);
            assert!(!live);
        }
        assert_eq!(
            crabby_ctx_free_mesh(ctx, tile.lods[0]),
            CRABBY_OK,
            "freeing twice is fine"
        );
        // Errors.
        errors.push(crabby_ctx_upload_terrain_tile(ctx, null(), ground, &mut tile));
        let data = crabby_terrain_tile_data_new([0.0f32; 4].as_ptr(), 4, 1.0);
        errors.push(crabby_ctx_upload_terrain_tile(
            ctx,
            data,
            CrabbyMaterial { id: 999 },
            &mut tile,
        ));
        crabby_terrain_tile_data_free(data);
        errors.push(crabby_ctx_is_mesh_live(ctx, tile.lods[0], null_mut()));
    }
}

#[test]
fn tiles_upload_and_free_from_c() {
    let mut errors: Vec<CrabbyStatus> = Vec::new();
    unsafe {
        let g = crabby_game_new(c"stream".as_ptr(), 64, 64);
        let mut world = null_mut();
        let user = (&raw mut errors).cast();
        assert_eq!(
            crabby_game_run_headless(g, 1, Some(stream), user, &mut world),
            CRABBY_OK
        );
        crabby_world_free(world);
    }
    assert_eq!(errors, vec![CRABBY_INVALID_ARGUMENT; 3]);
    assert!(unsafe { crabby_terrain_tile_data_new([0.0f32; 3].as_ptr(), 3, 1.0) }.is_null());
}
