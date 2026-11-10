use ponko::{Game, Phase};
use std::{cell::RefCell, rc::Rc};

#[test]
fn fixed_phases_run_in_order_and_frame_phases_stay_out_of_headless() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut game = Game::new("phases", 1, 1);
    for phase in [
        Phase::Draw,
        Phase::SimLast,
        Phase::Late,
        Phase::AfterPhysics,
        Phase::Animate,
        Phase::Physics,
        Phase::BeforePhysics,
        Phase::Frame,
        Phase::Input,
    ] {
        let log = log.clone();
        game = game.add(phase, move |_| log.borrow_mut().push(phase));
    }
    let log2 = log.clone();
    game = game.sim_system(move |_| log2.borrow_mut().push(Phase::AfterPhysics));
    game.run_headless(2, |_| {});
    let tick = [
        Phase::Input,
        Phase::BeforePhysics,
        Phase::Physics,
        Phase::AfterPhysics,
        Phase::AfterPhysics,
        Phase::SimLast,
    ];
    assert_eq!(*log.borrow(), tick.repeat(2));
}

#[cfg(feature = "physics3d")]
#[test]
fn physics_builder_placement_does_not_change_observed_tick() {
    #[allow(unused_imports)]
    #[cfg(feature = "physics")]
    use ponko::d2::Physics;
    #[allow(unused_imports)]
    #[cfg(feature = "physics3d")]
    use ponko::d3::Collider as Collider3D;
    #[allow(unused_imports)]
    #[cfg(feature = "physics3d")]
    use ponko::d3::RigidBody as RigidBody3D;
    #[allow(unused_imports)]
    #[cfg(feature = "3d")]
    use ponko::d3::Transform as Transform3D;
    #[allow(unused_imports)]
    #[cfg(feature = "physics3d")]
    use ponko::d3::Velocity as Velocity3D;
    #[allow(unused_imports)]
    use ponko::input::Input;
    use ponko::prelude::*;
    fn observe(ctx: &mut Ctx) {
        let y = ctx.world.query::<&Transform3D>().iter().next().unwrap().position.y;
        ctx.world.resource_mut::<Vec<f32>>().push(y);
    }
    fn run(physics_first: bool) -> Vec<f32> {
        let game = Game::new("phases", 1, 1).fixed_hz(50);
        let game = if physics_first {
            game.physics3d(vec3(0.0, -9.81, 0.0)).sim_system(observe)
        } else {
            game.sim_system(observe).physics3d(vec3(0.0, -9.81, 0.0))
        };
        let world = game.run_headless(3, |ctx| {
            ctx.world.insert_resource(Vec::<f32>::new());
            ctx.world.spawn((
                Transform3D::at(vec3(0.0, 10.0, 0.0)),
                RigidBody3D::dynamic(),
                Collider3D::sphere(0.5),
            ));
        });
        world.resource::<Vec<f32>>().clone()
    }
    let a = run(true);
    assert_eq!(a, run(false));
    assert!(a[0] < 10.0 && a[2] < a[1]);
}

/// The contract since physics moved into its own phase: `BeforePhysics`
/// feeds the current tick's step; a plain `sim_system` runs after it, even
/// when it was registered before `.physics3d()`.
#[cfg(feature = "physics3d")]
#[test]
fn before_physics_feeds_the_same_tick_and_sim_system_sees_the_result() {
    #[allow(unused_imports)]
    #[cfg(feature = "physics")]
    use ponko::d2::Physics;
    #[allow(unused_imports)]
    #[cfg(feature = "physics3d")]
    use ponko::d3::Collider as Collider3D;
    #[allow(unused_imports)]
    #[cfg(feature = "physics3d")]
    use ponko::d3::RigidBody as RigidBody3D;
    #[allow(unused_imports)]
    #[cfg(feature = "3d")]
    use ponko::d3::Transform as Transform3D;
    #[allow(unused_imports)]
    #[cfg(feature = "physics3d")]
    use ponko::d3::Velocity as Velocity3D;
    #[allow(unused_imports)]
    use ponko::input::Input;
    use ponko::prelude::*;
    #[derive(Default)]
    struct Seen(Vec<(&'static str, f32)>);
    fn x(ctx: &mut Ctx) -> f32 {
        ctx.world.query::<&Transform3D>().iter().next().unwrap().position.x
    }
    fn push(ctx: &mut Ctx, who: &'static str) {
        let x = x(ctx);
        ctx.world.resource_mut::<Seen>().0.push((who, x));
    }
    fn kick(ctx: &mut Ctx) {
        if ctx.time.sim_tick() == 0 {
            ctx.world
                .query::<&mut Velocity3D>()
                .for_each(|v| v.linear = vec3(5.0, 0.0, 0.0));
        }
    }
    let world = Game::new("phases", 1, 1)
        .fixed_hz(50)
        .sim_system(|ctx: &mut Ctx| push(ctx, "sim_system"))
        .add(Phase::BeforePhysics, kick)
        .add(Phase::BeforePhysics, |ctx: &mut Ctx| push(ctx, "before"))
        .physics3d(Vec3::ZERO)
        .run_headless(1, |ctx| {
            ctx.world.insert_resource(Seen::default());
            ctx.world.spawn((
                Transform3D::at(Vec3::ZERO),
                RigidBody3D::dynamic(),
                Collider3D::sphere(0.5),
                Velocity3D::default(),
            ));
        });
    let seen = &world.resource::<Seen>().0;
    assert_eq!(seen[0], ("before", 0.0), "{seen:?}");
    assert_eq!(seen[1].0, "sim_system");
    // The kick from this tick's BeforePhysics moved the body in this tick's step.
    assert!((seen[1].1 - 5.0 / 50.0).abs() < 1e-3, "{seen:?}");
}
