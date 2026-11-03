//! 3D from C: transforms and models as plain components and query columns,
//! meshes and materials, the camera and the light.
use crabby_ffi::*;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

use CrabbyStatus::*;

#[derive(Default)]
struct Ids {
    transform: u32,
    model: u32,
    cubes: Vec<u64>,
    mesh: CrabbyMesh,
    errors: Vec<CrabbyStatus>,
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
const WHITE: CrabbyColor = CrabbyColor {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};

unsafe extern "C" fn setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let ids = &mut *user.cast::<Ids>();
        let w = crabby_ctx_world(ctx);
        assert_eq!(crabby_transform3d_component(w, &mut ids.transform), CRABBY_OK);
        assert_eq!(crabby_model_component(w, &mut ids.model), CRABBY_OK);

        let mut stone = CrabbyMaterial::default();
        let desc = crabby_material_desc(WHITE);
        assert_eq!(crabby_ctx_create_material(ctx, &desc, &mut stone), CRABBY_OK);
        let cube = crabby_mesh_data_cube(1.0);
        assert_eq!(crabby_ctx_create_mesh(ctx, cube, stone, &mut ids.mesh), CRABBY_OK);
        // A material this game didn't make.
        let mut other = CrabbyMesh::default();
        ids.errors
            .push(crabby_ctx_create_mesh(ctx, cube, CrabbyMaterial { id: 99 }, &mut other));
        crabby_mesh_data_free(cube);
        let shiny = CrabbyMaterialDesc { metallic: 2.0, ..desc };
        ids.errors.push(crabby_ctx_create_material(ctx, &shiny, &mut stone));

        for i in 0..3 {
            let t = crabby_transform3d_at(v3(i as f32 * 2.0, 0.0, 0.0));
            let m = crabby_model(ids.mesh);
            let parts = [value(ids.transform, &t), value(ids.model, &m)];
            let mut e = 0;
            assert_eq!(crabby_spawn(w, parts.as_ptr(), 2, &mut e), CRABBY_OK);
            ids.cubes.push(e);
        }

        let mut camera = std::mem::zeroed::<CrabbyCamera3D>();
        assert_eq!(crabby_ctx_camera3d(ctx, &mut camera), CRABBY_OK);
        camera.position = v3(0.0, 5.0, 10.0);
        assert_eq!(crabby_ctx_set_camera3d(ctx, &camera), CRABBY_OK);
        assert_eq!(crabby_ctx_camera3d_look_at(ctx, v3(0.0, 0.0, 0.0)), CRABBY_OK);
        let bad = CrabbyCamera3D { near: 0.0, ..camera };
        ids.errors.push(crabby_ctx_set_camera3d(ctx, &bad));

        let mut light = std::mem::zeroed::<CrabbyLighting>();
        assert_eq!(crabby_ctx_lighting(ctx, &mut light), CRABBY_OK);
        assert!(light.shadows);
        light.sun_intensity = 5.0;
        assert_eq!(crabby_ctx_set_lighting(ctx, &light), CRABBY_OK);
        let dark = CrabbyLighting {
            sun_intensity: -1.0,
            ..light
        };
        ids.errors.push(crabby_ctx_set_lighting(ctx, &dark));
        assert_eq!(light.shadow_cascades, 4);
        let many = CrabbyLighting {
            shadow_cascades: 5,
            ..light
        };
        ids.errors.push(crabby_ctx_set_lighting(ctx, &many));

        let mut sky = std::mem::zeroed::<CrabbySky>();
        assert_eq!(crabby_ctx_sky(ctx, &mut sky), CRABBY_OK);
        assert!(sky.atmosphere && sky.fog_density == 0.0);
        sky.fog_density = 0.01;
        assert_eq!(crabby_ctx_set_sky(ctx, &sky), CRABBY_OK);
        let flat = CrabbySky {
            fog_falloff: 0.0,
            ..sky
        };
        ids.errors.push(crabby_ctx_set_sky(ctx, &flat));
        assert!(crabby_sun_direction_at(12.0).y < -0.8);

        let mut post = std::mem::zeroed::<CrabbyPost>();
        assert_eq!(crabby_ctx_post(ctx, &mut post), CRABBY_OK);
        assert_eq!((post.tonemap, post.contrast, post.has_lut), (0, 1.0, false));
        post.tonemap = 1;
        post.bloom = 0.3;
        assert_eq!(crabby_ctx_set_post(ctx, &post), CRABBY_OK);
        let mut back = std::mem::zeroed::<CrabbyPost>();
        assert_eq!(crabby_ctx_post(ctx, &mut back), CRABBY_OK);
        assert_eq!(back, post);
        ids.errors
            .push(crabby_ctx_set_post(ctx, &CrabbyPost { tonemap: 7, ..post }));

        let mut wind = CrabbyShader::default();
        let source = c"fn vertex(v: Vertex) -> Vertex { var o = v; o.position.x += material.custom[0].x; return o; }";
        assert_eq!(
            crabby_ctx_create_shader(ctx, c"wind".as_ptr(), source.as_ptr(), &mut wind),
            CRABBY_OK
        );
        let mut leaves = CrabbyMaterial::default();
        let mut desc = crabby_material_desc(WHITE);
        desc.has_shader = true;
        desc.shader = wind;
        desc.double_sided = true;
        desc.custom[0] = 0.2;
        assert_eq!(crabby_ctx_create_material(ctx, &desc, &mut leaves), CRABBY_OK);
        desc.blend = 1;
        assert_eq!(crabby_ctx_update_material(ctx, leaves, &desc), CRABBY_OK);
        let broken = c"fn surface(s: Surface) -> Surface { return 1; }";
        ids.errors.push(crabby_ctx_create_shader(
            ctx,
            c"broken".as_ptr(),
            broken.as_ptr(),
            &mut wind,
        ));
        desc.shader = CrabbyShader { id: 42 };
        ids.errors.push(crabby_ctx_create_material(ctx, &desc, &mut leaves));
    }
}

/// Spins every model a little around Y, through a query column.
unsafe extern "C" fn spin(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe extern "C" fn each(chunk: *const CrabbyChunk, _: *mut c_void) {
        unsafe {
            let chunk = &*chunk;
            let transforms = std::slice::from_raw_parts_mut((*chunk.columns).cast::<CrabbyTransform3D>(), chunk.count);
            let turn = crabby_quat_from_axis_angle(v3(0.0, 1.0, 0.0), 0.1);
            for t in transforms {
                t.rotation = crabby_quat_mul(turn, t.rotation);
            }
        }
    }
    unsafe {
        let ids = &*user.cast::<Ids>();
        let columns = [ids.transform, ids.model];
        assert_eq!(
            crabby_query(crabby_ctx_world(ctx), columns.as_ptr(), 2, Some(each), null_mut()),
            CRABBY_OK
        );
    }
}

#[test]
fn games_build_and_move_3d_worlds_from_c() {
    let mut ids = Ids::default();
    let user = (&raw mut ids).cast();
    unsafe {
        let g = crabby_game_new(c"3d".as_ptr(), 640, 480);
        assert_eq!(
            crabby_game_sim_system(g, c"spin".as_ptr(), 1, Some(spin), user),
            CRABBY_OK
        );
        let mut world = null_mut();
        assert_eq!(crabby_game_run_headless(g, 5, Some(setup), user, &mut world), CRABBY_OK);
        assert_eq!(
            ids.errors, [CRABBY_INVALID_ARGUMENT; 9],
            "bad material, bad metallic, bad camera, negative light, bad tone map, broken shader, unknown shader"
        );

        // Five ticks of 0.1 radians, as Rust sees it through the same bytes.
        let mut t = std::mem::zeroed::<CrabbyTransform3D>();
        let size = size_of::<CrabbyTransform3D>();
        assert_eq!(
            crabby_get(world, ids.cubes[1], ids.transform, (&raw mut t).cast(), size),
            CRABBY_OK
        );
        assert_eq!(t.position.x, 2.0);
        let expected = crabby_quat_from_axis_angle(v3(0.0, 1.0, 0.0), 0.5);
        assert!((t.rotation.y - expected.y).abs() < 1e-5, "{:?}", t.rotation);
        let mut m = crabby_model(CrabbyMesh::default());
        let size = size_of::<CrabbyModel>();
        assert_eq!(
            crabby_get(world, ids.cubes[2], ids.model, (&raw mut m).cast(), size),
            CRABBY_OK
        );
        assert_eq!(m.mesh, ids.mesh);
        crabby_world_free(world);
    }
}

#[test]
fn mesh_data_round_trips_and_is_checked() {
    let positions = [v3(0.0, 0.0, 0.0), v3(1.0, 0.0, 0.0), v3(0.0, 1.0, 0.0)];
    unsafe {
        let d = crabby_mesh_data_new(positions.as_ptr(), null(), null(), 3, [0, 1, 2].as_ptr(), 3);
        assert!(!d.is_null());
        let mut view = std::mem::zeroed::<CrabbyMeshView>();
        assert_eq!(crabby_mesh_data_view(d, &mut view), CRABBY_OK);
        assert_eq!((view.vertex_count, view.index_count), (3, 3));
        assert!(view.normals.is_null() && view.uvs.is_null());
        assert_eq!(*view.positions.add(1), positions[1]);
        crabby_mesh_data_free(d);

        // Two indices: not a whole triangle.
        assert!(crabby_mesh_data_new(positions.as_ptr(), null(), null(), 3, [0, 1].as_ptr(), 2).is_null());
        let error = std::ffi::CStr::from_ptr(crabby_last_error()).to_str().unwrap();
        assert!(error.contains("3 per triangle"), "{error}");
    }
}
