//! Raycast-wheel vehicles (feature `physics3d`): suspension, throttle,
//! brakes, steering and fast driving over terrain, all headless.
#![cfg(feature = "physics3d")]

use std::cell::RefCell;
use std::rc::Rc;

use ponko::*;

const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);
/// The chassis's origin rests this high over flat ground: mount 0.3 m
/// below it, half of 0.3 m of travel, a 0.35 m wheel.
const REST_HEIGHT: f32 = 0.3 + 0.15 + 0.35;

/// A 1200 kg box car facing -Z: front wheels first, then the rear.
fn car(at: Vec3) -> (Transform3D, RigidBody3D, Collider3D, Velocity3D, Vehicle3D) {
    let mut vehicle = Vehicle3D::new();
    for (x, z) in [(-0.8, -1.3), (0.8, -1.3), (-0.8, 1.3), (0.8, 1.3)] {
        vehicle = vehicle.with_wheel(Wheel3D::new(vec3(x, -0.3, z), 0.35));
    }
    (
        Transform3D::at(at),
        RigidBody3D::dynamic().mass(1200.0).center_of_mass(vec3(0.0, -0.3, 0.0)),
        Collider3D::cuboid(vec3(1.8, 0.6, 4.2)),
        Velocity3D::default(),
        vehicle,
    )
}

fn floor() -> (Transform3D, RigidBody3D, Collider3D) {
    (
        Transform3D::at(vec3(0.0, -0.5, 0.0)),
        RigidBody3D::fixed(),
        Collider3D::cuboid(vec3(400.0, 1.0, 400.0)),
    )
}

/// What a drive recorded, tick by tick.
#[derive(Clone, Copy, Debug)]
struct Sample {
    position: Vec3,
    forward: Vec3,
    velocity: Vec3,
    vehicle: Vehicle3D,
}

/// Spawns `world` and a car at `start`, drives it for `ticks` with
/// `drive(tick, wheels)` setting the inputs, and records every tick.
fn drive(
    ticks: u64,
    start: Vec3,
    velocity: Vec3,
    world: impl FnOnce(&mut Ctx) + 'static,
    mut drive: impl FnMut(u64, &mut [Wheel3D]) + 'static,
) -> Vec<Sample> {
    let samples = Rc::new(RefCell::new(Vec::new()));
    let out = samples.clone();
    let car_id = Rc::new(RefCell::new(None::<Entity>));
    let (set, get) = (car_id.clone(), car_id.clone());
    Game::new("t", 1, 1)
        .sim_system(move |ctx: &mut Ctx| {
            let e = get.borrow().expect("spawned");
            let tick = ctx.time.sim_tick();
            if let Some(v) = ctx.world.get_mut::<Vehicle3D>(e) {
                drive(tick, v.wheels_mut());
            }
        })
        .physics3d(GRAVITY)
        .sim_system(move |ctx: &mut Ctx| {
            let e = car_id.borrow().expect("spawned");
            let t = *ctx.world.get::<Transform3D>(e).unwrap();
            out.borrow_mut().push(Sample {
                position: t.position,
                forward: t.forward(),
                velocity: ctx.world.get::<Velocity3D>(e).unwrap().linear,
                vehicle: *ctx.world.get::<Vehicle3D>(e).unwrap(),
            });
        })
        .run_headless(ticks, move |ctx| {
            world(ctx);
            let mut c = car(start);
            c.3 = Velocity3D::linear(velocity);
            *set.borrow_mut() = Some(ctx.world.spawn(c));
        });
    samples.take()
}

fn on_floor(ctx: &mut Ctx) {
    ctx.world.spawn(floor());
}

#[test]
fn a_car_at_rest_settles_at_its_suspension_rest_height() {
    let run = drive(250, vec3(0.0, 1.5, 0.0), Vec3::ZERO, on_floor, |_, _| {});
    let last = run.last().unwrap();
    assert!(
        (last.position.y - REST_HEIGHT).abs() < 0.01,
        "rests at {} (wanted {REST_HEIGHT})",
        last.position.y
    );
    assert!(last.velocity.length() < 0.01, "{}", last.velocity);
    assert!(
        vec2(last.position.x, last.position.z).length() < 0.01,
        "didn't creep: {}",
        last.position
    );
    for w in last.vehicle.wheels() {
        assert!(w.grounded);
        assert!((w.compression - 0.15).abs() < 0.01, "{}", w.compression);
        assert!((w.sprung_mass - 300.0).abs() < 1.0, "{}", w.sprung_mass);
        assert!(
            (w.suspension_force - 300.0 * 9.81).abs() < 30.0,
            "{}",
            w.suspension_force
        );
        assert!(w.contact_point.y.abs() < 1e-3 && w.contact_normal.y > 0.99);
        // Drawn on the ground: the wheel's bottom touches it.
        let pose = w.pose(&Transform3D::at(last.position));
        assert!((pose.position.y - 0.35).abs() < 0.01, "{}", pose.position.y);
    }
    // While falling, the wheels hung fully extended.
    let first = run.first().unwrap().vehicle.wheels()[0];
    assert!(!first.grounded && first.compression == 0.0);
    assert!((first.local_position.y - -0.6).abs() < 1e-5, "{}", first.local_position);
}

#[test]
fn throttle_accelerates_and_spins_the_wheels() {
    let run = drive(
        200,
        vec3(0.0, REST_HEIGHT, 0.0),
        Vec3::ZERO,
        on_floor,
        |tick, wheels| {
            if tick > 20 {
                for w in &mut wheels[2..] {
                    w.motor_torque = 600.0;
                }
            }
        },
    );
    let last = run.last().unwrap();
    let speed = last.velocity.dot(Vec3::NEG_Z);
    assert!(speed > 6.0, "{speed} m/s after 3.6 s");
    assert!(last.velocity.x.abs() < 0.05, "straight: {}", last.velocity);
    assert!(last.position.z < -8.0, "{}", last.position);
    let rear = last.vehicle.wheels()[3];
    assert!(rear.angular_velocity > 0.0 && rear.rpm() > 100.0, "{}", rear.rpm());
    assert!(
        rear.forward_slip > 0.0 && rear.forward_slip < 0.4,
        "grips: {}",
        rear.forward_slip
    );
    // The free front wheels roll along at the road's speed.
    let front = last.vehicle.wheels()[0];
    assert!(
        (front.angular_velocity * 0.35 - speed).abs() < 0.2,
        "{} vs {speed}",
        front.angular_velocity * 0.35
    );
    // Accelerating squats the rear.
    let w = last.vehicle.wheels();
    assert!(w[2].compression > w[0].compression);
}

#[test]
fn brakes_stop_a_moving_car() {
    let run = drive(
        300,
        vec3(0.0, REST_HEIGHT, 0.0),
        vec3(0.0, 0.0, -20.0),
        on_floor,
        |tick, wheels| {
            for w in wheels.iter_mut() {
                // Roll along first, then brake hard.
                w.brake_torque = if tick > 25 { 3000.0 } else { 0.0 };
            }
        },
    );
    let braking = &run[30];
    assert!(braking.velocity.length() > 15.0, "{}", braking.velocity);
    assert!(
        braking.vehicle.wheels()[0].forward_slip < 0.0,
        "the tyres slip backwards"
    );
    // Weight moves forward under braking.
    let w = braking.vehicle.wheels();
    assert!(
        w[0].compression > w[2].compression,
        "{} {}",
        w[0].compression,
        w[2].compression
    );
    let last = run.last().unwrap();
    assert!(last.velocity.length() < 0.3, "stopped: {}", last.velocity);
    assert!(last.position.x.abs() < 0.5, "in a straight line: {}", last.position);
    let stopped_at = run.iter().position(|s| s.velocity.length() < 0.3).unwrap();
    assert!(stopped_at < 250, "within ~5 s: tick {stopped_at}");
}

#[test]
fn steering_turns_the_car() {
    for (steer, side) in [(0.3f32, 1.0f32), (-0.3, -1.0)] {
        let run = drive(
            250,
            vec3(0.0, REST_HEIGHT, 0.0),
            vec3(0.0, 0.0, -8.0),
            on_floor,
            move |_, wheels| {
                for (i, w) in wheels.iter_mut().enumerate() {
                    if i < 2 {
                        w.steer_angle = steer;
                    } else {
                        w.motor_torque = 150.0;
                    }
                }
            },
        );
        // Positive steers right: from -Z towards +X.
        let early = &run[50];
        assert!(
            early.forward.x * side > 0.3,
            "steer {steer}: faces {} after 1 s",
            early.forward
        );
        assert!(early.position.x * side > 0.5, "{}", early.position);
        // The front wheels are drawn turned: steering right swings the
        // axle's +X end back (+Z).
        let axle = early.vehicle.wheels()[0].local_rotation * Vec3::X;
        assert!((axle.z - steer.sin()).abs() < 1e-4 && axle.y.abs() < 1e-4, "{axle}");
        // It goes round in a circle (of about wheelbase / tan(steer) =
        // 8.4 m radius), on its wheels.
        let farthest = run.iter().map(|s| s.position.length()).fold(0.0, f32::max);
        assert!(farthest > 10.0 && farthest < 30.0, "{farthest}");
        for s in &run {
            assert!(s.position.y > 0.6 && s.position.y < 1.0, "{}", s.position);
        }
    }
}

fn hills(x: f32, z: f32) -> f32 {
    (x * 0.05).sin() * 2.0 + (z * 0.04).cos() * 3.0 + (x * 0.011 + z * 0.017).sin() * 4.0
}

const N: usize = 129;
const SIZE: f32 = 512.0;

fn terrain(ctx: &mut Ctx) {
    let corner = vec3(-SIZE / 2.0, 0.0, -SIZE / 2.0);
    let step = SIZE / (N - 1) as f32;
    let heights: Vec<f32> = (0..N * N)
        .map(|i| hills(corner.x + (i % N) as f32 * step, corner.z + (i / N) as f32 * step))
        .collect();
    let ground = ctx.physics3d.heightfield(&heights, SIZE);
    ctx.world
        .spawn((Transform3D::at(corner), RigidBody3D::fixed(), Collider3D::mesh(ground)));
}

/// The terrain's height under (x, z), as its triangles have it.
fn ground_under(x: f32, z: f32) -> f32 {
    // Between grid points the surface is flat triangles; the corners'
    // lowest height is a safe lower bound.
    let step = SIZE / (N - 1) as f32;
    let (gx, gz) = (((x + SIZE / 2.0) / step).floor(), ((z + SIZE / 2.0) / step).floor());
    let corner = |dx: f32, dz: f32| hills((gx + dx) * step - SIZE / 2.0, (gz + dz) * step - SIZE / 2.0);
    corner(0.0, 0.0)
        .min(corner(1.0, 0.0))
        .min(corner(0.0, 1.0))
        .min(corner(1.0, 1.0))
}

#[test]
fn a_car_at_30_m_s_on_a_heightfield_never_falls_through() {
    let start = vec3(0.0, hills(0.0, 150.0) + REST_HEIGHT + 0.5, 150.0);
    let run = drive(300, start, vec3(0.0, 0.0, -30.0), terrain, |_, wheels| {
        for w in &mut wheels[2..] {
            w.motor_torque = 500.0;
        }
    });
    let mut fastest = 0.0f32;
    let mut grounded = 0;
    for s in &run {
        assert!(s.position.is_finite() && s.velocity.is_finite());
        let floor = ground_under(s.position.x, s.position.z);
        assert!(
            s.position.y > floor,
            "fell through at {}: ground at {floor}",
            s.position
        );
        fastest = fastest.max(s.velocity.length());
        grounded += s.vehicle.wheels().iter().filter(|w| w.grounded).count();
    }
    let last = run.last().unwrap();
    assert!(fastest >= 25.0, "{fastest}");
    assert!(
        last.position.z < start.z - 150.0,
        "covered the distance: {}",
        last.position
    );
    assert!(
        grounded as f32 / (run.len() * 4) as f32 > 0.8,
        "mostly on the ground: {grounded}"
    );
    assert!(
        last.forward.dot(Vec3::NEG_Z) > 0.9,
        "still heading on: {}",
        last.forward
    );
}

#[test]
fn a_parked_car_falls_asleep_and_wakes_up_for_the_throttle() {
    let run = drive(
        600,
        vec3(0.0, REST_HEIGHT, 0.0),
        Vec3::ZERO,
        on_floor,
        |tick, wheels| {
            if tick > 500 {
                for w in wheels.iter_mut() {
                    w.motor_torque = 400.0;
                }
            }
        },
    );
    assert!(run[490].velocity == Vec3::ZERO, "asleep: {}", run[490].velocity);
    assert!(run.last().unwrap().velocity.length() > 1.0, "woke up and drove off");
}

#[test]
fn mass_and_center_of_mass_overrides_reach_the_body() {
    struct Body(Entity);
    let mut seen = (None, None);
    let probe = Rc::new(RefCell::new(None));
    let out = probe.clone();
    Game::new("t", 1, 1)
        .physics3d(Vec3::ZERO)
        .sim_system(move |ctx: &mut Ctx| {
            let e = ctx.world.resource::<Body>().0;
            *out.borrow_mut() = Some((ctx.physics3d.mass(e), ctx.physics3d.center_of_mass(e)));
        })
        .run_headless(2, |ctx| {
            let e = ctx.world.spawn((
                Transform3D::at(vec3(5.0, 1.0, 0.0)),
                RigidBody3D::dynamic().mass(1200.0).center_of_mass(vec3(0.0, -0.4, 0.2)),
                Collider3D::cuboid(vec3(2.0, 1.0, 4.0)).offset(vec3(0.0, 0.5, 0.0)),
            ));
            ctx.world.insert_resource(Body(e));
        });
    if let Some(p) = probe.take() {
        seen = p;
    }
    assert!((seen.0.unwrap() - 1200.0).abs() < 1e-2, "{seen:?}");
    assert!((seen.1.unwrap() - vec3(5.0, 0.6, 0.2)).length() < 1e-4, "{seen:?}");
}

#[test]
fn wheels_on_a_fixed_or_missing_body_do_nothing() {
    let mut vehicle = Vehicle3D::new().with_wheel(Wheel3D::new(Vec3::ZERO, 0.3));
    vehicle.wheels_mut()[0].motor_torque = 100.0;
    let mut world = Game::new("t", 1, 1).physics3d(GRAVITY).run_headless(10, move |ctx| {
        ctx.world.spawn(floor());
        ctx.world.spawn((
            Transform3D::at(vec3(0.0, 0.5, 0.0)),
            RigidBody3D::fixed(),
            Collider3D::sphere(0.2),
            vehicle,
        ));
        // No body at all.
        ctx.world.spawn((Transform3D::default(), vehicle));
    });
    let mut q = world.query::<&Vehicle3D>();
    q.for_each(|v| assert_eq!(v.wheels()[0].angular_velocity, 0.0));
}

#[test]
#[should_panic(expected = "a vehicle has at most 8 wheels")]
fn a_ninth_wheel_is_refused() {
    let mut v = Vehicle3D::new();
    for _ in 0..9 {
        v = v.with_wheel(Wheel3D::default());
    }
}

#[test]
fn vehicles_save_and_load_as_json() {
    let mut world = World::new();
    world.reflect::<Vehicle3D>();
    let mut v = Vehicle3D::new().with_wheel(Wheel3D::new(vec3(1.0, 0.0, 0.0), 0.4));
    v.wheels_mut()[0].sideways_friction.stiffness = 0.7;
    let e = world.spawn((v,));
    let json = world.get_json(e, "Vehicle3D").unwrap();
    world.get_mut::<Vehicle3D>(e).unwrap().wheels_mut()[0].radius = 9.0;
    world.set_json(e, "Vehicle3D", json).unwrap();
    assert_eq!(world.get::<Vehicle3D>(e), Some(&v));
}
