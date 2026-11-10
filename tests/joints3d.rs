#![cfg(feature = "physics3d")]
#[allow(unused_imports)]
#[cfg(feature = "3d")]
use ponko::d3::Transform as Transform3D;
#[allow(unused_imports)]
#[cfg(feature = "physics3d")]
use ponko::d3::{
    Collider as Collider3D, Joint as Joint3D, JointAxis as JointAxis3D, JointDrive as JointDrive3D,
    JointMotion as JointMotion3D, RigidBody as RigidBody3D, Velocity as Velocity3D,
};
use ponko::prelude::*;
use std::{cell::Cell, rc::Rc};

fn body(ctx: &mut Ctx, position: Vec3, fixed: bool) -> Entity {
    ctx.world.spawn((
        Transform3D::at(position),
        if fixed {
            RigidBody3D::fixed()
        } else {
            RigidBody3D::dynamic()
        },
        Collider3D::cuboid(Vec3::ONE).density(1.0),
        Velocity3D::default(),
    ))
}

#[test]
fn higher_dominance_pusher_keeps_velocity_while_equal_groups_react() {
    for reverse in [false, true] {
        for locked in [false, true] {
            for dominance in [0, 1] {
                let mut ids = None;
                let world = Game::new("test", 1, 1).physics3d(Vec3::ZERO).run_headless(80, |ctx| {
                    let (a, b) = if reverse {
                        let b = body(ctx, Vec3::X * 2.0, false);
                        (body(ctx, Vec3::ZERO, false), b)
                    } else {
                        (body(ctx, Vec3::ZERO, false), body(ctx, Vec3::X * 2.0, false))
                    };
                    ctx.world.get_mut::<RigidBody3D>(a).unwrap().dominance = dominance;
                    ctx.world.get_mut::<RigidBody3D>(a).unwrap().lock_rotation = locked;
                    ctx.world.get_mut::<RigidBody3D>(b).unwrap().lock_rotation = locked;
                    ctx.world.get_mut::<Velocity3D>(a).unwrap().linear = Vec3::X * 4.0;
                    ctx.world.get_mut::<Velocity3D>(b).unwrap().linear = Vec3::X;
                    ids = Some((a, b));
                });
                let (a, b) = ids.unwrap();
                let speed = world.get::<Velocity3D>(a).unwrap().linear.x;
                let pushed = world.get::<Velocity3D>(b).unwrap().linear.x;
                if dominance == 1 {
                    assert!((speed - 4.0).abs() < 1e-5, "{speed}");
                    assert!(pushed > 3.9, "{pushed}");
                    let distance = world.get::<Transform3D>(b).unwrap().position.x
                        - world.get::<Transform3D>(a).unwrap().position.x;
                    assert!(distance > 0.9, "must stay ahead of pusher: {distance}");
                } else {
                    assert!(speed < 3.0, "equal groups must react: {speed}");
                }
            }
        }
    }
}

#[test]
fn spherical_pendulum_keeps_anchor_distance() {
    let mut bob = None;
    let world = Game::new("test", 1, 1)
        .physics3d(vec3(0.0, -9.81, 0.0))
        .run_headless(250, |ctx| {
            let a = body(ctx, Vec3::ZERO, true);
            let b = body(ctx, vec3(0.0, -2.0, 0.0), false);
            ctx.world.get_mut::<Velocity3D>(b).unwrap().linear = Vec3::Z * 3.0;
            let mut j = Joint3D::spherical(a, b);
            j.frame_b.position = Vec3::Y * 2.0;
            ctx.world.spawn((j,));
            bob = Some(b);
        });
    let p = world.get::<Transform3D>(bob.unwrap()).unwrap().position;
    assert!((p.length() - 2.0).abs() < 0.03, "{p}");
}

fn hinge(drive: Option<JointDrive3D>, limits: bool) -> (f32, f32) {
    let mut id = None;
    let world = Game::new("test", 1, 1).physics3d(Vec3::ZERO).run_headless(200, |ctx| {
        let a = body(ctx, Vec3::ZERO, true);
        let b = body(ctx, Vec3::ZERO, false);
        let mut j = Joint3D::revolute(a, b);
        if limits {
            j = j.motion(JointAxis3D::AngularX, JointMotion3D::Limited(-0.4, 0.4));
        }
        if let Some(d) = drive {
            j = j.drive(JointAxis3D::AngularX, d);
        } else {
            ctx.world.get_mut::<Velocity3D>(b).unwrap().angular = Vec3::X * 3.0;
        }
        ctx.world.spawn((j,));
        id = Some(b);
    });
    let b = id.unwrap();
    (
        world.get::<Transform3D>(b).unwrap().rotation.to_scaled_axis().x,
        world.get::<Velocity3D>(b).unwrap().angular.x,
    )
}
#[test]
fn hinge_stops_at_angular_limit() {
    let (angle, _) = hinge(None, true);
    assert!(angle.abs() <= 0.42, "{angle}");
    assert!(angle > 0.3, "{angle}");
}
#[test]
fn motor_reaches_target_velocity() {
    let (_, speed) = hinge(Some(JointDrive3D::velocity(2.0, 20.0)), false);
    assert!((speed - 2.0).abs() < 0.02, "{speed}");
}
#[test]
fn spring_drive_reaches_target_angle() {
    let (angle, speed) = hinge(Some(JointDrive3D::spring(0.6, 100.0, 20.0)), false);
    assert!((angle - 0.6).abs() < 0.02, "{angle}");
    assert!(speed.abs() < 0.02, "{speed}");
}

#[test]
fn fixed_joint_preserves_relative_pose() {
    let mut ids = None;
    let world = Game::new("test", 1, 1)
        .physics3d(vec3(0.0, -9.81, 0.0))
        .run_headless(150, |ctx| {
            let a = body(ctx, Vec3::ZERO, false);
            let b = body(ctx, Vec3::X * 2.0, false);
            ctx.world.get_mut::<Velocity3D>(b).unwrap().angular = Vec3::Z * 2.0;
            let mut j = Joint3D::fixed(a, b);
            j.frame_a.position = Vec3::X * 2.0;
            ctx.world.spawn((j,));
            ids = Some((a, b));
        });
    let (a, b) = ids.unwrap();
    let a = world.get::<Transform3D>(a).unwrap();
    let b = world.get::<Transform3D>(b).unwrap();
    assert!((a.position + a.rotation * Vec3::X * 2.0 - b.position).length() < 0.03);
    assert!(a.rotation.angle_between(b.rotation) < 0.03);
}

#[test]
fn joined_collision_toggle_controls_contact_response() {
    for enabled in [false, true] {
        let mut id = None;
        let world = Game::new("test", 1, 1).physics3d(Vec3::ZERO).run_headless(100, |ctx| {
            let a = body(ctx, Vec3::ZERO, true);
            let b = body(ctx, Vec3::X * 0.8, false);
            let mut j = Joint3D::generic(a, b);
            j.contacts_enabled = enabled;
            ctx.world.spawn((j,));
            id = Some(b);
        });
        let x = world.get::<Transform3D>(id.unwrap()).unwrap().position.x;
        if enabled {
            assert!(x > 0.95, "{x}");
        } else {
            assert!((x - 0.8).abs() < 1e-5, "{x}");
        }
    }
}

#[test]
fn despawning_either_endpoint_or_joint_entity_removes_joint() {
    for remove in 0..3 {
        let target = Rc::new(Cell::new(None));
        let slot = target.clone();
        Game::new("test", 1, 1)
            .physics3d(Vec3::ZERO)
            .sim_system(move |ctx: &mut Ctx| {
                if ctx.time.sim_tick() == 2 {
                    assert_eq!(ctx.physics3d.joint_count(), 1);
                    ctx.world.despawn(slot.get().unwrap());
                }
                if ctx.time.sim_tick() == 4 {
                    assert_eq!(ctx.physics3d.joint_count(), 0);
                }
            })
            .run_headless(6, |ctx| {
                let a = body(ctx, Vec3::ZERO, true);
                let b = body(ctx, Vec3::X, false);
                let j = ctx.world.spawn((Joint3D::fixed(a, b),));
                target.set(Some([a, b, j][remove]));
            });
    }
}

#[test]
fn prismatic_limits_stop_translation_and_lock_other_axes() {
    let mut id = None;
    let world = Game::new("test", 1, 1).physics3d(Vec3::ZERO).run_headless(100, |ctx| {
        let a = body(ctx, Vec3::ZERO, true);
        let b = body(ctx, Vec3::ZERO, false);
        ctx.world.get_mut::<Velocity3D>(b).unwrap().linear = vec3(3.0, 1.0, 1.0);
        ctx.world
            .spawn((Joint3D::prismatic(a, b).motion(JointAxis3D::LinearX, JointMotion3D::Limited(-1.0, 1.0)),));
        id = Some(b);
    });
    let p = world.get::<Transform3D>(id.unwrap()).unwrap().position;
    assert!(p.x > 0.9 && p.x < 1.03 && p.y.abs() < 0.02 && p.z.abs() < 0.02, "{p}");
}

#[test]
fn spherical_cone_and_twist_limits_bound_driven_rotation() {
    let mut id = None;
    let world = Game::new("test", 1, 1).physics3d(Vec3::ZERO).run_headless(250, |ctx| {
        let a = body(ctx, Vec3::ZERO, true);
        let b = body(ctx, Vec3::ZERO, false);
        ctx.world.get_mut::<Velocity3D>(b).unwrap().angular = vec3(2.0, 3.0, 2.0);
        let joint = Joint3D::spherical(a, b)
            .motion(JointAxis3D::AngularX, JointMotion3D::Limited(-0.785, 0.785))
            .swing_cone(0.524);
        ctx.world.spawn((joint,));
        id = Some(b);
    });
    let q = world.get::<Transform3D>(id.unwrap()).unwrap().rotation;
    let swing = (q * Vec3::X).dot(Vec3::X).clamp(-1.0, 1.0).acos();
    let twist = 2.0 * q.x.atan2(q.w);
    assert!(swing < 0.56, "swing {swing}");
    assert!(twist.abs() < 0.82, "twist {twist}");
}

#[test]
fn changed_joint_drive_updates_and_removed_component_detaches() {
    let connection = Rc::new(Cell::new(None));
    let joint_slot = connection.clone();
    let moving = Rc::new(Cell::new(None));
    let moving_slot = moving.clone();
    Game::new("test", 1, 1)
        .physics3d(Vec3::ZERO)
        .sim_system(move |ctx: &mut Ctx| {
            if ctx.time.sim_tick() == 50 {
                assert_eq!(ctx.physics3d.joint_count(), 1);
                let j = ctx.world.get_mut::<Joint3D>(joint_slot.get().unwrap()).unwrap();
                j.drives[3] = Some(JointDrive3D::velocity(-2.0, 20.0));
            }
            if ctx.time.sim_tick() == 100 {
                assert_eq!(ctx.physics3d.joint_count(), 1);
                assert!(
                    ctx.world
                        .get::<Velocity3D>(moving_slot.get().unwrap())
                        .unwrap()
                        .angular
                        .x
                        < -1.9
                );
                ctx.world.remove::<Joint3D>(joint_slot.get().unwrap());
            }
            if ctx.time.sim_tick() == 102 {
                assert_eq!(ctx.physics3d.joint_count(), 0);
            }
        })
        .run_headless(104, |ctx| {
            let a = body(ctx, Vec3::ZERO, true);
            let b = body(ctx, Vec3::ZERO, false);
            let j = Joint3D::revolute(a, b).drive(JointAxis3D::AngularX, JointDrive3D::velocity(2.0, 20.0));
            connection.set(Some(ctx.world.spawn((j,))));
            moving.set(Some(b));
        });
}
