#![cfg(feature = "physics3d")]
#[allow(unused_imports)]
#[cfg(feature = "3d")]
use ponko::d3::Transform;
#[allow(unused_imports)]
#[cfg(feature = "physics3d")]
use ponko::d3::{Collider, Joint, JointAxis, JointDrive, JointMotion, RigidBody};
use ponko::prelude::*;

#[test]
fn guide_joint_and_dominance_snippets_compile_and_step() {
    Game::new("joints", 1, 1).physics3d(Vec3::ZERO).run_headless(5, |ctx| {
        // ANCHOR: joints
        let a = ctx
            .world
            .spawn((Transform::default(), RigidBody::fixed(), Collider::sphere(0.1)));
        let b = ctx.world.spawn((
            Transform::at(vec3(0.0, -1.0, 0.0)),
            RigidBody::dynamic(),
            Collider::sphere(0.2),
        ));
        let mut joint = Joint::spherical(a, b)
            .motion(JointAxis::AngularX, JointMotion::Limited(-0.785, 0.785))
            .swing_cone(0.524)
            .drive(JointAxis::AngularX, JointDrive::spring(0.0, 100.0, 20.0));
        joint.frame_b.position = Vec3::Y;
        joint.contacts_enabled = false;
        let connection = ctx.world.spawn((joint,));
        // Despawn `connection` to detach; either body's despawn also removes the joint.
        // ANCHOR_END: joints
        assert!(ctx.world.get::<Joint>(connection).is_some());
        // ANCHOR: dominance
        let pusher = RigidBody::dynamic().dominance(1);
        let pushed = RigidBody::dynamic(); // group 0
        // Both remain dynamic: gravity, walls and equal-group bodies still affect them.
        ctx.world
            .spawn((Transform::at(Vec3::X * 3.0), pusher, Collider::cuboid(Vec3::ONE)));
        ctx.world
            .spawn((Transform::at(Vec3::X * 4.0), pushed, Collider::cuboid(Vec3::ONE)));
        // ANCHOR_END: dominance
    });
}
