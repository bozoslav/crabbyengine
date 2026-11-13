//! Prefabs with children from C: a tree spawned through
//! `ponko_ctx_spawn_prefab`, its parts, a C component's entity field naming
//! the new copy, and despawning the tree.
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::null_mut;

use PonkoFieldType::*;
use PonkoStatus::*;

const FILE: &str = "test-ffi-prefab-tree.json";

/// A game component made in C: the entity it follows.
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Follow {
    target: u64,
}

#[derive(Default)]
struct Run {
    root: u64,
    parts: Vec<u64>,
    sized: usize,
    follow_target: u64,
    part_x: f32,
    despawned: usize,
    left: usize,
}

unsafe extern "C" fn setup(ctx: *mut PonkoCtx, user: *mut c_void) {
    unsafe {
        let run = &mut *user.cast::<Run>();
        let w = ponko_ctx_world(ctx);
        let mut follow = 0;
        assert_eq!(
            ponko_component_register(
                w,
                c"Follow".as_ptr(),
                size_of::<Follow>(),
                align_of::<Follow>(),
                &mut follow
            ),
            PONKO_OK
        );
        let fields = [PonkoField {
            name: c"target".as_ptr(),
            kind: PONKO_FIELD_ENTITY as u32,
            offset: 0,
        }];
        assert_eq!(ponko_component_reflect(w, follow, fields.as_ptr(), 1), PONKO_OK);
        let mut root = 0;
        assert_eq!(
            ponko_ctx_spawn_prefab(
                ctx,
                c"test-ffi-prefab-tree.json".as_ptr(),
                PonkoVec2 { x: 100.0, y: 50.0 },
                &mut root
            ),
            PONKO_OK
        );
        run.root = root;
        // Sized first, then read.
        assert_eq!(
            ponko_ctx_prefab_parts(ctx, root, null_mut(), 0, &mut run.sized),
            PONKO_OK
        );
        run.parts = vec![0; run.sized];
        let mut n = 0;
        assert_eq!(
            ponko_ctx_prefab_parts(ctx, root, run.parts.as_mut_ptr(), run.parts.len(), &mut n),
            PONKO_OK
        );
        assert_eq!(n, run.sized);
        let mut f = Follow::default();
        assert_eq!(
            ponko_get(w, root, follow, (&raw mut f).cast(), size_of::<Follow>()),
            PONKO_OK
        );
        run.follow_target = f.target;
        let mut transform = 0;
        assert_eq!(ponko_transform_component(w, &mut transform), PONKO_OK);
        let mut t = PonkoTransform2D::default();
        assert_eq!(
            ponko_get(w, run.parts[0], transform, (&raw mut t).cast(), size_of_val(&t)),
            PONKO_OK
        );
        run.part_x = t.position.x;
        assert_eq!(ponko_ctx_despawn_prefab(ctx, root, &mut run.despawned), PONKO_OK);
        assert_eq!(ponko_world_len(w, &mut run.left), PONKO_OK);
    }
}

#[test]
fn a_prefab_tree_is_spawned_listed_and_despawned_from_c() {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(
        assets.join(FILE),
        r#"{ "format": "ponko-prefab", "version": 2, "entities": [
             { "entity": 7, "components": { "Transform2D": {}, "Follow": { "target": 9 } } },
             { "entity": 9, "components": { "Transform2D": { "position": [5.0, 0.0] } } } ] }"#,
    )
    .unwrap();
    let mut run = Run::default();
    let status = unsafe {
        let mut world = null_mut();
        let game = ponko_game_new(c"prefabs".as_ptr(), 1, 1);
        let status = ponko_game_run_headless(game, 0, Some(setup), (&raw mut run).cast(), &mut world);
        ponko_world_free(world);
        status
    };
    let _ = std::fs::remove_file(assets.join(FILE));
    let _ = std::fs::remove_dir(&assets); // only if empty
    assert_eq!(status, PONKO_OK);
    assert_eq!(run.sized, 1);
    assert_eq!(run.follow_target, run.parts[0], "the C entity field names the new part");
    assert_ne!(run.parts[0], 9);
    assert_eq!(run.part_x, 105.0, "placed relative to the root");
    assert_eq!((run.despawned, run.left), (2, 0));
}
