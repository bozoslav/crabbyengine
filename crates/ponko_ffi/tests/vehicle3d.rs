//! Raycast-wheel vehicles from C: the component, its wheels as a query
//! column, mass and center of mass on 3D bodies.
#![cfg(feature = "physics3d")]
use ponko_ffi::*;
use std::ffi::c_void;
use std::ptr::null_mut;

use CrabbyBodyKind::*;
use CrabbyStatus::*;

#[derive(Default)]
struct Ids {
    transform: u32,
    body: u32,
    collider: u32,
    vehicle: u32,
    car: u64,
    center: Option<CrabbyVec3>,
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

unsafe fn ids_of(w: *mut CrabbyWorld, ids: &mut Ids) {
    unsafe {
        assert_eq!(crabby_transform3d_component(w, &mut ids.transform), CRABBY_OK);
        assert_eq!(crabby_rigid_body3d_component(w, &mut ids.body), CRABBY_OK);
        assert_eq!(crabby_collider3d_component(w, &mut ids.collider), CRABBY_OK);
        assert_eq!(crabby_vehicle3d_component(w, &mut ids.vehicle), CRABBY_OK);
    }
}

fn car() -> CrabbyVehicle3D {
    let mut v = crabby_vehicle3d();
    for (x, z) in [(-0.8, -1.3), (0.8, -1.3), (-0.8, 1.3), (0.8, 1.3)] {
        assert_eq!(
            unsafe { crabby_vehicle3d_add_wheel(&mut v, crabby_wheel3d(v3(x, -0.3, z), 0.35)) },
            CRABBY_OK
        );
    }
    v
}

unsafe extern "C" fn setup(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let ids = &mut *user.cast::<Ids>();
        let w = crabby_ctx_world(ctx);
        ids_of(w, ids);
        let (floor_at, fixed, floor) = (
            crabby_transform3d_at(v3(0.0, -0.5, 0.0)),
            crabby_rigid_body3d(CRABBY_BODY_FIXED as u32),
            crabby_collider3d_box(v3(200.0, 1.0, 200.0)),
        );
        let ground = [
            value(ids.transform, &floor_at),
            value(ids.body, &fixed),
            value(ids.collider, &floor),
        ];
        let mut e = 0;
        assert_eq!(crabby_spawn(w, ground.as_ptr(), 3, &mut e), CRABBY_OK);
        let at = crabby_transform3d_at(v3(0.0, 1.0, 0.0));
        let mut chassis = crabby_rigid_body3d(CRABBY_BODY_DYNAMIC as u32);
        chassis.mass = 1200.0;
        chassis.has_center_of_mass = true;
        chassis.center_of_mass = v3(0.0, -0.3, 0.0);
        let shape = crabby_collider3d_box(v3(1.8, 0.6, 4.2));
        let vehicle = car();
        let values = [
            value(ids.transform, &at),
            value(ids.body, &chassis),
            value(ids.collider, &shape),
            value(ids.vehicle, &vehicle),
        ];
        assert_eq!(crabby_spawn(w, values.as_ptr(), 4, &mut ids.car), CRABBY_OK);
    }
}

unsafe extern "C" fn throttle(chunk: *const CrabbyChunk, _: *mut c_void) {
    unsafe {
        let chunk = &*chunk;
        let cars = (*chunk.columns).cast::<CrabbyVehicle3D>();
        for i in 0..chunk.count {
            let car = &mut *cars.add(i);
            car.wheels[2].motor_torque = 500.0;
            car.wheels[3].motor_torque = 500.0;
        }
    }
}

unsafe extern "C" fn drive(ctx: *mut CrabbyCtx, user: *mut c_void) {
    unsafe {
        let ids = &mut *user.cast::<Ids>();
        let tick = crabby_ctx_sim_tick(ctx);
        if tick == 99 {
            let mut c = v3(0.0, 0.0, 0.0);
            assert_eq!(crabby_ctx_body_center_of_mass3d(ctx, ids.car, &mut c), CRABBY_OK);
            ids.center = Some(c);
            assert_eq!(crabby_ctx_body_center_of_mass3d(ctx, 12345, &mut c), CRABBY_NOT_FOUND);
        }
        if tick >= 100 {
            let w = crabby_ctx_world(ctx);
            assert_eq!(crabby_query(w, &ids.vehicle, 1, Some(throttle), null_mut()), CRABBY_OK);
        }
    }
}

#[test]
fn a_c_car_settles_on_its_wheels_and_drives_off() {
    let mut ids = Ids::default();
    let user = (&raw mut ids).cast::<c_void>();
    let mut world = null_mut();
    unsafe {
        let g = crabby_game_new(c"vehicle".as_ptr(), 1, 1);
        assert_eq!(
            crabby_game_sim_system(g, c"drive".as_ptr(), 1, Some(drive), user),
            CRABBY_OK
        );
        assert_eq!(crabby_game_physics3d(g, v3(0.0, -9.81, 0.0)), CRABBY_OK);
        assert_eq!(
            crabby_game_run_headless(g, 200, Some(setup), user, &mut world),
            CRABBY_OK
        );

        let center = ids.center.expect("asked");
        assert!(
            (center.y - 0.5).abs() < 0.01,
            "rests 0.8 m up, center 0.3 m below: {}",
            center.y
        );
        let mut v = crabby_vehicle3d();
        let n = size_of::<CrabbyVehicle3D>();
        assert_eq!(
            crabby_get(world, ids.car, ids.vehicle, (&raw mut v).cast(), n),
            CRABBY_OK
        );
        let mut t = crabby_transform3d_at(v3(0.0, 0.0, 0.0));
        let n = size_of::<CrabbyTransform3D>();
        assert_eq!(
            crabby_get(world, ids.car, ids.transform, (&raw mut t).cast(), n),
            CRABBY_OK
        );
        assert_eq!(v.wheel_count, 4);
        assert!(v.wheels.iter().take(4).all(|w| w.grounded));
        assert!(v.wheels[3].angular_velocity > 0.0 && v.wheels[3].motor_torque == 500.0);
        assert!(t.position.z < -3.0, "drove forward (-Z): {}", t.position.z);
        let pose = crabby_wheel3d_pose(&v.wheels[0], t);
        assert!((pose.position.y - 0.35).abs() < 0.05, "{}", pose.position.y);
        crabby_world_free(world);
    }
}

#[test]
fn bodies_keep_their_mass_and_center_and_bad_ones_are_refused() {
    let mut ids = Ids::default();
    unsafe {
        let w = crabby_world_new();
        ids_of(w, &mut ids);
        let mut body = crabby_rigid_body3d(CRABBY_BODY_DYNAMIC as u32);
        body.mass = 900.0;
        body.has_center_of_mass = true;
        body.center_of_mass = v3(0.1, -0.4, 0.2);
        let vehicle = car();
        let values = [value(ids.body, &body), value(ids.vehicle, &vehicle)];
        let mut e = 0;
        assert_eq!(crabby_spawn(w, values.as_ptr(), 2, &mut e), CRABBY_OK);
        let mut back = crabby_rigid_body3d(0);
        let n = size_of::<CrabbyRigidBody3D>();
        assert_eq!(crabby_get(w, e, ids.body, (&raw mut back).cast(), n), CRABBY_OK);
        assert_eq!(back, body);
        let mut v = crabby_vehicle3d();
        let n = size_of::<CrabbyVehicle3D>();
        assert_eq!(crabby_get(w, e, ids.vehicle, (&raw mut v).cast(), n), CRABBY_OK);
        assert_eq!(v, vehicle);

        for bad in [
            CrabbyRigidBody3D { mass: -1.0, ..body },
            CrabbyRigidBody3D {
                center_of_mass: v3(f32::NAN, 0.0, 0.0),
                ..body
            },
        ] {
            let n = size_of::<CrabbyRigidBody3D>();
            assert_eq!(
                crabby_set(w, e, ids.body, (&raw const bad).cast(), n),
                CRABBY_INVALID_ARGUMENT
            );
        }
        // Without the flag the center is ignored, NaN or not.
        let ignored = CrabbyRigidBody3D {
            has_center_of_mass: false,
            center_of_mass: v3(f32::NAN, 0.0, 0.0),
            ..body
        };
        let n = size_of::<CrabbyRigidBody3D>();
        assert_eq!(crabby_set(w, e, ids.body, (&raw const ignored).cast(), n), CRABBY_OK);
        crabby_world_free(w);
    }
}

#[test]
fn friction_curves_and_wheels_match_rust() {
    let c = crabby_friction_curve_sideways();
    assert_eq!(crabby_friction_curve_evaluate(c, 0.2), 1.0);
    assert_eq!(crabby_friction_curve_evaluate(c, -9.0), 0.75);
    let rust = ponko::d3::Wheel::new(ponko::vec3(1.0, 2.0, 3.0), 0.4);
    let w = crabby_wheel3d(v3(1.0, 2.0, 3.0), 0.4);
    assert_eq!(
        (w.mass, w.suspension_distance, w.spring, w.damper, w.damping_rate),
        (
            rust.mass,
            rust.suspension_distance,
            rust.spring,
            rust.damper,
            rust.damping_rate
        )
    );
    assert_eq!(w.forward_friction.extremum_slip, rust.forward_friction.extremum_slip);
    assert_eq!(w.local_position.y, rust.local_position.y);
    let mut v = crabby_vehicle3d();
    assert_eq!(
        unsafe { crabby_vehicle3d_add_wheel(std::ptr::null_mut(), w) },
        CRABBY_INVALID_ARGUMENT
    );
    for _ in 0..8 {
        assert_eq!(unsafe { crabby_vehicle3d_add_wheel(&mut v, w) }, CRABBY_OK);
    }
    assert_eq!(
        unsafe { crabby_vehicle3d_add_wheel(&mut v, w) },
        CRABBY_INVALID_ARGUMENT
    );
}
