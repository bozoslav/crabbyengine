use ponko::{Ctx, Game, Phase, Plugin, Systems};

struct Marker;
struct Area(&'static str);
impl Plugin for Area {
    fn build(&self, game: Game) -> Game {
        let name = self.0;
        game.add(Phase::BeforePhysics, move |ctx| {
            ctx.world.resource_mut::<Vec<&str>>().push(name)
        })
    }
    fn setup(&self, ctx: &mut Ctx) {
        ctx.world.resource_mut::<Vec<&str>>().push(self.0);
        ctx.commands.spawn((Marker,));
    }
}

#[test]
fn plugins_and_setups_interleave_and_apply_commands_before_systems() {
    let world = Game::new("plugins", 1, 1)
        .setup(|ctx| ctx.world.insert_resource(Vec::<&str>::new()))
        .plugin(Area("first"))
        .setup(|ctx| {
            assert_eq!(ctx.world.query::<&Marker>().count(), 1);
            ctx.world.resource_mut::<Vec<&str>>().push("user");
        })
        .plugin(Area("second"))
        .run_headless(1);
    assert_eq!(
        *world.resource::<Vec<&str>>(),
        ["first", "user", "second", "first", "second"]
    );
}

#[test]
fn plugin_setup_precedes_legacy_setup_and_runs_at_zero_ticks() {
    struct Init;
    impl Plugin for Init {
        fn build(&self, game: Game) -> Game {
            game
        }
        fn setup(&self, ctx: &mut Ctx) {
            ctx.commands.spawn((Marker,));
        }
    }
    let mut world = Game::new("plugins", 1, 1).plugin(Init).run_headless(0, |ctx| {
        assert_eq!(ctx.world.query::<&Marker>().count(), 1);
    });
    assert_eq!(world.query::<&Marker>().count(), 1);
}

#[test]
fn ids_distinguish_instances_of_the_same_closure_and_do_not_recycle() {
    let mut game = Game::new("ids", 1, 1);
    let add = |value| move |ctx: &mut Ctx| ctx.world.resource_mut::<Vec<i32>>().push(value);
    let first = game.add_system(Phase::AfterPhysics, add(1));
    let second = game.add_system(Phase::AfterPhysics, add(2));
    assert_ne!(first, second);
    let mut game = game.remove(first).remove(first);
    let third = game.add_system(Phase::AfterPhysics, add(3));
    assert_ne!(first, third);
    let world = game.run_headless(1, |ctx| ctx.world.insert_resource(Vec::<i32>::new()));
    assert_eq!(*world.resource::<Vec<i32>>(), [2, 3]);
}

#[test]
#[allow(deprecated)] // the legacy spellings are what this pins
fn builtin_ids_and_legacy_removal_coexist() {
    let game = Game::new("ids", 1, 1)
        .remove(Systems::DRAW_SPRITES)
        .remove_system(ponko::animate_sprites);
    game.run_headless(0, |_| {});
}

#[cfg(feature = "physics")]
#[test]
fn stable_physics_id_removes_step_after_repeated_configuration() {
    use ponko::*;
    let mut world = Game::new("physics ids", 1, 1)
        .physics(vec2(0.0, 980.0))
        .physics(vec2(0.0, 980.0))
        .remove(Systems::PHYSICS)
        .run_headless(60, |ctx| {
            ctx.world.spawn((
                Transform2D::at(vec2(0.0, 0.0)),
                RigidBody::dynamic(),
                Collider::circle(8.0),
            ));
        });
    assert_eq!(world.query::<&Transform2D>().iter().next().unwrap().position.y, 0.0);
}

#[test]
#[allow(deprecated)] // the legacy spellings are what this pins
fn builtin_registrations_have_individual_handles_and_named_removal_stays_exact() {
    let mut game = Game::new("ids", 1, 1);
    let first = game.add_system(Phase::Input, ponko::systems::rebuild_spatial_grid);
    let second = game.add_system(Phase::Input, ponko::systems::rebuild_spatial_grid);
    assert_ne!(first, second);
    let mut game = game
        .remove(first)
        .named_system("custom sprites", ponko::systems::draw_sprites);
    assert_eq!(game.remove_named_system("custom sprites"), 1);
    // The default registration must remain removable by its original spelling.
    game.remove_system(ponko::draw_sprites).run_headless(0, |_| {});
}

#[test]
fn plugin_setup_is_shared_by_profiled_runs() {
    let (world, _) = Game::new("plugins", 1, 1)
        .setup(|ctx| ctx.world.insert_resource(Vec::<&str>::new()))
        .plugin(Area("profile"))
        .profile_headless(1, 2);
    assert_eq!(*world.resource::<Vec<&str>>(), ["profile"; 4]);
}

#[test]
fn user_ids_from_other_builders_are_harmless() {
    let mut first = Game::new("first", 1, 1);
    let foreign = first.add_system(Phase::Input, |_| {});
    let mut second = Game::new("second", 1, 1);
    let local = second.add_system(Phase::Input, |ctx| ctx.world.insert_resource(42usize));
    assert_ne!(foreign, local);
    let world = second.remove(foreign).run_headless(1, |_| {});
    assert_eq!(*world.resource::<usize>(), 42);
}

#[test]
fn plugin_systems_run_by_phase_not_by_plugin_order() {
    struct Late;
    impl Plugin for Late {
        fn build(&self, game: Game) -> Game {
            game.add(Phase::SimLast, |ctx| {
                ctx.world.resource_mut::<Vec<&str>>().push("sim last")
            })
            .add(Phase::AfterPhysics, |ctx| {
                ctx.world.resource_mut::<Vec<&str>>().push("after physics")
            })
        }
    }
    struct Early;
    impl Plugin for Early {
        fn build(&self, game: Game) -> Game {
            game.add(Phase::Input, |ctx| ctx.world.resource_mut::<Vec<&str>>().push("input"))
                .add(Phase::BeforePhysics, |ctx| {
                    ctx.world.resource_mut::<Vec<&str>>().push("before physics")
                })
        }
    }
    let world = Game::new("phases", 1, 1)
        .plugin(Late)
        .plugin(Early)
        .setup(|ctx| ctx.world.insert_resource(Vec::<&str>::new()))
        .run_headless(1);
    assert_eq!(
        *world.resource::<Vec<&str>>(),
        ["input", "before physics", "after physics", "sim last"]
    );
}

#[test]
fn removing_a_plugin_system_by_id_keeps_its_siblings() {
    struct Counter;
    impl Plugin for Counter {
        fn build(&self, mut game: Game) -> Game {
            let doomed = game.add_system(Phase::AfterPhysics, |ctx| *ctx.world.resource_mut::<u32>() += 100);
            game.add(Phase::AfterPhysics, |ctx| *ctx.world.resource_mut::<u32>() += 1)
                .remove(doomed)
        }
        fn setup(&self, ctx: &mut Ctx) {
            ctx.world.insert_resource(0u32);
        }
    }
    let world = Game::new("remove", 1, 1).plugin(Counter).run_headless(2, |_| {});
    assert_eq!(*world.resource::<u32>(), 2);
}
