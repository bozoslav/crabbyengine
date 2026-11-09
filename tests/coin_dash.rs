//! The `coin_dash` reference game, played headlessly: a win, a loss, restarts
//! and the input mapping, with no window and no GPU. The test drives the game's
//! own systems (`examples/coin_dash/game.rs`) with a scripted player.

#[path = "../examples/coin_dash/game.rs"]
mod game;

use game::*;
// The game has its own `Phase` (round state); name it explicitly so it wins over `ponko::Phase`.
use game::Phase;
use ponko::*;

/// The sim runs at the default 60 Hz: ticks per second, for readable run lengths.
const HZ: u64 = 60;

/// A system that runs before the game's rules and decides what the player
/// does this tick (in place of `read_input`, which needs a window's frames).
fn script(f: impl FnMut(&mut Ctx) + 'static) -> Game {
    install(Game::new("coin dash test", 800, 600).seed(SEED).sim_system(f))
}

fn player_and_coins(ctx: &mut Ctx) -> (Vec2, Vec<Vec2>) {
    let mut player = Vec2::ZERO;
    for (t, _) in ctx.world.query::<(&Transform2D, &Player)>().iter() {
        player = t.position;
    }
    let coins = ctx
        .world
        .query::<(&Transform2D, &Coin)>()
        .iter()
        .map(|(t, _)| t.position)
        .collect();
    (player, coins)
}

/// Walks straight to the nearest coin.
fn chase_nearest_coin(ctx: &mut Ctx) {
    let (player, coins) = player_and_coins(ctx);
    let nearest = coins
        .into_iter()
        .min_by(|a, b| a.distance(player).total_cmp(&b.distance(player)));
    let direction = nearest.map_or(Vec2::ZERO, |c| (c - player).normalize_or_zero());
    ctx.world.resource_mut::<Intent>().direction = direction;
}

fn coin_layout(world: &mut World) -> Vec<Vec2> {
    let mut coins: Vec<Vec2> = world
        .query::<(&Transform2D, &Coin)>()
        .iter()
        .map(|(t, _)| t.position)
        .collect();
    coins.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    coins
}

#[test]
fn collecting_every_coin_wins_the_round() {
    let mut world = script(chase_nearest_coin).run_headless(ROUND_SECONDS as u64 * HZ, setup);
    let round = world.resource::<Round>().clone();
    assert_eq!(round.phase, Phase::Won);
    assert_eq!((round.collected, round.number), (COINS, 1));
    assert!(round.time_left > 0.0, "won with time to spare, got {}", round.time_left);
    assert_eq!(world.query::<&Coin>().iter().count(), 0);
}

#[test]
fn a_finished_round_stays_finished_until_restart() {
    let mut frozen = None;
    let mut world = script(move |ctx| {
        chase_nearest_coin(ctx);
        // Once won, keep "walking" and let the clock run: nothing may change.
        let round = ctx.world.resource::<Round>().clone();
        if round.phase == Phase::Won {
            let seen = (round.collected, round.time_left);
            assert_eq!(*frozen.get_or_insert(seen), seen, "state changed after winning");
        }
    })
    .run_headless(ROUND_SECONDS as u64 * HZ, setup);
    assert_eq!(world.resource::<Round>().phase, Phase::Won);
    assert_eq!(world.query::<&Player>().iter().count(), 1);
}

#[test]
fn running_out_of_time_loses_the_round() {
    let ticks = ROUND_SECONDS as u64 * HZ + 10; // a few spare ticks for rounding
    let world = script(|_| {}).run_headless(ticks, setup);
    let round = world.resource::<Round>();
    assert_eq!(round.phase, Phase::Lost);
    assert_eq!((round.collected, round.time_left), (0, 0.0));
}

/// Plays one round with `play`, asks for a restart on the first tick the round
/// is over, and stops on that tick.
fn restart_when_over(mut play: impl FnMut(&mut Ctx) + 'static) -> impl FnMut(&mut Ctx) + 'static {
    move |ctx| {
        if ctx.world.resource::<Round>().phase == Phase::Playing {
            play(ctx);
        } else {
            ctx.world.resource_mut::<Intent>().restart = true;
            ctx.quit(); // the tick still finishes, so `rules` handles the restart
        }
    }
}

fn assert_fresh_round(world: &mut World, number: u32) {
    let round = world.resource::<Round>().clone();
    assert_eq!(round.phase, Phase::Playing);
    assert_eq!((round.number, round.collected), (number, 0));
    assert_eq!(round.time_left, ROUND_SECONDS);
    assert_eq!(world.query::<&Coin>().iter().count(), COINS);
    let players: Vec<Vec2> = world
        .query::<(&Transform2D, &Player)>()
        .iter()
        .map(|(t, _)| t.position)
        .collect();
    assert_eq!(players, [ARENA / 2.0], "one player, back in the middle");
}

#[test]
fn restarting_after_a_win_starts_a_fresh_round() {
    let mut world = script(restart_when_over(chase_nearest_coin)).run_headless(2 * ROUND_SECONDS as u64 * HZ, setup);
    assert_fresh_round(&mut world, 2);
}

#[test]
fn restarting_after_a_loss_starts_a_fresh_round() {
    let mut world = script(restart_when_over(|_| {})).run_headless(2 * ROUND_SECONDS as u64 * HZ, setup);
    assert_fresh_round(&mut world, 2);
}

#[test]
fn restart_works_in_the_middle_of_a_round_too() {
    let taken_before_restart = std::rc::Rc::new(std::cell::Cell::new(0));
    let seen = taken_before_restart.clone();
    let mut world = script(move |ctx| {
        chase_nearest_coin(ctx);
        if ctx.sim_tick() == 2 * HZ {
            seen.set(ctx.world.resource::<Round>().collected);
            ctx.world.resource_mut::<Intent>().restart = true;
            ctx.quit();
        }
    })
    .run_headless(10 * HZ, setup);
    assert!(taken_before_restart.get() > 0, "the round was under way");
    assert_fresh_round(&mut world, 2);
}

#[test]
fn the_restarted_round_can_be_won_too() {
    // Win, restart, win again: the whole loop twice.
    let mut restarted = false;
    let mut world = script(move |ctx| {
        let round = ctx.world.resource::<Round>().clone();
        if round.phase == Phase::Won && round.number == 1 && !restarted {
            restarted = true;
            ctx.world.resource_mut::<Intent>().restart = true;
        }
        chase_nearest_coin(ctx);
    })
    .run_headless(2 * ROUND_SECONDS as u64 * HZ, setup);
    let round = world.resource::<Round>().clone();
    assert_eq!((round.phase, round.number, round.collected), (Phase::Won, 2, COINS));
    assert_eq!(world.query::<&Coin>().iter().count(), 0);
}

#[test]
fn rounds_are_reproducible_and_each_round_has_its_own_layout() {
    let layout_after = |ticks| coin_layout(&mut script(|_| {}).run_headless(ticks, setup));
    assert_eq!(layout_after(0), layout_after(0), "same seed, same coins");
    assert_eq!(layout_after(0).len(), COINS);

    // Same inputs on both runs: identical games, tick for tick.
    let play = || script(restart_when_over(chase_nearest_coin)).run_headless(2 * ROUND_SECONDS as u64 * HZ, setup);
    let (mut a, mut b) = (play(), play());
    assert_eq!(coin_layout(&mut a), coin_layout(&mut b));
    assert_eq!(a.resource::<Round>().time_left, b.resource::<Round>().time_left);

    // Round 2 is not round 1 again.
    assert_ne!(layout_after(0), coin_layout(&mut a));
}

#[test]
fn the_player_stays_in_the_arena_and_diagonals_are_not_faster() {
    let ticks = HZ; // one second
    let mut world = script(|ctx| ctx.world.resource_mut::<Intent>().direction = vec2(1.0, 1.0)).run_headless(1, setup);
    let moved = world
        .query::<(&Transform2D, &Player)>()
        .iter()
        .next()
        .unwrap()
        .0
        .position
        - ARENA / 2.0;
    let one_tick = SPEED / HZ as f32;
    assert!(
        (moved.length() - one_tick).abs() < 1e-3,
        "moved {moved}, expected length {one_tick}"
    );

    let mut world =
        script(|ctx| ctx.world.resource_mut::<Intent>().direction = vec2(1.0, 1.0)).run_headless(ticks * 10, setup);
    let at = world
        .query::<(&Transform2D, &Player)>()
        .iter()
        .next()
        .unwrap()
        .0
        .position;
    assert_eq!(at, ARENA, "pinned in the bottom-right corner");
}

#[test]
fn keys_and_gamepad_map_to_the_same_intent() {
    Game::new("t", 800, 600).run_headless(0, |ctx| {
        setup(ctx);
        // Keyboard: A is left, S is down.
        ctx.feed_input(InputEvent::Key(Key::A, true));
        ctx.feed_input(InputEvent::Key(Key::S, true));
        read_input(ctx);
        let d = ctx.world.resource::<Intent>().direction;
        assert!(d.x < 0.0 && d.y > 0.0, "left and down, got {d}");
        ctx.feed_input(InputEvent::Key(Key::A, false));
        ctx.feed_input(InputEvent::Key(Key::S, false));

        // Arrow keys do the same as WASD.
        ctx.feed_input(InputEvent::Key(Key::Right, true));
        read_input(ctx);
        assert!(ctx.world.resource::<Intent>().direction.x > 0.0);
        ctx.feed_input(InputEvent::Key(Key::Right, false));

        // The left stick, past its dead zone, pushes up.
        let pad = Gamepad::from_slot(0).unwrap();
        ctx.feed_input(InputEvent::GamepadAxis(pad, GamepadAxis::LeftStickY, -1.0));
        read_input(ctx);
        assert!(ctx.world.resource::<Intent>().direction.y < 0.0, "stick up");
        ctx.feed_input(InputEvent::GamepadAxis(pad, GamepadAxis::LeftStickY, 0.0));

        // Nothing held: standing still.
        read_input(ctx);
        assert_eq!(ctx.world.resource::<Intent>().direction, Vec2::ZERO);
    });
}

#[test]
fn every_restart_binding_asks_for_a_restart_that_the_sim_takes_once() {
    let pad = Gamepad::from_slot(0).unwrap();
    let presses = [
        (InputEvent::Key(Key::R, true), InputEvent::Key(Key::R, false)),
        (InputEvent::Key(Key::Enter, true), InputEvent::Key(Key::Enter, false)),
        (
            InputEvent::GamepadButton(pad, GamepadButton::South, true),
            InputEvent::GamepadButton(pad, GamepadButton::South, false),
        ),
    ];
    Game::new("t", 800, 600).run_headless(0, |ctx| {
        setup(ctx);
        for (n, (press, release)) in presses.into_iter().enumerate() {
            assert!(!ctx.world.resource::<Intent>().restart, "not pressed yet");
            ctx.feed_input(press);
            read_input(ctx);
            ctx.feed_input(release);
            // A frame without a sim tick (the release above, another frame) must not lose it.
            assert!(ctx.world.resource::<Intent>().restart, "{press:?} requests a restart");
            rules(ctx);
            assert!(!ctx.world.resource::<Intent>().restart, "taken exactly once");
            assert_eq!(
                ctx.world.resource::<Round>().number,
                n as u32 + 2,
                "one new round per press"
            );
        }
    });
}
