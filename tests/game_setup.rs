use ponko::Game;

struct Marker;

#[test]
fn builder_setups_run_once_in_order_and_apply_commands_between_callbacks() {
    let world = Game::new("setup", 1, 1)
        .sim_system(|ctx| *ctx.world.resource_mut::<usize>() += 1)
        .setup(|ctx| {
            ctx.world.insert_resource(0usize);
            ctx.commands.spawn((Marker,));
        })
        .setup(|ctx| {
            assert_eq!(ctx.world.query::<&Marker>().count(), 1);
            assert_eq!(*ctx.world.resource::<usize>(), 0);
            *ctx.world.resource_mut::<usize>() = 10;
        })
        .run_headless(3);
    assert_eq!(*world.resource::<usize>(), 13);
}

#[test]
fn runtime_branches_can_choose_different_capturing_setup_closures() {
    for mode in [true, false] {
        let game = Game::new("setup", 1, 1);
        let game = if mode {
            let text = String::from("server");
            game.setup(move |ctx| ctx.world.insert_resource(text))
        } else {
            let number = 7usize;
            game.setup(move |ctx| ctx.world.insert_resource(number.to_string()))
        };
        let world = game.run_headless(0);
        assert_eq!(world.resource::<String>(), if mode { "server" } else { "7" });
    }
}

#[test]
fn stored_setup_is_used_by_headless_profiling() {
    let (world, _) = Game::new("profile", 1, 1)
        .setup(|ctx| ctx.world.insert_resource(42usize))
        .profile_headless(0, 1);
    assert_eq!(*world.resource::<usize>(), 42);
}

#[test]
fn a_game_of_plugins_and_systems_runs_without_a_setup_closure() {
    struct Ticks(u32);
    struct Counting;
    impl ponko::Plugin for Counting {
        fn build(&self, game: Game) -> Game {
            game.sim_system(|ctx| ctx.world.resource_mut::<Ticks>().0 += 1)
        }
        fn setup(&self, ctx: &mut ponko::Ctx) {
            ctx.world.insert_resource(Ticks(0));
        }
    }
    let world = Game::new("plugins", 1, 1)
        .plugin(Counting)
        .sim_system(|ctx| ctx.world.resource_mut::<Ticks>().0 += 10)
        .configured()
        .run_headless(2);
    assert_eq!(world.resource::<Ticks>().0, 22);
    // A plain game with nothing to set up runs too, and setups can follow.
    let world = Game::new("empty", 1, 1)
        .configured()
        .setup(|ctx| ctx.world.insert_resource(Ticks(5)))
        .run_headless(1);
    assert_eq!(world.resource::<Ticks>().0, 5);
}
