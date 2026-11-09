//! The game: rules, input mapping and drawing. Shared with `tests/coin_dash.rs`.
//!
//! Three layers, each testable on its own:
//!
//! 1. `read_input` (frame system) turns keys and gamepad into an [`Intent`].
//! 2. `rules` (sim system) is the whole game: it reads only the `Intent` and
//!    the world, so a test can replace step 1 with a script and get the same
//!    result on every run.
//! 3. Drawing: entities with a `Sprite` are drawn by the engine's built-in
//!    `draw_sprites`; `hud` adds the text. Neither changes the game.
//!
//! The game moves through four [`Screen`]s: `Title` → `Playing` ⇄ `Paused`,
//! `Playing` → `Result` → `Playing` again (or back to `Title`). Only `rules`
//! changes the screen. The best time and most coins are kept in a small
//! [`Progress`] file that survives a missing or damaged file.

// The test file includes this module but needs only part of it.
#![allow(dead_code)]

use ponko::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Size of the playfield, in world units (the window starts the same size).
pub const ARENA: Vec2 = vec2(800.0, 600.0);
/// Coins on the field at the start of a round.
pub const COINS: usize = 8;
/// Seconds to collect them all.
pub const ROUND_SECONDS: f32 = 20.0;
/// Player speed, units per second.
pub const SPEED: f32 = 240.0;
pub const PLAYER_RADIUS: f32 = 14.0;
pub const COIN_RADIUS: f32 = 10.0;

/// The player. Position is the entity's [`Transform2D`].
pub struct Player;
/// A coin to collect.
pub struct Coin;

/// The two procedural pictures, made once at startup.
#[derive(Clone, Copy)]
pub struct Art {
    disc: Texture,
}

/// Which screen the game is on. A resource, so tests and tools can read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// The menu shown at startup: best results, "press Enter".
    Title,
    /// A round is running.
    Playing,
    /// A round is frozen. Nothing moves and the clock stops.
    Paused,
    /// The round ended; waiting for "play again" or "back to title".
    Result(Outcome),
}

/// How a round ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Won,
    Lost,
}

/// Round bookkeeping. One resource, so tests and tools can read it directly.
#[derive(Debug, Clone)]
pub struct Round {
    pub collected: usize,
    pub time_left: f32,
    /// 0 before the first round, then 1 for the first round, +1 for every new one.
    pub number: u32,
    /// The finished round beat a saved best (set when it ends).
    pub new_best: bool,
}

/// What the player asked for since the last simulation tick. The sim reads
/// this, never the keyboard, so a test can write it directly.
#[derive(Debug, Default, Clone, Copy)]
pub struct Intent {
    /// Movement, each axis in `-1..=1`; y is positive down.
    pub direction: Vec2,
    // The buttons are latched until the sim takes them, so a tap between two
    // ticks (a frame with no tick) is not lost.
    /// Start, resume, play again.
    pub confirm: bool,
    /// Start a new round right now.
    pub restart: bool,
    /// Pause or resume; on the title screen, quit.
    pub pause: bool,
    /// Back to the title screen; on the title screen, quit.
    pub back: bool,
}

// ANCHOR: controls
actions! {
    /// The action names from [`bindings`], resolved once at startup. Queries by
    /// `ActionId` are an array index, and a name that `bindings` doesn't define
    /// is logged once when resolved instead of silently reading false forever.
    pub struct Act {
        movement = "move",
        confirm = "confirm",
        restart = "restart",
        pause = "pause",
        back = "back",
    }
}
// ANCHOR_END: controls

/// Random seed for coin layouts: the same seed and the same inputs always give
/// the same game.
pub const SEED: u64 = 2024;

/// The game without a window, ready for `run` or `run_headless`.
pub fn build() -> Game {
    install(Game::new("Coin Dash", ARENA.x as u32, ARENA.y as u32).seed(SEED))
}

/// Adds the game's systems to `game`. A test passes a game that already has a
/// scripted sim system, which then runs before `rules` on every tick.
pub fn install(game: Game) -> Game {
    game.sim_system(rules).system(read_input).system(hud)
}

/// Startup: the bindings, the pictures, the saved progress (from the default
/// place, see [`save_path`]) and the title screen.
pub fn setup(ctx: &mut Ctx) {
    setup_with(ctx, save_path());
}

/// [`setup`] with an explicit save file (`None`: don't keep progress).
pub fn setup_with(ctx: &mut Ctx, save: Option<PathBuf>) {
    ctx.gfx.clear(Color::hex(0x14181f));
    ctx.input.set_actions(bindings());
    let disc = disc(ctx);
    ctx.world.insert_resource(Art { disc });
    ctx.world.insert_resource(Act::resolve(ctx));
    ctx.world.insert_resource(Intent::default());
    ctx.world.insert_resource(Round {
        collected: 0,
        time_left: ROUND_SECONDS,
        number: 0,
        new_best: false,
    });
    let (progress, load) = match &save {
        Some(path) => Progress::load(path),
        None => (Progress::default(), LoadStatus::Fresh),
    };
    ctx.world.insert_resource(progress);
    ctx.world.insert_resource(SaveFile {
        path: save,
        load,
        save_failed: false,
    });
    ctx.world.insert_resource(Screen::Title);
}

/// A white 32x32 disc, generated here so the game needs no image files. Sprites
/// tint and scale it into the player and the coins.
fn disc(ctx: &mut Ctx) -> Texture {
    const SIZE: u32 = 32;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let d = vec2(x as f32 + 0.5, y as f32 + 0.5).distance(Vec2::splat(SIZE as f32 / 2.0));
            let alpha = (SIZE as f32 / 2.0 - d).clamp(0.0, 1.0); // soft 1 px edge
            rgba.extend([255, 255, 255, (alpha * 255.0) as u8]);
        }
    }
    ctx.gfx.create_texture(SIZE, SIZE, &rgba)
}

/// A disc sprite with a radius in world units.
fn ball(art: Art, at: Vec2, radius: f32, color: Color) -> (Transform2D, Sprite) {
    let scale = Vec2::splat(radius / 16.0); // the disc is 16 units in radius
    (
        Transform2D::at(at).scaled(scale),
        Sprite::new(art.disc).with_color(color),
    )
}

/// Keyboard and gamepad, bound to the same names. Games would save this with
/// `ActionMap::to_json` to let players rebind.
pub fn bindings() -> ActionMap {
    let pad = Gamepad::from_slot(0).expect("slot 0 exists");
    let button = |key, pad_button| {
        vec![
            ButtonBinding::Key(key),
            ButtonBinding::Gamepad {
                pad,
                button: pad_button,
            },
        ]
    };
    let mut map = ActionMap::default();
    map.set(
        "move",
        Action::Axis2(vec![
            Axis2Binding::Keys {
                left: Key::A,
                right: Key::D,
                up: Key::W,
                down: Key::S,
            },
            Axis2Binding::Keys {
                left: Key::Left,
                right: Key::Right,
                up: Key::Up,
                down: Key::Down,
            },
            Axis2Binding::Gamepad {
                pad,
                stick: GamepadStick::Left,
                deadzone: 0.2,
            },
        ]),
    );
    let mut confirm = button(Key::Enter, GamepadButton::South);
    confirm.push(ButtonBinding::Key(Key::Space));
    map.set("confirm", Action::Button(confirm));
    map.set("restart", Action::Button(button(Key::R, GamepadButton::West)));
    let mut pause = button(Key::Escape, GamepadButton::Start);
    pause.push(ButtonBinding::Key(Key::P));
    map.set("pause", Action::Button(pause));
    map.set("back", Action::Button(button(Key::Q, GamepadButton::East)));
    map
}

// ANCHOR: read_input
/// Frame system: named actions to [`Intent`]. Runs once per rendered frame.
pub fn read_input(ctx: &mut Ctx) {
    let act = *ctx.world.resource::<Act>();
    let direction = ctx.input.action_axis2(act.movement);
    let confirm = ctx.input.action_pressed(act.confirm);
    let restart = ctx.input.action_pressed(act.restart);
    let pause = ctx.input.action_pressed(act.pause);
    let back = ctx.input.action_pressed(act.back);
    let intent = ctx.world.resource_mut::<Intent>();
    intent.direction = direction;
    intent.confirm |= confirm;
    intent.restart |= restart;
    intent.pause |= pause;
    intent.back |= back;
}
// ANCHOR_END: read_input

/// Starts the next round: clears the field, then places the player in the
/// middle and the coins at random spots. The spots come from the context's
/// seeded random numbers, so a given seed and round number always give one layout.
pub fn new_round(ctx: &mut Ctx, number: u32) {
    ctx.world.clear();
    let art = *ctx.world.resource::<Art>();
    let mut rng = ctx.random().stream(u64::from(number));
    let (transform, sprite) = ball(art, ARENA / 2.0, PLAYER_RADIUS, Color::hex(0x3a86ff));
    ctx.world.spawn((transform, sprite, Player));
    for _ in 0..COINS {
        let at = vec2(rng.range(40.0, ARENA.x - 40.0), rng.range(40.0, ARENA.y - 40.0));
        let (transform, sprite) = ball(art, at, COIN_RADIUS, Color::hex(0xffbe0b));
        ctx.world.spawn((transform, sprite, Coin));
    }
    ctx.world.insert_resource(Round {
        collected: 0,
        time_left: ROUND_SECONDS,
        number,
        new_best: false,
    });
    ctx.world.insert_resource(Screen::Playing);
}

/// Back to the title screen: the field is emptied, the saved progress stays.
fn to_title(ctx: &mut Ctx) {
    ctx.world.clear();
    ctx.world.insert_resource(Screen::Title);
}

/// Sim system: the whole game, one fixed step at a time. What a button does
/// depends on the screen:
///
/// | Screen | confirm | restart | pause | back |
/// |---|---|---|---|---|
/// | Title | play | | quit | quit |
/// | Playing | | new round | pause | |
/// | Paused | resume | new round | resume | title |
/// | Result | play again | play again | title | title |
pub fn rules(ctx: &mut Ctx) {
    let Intent {
        direction,
        confirm,
        restart,
        pause,
        back,
    } = std::mem::take(ctx.world.resource_mut::<Intent>());
    // `take` cleared the latches; keep the held direction for the next tick.
    ctx.world.resource_mut::<Intent>().direction = direction;

    let next_round = ctx.world.resource::<Round>().number + 1;
    match *ctx.world.resource::<Screen>() {
        Screen::Title => {
            if confirm {
                new_round(ctx, next_round);
            } else if pause || back {
                ctx.quit();
            }
        }
        Screen::Playing => {
            if pause {
                ctx.world.insert_resource(Screen::Paused);
            } else if restart {
                new_round(ctx, next_round);
            } else {
                play(ctx, direction);
            }
        }
        Screen::Paused => {
            if back {
                to_title(ctx);
            } else if restart {
                new_round(ctx, next_round);
            } else if pause || confirm {
                ctx.world.insert_resource(Screen::Playing);
            }
        }
        Screen::Result(_) => {
            if confirm || restart {
                new_round(ctx, next_round);
            } else if pause || back {
                to_title(ctx);
            }
        }
    }
}

/// One tick of a running round: move, pick up coins, run the clock, end the
/// round when it is won or the time is up.
fn play(ctx: &mut Ctx, direction: Vec2) {
    // Move, clamped to the arena. A diagonal is not faster than a straight line.
    let step = direction.clamp_length_max(1.0) * SPEED * ctx.dt();
    let mut player = Vec2::ZERO;
    for (t, _) in ctx.world.query::<(&mut Transform2D, &Player)>().iter() {
        t.position = (t.position + step).clamp(Vec2::ZERO, ARENA);
        player = t.position;
    }

    // Pick up the coins the player touches.
    let reach = PLAYER_RADIUS + COIN_RADIUS;
    let mut touched = Vec::new(); // empty Vecs don't allocate
    for (e, t, _) in ctx.world.query::<(Entity, &Transform2D, &Coin)>().iter() {
        if t.position.distance(player) <= reach {
            touched.push(e);
        }
    }
    for e in &touched {
        ctx.world.despawn(*e);
    }

    let dt = ctx.dt();
    let round = ctx.world.resource_mut::<Round>();
    round.collected += touched.len();
    round.time_left = (round.time_left - dt).max(0.0);
    if round.collected >= COINS {
        finish_round(ctx, Outcome::Won);
    } else if round.time_left <= 0.0 {
        finish_round(ctx, Outcome::Lost);
    }
}

/// Ends the round: records the result in the saved progress (and the file)
/// and shows the result screen.
fn finish_round(ctx: &mut Ctx, outcome: Outcome) {
    let (collected, elapsed) = {
        let round = ctx.world.resource::<Round>();
        (round.collected, ROUND_SECONDS - round.time_left)
    };
    let new_best = ctx.world.resource_mut::<Progress>().record(outcome, collected, elapsed);
    let progress = ctx.world.resource::<Progress>().clone();
    let save = ctx.world.resource_mut::<SaveFile>();
    save.save_failed = match &save.path {
        Some(path) => progress.save(path).is_err(),
        None => false,
    };
    ctx.world.resource_mut::<Round>().new_best = new_best;
    ctx.world.insert_resource(Screen::Result(outcome));
}

// ---- Saved progress ---------------------------------------------------------

/// Layout version of the save file. A file with another version is treated as
/// unreadable (and kept as a `.bad` copy), never half-understood.
pub const SAVE_VERSION: u32 = 1;

/// What survives between runs: the best results over all rounds.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    pub version: u32,
    /// Fastest win, in seconds into the round.
    pub best_time: Option<f32>,
    /// Most coins in one round, won or not (the high score).
    pub best_coins: usize,
    pub rounds_played: u32,
    pub rounds_won: u32,
}

/// How the saved progress was found at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStatus {
    /// There was no file (first run), or saving is off.
    Fresh,
    /// The file was read.
    Loaded,
    /// The file was damaged or from another version: the game started from
    /// zero and kept the old file next to it as `<name>.bad`.
    Recovered,
}

/// Where progress is kept and what happened to it. A resource.
#[derive(Debug, Clone)]
pub struct SaveFile {
    /// `None`: progress is not saved.
    pub path: Option<PathBuf>,
    pub load: LoadStatus,
    /// The last attempt to write failed (a read-only disk, say). The game goes on.
    pub save_failed: bool,
}

impl Progress {
    /// Reads `path`. A missing file is a first run; a file that can't be read,
    /// doesn't parse or holds impossible numbers is recovered from: zeroed
    /// progress, and the old bytes moved to `<name>.bad` for inspection.
    pub fn load(path: &Path) -> (Progress, LoadStatus) {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Progress::default(), LoadStatus::Fresh),
            Err(_) => return (Progress::default(), LoadStatus::Recovered),
        };
        match serde_json::from_slice::<Progress>(&bytes) {
            Ok(progress) if progress.is_valid() => (progress, LoadStatus::Loaded),
            _ => {
                let mut backup = path.as_os_str().to_owned();
                backup.push(".bad");
                let _ = std::fs::rename(path, backup); // best effort
                (Progress::default(), LoadStatus::Recovered)
            }
        }
    }

    /// Writes `path` (creating its folder) through a temporary file, so a
    /// crash in the middle leaves the old file, not half a new one.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut temp = path.as_os_str().to_owned();
        temp.push(".tmp");
        let json = Progress {
            version: SAVE_VERSION,
            ..self.clone()
        };
        std::fs::write(&temp, serde_json::to_vec_pretty(&json).map_err(std::io::Error::other)?)?;
        std::fs::rename(&temp, path)
    }

    /// Whether the numbers could have come from this game.
    fn is_valid(&self) -> bool {
        self.version == SAVE_VERSION
            && self.best_coins <= COINS
            && self.rounds_won <= self.rounds_played
            && self
                .best_time
                .is_none_or(|t| t.is_finite() && t > 0.0 && t <= ROUND_SECONDS)
    }

    /// Counts a finished round; true when it set a new best time or high score.
    pub fn record(&mut self, outcome: Outcome, collected: usize, elapsed: f32) -> bool {
        self.version = SAVE_VERSION;
        self.rounds_played += 1;
        let mut new_best = collected > self.best_coins;
        self.best_coins = self.best_coins.max(collected);
        if outcome == Outcome::Won {
            self.rounds_won += 1;
            // A win can't take longer than the round or no time at all.
            let elapsed = elapsed.clamp(f32::EPSILON, ROUND_SECONDS);
            if self.best_time.is_none_or(|best| elapsed < best) {
                self.best_time = Some(elapsed);
                new_best = true;
            }
        }
        new_best
    }
}

/// Where progress lives by default: the player's data folder, or the file named
/// by the `COIN_DASH_SAVE` environment variable (handy for tests and smoke
/// runs). `None` when there is no such place (the web build), so the game
/// just doesn't keep progress.
pub fn save_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("COIN_DASH_SAVE") {
        return Some(PathBuf::from(path));
    }
    let var = |name| std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from);
    let base = if cfg!(windows) {
        var("APPDATA")?
    } else if cfg!(target_os = "macos") {
        var("HOME")?.join("Library/Application Support")
    } else {
        var("XDG_DATA_HOME").or_else(|| var("HOME").map(|h| h.join(".local/share")))?
    };
    Some(base.join("coin-dash").join("progress.json"))
}

// ---- Drawing ----------------------------------------------------------------

/// One line of text on a screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub text: String,
    pub size: f32,
    pub color: Color,
}

fn line(text: impl Into<String>, size: f32, color: Color) -> Line {
    Line {
        text: text.into(),
        size,
        color,
    }
}

/// The text of the current screen, top to bottom. Pure, so a test can check
/// what the player would read without a window.
pub fn screen_lines(world: &World) -> Vec<Line> {
    let gold = Color::hex(0xffbe0b);
    let green = Color::hex(0x8ac926);
    let red = Color::hex(0xff595e);
    let progress = world.resource::<Progress>();
    let save = world.resource::<SaveFile>();
    let best = if progress.rounds_played == 0 {
        "No rounds played yet".to_string()
    } else {
        let time = progress.best_time.map_or("none".to_string(), |t| format!("{t:.1} s"));
        format!(
            "Best time {time}   Most coins {}/{COINS}   Won {} of {}",
            progress.best_coins, progress.rounds_won, progress.rounds_played
        )
    };
    let mut lines = match *world.resource::<Screen>() {
        Screen::Playing => return Vec::new(),
        Screen::Title => {
            let mut lines = vec![
                line("COIN DASH", 56.0, gold),
                line(
                    format!("Collect {COINS} coins in {ROUND_SECONDS:.0} seconds"),
                    20.0,
                    Color::WHITE,
                ),
                line(best, 20.0, Color::WHITE),
            ];
            if save.load == LoadStatus::Recovered {
                lines.push(line("Saved progress was unreadable and was reset", 20.0, red));
            }
            lines.push(line("Enter: play    Esc: quit", 20.0, Color::WHITE));
            lines
        }
        Screen::Paused => vec![
            line("Paused", 48.0, Color::WHITE),
            line("Esc: resume    R: restart    Q: title", 20.0, Color::WHITE),
        ],
        Screen::Result(outcome) => {
            let round = world.resource::<Round>();
            let mut lines = match outcome {
                Outcome::Won => vec![
                    line("You win!", 48.0, green),
                    line(
                        format!("Time {:.1} s", ROUND_SECONDS - round.time_left),
                        24.0,
                        Color::WHITE,
                    ),
                ],
                Outcome::Lost => vec![
                    line("Time's up", 48.0, red),
                    line(format!("Coins {}/{COINS}", round.collected), 24.0, Color::WHITE),
                ],
            };
            if round.new_best {
                lines.push(line("New best!", 24.0, gold));
            }
            lines.push(line(best, 20.0, Color::WHITE));
            lines.push(line("Enter: play again    Q: title", 20.0, Color::WHITE));
            lines
        }
    };
    if save.save_failed {
        lines.push(line("Progress could not be saved", 20.0, red));
    }
    lines
}

/// Frame system: the text on top of the sprites. Read-only.
pub fn hud(ctx: &mut Ctx) {
    if matches!(*ctx.world.resource::<Screen>(), Screen::Playing | Screen::Paused) {
        let round = ctx.world.resource::<Round>().clone();
        let status = format!(
            "Round {}   Coins {}/{}   Time {:.1}",
            round.number, round.collected, COINS, round.time_left
        );
        ctx.draw_text(&status, vec2(12.0, 8.0), 20.0, Color::WHITE);
    }
    // The block of lines, centred on the arena.
    let lines = screen_lines(&ctx.world);
    let height: f32 = lines.iter().map(|l| l.size * 1.5).sum();
    let mut y = (ARENA.y - height) / 2.0;
    for l in lines {
        let width = ctx.gfx.measure_text(&l.text, l.size).x;
        ctx.draw_text(&l.text, vec2((ARENA.x - width) / 2.0, y), l.size, l.color);
        y += l.size * 1.5;
    }
}
