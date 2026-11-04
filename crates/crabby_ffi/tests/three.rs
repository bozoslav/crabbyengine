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
        desc.blend = 2; // cut out
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

unsafe extern "C" fn terrain(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let errors = &mut *user.cast::<Vec<CrabbyStatus>>();
        let w = crabby_ctx_world(ctx);
        let (mut transform, mut tile_id) = (0, 0);
        assert_eq!(crabby_transform3d_component(w, &mut transform), CRABBY_OK);
        assert_eq!(crabby_terrain_tile_component(w, &mut tile_id), CRABBY_OK);
        let mut ground = CrabbyMaterial::default();
        assert_eq!(
            crabby_ctx_create_material(ctx, &crabby_material_desc(WHITE), &mut ground),
            CRABBY_OK
        );
        let heights: Vec<f32> = (0..81).map(|i| (i % 9) as f32 * 0.5).collect();
        let mut tile = std::mem::zeroed::<CrabbyTerrainTile>();
        assert_eq!(
            crabby_ctx_create_terrain_tile(ctx, heights.as_ptr(), heights.len(), 16.0, ground, &mut tile),
            CRABBY_OK
        );
        assert_eq!(tile.lod_distance, 16.0);
        let ids: Vec<u32> = tile.lods.iter().map(|m| m.id).collect();
        assert!(ids.windows(2).all(|p| p[0] != p[1]), "one mesh per level: {ids:?}");
        let t = crabby_transform3d_at(v3(0.0, 0.0, 0.0));
        let parts = [value(transform, &t), value(tile_id, &tile)];
        let mut e = 0;
        assert_eq!(crabby_spawn(w, parts.as_ptr(), 2, &mut e), CRABBY_OK);
        // A terrain material from two layers and a weight map.
        let pixels = [200u8; 2 * 2 * 4 * 2];
        let mut layers = CrabbyTextureArray::default();
        assert_eq!(
            crabby_ctx_create_texture_array(
                ctx,
                2,
                2,
                pixels.as_ptr(),
                pixels.len(),
                CRABBY_TEXTURE_COLOR,
                &mut layers
            ),
            CRABBY_OK
        );
        assert_eq!((layers.width, layers.layers), (2, 2));
        let mut weights = CrabbyTextureArray::default();
        assert_eq!(
            crabby_ctx_create_texture_array(ctx, 2, 2, pixels.as_ptr(), 16, CRABBY_TEXTURE_DATA, &mut weights),
            CRABBY_OK
        );
        let size = CrabbyVec2 { x: 16.0, y: 16.0 };
        let desc = crabby_terrain_material_desc(layers, weights, CrabbyVec2::default(), size);
        let mut terrain = CrabbyMaterial::default();
        assert_eq!(crabby_ctx_create_terrain_material(ctx, &desc, &mut terrain), CRABBY_OK);
        // A made-up layer count, half an image, a bad kind, no weight area.
        let forged = CrabbyTerrainMaterialDesc {
            layers: CrabbyTextureArray { layers: 3, ..layers },
            ..desc
        };
        errors.push(crabby_ctx_create_terrain_material(ctx, &forged, &mut terrain));
        errors.push(crabby_ctx_create_texture_array(
            ctx,
            2,
            2,
            pixels.as_ptr(),
            8,
            CRABBY_TEXTURE_COLOR,
            &mut layers,
        ));
        errors.push(crabby_ctx_create_texture_array(
            ctx,
            2,
            2,
            pixels.as_ptr(),
            16,
            7,
            &mut layers,
        ));
        let flat = CrabbyTerrainMaterialDesc {
            weights_size: CrabbyVec2::default(),
            ..desc
        };
        errors.push(crabby_ctx_create_terrain_material(ctx, &flat, &mut terrain));
        // Not a square grid, no size, and another game's material.
        errors.push(crabby_ctx_create_terrain_tile(
            ctx,
            heights.as_ptr(),
            80,
            16.0,
            ground,
            &mut tile,
        ));
        errors.push(crabby_ctx_create_terrain_tile(
            ctx,
            heights.as_ptr(),
            81,
            0.0,
            ground,
            &mut tile,
        ));
        errors.push(crabby_ctx_create_terrain_tile(
            ctx,
            heights.as_ptr(),
            81,
            16.0,
            CrabbyMaterial { id: 99 },
            &mut tile,
        ));
    }
}

#[test]
fn terrain_tiles_are_made_and_spawned_from_c() {
    let mut errors: Vec<CrabbyStatus> = Vec::new();
    unsafe {
        let g = crabby_game_new(c"terrain".as_ptr(), 64, 64);
        let mut world = null_mut();
        let user = (&raw mut errors).cast();
        assert_eq!(
            crabby_game_run_headless(g, 1, Some(terrain), user, &mut world),
            CRABBY_OK
        );
        crabby_world_free(world);
    }
    assert_eq!(errors, vec![CRABBY_INVALID_ARGUMENT; 7]);
}

unsafe extern "C" fn effects(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let errors = &mut *user.cast::<Vec<CrabbyStatus>>();
        let mut grey = CrabbyEffect::default();
        let source = c"fn effect(p: Pixel) -> vec3<f32> { return vec3<f32>(dot(p.color, vec3<f32>(0.3, 0.6, 0.1))); }";
        assert_eq!(
            crabby_ctx_create_effect(ctx, c"grey".as_ptr(), source.as_ptr(), &mut grey),
            CRABBY_OK
        );
        let mut values = CrabbyMaterial::default();
        assert_eq!(
            crabby_ctx_create_material(ctx, &crabby_material_desc(WHITE), &mut values),
            CRABBY_OK
        );
        let list = [
            CrabbyScreenEffect {
                effect: grey,
                has_material: false,
                material: CrabbyMaterial::default(),
            },
            CrabbyScreenEffect {
                effect: grey,
                has_material: true,
                material: values,
            },
        ];
        assert_eq!(crabby_ctx_set_effects(ctx, list.as_ptr(), 2), CRABBY_OK);
        let mut back = [CrabbyScreenEffect::default(); 4];
        let mut n = 0;
        assert_eq!(crabby_ctx_effects(ctx, back.as_mut_ptr(), 4, &mut n), CRABBY_OK);
        assert_eq!((n, back[1]), (2, list[1]));
        // A broken shader, an unknown effect, and another game's material.
        let mut other = CrabbyEffect::default();
        errors.push(crabby_ctx_create_effect(
            ctx,
            c"bad".as_ptr(),
            c"fn effect(p: Pixel) -> f32 { return 1.0; }".as_ptr(),
            &mut other,
        ));
        let unknown = [CrabbyScreenEffect {
            effect: CrabbyEffect { id: 9 },
            ..list[0]
        }];
        errors.push(crabby_ctx_set_effects(ctx, unknown.as_ptr(), 1));
        let foreign = [CrabbyScreenEffect {
            material: CrabbyMaterial { id: 99 },
            ..list[1]
        }];
        errors.push(crabby_ctx_set_effects(ctx, foreign.as_ptr(), 1));
        // The bad lists changed nothing.
        assert_eq!(crabby_ctx_effects(ctx, back.as_mut_ptr(), 4, &mut n), CRABBY_OK);
        assert_eq!(n, 2);
    }
}

#[test]
fn screen_effects_are_made_and_listed_from_c() {
    let mut errors: Vec<CrabbyStatus> = Vec::new();
    unsafe {
        let g = crabby_game_new(c"effects".as_ptr(), 64, 64);
        let mut world = null_mut();
        let user = (&raw mut errors).cast();
        assert_eq!(
            crabby_game_run_headless(g, 1, Some(effects), user, &mut world),
            CRABBY_OK
        );
        crabby_world_free(world);
    }
    assert_eq!(errors, vec![CRABBY_INVALID_ARGUMENT; 3]);
}
