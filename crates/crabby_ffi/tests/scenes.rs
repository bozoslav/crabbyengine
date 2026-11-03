//! Scenes and prefabs from C: worlds and entities saved under `assets/`.
use crabby_ffi::*;
use std::ffi::{CStr, c_void};
use std::ptr::null_mut;

use CrabbyStatus::*;

/// A file under this crate's `assets/` (created by the save), deleted when
/// the test ends, with the folder once it's empty.
struct TempAsset(&'static CStr);
impl Drop for TempAsset {
    fn drop(&mut self) {
        let name = self.0.to_str().unwrap();
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        let _ = std::fs::remove_file(assets.join(name));
        let _ = std::fs::remove_dir(assets); // only if empty
    }
}

#[derive(Default)]
struct Run {
    file: Option<&'static CStr>,
    transform: u32,
    spawned: Vec<u64>,
    errors: Vec<CrabbyStatus>,
}

fn at(x: f32, y: f32) -> CrabbyTransform2D {
    CrabbyTransform2D {
        position: CrabbyVec2 { x, y },
        rotation: 0.5,
        ..Default::default()
    }
}

unsafe extern "C" fn prefab_setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let run = &mut *user.cast::<Run>();
        let file = run.file.unwrap().as_ptr();
        let w = crabby_ctx_world(ctx);
        assert_eq!(crabby_transform_component(w, &mut run.transform), CRABBY_OK);
        let t = at(70.0, 80.0);
        let v = CrabbyValue {
            component: run.transform,
            data: (&raw const t).cast(),
            size: size_of_val(&t),
        };
        let mut model = 0;
        assert_eq!(crabby_spawn(w, &v, 1, &mut model), CRABBY_OK);
        assert_eq!(crabby_ctx_save_prefab(ctx, model, file), CRABBY_OK);
        assert_eq!(crabby_despawn(w, model), CRABBY_OK);
        for x in [0.0, 10.0] {
            let mut e = 0;
            assert_eq!(
                crabby_ctx_spawn_prefab(ctx, file, CrabbyVec2 { x, y: 100.0 }, &mut e),
                CRABBY_OK
            );
            run.spawned.push(e);
        }
        run.errors.push(crabby_ctx_spawn_prefab(
            ctx,
            c"test-ffi-missing.json".as_ptr(),
            CrabbyVec2::default(),
            &mut 0,
        ));
        run.errors
            .push(crabby_ctx_load_scene(ctx, c"test-ffi-missing.json".as_ptr()));
        run.errors.push(crabby_ctx_save_prefab(ctx, model, file));
    }
}

#[test]
fn prefabs_are_saved_and_spawned_from_c() {
    let file = TempAsset(c"test-ffi-prefab.json");
    let mut run = Run {
        file: Some(file.0),
        ..Default::default()
    };
    let mut world = null_mut();
    unsafe {
        let game = crabby_game_new(c"scenes".as_ptr(), 1, 1);
        let user = (&raw mut run).cast();
        assert_eq!(
            crabby_game_run_headless(game, 0, Some(prefab_setup), user, &mut world),
            CRABBY_OK
        );
        assert_eq!(run.spawned.len(), 2);
        for (e, x) in run.spawned.iter().zip([0.0, 10.0]) {
            let mut t = CrabbyTransform2D::default();
            assert_eq!(
                crabby_get(world, *e, run.transform, (&raw mut t).cast(), size_of_val(&t)),
                CRABBY_OK
            );
            assert_eq!((t.position.x, t.position.y, t.rotation), (x, 100.0, 0.5));
        }
        // Missing files, and saving a dead entity, are errors.
        assert!(
            run.errors.iter().all(|s| *s == CRABBY_INVALID_ARGUMENT),
            "{:?}",
            run.errors
        );
        crabby_world_free(world);
    }
}

unsafe extern "C" fn save_setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let run = &mut *user.cast::<Run>();
        let w = crabby_ctx_world(ctx);
        assert_eq!(crabby_transform_component(w, &mut run.transform), CRABBY_OK);
        let t = at(1.0, 2.0);
        let v = CrabbyValue {
            component: run.transform,
            data: (&raw const t).cast(),
            size: size_of_val(&t),
        };
        let mut e = 0;
        assert_eq!(crabby_spawn(w, &v, 1, &mut e), CRABBY_OK);
        run.spawned.push(e);
        assert_eq!(crabby_ctx_save_scene(ctx, run.file.unwrap().as_ptr()), CRABBY_OK);
    }
}
unsafe extern "C" fn load_setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let run = &mut *user.cast::<Run>();
        assert_eq!(crabby_ctx_load_scene(ctx, run.file.unwrap().as_ptr()), CRABBY_OK);
    }
}

#[test]
fn a_scene_saved_from_c_loads_back() {
    let file = TempAsset(c"test-ffi-scene.json");
    let mut run = Run {
        file: Some(file.0),
        ..Default::default()
    };
    unsafe {
        let mut world = null_mut();
        let user = (&raw mut run).cast();
        let game = crabby_game_new(c"scenes".as_ptr(), 1, 1);
        assert_eq!(
            crabby_game_run_headless(game, 0, Some(save_setup), user, &mut world),
            CRABBY_OK
        );
        crabby_world_free(world);
        let game = crabby_game_new(c"scenes".as_ptr(), 1, 1);
        assert_eq!(
            crabby_game_run_headless(game, 0, Some(load_setup), user, &mut world),
            CRABBY_OK
        );
        let mut n = 0;
        assert_eq!(crabby_world_len(world, &mut n), CRABBY_OK);
        assert_eq!(n, 1);
        let mut t = CrabbyTransform2D::default();
        assert_eq!(
            crabby_get(
                world,
                run.spawned[0],
                run.transform,
                (&raw mut t).cast(),
                size_of_val(&t)
            ),
            CRABBY_OK
        );
        assert_eq!((t.position.x, t.position.y), (1.0, 2.0));
        crabby_world_free(world);
    }
}
