//! Morph targets from C: shapes named on mesh data, weights on entities,
//! finding a target by name.
use crabby_ffi::*;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

use CrabbyStatus::*;

struct Ids {
    mesh: CrabbyMesh,
    entity: u64,
}

fn value<T>(component: u32, v: &T) -> CrabbyValue {
    CrabbyValue {
        component,
        data: (v as *const T).cast(),
        size: size_of::<T>(),
    }
}

fn v3(x: f32, y: f32, z: f32) -> CrabbyVec3 {
    CrabbyVec3 { x, y, z }
}

unsafe extern "C" fn setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let ids = &mut *user.cast::<Ids>();
        let w = crabby_ctx_world(ctx);
        let (mut transform, mut model) = (0u32, 0u32);
        assert_eq!(crabby_transform3d_component(w, &mut transform), CRABBY_OK);
        assert_eq!(crabby_model_component(w, &mut model), CRABBY_OK);

        // Shapes on mesh data: one offset per corner, normals optional.
        let d = crabby_mesh_data_plane(1.0);
        let lift = v3(0.0, 0.5, 0.0);
        let corners = [lift; 4];
        assert_eq!(
            crabby_mesh_data_add_morph_target(d, c"lift".as_ptr(), corners.as_ptr(), corners.as_ptr(), 4),
            CRABBY_OK
        );
        assert_eq!(crabby_mesh_data_morph_target_count(d), 1);
        // Three offsets for four corners; a null name; a null positions.
        assert_eq!(
            crabby_mesh_data_add_morph_target(d, c"short".as_ptr(), corners.as_ptr(), null(), 3),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_mesh_data_add_morph_target(d, null(), corners.as_ptr(), null(), 4),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_mesh_data_add_morph_target(d, c"nowhere".as_ptr(), null(), null(), 4),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(crabby_mesh_data_morph_target_count(d), 1);
        crabby_mesh_data_clear_morph_targets(d);
        assert_eq!(crabby_mesh_data_morph_target_count(d), 0);
        // Re-add for the mesh, this time with no normal offsets.
        assert_eq!(
            crabby_mesh_data_add_morph_target(d, c"lift".as_ptr(), corners.as_ptr(), null(), 4),
            CRABBY_OK
        );

        let mut material = CrabbyMaterial::default();
        let desc = crabby_material_desc(CrabbyColor {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        });
        assert_eq!(crabby_ctx_create_material(ctx, &desc, &mut material), CRABBY_OK);
        assert_eq!(crabby_ctx_create_mesh(ctx, d, material, &mut ids.mesh), CRABBY_OK);
        crabby_mesh_data_free(d);

        // Finding a target by name.
        let mut target = 99usize;
        assert_eq!(
            crabby_ctx_morph_target(ctx, ids.mesh, c"lift".as_ptr(), &mut target),
            CRABBY_OK
        );
        assert_eq!(target, 0);
        assert_eq!(
            crabby_ctx_morph_target(ctx, ids.mesh, c"sag".as_ptr(), &mut target),
            CRABBY_NOT_FOUND
        );
        assert_eq!(
            crabby_ctx_morph_target(ctx, ids.mesh, null(), &mut target),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_ctx_morph_target(ctx, CrabbyMesh { id: 999 }, c"lift".as_ptr(), &mut target),
            CRABBY_NOT_FOUND,
            "a mesh without morph targets (or a freed one) has none"
        );

        // Weights on entities.
        let t = crabby_transform3d_at(v3(0.0, 0.0, 0.0));
        let m = crabby_model(ids.mesh);
        let parts = [value(transform, &t), value(model, &m)];
        assert_eq!(crabby_spawn(w, parts.as_ptr(), 2, &mut ids.entity), CRABBY_OK);

        let mut weight = f32::NAN;
        assert_eq!(
            crabby_ctx_morph_weight(ctx, ids.entity, 0, &mut weight),
            CRABBY_NOT_FOUND
        );
        assert_eq!(crabby_ctx_set_morph_weight(ctx, ids.entity, 0, 0.5), CRABBY_OK);
        assert_eq!(crabby_ctx_set_morph_weight(ctx, ids.entity, 2, 1.0), CRABBY_OK);
        assert_eq!(crabby_ctx_morph_weight(ctx, ids.entity, 0, &mut weight), CRABBY_OK);
        assert_eq!(weight, 0.5);
        assert_eq!(crabby_ctx_morph_weight(ctx, ids.entity, 1, &mut weight), CRABBY_OK);
        assert_eq!(weight, 0.0, "unset targets are 0");
        assert_eq!(crabby_ctx_morph_weight(ctx, ids.entity, 2, &mut weight), CRABBY_OK);
        assert_eq!(weight, 1.0);
        assert_eq!(
            crabby_ctx_morph_weight(ctx, ids.entity, 0, null_mut()),
            CRABBY_INVALID_ARGUMENT,
            "the output must be writable"
        );
        // A model spawned without weights has none: it shows the mesh's defaults.
        let mut plain = 0u64;
        assert_eq!(crabby_spawn(w, parts.as_ptr(), 2, &mut plain), CRABBY_OK);
        assert_eq!(crabby_ctx_morph_weight(ctx, plain, 0, &mut weight), CRABBY_NOT_FOUND);

        assert_eq!(crabby_ctx_clear_morph_weights(ctx, ids.entity), CRABBY_OK);
        assert_eq!(
            crabby_ctx_morph_weight(ctx, ids.entity, 0, &mut weight),
            CRABBY_NOT_FOUND
        );
        // Clearing again is fine.
        assert_eq!(crabby_ctx_clear_morph_weights(ctx, ids.entity), CRABBY_OK);

        // Bad entities and targets.
        assert_eq!(crabby_ctx_set_morph_weight(ctx, 12345, 0, 0.0), CRABBY_INVALID_ARGUMENT);
        assert_eq!(
            crabby_ctx_set_morph_weight(ctx, ids.entity, 1usize << 16, 0.5),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_ctx_morph_weight(ctx, 12345, 0, &mut weight),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(crabby_ctx_clear_morph_weights(ctx, 12345), CRABBY_INVALID_ARGUMENT);
    }
}

#[test]
fn morph_targets_and_weights_work_from_c() {
    let mut ids = Ids {
        mesh: CrabbyMesh::default(),
        entity: 0,
    };
    let user = (&raw mut ids).cast();
    unsafe {
        let g = crabby_game_new(c"morph".as_ptr(), 1, 1);
        let mut world = null_mut();
        assert_eq!(crabby_game_run_headless(g, 1, Some(setup), user, &mut world), CRABBY_OK);
        crabby_world_free(world);
    }
}
