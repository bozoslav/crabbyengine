//! Context calls that look window-bound but have a defined headless result:
//! idle input, the 2D and 3D camera maths, the spatial grid, textures and
//! texture arrays, 2D gravity. What needs a real window (the pixels drawn)
//! is on the allowlist of `scripts/check-ffi-coverage.py`.
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

use CrabbyStatus::*;

const BAD: CrabbyStatus = CRABBY_INVALID_ARGUMENT;

type Body = Box<dyn FnOnce(*mut CrabbyCtx)>;
struct Slot {
    body: Option<Body>,
    panic: Option<Box<dyn std::any::Any + Send>>,
}

/// Runs `body` once in the setup of a headless game with `configure` applied.
/// A failed assertion inside `body` fails the test (it must not unwind
/// through the C frames, so it is caught and rethrown after the run).
fn with_ctx(configure: impl FnOnce(*mut CrabbyGame), body: impl FnOnce(*mut CrabbyCtx)) {
    unsafe extern "C" fn trampoline(ctx: *mut CrabbyCtx, user: *mut c_void) {
        // SAFETY: `user` is the `Slot` below, alive for the run.
        let slot = unsafe { &mut *user.cast::<Slot>() };
        let body = slot.body.take().expect("setup runs once");
        if let Err(p) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| body(ctx))) {
            slot.panic = Some(p);
        }
    }
    let body: Box<dyn FnOnce(*mut CrabbyCtx) + '_> = Box::new(body);
    // SAFETY: the box is consumed inside the headless run, before this returns.
    let body: Body = unsafe { std::mem::transmute(body) };
    let mut slot = Slot {
        body: Some(body),
        panic: None,
    };
    unsafe {
        let game = crabby_game_new(c"headless-ctx".as_ptr(), 1, 1);
        configure(game);
        let mut world = null_mut();
        let status = crabby_game_run_headless(game, 1, Some(trampoline), (&raw mut slot).cast(), &mut world);
        if let Some(p) = slot.panic.take() {
            std::panic::resume_unwind(p);
        }
        assert_eq!(status, CRABBY_OK);
        assert!(slot.body.is_none(), "the body ran");
        crabby_world_free(world);
    }
}

fn v2(x: f32, y: f32) -> CrabbyVec2 {
    CrabbyVec2 { x, y }
}
fn v3(x: f32, y: f32, z: f32) -> CrabbyVec3 {
    CrabbyVec3 { x, y, z }
}
fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

#[test]
fn a_headless_run_has_no_input_and_rejects_unknown_keys_and_buttons() {
    with_ctx(
        |_| {},
        |ctx| unsafe {
            for key in [CrabbyKey::CRABBY_KEY_A as u32, CrabbyKey::CRABBY_KEY_A as u32 + 1] {
                for call in [crabby_ctx_key_pressed, crabby_ctx_key_released] {
                    let mut state = true;
                    assert_eq!(call(ctx, key, &mut state), CRABBY_OK);
                    assert!(!state, "no key is pressed or released without a window");
                }
            }
            for button in [
                CrabbyMouseButton::CRABBY_MOUSE_LEFT as u32,
                CrabbyMouseButton::CRABBY_MOUSE_FORWARD as u32,
            ] {
                for call in [
                    crabby_ctx_mouse_down,
                    crabby_ctx_mouse_pressed,
                    crabby_ctx_mouse_released,
                ] {
                    let mut state = true;
                    assert_eq!(call(ctx, button, &mut state), CRABBY_OK);
                    assert!(!state);
                }
            }
            let mut state = false;
            for call in [
                crabby_ctx_key_pressed,
                crabby_ctx_key_released,
                crabby_ctx_mouse_down,
                crabby_ctx_mouse_pressed,
                crabby_ctx_mouse_released,
            ] {
                assert_eq!(call(ctx, 100_000, &mut state), BAD, "unknown key or button");
                assert_eq!(call(ctx, 0, null_mut()), BAD, "output is null");
                assert_eq!(call(null_mut(), 0, &mut state), BAD, "ctx is null");
            }
            // Five buttons exist; a key number is not a button.
            assert_eq!(
                crabby_ctx_mouse_down(ctx, CrabbyMouseButton::CRABBY_MOUSE_FORWARD as u32 + 1, &mut state),
                BAD
            );
            let mut wheel = v2(9.0, 9.0);
            assert_eq!(crabby_ctx_mouse_wheel(ctx, &mut wheel), CRABBY_OK);
            assert_eq!((wheel.x, wheel.y), (0.0, 0.0));
            assert_eq!(crabby_ctx_mouse_wheel(ctx, null_mut()), BAD);
            let mut pos = v2(9.0, 9.0);
            assert_eq!(crabby_ctx_mouse_pos(ctx, &mut pos), CRABBY_OK);
            assert_eq!((pos.x, pos.y), (0.0, 0.0));
            assert_eq!(crabby_ctx_mouse_pos(null_mut(), &mut pos), BAD);
        },
    );
}

#[test]
fn the_2d_camera_maps_the_mouse_to_the_world() {
    with_ctx(
        |_| {},
        |ctx| unsafe {
            let mut camera = CrabbyCamera {
                position: v2(5.0, 6.0),
                zoom: 1.0,
            };
            assert_eq!(crabby_ctx_camera(ctx, &mut camera), CRABBY_OK);
            assert_eq!((camera.position.x, camera.position.y, camera.zoom), (0.0, 0.0, 1.0));

            // With the mouse at the window's corner, the world point under it
            // is the camera position.
            let moved = CrabbyCamera {
                position: v2(100.0, -40.0),
                zoom: 2.0,
            };
            assert_eq!(crabby_ctx_set_camera(ctx, &moved), CRABBY_OK);
            let mut back = CrabbyCamera {
                position: v2(0.0, 0.0),
                zoom: 0.0,
            };
            assert_eq!(crabby_ctx_camera(ctx, &mut back), CRABBY_OK);
            assert_eq!(back, moved);
            let mut world = v2(1.0, 1.0);
            assert_eq!(crabby_ctx_mouse_world(ctx, &mut world), CRABBY_OK);
            assert_eq!((world.x, world.y), (100.0, -40.0));

            for zoom in [0.0, -1.0, f32::NAN, f32::INFINITY] {
                let bad = CrabbyCamera { zoom, ..moved };
                assert_eq!(crabby_ctx_set_camera(ctx, &bad), BAD, "zoom {zoom}");
            }
            assert_eq!(crabby_ctx_set_camera(ctx, null()), BAD);
            assert_eq!(crabby_ctx_camera(ctx, &mut back), CRABBY_OK);
            assert_eq!(back, moved, "a refused camera changes nothing");
            assert_eq!(crabby_ctx_camera(ctx, null_mut()), BAD);
            assert_eq!(crabby_ctx_mouse_world(ctx, null_mut()), BAD);
        },
    );
}

#[test]
fn a_screen_ray_starts_at_the_3d_camera_and_the_middle_looks_straight_ahead() {
    with_ctx(
        |_| {},
        |ctx| unsafe {
            let mut camera = std::mem::zeroed::<CrabbyCamera3D>();
            assert_eq!(crabby_ctx_camera3d(ctx, &mut camera), CRABBY_OK);
            camera.position = v3(1.0, 2.0, 3.0);
            assert_eq!(crabby_ctx_set_camera3d(ctx, &camera), CRABBY_OK);
            assert_eq!(crabby_ctx_camera3d_look_at(ctx, v3(1.0, 2.0, -7.0)), CRABBY_OK);
            let size = crabby_ctx_window_size(ctx);
            let (mut origin, mut direction) = (v3(0.0, 0.0, 0.0), v3(0.0, 0.0, 0.0));
            let middle = v2(size.x / 2.0, size.y / 2.0);
            assert_eq!(
                crabby_ctx_screen_ray(ctx, middle, &mut origin, &mut direction),
                CRABBY_OK
            );
            assert_eq!((origin.x, origin.y, origin.z), (1.0, 2.0, 3.0));
            assert!(close(direction.z, -1.0), "straight along -Z: {direction:?}");
            assert!(close(direction.x, 0.0) && close(direction.y, 0.0), "{direction:?}");

            // The right edge looks to the right, the top edge up; unit length.
            assert_eq!(
                crabby_ctx_screen_ray(ctx, v2(size.x, size.y / 2.0), &mut origin, &mut direction),
                CRABBY_OK
            );
            assert!(direction.x > 0.1 && close(direction.y, 0.0), "{direction:?}");
            assert!(close(direction.x.hypot(direction.z), 1.0));
            assert_eq!(
                crabby_ctx_screen_ray(ctx, v2(size.x / 2.0, 0.0), &mut origin, &mut direction),
                CRABBY_OK
            );
            assert!(direction.y > 0.1 && close(direction.x, 0.0), "{direction:?}");

            assert_eq!(crabby_ctx_screen_ray(ctx, middle, null_mut(), &mut direction), BAD);
            assert_eq!(crabby_ctx_screen_ray(ctx, middle, &mut origin, null_mut()), BAD);
            assert_eq!(
                crabby_ctx_screen_ray(null_mut(), middle, &mut origin, &mut direction),
                BAD
            );
        },
    );
}

#[test]
fn headless_runs_draw_no_models_and_report_no_decal_support() {
    with_ctx(
        |_| {},
        |ctx| unsafe {
            let mut drawn = 99;
            assert_eq!(crabby_ctx_models_drawn(ctx, &mut drawn), CRABBY_OK);
            assert_eq!(drawn, 0);
            let mut decals = true;
            assert_eq!(crabby_ctx_supports_decals(ctx, &mut decals), CRABBY_OK);
            assert!(!decals, "no GPU in a headless run");
            assert_eq!(crabby_ctx_models_drawn(ctx, null_mut()), BAD);
            assert_eq!(crabby_ctx_supports_decals(ctx, null_mut()), BAD);
            assert_eq!(crabby_ctx_models_drawn(null_mut(), &mut drawn), BAD);
        },
    );
}

#[test]
fn the_built_in_spatial_grid_answers_rectangle_queries_and_exposes_its_view() {
    // Ten entities 10 apart on the x axis: 0, 10, ..., 90.
    unsafe extern "C" fn spawn_row(ctx: *mut CrabbyCtx, _: *mut c_void) {
        unsafe {
            let w = crabby_ctx_world(ctx);
            let mut t = 0;
            assert_eq!(crabby_transform_component(w, &mut t), CRABBY_OK);
            for x in 0..10 {
                let transform = CrabbyTransform2D {
                    position: v2(x as f32 * 10.0, 0.0),
                    ..Default::default()
                };
                let value = CrabbyValue {
                    component: t,
                    data: (&transform as *const CrabbyTransform2D).cast(),
                    size: size_of::<CrabbyTransform2D>(),
                };
                assert_eq!(crabby_spawn(w, &value, 1, &mut 0), CRABBY_OK);
            }
        }
    }
    #[derive(Default)]
    struct Seen {
        rect: Vec<f32>,
        total: usize,
        view_len: usize,
        entry_size: usize,
        starts_end: u32,
        buckets_power_of_two: bool,
        cell: f32,
        errors: Vec<CrabbyStatus>,
    }
    unsafe extern "C" fn ask_on_tick_1(ctx: *mut CrabbyCtx, user: *mut c_void) {
        unsafe {
            let seen = &mut *user.cast::<Seen>();
            if crabby_ctx_sim_tick(ctx) != 1 {
                return;
            }
            let mut hits = [CrabbyGridHit {
                entity: CrabbyEntity {
                    index: 0,
                    generation: 0,
                },
                position: v2(0.0, 0.0),
            }; 16];
            let mut n = 0;
            // x in [15, 45]: 20, 30, 40.
            assert_eq!(
                crabby_ctx_spatial_rect(ctx, v2(15.0, -1.0), v2(45.0, 1.0), hits.as_mut_ptr(), 16, &mut n),
                CRABBY_OK
            );
            seen.rect = hits[..n].iter().map(|h| h.position.x).collect();
            seen.rect.sort_by(f32::total_cmp);
            assert_eq!(
                crabby_ctx_spatial_rect(
                    ctx,
                    v2(-1000.0, -1000.0),
                    v2(1000.0, 1000.0),
                    null_mut(),
                    0,
                    &mut seen.total
                ),
                CRABBY_OK,
                "a null buffer of capacity 0 only counts"
            );
            assert_eq!(
                crabby_ctx_spatial_rect(ctx, v2(0.0, 0.0), v2(1.0, 1.0), hits.as_mut_ptr(), 16, null_mut()),
                BAD
            );
            let mut view = std::mem::zeroed::<CrabbyGridView>();
            assert_eq!(crabby_ctx_spatial_view(ctx, &mut view), CRABBY_OK);
            seen.view_len = view.len;
            seen.entry_size = view.entry_size;
            seen.starts_end = *view.starts.add(view.bucket_count);
            seen.buckets_power_of_two = view.bucket_count.is_power_of_two();
            seen.cell = 1.0 / view.inv_cell;
            assert_eq!(crabby_ctx_spatial_view(ctx, null_mut()), BAD);
            assert_eq!(crabby_ctx_spatial_view(null(), &mut view), BAD);
            seen.errors.push(crabby_ctx_spatial_rect(
                null(),
                v2(0.0, 0.0),
                v2(1.0, 1.0),
                hits.as_mut_ptr(),
                16,
                &mut n,
            ));
        }
    }
    let mut seen = Seen::default();
    unsafe {
        let game = crabby_game_new(c"grid".as_ptr(), 1, 1);
        assert_eq!(crabby_game_spatial_grid(game, 16.0), CRABBY_OK);
        assert_eq!(
            crabby_game_sim_system(game, null(), 1, Some(ask_on_tick_1), (&raw mut seen).cast()),
            CRABBY_OK
        );
        let mut world = null_mut();
        assert_eq!(
            crabby_game_run_headless(game, 2, Some(spawn_row), null_mut(), &mut world),
            CRABBY_OK
        );
        crabby_world_free(world);
    }
    assert_eq!(seen.rect, [20.0, 30.0, 40.0]);
    assert_eq!(seen.total, 10);
    assert_eq!(seen.view_len, 10);
    assert_eq!(seen.entry_size, size_of::<CrabbyGridHit>());
    assert_eq!(seen.starts_end, 10, "the last offset is the entry count");
    assert!(seen.buckets_power_of_two);
    assert!(close(seen.cell, 16.0));
    assert_eq!(seen.errors, [BAD]);
}

#[test]
fn spatial_queries_without_a_grid_find_nothing() {
    with_ctx(
        |_| {},
        |ctx| unsafe {
            let mut n = 99;
            let mut view = std::mem::zeroed::<CrabbyGridView>();
            assert_eq!(
                crabby_ctx_spatial_rect(ctx, v2(-1e6, -1e6), v2(1e6, 1e6), null_mut(), 0, &mut n),
                CRABBY_OK
            );
            assert_eq!(n, 0);
            assert_eq!(crabby_ctx_spatial_view(ctx, &mut view), CRABBY_OK);
            assert_eq!(view.len, 0);
        },
    );
}

#[test]
fn an_owned_grids_view_describes_its_buckets() {
    unsafe {
        let grid = crabby_grid_new(8.0);
        let items: Vec<_> = (0..20u32)
            .map(|i| CrabbyGridItem {
                entity: CrabbyEntity {
                    index: i,
                    generation: 0,
                },
                position: v2(i as f32 * 4.0, 0.0),
                data: i,
            })
            .collect();
        assert_eq!(crabby_grid_rebuild(grid, items.as_ptr(), items.len()), CRABBY_OK);
        let mut view = std::mem::zeroed::<CrabbyGridView>();
        assert_eq!(crabby_grid_view(grid, &mut view), CRABBY_OK);
        assert_eq!(view.len, 20);
        assert_eq!(view.entry_size, size_of::<CrabbyGridItem>());
        assert!(view.bucket_count.is_power_of_two());
        assert!(close(view.inv_cell, 1.0 / 8.0));
        assert_eq!(*view.starts, 0);
        assert_eq!(*view.starts.add(view.bucket_count), 20);
        // Every entry's data is still there, in some bucket order.
        let entries = std::slice::from_raw_parts(view.entries.cast::<CrabbyGridItem>(), view.len);
        let mut data: Vec<u32> = entries.iter().map(|i| i.data).collect();
        data.sort_unstable();
        assert_eq!(data, (0..20).collect::<Vec<_>>());

        assert_eq!(crabby_grid_view(grid, null_mut()), BAD);
        assert_eq!(crabby_grid_view(null(), &mut view), BAD);
        crabby_grid_free(grid);
    }
}

#[test]
fn texture_arrays_load_from_files_and_decals_need_the_data_kind() {
    let crab = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/crab.png\0");
    let crab = crab.as_ptr().cast();
    with_ctx(
        |_| {},
        |ctx| unsafe {
            let paths = [crab, crab, crab];
            let mut array = CrabbyTextureArray::default();
            assert_eq!(
                crabby_ctx_load_texture_array(ctx, paths.as_ptr(), 3, CRABBY_TEXTURE_COLOR, &mut array),
                CRABBY_OK
            );
            assert_eq!(array.layers, 3);
            assert!(array.width > 0 && array.height > 0);
            let mut single = CrabbyTexture {
                id: 0,
                width: 0,
                height: 0,
            };
            assert_eq!(crabby_ctx_load_texture(ctx, crab, &mut single), CRABBY_OK);
            assert_eq!((array.width, array.height), (single.width, single.height));

            // Decals decode colors themselves: only a Data array will do.
            assert_eq!(crabby_ctx_set_decal_textures(ctx, array), BAD);
            let mut data = CrabbyTextureArray::default();
            assert_eq!(
                crabby_ctx_load_texture_array(ctx, paths.as_ptr(), 2, CRABBY_TEXTURE_DATA, &mut data),
                CRABBY_OK
            );
            assert_eq!(data.layers, 2);
            assert_eq!(crabby_ctx_set_decal_textures(ctx, data), CRABBY_OK);
            // A forged handle was not made by this context.
            let forged = CrabbyTextureArray { id: 999, ..data };
            assert_eq!(crabby_ctx_set_decal_textures(ctx, forged), BAD);

            // Refusals leave the output alone and name the problem.
            let mut untouched = CrabbyTextureArray {
                id: 7,
                width: 7,
                height: 7,
                layers: 7,
            };
            assert_eq!(
                crabby_ctx_load_texture_array(ctx, paths.as_ptr(), 0, CRABBY_TEXTURE_COLOR, &mut untouched),
                BAD,
                "no files"
            );
            let missing = [c"no-such-layer.png".as_ptr()];
            assert_eq!(
                crabby_ctx_load_texture_array(ctx, missing.as_ptr(), 1, CRABBY_TEXTURE_COLOR, &mut untouched),
                BAD
            );
            assert_eq!(
                crabby_ctx_load_texture_array(ctx, paths.as_ptr(), 3, 9, &mut untouched),
                BAD,
                "unknown kind"
            );
            assert_eq!(
                crabby_ctx_load_texture_array(ctx, null(), 3, CRABBY_TEXTURE_COLOR, &mut untouched),
                BAD
            );
            assert_eq!(
                crabby_ctx_load_texture_array(ctx, paths.as_ptr(), 3, CRABBY_TEXTURE_COLOR, null_mut()),
                BAD
            );
            assert_eq!(untouched.id, 7);
        },
    );
}

#[test]
fn texture_pixels_are_checked_against_the_size_and_filters_accept_any_handle() {
    with_ctx(
        |_| {},
        |ctx| unsafe {
            let pixels = [200u8; 2 * 2 * 4];
            let mut texture = CrabbyTexture {
                id: 0,
                width: 0,
                height: 0,
            };
            assert_eq!(
                crabby_ctx_create_texture(ctx, 2, 2, pixels.as_ptr(), pixels.len(), &mut texture),
                CRABBY_OK
            );
            assert_eq!(
                crabby_ctx_update_texture(ctx, texture, pixels.as_ptr(), pixels.len()),
                CRABBY_OK
            );
            assert_eq!(
                crabby_ctx_update_texture(ctx, texture, pixels.as_ptr(), pixels.len() - 1),
                BAD,
                "one byte short"
            );
            assert_eq!(crabby_ctx_update_texture(ctx, texture, null(), pixels.len()), BAD);
            assert_eq!(
                crabby_ctx_update_texture(null_mut(), texture, pixels.as_ptr(), pixels.len()),
                BAD
            );
        },
    );
}

#[test]
fn a_shadow_mesh_goes_on_live_entities_only() {
    with_ctx(
        |_| {},
        |ctx| unsafe {
            let world = crabby_ctx_world(ctx);
            let mut transform = 0;
            assert_eq!(crabby_transform3d_component(world, &mut transform), CRABBY_OK);
            let at = crabby_transform3d_at(v3(0.0, 0.0, 0.0));
            let value = CrabbyValue {
                component: transform,
                data: (&at as *const CrabbyTransform3D).cast(),
                size: size_of::<CrabbyTransform3D>(),
            };
            let mut entity = 0;
            assert_eq!(crabby_spawn(world, &value, 1, &mut entity), CRABBY_OK);
            let mut material = CrabbyMaterial::default();
            let desc = crabby_material_desc(CrabbyColor {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            });
            assert_eq!(crabby_ctx_create_material(ctx, &desc, &mut material), CRABBY_OK);
            let mut mesh = CrabbyMesh::default();
            assert_eq!(
                crabby_ctx_create_mesh(ctx, crabby_mesh_data_sphere(1.0, 8), material, &mut mesh),
                CRABBY_OK
            );
            assert_eq!(crabby_ctx_set_shadow_mesh(ctx, entity, mesh, true), CRABBY_OK);
            assert_eq!(
                crabby_ctx_set_shadow_mesh(ctx, entity, mesh, false),
                CRABBY_OK,
                "switched off"
            );
            assert_eq!(crabby_despawn(world, entity), CRABBY_OK);
            assert_eq!(
                crabby_ctx_set_shadow_mesh(ctx, entity, mesh, true),
                BAD,
                "the entity is gone"
            );
            assert_eq!(crabby_ctx_set_shadow_mesh(null_mut(), entity, mesh, true), BAD);
        },
    );
}

#[cfg(feature = "physics")]
mod gravity {
    use super::*;

    #[derive(Default)]
    struct Ids {
        transform: u32,
        body: u32,
        collider: u32,
        velocity: u32,
        ball: u64,
        samples: Vec<f32>,
        in_circle: Vec<usize>,
        errors: Vec<CrabbyStatus>,
    }

    unsafe extern "C" fn setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
        unsafe {
            let ids = &mut *user.cast::<Ids>();
            let w = crabby_ctx_world(ctx);
            assert_eq!(crabby_transform_component(w, &mut ids.transform), CRABBY_OK);
            assert_eq!(crabby_rigid_body_component(w, &mut ids.body), CRABBY_OK);
            assert_eq!(crabby_collider_component(w, &mut ids.collider), CRABBY_OK);
            assert_eq!(crabby_velocity_component(w, &mut ids.velocity), CRABBY_OK);
            let at = |x: f32, y: f32| CrabbyTransform2D {
                position: v2(x, y),
                ..Default::default()
            };
            let value = |component, v: &dyn Fn() -> *const c_void, size| CrabbyValue {
                component,
                data: v(),
                size,
            };
            let (ball_at, body, shape, still) = (
                at(0.0, 0.0),
                crabby_rigid_body(CrabbyBodyKind::CRABBY_BODY_DYNAMIC as u32),
                crabby_collider_circle(10.0),
                CrabbyVelocity::default(),
            );
            let values = [
                value(
                    ids.transform,
                    &|| (&ball_at as *const CrabbyTransform2D).cast(),
                    size_of::<CrabbyTransform2D>(),
                ),
                value(
                    ids.body,
                    &|| (&body as *const CrabbyRigidBody).cast(),
                    size_of::<CrabbyRigidBody>(),
                ),
                value(
                    ids.collider,
                    &|| (&shape as *const CrabbyCollider).cast(),
                    size_of::<CrabbyCollider>(),
                ),
                value(
                    ids.velocity,
                    &|| (&still as *const CrabbyVelocity).cast(),
                    size_of::<CrabbyVelocity>(),
                ),
            ];
            assert_eq!(crabby_spawn(w, values.as_ptr(), 4, &mut ids.ball), CRABBY_OK);
        }
    }

    unsafe extern "C" fn watch(ctx: *mut CrabbyCtx, user: *mut c_void) {
        unsafe {
            let ids = &mut *user.cast::<Ids>();
            let tick = crabby_ctx_sim_tick(ctx);
            let w = crabby_ctx_world(ctx);
            let mut v = CrabbyVelocity::default();
            if tick > 0 {
                assert_eq!(
                    crabby_get(
                        w,
                        ids.ball,
                        ids.velocity,
                        (&raw mut v).cast(),
                        size_of::<CrabbyVelocity>()
                    ),
                    CRABBY_OK
                );
                ids.samples.push(v.linear.y);
            }
            if tick == 4 {
                // The game started with no gravity: the ball hasn't moved.
                assert_eq!(crabby_ctx_set_gravity(ctx, v2(0.0, 600.0)), CRABBY_OK);
                ids.errors.push(crabby_ctx_set_gravity(ctx, v2(f32::NAN, 0.0)));
                ids.errors.push(crabby_ctx_set_gravity(ctx, v2(0.0, f32::INFINITY)));
                ids.errors.push(crabby_ctx_set_gravity(null_mut(), v2(0.0, 1.0)));
            }
            if tick == 8 {
                // Circle queries: around the ball (it has fallen), far away, and a
                // zero radius at the ball's centre (a point inside it).
                let mut found = [CrabbyEntity::default(); 4];
                let mut n = 0;
                let mut t = CrabbyTransform2D::default();
                assert_eq!(
                    crabby_get(
                        w,
                        ids.ball,
                        ids.transform,
                        (&raw mut t).cast(),
                        size_of::<CrabbyTransform2D>()
                    ),
                    CRABBY_OK
                );
                for (center, radius) in [(t.position, 5.0), (v2(500.0, 500.0), 5.0), (t.position, 0.0)] {
                    assert_eq!(
                        crabby_ctx_physics_in_circle(ctx, center, radius, found.as_mut_ptr(), 4, &mut n),
                        CRABBY_OK
                    );
                    ids.in_circle.push(n);
                }
                // A null buffer only counts.
                assert_eq!(
                    crabby_ctx_physics_in_circle(ctx, t.position, 50.0, null_mut(), 0, &mut n),
                    CRABBY_OK
                );
                ids.in_circle.push(n);
                ids.errors.push(crabby_ctx_physics_in_circle(
                    ctx,
                    t.position,
                    5.0,
                    found.as_mut_ptr(),
                    4,
                    null_mut(),
                ));
            }
        }
    }

    #[test]
    fn changing_gravity_mid_run_pulls_bodies_and_circle_queries_find_them() {
        let mut ids = Ids::default();
        let user = (&raw mut ids).cast::<c_void>();
        unsafe {
            let g = crabby_game_new(c"gravity".as_ptr(), 1, 1);
            assert_eq!(crabby_game_physics(g, v2(0.0, 0.0)), CRABBY_OK);
            assert_eq!(
                crabby_game_sim_system(g, c"watch".as_ptr(), 1, Some(watch), user),
                CRABBY_OK
            );
            let mut world = null_mut();
            assert_eq!(
                crabby_game_run_headless(g, 12, Some(setup), user, &mut world),
                CRABBY_OK
            );
            crabby_world_free(world);
        }
        // samples[i] is the velocity read at the start of tick i + 1.
        assert!(
            ids.samples[..4].iter().all(|v| v.abs() < 1e-3),
            "weightless: {:?}",
            ids.samples
        );
        assert!(
            ids.samples.last().unwrap() > &10.0,
            "falling after gravity was switched on: {:?}",
            ids.samples
        );
        assert!(
            ids.samples.windows(2).skip(4).all(|w| w[1] >= w[0]),
            "speeds up: {:?}",
            ids.samples
        );
        assert_eq!(ids.errors, [BAD, BAD, BAD, BAD]);
        assert_eq!(ids.in_circle, [1, 0, 1, 1], "{:?}", ids.in_circle);
    }
}
