//! Scenes and prefabs from C: worlds and entities saved under `assets/`.
use ponko_ffi::*;
use std::ffi::{CStr, c_void};
use std::ptr::null_mut;
use std::sync::{Mutex, MutexGuard, PoisonError};

use CrabbyStatus::*;

/// A file under this crate's `assets/` (created by the save), deleted when
/// the test ends, with the folder once it's empty. It holds [`ASSETS`] for
/// the whole test: the tests run in parallel and share the folder, so one
/// test's cleanup could otherwise remove the folder between another test's
/// `create_dir_all` and its write.
struct TempAsset(&'static CStr, #[allow(dead_code)] MutexGuard<'static, ()>);
static ASSETS: Mutex<()> = Mutex::new(());
impl TempAsset {
    fn new(name: &'static CStr) -> Self {
        // A failed test poisons the lock; the folder is still usable.
        Self(name, ASSETS.lock().unwrap_or_else(PoisonError::into_inner))
    }
}
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
    let file = TempAsset::new(c"test-ffi-prefab.json");
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
    let file = TempAsset::new(c"test-ffi-scene.json");
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

unsafe extern "C" fn huge_id_setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let run = &mut *user.cast::<Run>();
        run.errors.push(crabby_ctx_load_scene(ctx, run.file.unwrap().as_ptr()));
    }
}

#[test]
fn scenes_with_huge_entity_ids_are_refused_from_c() {
    let file = TempAsset::new(c"test-ffi-huge-id.json");
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(
        assets.join(file.0.to_str().unwrap()),
        r#"{ "format": "crabby-world", "version": 1, "entities": [ { "entity": 2147483648,
  "components": { "Transform2D": { "position": [0.0, 0.0], "rotation": 0.0, "scale": [1.0, 1.0] } } } ] }"#,
    )
    .unwrap();
    let mut run = Run {
        file: Some(file.0),
        ..Default::default()
    };
    unsafe {
        let mut world = null_mut();
        let game = crabby_game_new(c"scenes".as_ptr(), 1, 1);
        assert_eq!(
            crabby_game_run_headless(game, 0, Some(huge_id_setup), (&raw mut run).cast(), &mut world),
            CRABBY_OK
        );
        assert_eq!(run.errors, [CRABBY_INVALID_ARGUMENT]);
        let message = CStr::from_ptr(crabby_last_error()).to_string_lossy().into_owned();
        assert!(message.contains("may only use indices below"), "{message}");
        let mut n = 1;
        assert_eq!(crabby_world_len(world, &mut n), CRABBY_OK);
        assert_eq!(n, 0);
        crabby_world_free(world);
    }
}

#[derive(Default)]
struct Lockstep {
    file: Option<&'static CStr>,
    /// What the load in setup, then the plain and the lockstep load at
    /// tick 3, returned.
    statuses: Vec<CrabbyStatus>,
}

unsafe extern "C" fn load_in_setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let run = &mut *user.cast::<Lockstep>();
        run.statuses
            .push(crabby_ctx_load_scene(ctx, run.file.unwrap().as_ptr()));
    }
}

unsafe extern "C" fn load_in_session(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let run = &mut *user.cast::<Lockstep>();
        if crabby_ctx_sim_tick(ctx) == 3 {
            let file = run.file.unwrap().as_ptr();
            run.statuses.push(crabby_ctx_load_scene(ctx, file));
            run.statuses.push(crabby_ctx_load_scene_lockstep(ctx, file));
        }
    }
}

fn play_lockstep(game: *mut CrabbyGame, file: &'static CStr) -> Vec<CrabbyStatus> {
    let mut run = Lockstep {
        file: Some(file),
        ..Default::default()
    };
    unsafe {
        let user = (&raw mut run).cast::<c_void>();
        assert_eq!(
            crabby_game_sim_system(game, c"load".as_ptr(), 1, Some(load_in_session), user),
            CRABBY_OK
        );
        let mut world = null_mut();
        assert_eq!(
            crabby_game_run_headless(game, 10, Some(load_in_setup), user, &mut world),
            CRABBY_OK
        );
        crabby_world_free(world);
    }
    run.statuses
}

#[test]
fn loads_in_setup_and_sim_systems_work_in_a_lockstep_session_from_c() {
    let file = TempAsset::new(c"test-ffi-lockstep.json");
    let mut run = Run {
        file: Some(file.0),
        ..Default::default()
    };
    unsafe {
        let game = crabby_game_new(c"scenes".as_ptr(), 1, 1);
        let mut world = null_mut();
        let user = (&raw mut run).cast();
        assert_eq!(
            crabby_game_run_headless(game, 0, Some(save_setup), user, &mut world),
            CRABBY_OK
        );
        crabby_world_free(world);
    }
    let mut port = 0;
    let host = unsafe {
        let g = crabby_game_new(c"host".as_ptr(), 1, 1);
        assert_eq!(
            crabby_game_lockstep_host(g, c"127.0.0.1:0".as_ptr(), 2, 2, 30, &mut port),
            CRABBY_OK
        );
        g as usize
    };
    let name = file.0;
    let host = std::thread::spawn(move || play_lockstep(host as *mut CrabbyGame, name));
    let addr = std::ffi::CString::new(format!("127.0.0.1:{port}")).unwrap();
    let guest = unsafe {
        let g = crabby_game_new(c"guest".as_ptr(), 1, 1);
        assert_eq!(crabby_game_lockstep_join(g, addr.as_ptr()), CRABBY_OK);
        play_lockstep(g, name)
    };
    for statuses in [host.join().unwrap(), guest] {
        assert_eq!(
            statuses,
            [CRABBY_OK, CRABBY_OK, CRABBY_OK],
            "every peer runs setup and sim systems: both loads are taken"
        );
    }
}
