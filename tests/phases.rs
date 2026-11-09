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
    use ponko::*;
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
