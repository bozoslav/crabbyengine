//! The `coin_dash` reference game, played headlessly: the title screen, rounds,
//! pause, the result screen, saved progress (including a missing or damaged
//! save file) and the input mapping, with no window and no GPU. The tests drive
//! the game's own systems (`examples/coin_dash/game.rs`) with a scripted player
//! or with scheduled raw key presses.

#[path = "../examples/coin_dash/game.rs"]
mod game;

use game::*;
use ponko::*;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

/// The sim runs at the default 60 Hz: ticks per second, for readable run lengths.
const HZ: u64 = 60;

// ANCHOR: script
/// A system that runs before the game's rules and decides what the player
/// does this tick (in place of `read_input`, which needs a window's frames).
fn script(f: impl FnMut(&mut Ctx) + 'static) -> Game {
    install(Game::new("coin dash test", 800, 600).seed(SEED).sim_system(f))
}
// ANCHOR_END: script

/// Startup that skips the title screen: round 1 is already running, nothing is saved.
fn setup_playing(ctx: &mut Ctx) {
    setup_with(ctx, None);
    new_round(ctx, 1);
}

fn intent(ctx: &mut Ctx) -> &mut Intent {
    ctx.world.resource_mut::<Intent>()
}

fn screen(world: &World) -> Screen {
    *world.resource::<Screen>()
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

// ANCHOR: chase
/// Walks straight to the nearest coin.
fn chase_nearest_coin(ctx: &mut Ctx) {
    let (player, coins) = player_and_coins(ctx);
    let nearest = coins
        .into_iter()
        .min_by(|a, b| a.distance(player).total_cmp(&b.distance(player)));
    let direction = nearest.map_or(Vec2::ZERO, |c| (c - player).normalize_or_zero());
    ctx.world.resource_mut::<Intent>().direction = direction;
}
// ANCHOR_END: chase

fn coin_layout(world: &mut World) -> Vec<Vec2> {
    let mut coins: Vec<Vec2> = world
        .query::<(&Transform2D, &Coin)>()
        .iter()
        .map(|(t, _)| t.position)
        .collect();
    coins.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    coins
}

/// A fresh, empty folder for one test's save file (under the system temp dir).
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ponko-coin-dash-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Plays round 1 to a win (chasing coins) with progress kept in `save`.
fn win_a_round(save: &std::path::Path) -> World {
    let save = save.to_path_buf();
    script(chase_nearest_coin).run_headless(ROUND_SECONDS as u64 * HZ, move |ctx| {
        setup_with(ctx, Some(save));
        new_round(ctx, 1);
    })
}

/// Starts on the title screen with progress in `save`, runs `ticks`, returns the world.
fn open_game(save: Option<PathBuf>, ticks: u64) -> World {
    script(|_| {}).run_headless(ticks, move |ctx| setup_with(ctx, save))
}

// ---- Rounds -----------------------------------------------------------------

// ANCHOR: win
#[test]
fn collecting_every_coin_wins_the_round() {
    let mut world = script(chase_nearest_coin).run_headless(ROUND_SECONDS as u64 * HZ, setup_playing);
    let round = world.resource::<Round>().clone();
    assert_eq!(screen(&world), Screen::Result(Outcome::Won));
    assert_eq!((round.collected, round.number), (COINS, 1));
    assert!(round.time_left > 0.0, "won with time to spare, got {}", round.time_left);
    assert_eq!(world.query::<&Coin>().iter().count(), 0);
}
// ANCHOR_END: win

#[test]
fn a_finished_round_stays_finished_until_the_player_moves_on() {
    let mut frozen = None;
    let mut world = script(move |ctx| {
        chase_nearest_coin(ctx);
        // Once won, keep "walking" and let the clock run: nothing may change.
        if matches!(*ctx.world.resource::<Screen>(), Screen::Result(_)) {
            let round = ctx.world.resource::<Round>().clone();
            let seen = (round.collected, round.time_left);
            assert_eq!(*frozen.get_or_insert(seen), seen, "state changed after winning");
        }
    })
    .run_headless(ROUND_SECONDS as u64 * HZ, setup_playing);
    assert_eq!(screen(&world), Screen::Result(Outcome::Won));
    assert_eq!(world.query::<&Player>().iter().count(), 1);
}

#[test]
fn running_out_of_time_loses_the_round() {
    let ticks = ROUND_SECONDS as u64 * HZ + 10; // a few spare ticks for rounding
    let world = script(|_| {}).run_headless(ticks, setup_playing);
    let round = world.resource::<Round>();
    assert_eq!(screen(&world), Screen::Result(Outcome::Lost));
    assert_eq!((round.collected, round.time_left), (0, 0.0));
}

/// Plays one round with `play`, asks for a new round on the first tick the
/// result screen shows, and stops on that tick.
fn restart_when_over(mut play: impl FnMut(&mut Ctx) + 'static) -> impl FnMut(&mut Ctx) + 'static {
    move |ctx| {
        if screen(&ctx.world) == Screen::Playing {
            play(ctx);
        } else {
            intent(ctx).confirm = true;
            ctx.quit(); // the tick still finishes, so `rules` handles it
        }
    }
}

fn assert_fresh_round(world: &mut World, number: u32) {
    let round = world.resource::<Round>().clone();
    assert_eq!(screen(world), Screen::Playing);
    assert_eq!((round.number, round.collected, round.new_best), (number, 0, false));
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
fn playing_again_after_a_win_starts_a_fresh_round() {
    let mut world =
        script(restart_when_over(chase_nearest_coin)).run_headless(2 * ROUND_SECONDS as u64 * HZ, setup_playing);
    assert_fresh_round(&mut world, 2);
}

#[test]
fn playing_again_after_a_loss_starts_a_fresh_round() {
    let mut world = script(restart_when_over(|_| {})).run_headless(2 * ROUND_SECONDS as u64 * HZ, setup_playing);
    assert_fresh_round(&mut world, 2);
}

#[test]
fn restart_works_in_the_middle_of_a_round_too() {
    let taken_before_restart = Rc::new(Cell::new(0));
    let seen = taken_before_restart.clone();
    let mut world = script(move |ctx| {
        chase_nearest_coin(ctx);
        if ctx.sim_tick() == 2 * HZ {
            seen.set(ctx.world.resource::<Round>().collected);
            intent(ctx).restart = true;
            ctx.quit();
        }
    })
    .run_headless(10 * HZ, setup_playing);
    assert!(taken_before_restart.get() > 0, "the round was under way");
    assert_fresh_round(&mut world, 2);
}

#[test]
fn the_next_round_can_be_won_too() {
    // Win, play again, win again: the whole loop twice.
    let mut world = script(move |ctx| {
        if matches!(*ctx.world.resource::<Screen>(), Screen::Result(_)) && ctx.world.resource::<Round>().number == 1 {
            intent(ctx).confirm = true;
        }
        chase_nearest_coin(ctx);
    })
    .run_headless(2 * ROUND_SECONDS as u64 * HZ, setup_playing);
    let round = world.resource::<Round>().clone();
    assert_eq!(
        (screen(&world), round.number, round.collected),
        (Screen::Result(Outcome::Won), 2, COINS)
    );
    assert_eq!(world.query::<&Coin>().iter().count(), 0);
}

#[test]
fn rounds_are_reproducible_and_each_round_has_its_own_layout() {
    let layout_after = |ticks| coin_layout(&mut script(|_| {}).run_headless(ticks, setup_playing));
    assert_eq!(layout_after(0), layout_after(0), "same seed, same coins");
    assert_eq!(layout_after(0).len(), COINS);

    // Same inputs on both runs: identical games, tick for tick.
    let play =
        || script(restart_when_over(chase_nearest_coin)).run_headless(2 * ROUND_SECONDS as u64 * HZ, setup_playing);
    let (mut a, mut b) = (play(), play());
    assert_eq!(coin_layout(&mut a), coin_layout(&mut b));
    assert_eq!(a.resource::<Round>().time_left, b.resource::<Round>().time_left);

    // Round 2 is not round 1 again.
    assert_ne!(layout_after(0), coin_layout(&mut a));
}

#[test]
fn the_player_stays_in_the_arena_and_diagonals_are_not_faster() {
    let ticks = HZ; // one second
    let mut world = script(|ctx| intent(ctx).direction = vec2(1.0, 1.0)).run_headless(1, setup_playing);
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

    let mut world = script(|ctx| intent(ctx).direction = vec2(1.0, 1.0)).run_headless(ticks * 10, setup_playing);
    let at = world
        .query::<(&Transform2D, &Player)>()
        .iter()
        .next()
        .unwrap()
        .0
        .position;
    assert_eq!(at, ARENA, "pinned in the bottom-right corner");
}

// ---- Title, pause and result screens ----------------------------------------

#[test]
fn the_game_starts_on_an_empty_title_screen_that_ignores_movement() {
    let world = script(|ctx| intent(ctx).direction = vec2(1.0, 0.0)).run_headless(2 * HZ, |ctx| setup_with(ctx, None));
    assert_eq!(screen(&world), Screen::Title);
    assert_eq!(world.resource::<Round>().number, 0);
    assert!(world.is_empty(), "no player and no coins before the first round");
}

#[test]
fn confirm_on_the_title_screen_starts_round_one() {
    let mut world = script(|ctx| {
        if ctx.sim_tick() == 3 {
            intent(ctx).confirm = true;
            ctx.quit();
        }
    })
    .run_headless(10, |ctx| setup_with(ctx, None));
    assert_fresh_round(&mut world, 1);
}

#[test]
fn restart_and_pause_do_not_start_a_round_from_the_title_screen() {
    let world = script(|ctx| {
        let i = intent(ctx);
        i.restart = true;
        i.direction = vec2(1.0, 1.0);
    })
    .run_headless(5, |ctx| setup_with(ctx, None));
    assert_eq!(screen(&world), Screen::Title);
}

#[test]
fn pause_or_back_on_the_title_screen_quits_the_game() {
    for press in [|i: &mut Intent| i.pause = true, |i: &mut Intent| i.back = true] {
        let ticks = Rc::new(Cell::new(0));
        let counted = ticks.clone();
        script(move |ctx| {
            counted.set(counted.get() + 1);
            if ctx.sim_tick() == 2 {
                press(intent(ctx));
            }
        })
        .run_headless(100, |ctx| setup_with(ctx, None));
        assert_eq!(ticks.get(), 3, "the game stops on the tick after the request");
    }
}

#[test]
fn pausing_freezes_the_round_and_resuming_continues_it() {
    // Chase for one second, pause for five, resume.
    let at_pause = Rc::new(RefCell::new(None));
    let record = at_pause.clone();
    let mut world = script(move |ctx| {
        let tick = ctx.sim_tick();
        if tick == HZ {
            intent(ctx).pause = true;
        }
        if tick == 6 * HZ {
            intent(ctx).confirm = true; // resume
        }
        let snapshot = {
            let round = ctx.world.resource::<Round>();
            (round.collected, round.time_left, player_and_coins(ctx))
        };
        if screen(&ctx.world) == Screen::Paused {
            // Held movement and coins in reach must change nothing while paused.
            let first = record.borrow_mut().get_or_insert(snapshot.clone()).clone();
            assert_eq!(first, snapshot, "something moved at tick {tick} while paused");
        }
        chase_nearest_coin(ctx);
    })
    .run_headless(6 * HZ + 1, setup_playing);
    assert!(at_pause.borrow().is_some(), "the game was paused");
    // Resumed on the last tick: the clock runs again from where it stopped.
    assert_eq!(screen(&world), Screen::Playing);
    let (_, time_at_pause, _) = at_pause.borrow().clone().unwrap();
    let time_left = world.resource::<Round>().time_left;
    assert!(
        time_left <= time_at_pause && time_at_pause - time_left < 0.1,
        "{time_at_pause} -> {time_left}"
    );
    assert!(
        time_left > ROUND_SECONDS - 2.0,
        "five paused seconds were not counted: {time_left}"
    );
    assert_eq!(world.query::<&Player>().iter().count(), 1);
}

#[test]
fn pause_toggles_with_the_pause_button_and_confirm_resumes() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = seen.clone();
    script(move |ctx| {
        log.borrow_mut().push(screen(&ctx.world));
        match ctx.sim_tick() {
            1 | 5 => intent(ctx).pause = true, // pause ...
            3 => intent(ctx).confirm = true,   // ... resume with confirm
            7 => intent(ctx).pause = true,     // ... resume with pause
            8 => intent(ctx).confirm = true,   // confirm while playing does nothing
            _ => {}
        }
    })
    .run_headless(10, setup_playing);
    use Screen::*;
    // Each entry is the screen at the start of a tick, before that tick's rules ran,
    // so a request made on tick `t` shows on tick `t + 1`.
    assert_eq!(
        *seen.borrow(),
        [
            Playing, Playing, Paused, Paused, Playing, Playing, Paused, Paused, Playing, Playing
        ]
    );
}

#[test]
fn restart_from_the_pause_screen_starts_a_fresh_round_and_back_returns_to_the_title() {
    let mut world = script(|ctx| {
        match ctx.sim_tick() {
            HZ => intent(ctx).pause = true,
            n if n == HZ + 2 => {
                intent(ctx).restart = true;
                ctx.quit();
            }
            _ => {}
        }
        chase_nearest_coin(ctx);
    })
    .run_headless(5 * HZ, setup_playing);
    assert_fresh_round(&mut world, 2);

    let world = script(|ctx| match ctx.sim_tick() {
        5 => intent(ctx).pause = true,
        7 => {
            intent(ctx).back = true;
            ctx.quit();
        }
        _ => {}
    })
    .run_headless(20, setup_playing);
    assert_eq!(screen(&world), Screen::Title);
    assert!(world.is_empty(), "the field is cleared");
    assert_eq!(world.resource::<Round>().number, 1, "the round counter goes on");
}

#[test]
fn back_does_nothing_while_playing() {
    let world = script(|ctx| intent(ctx).back = true).run_headless(HZ, setup_playing);
    assert_eq!(screen(&world), Screen::Playing);
}

#[test]
fn result_screen_offers_play_again_and_back_to_the_title() {
    // Title -> play -> lose -> play again (round 2) -> lose -> back to title.
    let world = script(|ctx| {
        let over = matches!(*ctx.world.resource::<Screen>(), Screen::Result(_));
        let number = ctx.world.resource::<Round>().number;
        if over {
            if number == 1 {
                intent(ctx).restart = true; // the restart button works here too
            } else {
                intent(ctx).back = true;
                ctx.quit();
            }
        } else if screen(&ctx.world) == Screen::Title {
            intent(ctx).confirm = true;
        }
    })
    .run_headless(5 * ROUND_SECONDS as u64 * HZ, |ctx| setup_with(ctx, None));
    assert_eq!(screen(&world), Screen::Title);
    let round = world.resource::<Round>();
    assert_eq!(round.number, 2, "title, round 1, round 2");
    assert!(world.is_empty());
}

#[test]
fn pause_on_the_result_screen_goes_back_to_the_title() {
    let world = script(|ctx| {
        if matches!(*ctx.world.resource::<Screen>(), Screen::Result(_)) {
            intent(ctx).pause = true;
            ctx.quit();
        }
    })
    .run_headless(2 * ROUND_SECONDS as u64 * HZ, setup_playing);
    assert_eq!(screen(&world), Screen::Title);
}

#[test]
fn each_screen_shows_its_own_text() {
    let text = |world: &World| -> String {
        screen_lines(world)
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    };
    let title = open_game(None, 0);
    let text_title = text(&title);
    assert!(
        text_title.contains("COIN DASH") && text_title.contains("Enter: play"),
        "{text_title}"
    );
    assert!(text_title.contains("No rounds played yet"), "{text_title}");

    let playing = script(|_| {}).run_headless(1, setup_playing);
    assert!(
        screen_lines(&playing).is_empty(),
        "the HUD line is drawn by hud, nothing else"
    );

    let paused = script(|ctx| {
        if ctx.sim_tick() == 1 {
            intent(ctx).pause = true;
        }
    })
    .run_headless(3, setup_playing);
    assert!(text(&paused).contains("Paused"), "{}", text(&paused));

    let won = script(chase_nearest_coin).run_headless(ROUND_SECONDS as u64 * HZ, setup_playing);
    let text_won = text(&won);
    assert!(
        text_won.contains("You win!") && text_won.contains("Time "),
        "{text_won}"
    );
    assert!(text_won.contains("Enter: play again"), "{text_won}");

    let lost = script(|_| {}).run_headless(ROUND_SECONDS as u64 * HZ + 10, setup_playing);
    let text_lost = text(&lost);
    assert!(
        text_lost.contains("Time's up") && text_lost.contains(&format!("Coins 0/{COINS}")),
        "{text_lost}"
    );
}

// ---- Saved progress ---------------------------------------------------------

#[test]
fn progress_keeps_the_best_time_and_the_most_coins() {
    let mut p = Progress::default();
    // A loss with 3 coins: a high score but no time.
    assert!(p.record(Outcome::Lost, 3, ROUND_SECONDS));
    assert_eq!(
        (p.best_time, p.best_coins, p.rounds_played, p.rounds_won),
        (None, 3, 1, 0)
    );
    // A win in 14 s: best time, and best coins (8 > 3).
    assert!(p.record(Outcome::Won, COINS, 14.0));
    assert_eq!(
        (p.best_time, p.best_coins, p.rounds_played, p.rounds_won),
        (Some(14.0), COINS, 2, 1)
    );
    // A slower win, then a worse loss: no new best, counters still count.
    assert!(!p.record(Outcome::Won, COINS, 15.0));
    assert!(!p.record(Outcome::Lost, 1, ROUND_SECONDS));
    assert_eq!(
        (p.best_time, p.best_coins, p.rounds_played, p.rounds_won),
        (Some(14.0), COINS, 4, 2)
    );
    // A faster win beats the time.
    assert!(p.record(Outcome::Won, COINS, 9.5));
    assert_eq!(p.best_time, Some(9.5));
}

#[test]
fn a_finished_round_is_saved_and_the_next_run_starts_from_it() {
    let dir = temp_dir("roundtrip");
    let path = dir.join("save").join("progress.json"); // the folder doesn't exist yet
    let won = win_a_round(&path);
    let round = won.resource::<Round>().clone();
    let elapsed = ROUND_SECONDS - round.time_left;
    assert!(round.new_best, "the first win is a best");
    assert!(!won.resource::<SaveFile>().save_failed);

    // A new run (a new process, in effect) reads it back from the title screen.
    let again = open_game(Some(path.clone()), 0);
    let progress = again.resource::<Progress>().clone();
    assert_eq!(again.resource::<SaveFile>().load, LoadStatus::Loaded);
    assert_eq!(
        progress,
        Progress {
            version: SAVE_VERSION,
            best_time: Some(elapsed),
            best_coins: COINS,
            rounds_played: 1,
            rounds_won: 1
        }
    );
    let title: Vec<String> = screen_lines(&again).into_iter().map(|l| l.text).collect();
    assert!(
        title.iter().any(|t| t.contains(&format!("Best time {elapsed:.1} s"))),
        "{title:?}"
    );
    assert!(
        !path.with_extension("json.tmp").exists(),
        "no temporary file is left behind"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_second_round_adds_to_the_saved_progress_without_losing_a_better_best() {
    let dir = temp_dir("accumulate");
    let path = dir.join("progress.json");
    // A saved best that no scripted run can beat (0.5 s), from an earlier run.
    let earlier = Progress {
        version: SAVE_VERSION,
        best_time: Some(0.5),
        best_coins: COINS,
        rounds_played: 4,
        rounds_won: 3,
    };
    earlier.save(&path).unwrap();
    let won = win_a_round(&path);
    assert!(!won.resource::<Round>().new_best, "slower than 0.5 s");
    let (saved, status) = Progress::load(&path);
    assert_eq!(status, LoadStatus::Loaded);
    assert_eq!(
        saved,
        Progress {
            rounds_played: 5,
            rounds_won: 4,
            ..earlier
        }
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_missing_save_file_is_a_first_run_and_is_only_written_when_a_round_ends() {
    let dir = temp_dir("missing");
    let path = dir.join("progress.json");
    let world = open_game(Some(path.clone()), 5);
    assert_eq!(world.resource::<SaveFile>().load, LoadStatus::Fresh);
    assert_eq!(*world.resource::<Progress>(), Progress::default());
    assert!(
        !path.exists(),
        "opening the game and staying on the title writes nothing"
    );

    // Quitting a round halfway (back to the title) doesn't count either.
    let quit_midway = script(|ctx| match ctx.sim_tick() {
        0 => intent(ctx).confirm = true,
        30 => intent(ctx).pause = true,
        32 => intent(ctx).back = true,
        _ => {}
    })
    .run_headless(40, {
        let path = path.clone();
        move |ctx| setup_with(ctx, Some(path))
    });
    assert_eq!(screen(&quit_midway), Screen::Title);
    assert_eq!(quit_midway.resource::<Progress>().rounds_played, 0);
    assert!(!path.exists());

    win_a_round(&path);
    assert!(path.exists());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_damaged_save_file_is_replaced_by_fresh_progress_and_kept_as_a_backup() {
    let valid_but_impossible = [
        r#"{"version":1,"best_time":-3.0}"#,
        r#"{"version":1,"best_time":0.0}"#,
        r#"{"version":1,"best_time":999.0}"#,
        r#"{"version":1,"best_coins":99}"#,
        r#"{"version":1,"rounds_played":1,"rounds_won":5}"#,
        r#"{"version":7,"best_coins":2}"#, // from some other version of the game
        r#"{"best_coins":2}"#,             // no version
    ];
    let broken: [&[u8]; 6] = [
        b"",
        b"not json at all",
        b"{\"version\":1,\"best_coins\":",
        b"[]",
        b"{\"version\":1,\"best_coins\":\"many\"}",
        &[0xff, 0xfe, 0x00, 0x9f], // not even text
    ];
    let cases: Vec<Vec<u8>> = broken
        .iter()
        .map(|b| b.to_vec())
        .chain(valid_but_impossible.iter().map(|s| s.as_bytes().to_vec()))
        .collect();
    for (n, bytes) in cases.iter().enumerate() {
        let dir = temp_dir(&format!("damaged-{n}"));
        let path = dir.join("progress.json");
        std::fs::write(&path, bytes).unwrap();

        let world = open_game(Some(path.clone()), 0);
        let label = String::from_utf8_lossy(bytes).to_string();
        assert_eq!(world.resource::<SaveFile>().load, LoadStatus::Recovered, "{label:?}");
        assert_eq!(*world.resource::<Progress>(), Progress::default(), "{label:?}");
        assert_eq!(screen(&world), Screen::Title, "the game still starts: {label:?}");
        assert_eq!(
            std::fs::read(dir.join("progress.json.bad")).unwrap(),
            *bytes,
            "backup of {label:?}"
        );
        let title: Vec<String> = screen_lines(&world).into_iter().map(|l| l.text).collect();
        assert!(
            title.iter().any(|t| t.contains("unreadable")),
            "the player is told: {title:?}"
        );

        // And the game is playable and saves again: the next win replaces the file.
        win_a_round(&path);
        let (progress, status) = Progress::load(&path);
        assert_eq!(status, LoadStatus::Loaded, "{label:?}");
        assert_eq!((progress.rounds_played, progress.rounds_won), (1, 1), "{label:?}");
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn an_unreadable_save_location_does_not_stop_the_game() {
    // The "file" is a folder: it can't be read as a save, and can't be written.
    let dir = temp_dir("folder-as-file");
    let world = open_game(Some(dir.clone()), 0);
    assert_eq!(world.resource::<SaveFile>().load, LoadStatus::Recovered);
    assert_eq!(*world.resource::<Progress>(), Progress::default());

    // Losing a round tries to save, fails, says so, and the game carries on.
    let lost = script(|_| {}).run_headless(ROUND_SECONDS as u64 * HZ + 10, {
        let dir = dir.clone();
        move |ctx| {
            setup_with(ctx, Some(dir));
            new_round(ctx, 1);
        }
    });
    assert_eq!(screen(&lost), Screen::Result(Outcome::Lost));
    assert!(lost.resource::<SaveFile>().save_failed);
    let lines: Vec<String> = screen_lines(&lost).into_iter().map(|l| l.text).collect();
    assert!(lines.iter().any(|t| t.contains("could not be saved")), "{lines:?}");
    assert_eq!(lost.resource::<Progress>().rounds_played, 1, "still counted in memory");
    let _ = std::fs::remove_dir_all(dir);

    // A save path below a regular file can't be created either.
    let dir = temp_dir("below-a-file");
    let blocker = dir.join("blocker");
    std::fs::write(&blocker, b"x").unwrap();
    let won = win_a_round(&blocker.join("progress.json"));
    assert!(won.resource::<SaveFile>().save_failed);
    assert_eq!(screen(&won), Screen::Result(Outcome::Won));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_game_without_a_save_path_just_does_not_keep_progress() {
    let won = script(chase_nearest_coin).run_headless(ROUND_SECONDS as u64 * HZ, setup_playing);
    assert_eq!(won.resource::<SaveFile>().load, LoadStatus::Fresh);
    assert!(!won.resource::<SaveFile>().save_failed);
    assert_eq!(won.resource::<Progress>().rounds_won, 1);
}

// ---- Input ------------------------------------------------------------------

/// A scripted full session from raw key presses only: the real `read_input`
/// runs on every tick (as a sim system, since headless runs have no frames).
fn keyed(scenario: InputScenario, extra: impl FnMut(&mut Ctx) + 'static) -> Game {
    Game::new("coin dash keys", 800, 600)
        .seed(SEED)
        .input_scenario(scenario)
        .sim_system(read_input)
        .sim_system(extra)
        .sim_system(rules)
}

fn tap(scenario: InputScenario, tick: u64, key: Key) -> InputScenario {
    scenario
        .at(tick, InputEvent::Key(key, true))
        .at(tick + 1, InputEvent::Key(key, false))
}

#[test]
fn a_whole_session_can_be_played_from_key_presses() {
    let dir = temp_dir("session");
    let path = dir.join("progress.json");
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = seen.clone();

    // Enter starts; Esc pauses; P resumes; the player then chases to a win;
    // Space plays again; R restarts mid-round; Esc, Q returns to the title.
    let mut scenario = InputScenario::new();
    for (tick, key) in [
        (5, Key::Enter),
        (30, Key::Escape),
        (60, Key::P),
        (1500, Key::Space),
        (1600, Key::R),
        (1700, Key::Escape),
        (1710, Key::Q),
    ] {
        scenario = tap(scenario, tick, key);
    }
    let world = keyed(scenario, move |ctx| {
        // The "player's hands" for the chase: movement only while a round runs.
        if screen(&ctx.world) == Screen::Playing {
            let (player, coins) = player_and_coins(ctx);
            let nearest = coins
                .into_iter()
                .min_by(|a, b| a.distance(player).total_cmp(&b.distance(player)));
            // Same as holding the keys for the direction: written where read_input leaves it.
            intent(ctx).direction = nearest.map_or(Vec2::ZERO, |c| (c - player).normalize_or_zero());
        }
        log.borrow_mut().push((ctx.sim_tick(), screen(&ctx.world)));
    })
    .run_headless(1800, {
        let path = path.clone();
        move |ctx| setup_with(ctx, Some(path))
    });

    let at = |tick: u64| seen.borrow().iter().find(|(t, _)| *t == tick).unwrap().1;
    assert_eq!(at(4), Screen::Title);
    assert_eq!(at(10), Screen::Playing, "Enter started round 1");
    assert_eq!(at(40), Screen::Paused, "Escape paused");
    assert_eq!(at(70), Screen::Playing, "P resumed");
    assert_eq!(at(1400), Screen::Result(Outcome::Won), "round 1 won by chasing");
    assert_eq!(at(1510), Screen::Playing, "Space played again");
    assert_eq!(at(1650), Screen::Playing, "R restarted round 2 as round 3");
    assert_eq!(at(1705), Screen::Paused);
    assert_eq!(screen(&world), Screen::Title, "Q went back to the title");
    assert_eq!(world.resource::<Round>().number, 3);

    // The win was saved; the abandoned rounds were not counted.
    let (saved, status) = Progress::load(&path);
    assert_eq!(status, LoadStatus::Loaded);
    assert_eq!((saved.rounds_played, saved.rounds_won, saved.best_coins), (1, 1, COINS));
    assert_eq!(*world.resource::<Progress>(), saved);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn keys_and_gamepad_map_to_the_same_intent() {
    Game::new("t", 800, 600).run_headless(0, |ctx| {
        setup_playing(ctx);
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
fn every_button_binding_sets_its_own_request_which_the_sim_takes_once() {
    let pad = Gamepad::from_slot(0).unwrap();
    let key = |k| (InputEvent::Key(k, true), InputEvent::Key(k, false));
    let button = |b| {
        (
            InputEvent::GamepadButton(pad, b, true),
            InputEvent::GamepadButton(pad, b, false),
        )
    };
    type Flag = fn(&Intent) -> bool;
    type Press = (InputEvent, InputEvent); // press, release
    let table: [(&str, Flag, Vec<Press>); 4] = [
        (
            "confirm",
            |i| i.confirm,
            vec![key(Key::Enter), key(Key::Space), button(GamepadButton::South)],
        ),
        ("restart", |i| i.restart, vec![key(Key::R), button(GamepadButton::West)]),
        (
            "pause",
            |i| i.pause,
            vec![key(Key::Escape), key(Key::P), button(GamepadButton::Start)],
        ),
        ("back", |i| i.back, vec![key(Key::Q), button(GamepadButton::East)]),
    ];
    for (name, flag, presses) in table {
        for (press, release) in presses {
            // A fresh game per press: the engine keeps "pressed" until the frame ends.
            Game::new("t", 800, 600).run_headless(0, |ctx| {
                setup_with(ctx, None);
                assert!(!flag(ctx.world.resource::<Intent>()), "{name}: not pressed yet");
                ctx.feed_input(press);
                read_input(ctx);
                ctx.feed_input(release);
                // A frame without a sim tick must not lose it.
                assert!(flag(ctx.world.resource::<Intent>()), "{press:?} sets {name}");
                // Only that one button: the others are untouched.
                let i = *ctx.world.resource::<Intent>();
                let set = [i.confirm, i.restart, i.pause, i.back];
                assert_eq!(set.iter().filter(|s| **s).count(), 1, "{press:?}: {set:?}");
                rules(ctx);
                assert!(!flag(ctx.world.resource::<Intent>()), "{name}: taken exactly once");
            });
        }
    }
}

#[test]
fn every_control_name_is_a_binding() {
    // `Act` resolves names once; a name `bindings` doesn't define would
    // read false forever, so the two lists must agree.
    let map = bindings();
    for name in Act::NAMES {
        assert!(map.resolve(name).is_ok(), "`{name}` is not bound");
    }
}
