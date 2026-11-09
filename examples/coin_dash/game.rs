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

// The test file includes this module but needs only part of it.
#![allow(dead_code)]

use ponko::*;

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

/// Where the round stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Playing,
    Won,
    Lost,
}

/// Round bookkeeping. One resource, so tests and tools can read it directly.
#[derive(Debug, Clone)]
pub struct Round {
    pub phase: Phase,
    pub collected: usize,
    pub time_left: f32,
    /// 1 for the first round, +1 for every restart.
    pub number: u32,
}

/// What the player asked for since the last simulation tick. The sim reads
/// this, never the keyboard, so a test can write it directly.
#[derive(Debug, Default, Clone, Copy)]
pub struct Intent {
    /// Movement, each axis in `-1..=1`; y is positive down.
    pub direction: Vec2,
    /// Restart requested. Latched until the sim takes it, so a tap between two
    /// ticks (a frame with no tick) is not lost.
    pub restart: bool,
}

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

/// Startup: the input bindings and the first round.
pub fn setup(ctx: &mut Ctx) {
    ctx.clear(Color::hex(0x14181f));
    ctx.set_actions(bindings());
    let disc = disc(ctx);
    ctx.world.insert_resource(Art { disc });
    ctx.world.insert_resource(Intent::default());
    new_round(ctx, 1);
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
    ctx.create_texture(SIZE, SIZE, &rgba)
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
    map.set(
        "restart",
        Action::Button(vec![
            ButtonBinding::Key(Key::R),
            ButtonBinding::Key(Key::Enter),
            ButtonBinding::Gamepad {
                pad,
                button: GamepadButton::South,
            },
        ]),
    );
    map
}

/// Frame system: named actions to [`Intent`]. Runs once per rendered frame.
pub fn read_input(ctx: &mut Ctx) {
    if ctx.key_pressed(Key::Escape) {
        ctx.quit();
    }
    let direction = ctx.action_axis2("move");
    let restart = ctx.action_pressed("restart");
    let intent = ctx.world.resource_mut::<Intent>();
    intent.direction = direction;
    intent.restart |= restart;
}

/// Starts round `number`: clears the field, then places the player in the
/// middle and the coins at random spots. The spots come from the context's
/// seeded random numbers, so a given seed and tick always give one layout.
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
        phase: Phase::Playing,
        collected: 0,
        time_left: ROUND_SECONDS,
        number,
    });
}

/// Sim system: the whole game, one fixed step at a time.
pub fn rules(ctx: &mut Ctx) {
    let Intent { direction, restart } = std::mem::take(ctx.world.resource_mut::<Intent>());
    // `take` cleared the latch; keep the held direction for the next tick.
    ctx.world.resource_mut::<Intent>().direction = direction;

    if restart {
        let next = ctx.world.resource::<Round>().number + 1;
        new_round(ctx, next);
        return;
    }
    if ctx.world.resource::<Round>().phase != Phase::Playing {
        return; // Won or lost: wait for a restart.
    }

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
        round.phase = Phase::Won;
    } else if round.time_left <= 0.0 {
        round.phase = Phase::Lost;
    }
}

/// Frame system: the text on top of the sprites. Read-only.
pub fn hud(ctx: &mut Ctx) {
    let round = ctx.world.resource::<Round>().clone();
    let status = format!(
        "Round {}   Coins {}/{}   Time {:.1}",
        round.number, round.collected, COINS, round.time_left
    );
    ctx.draw_text(&status, vec2(12.0, 8.0), 20.0, Color::WHITE);
    let banner = match round.phase {
        Phase::Playing => None,
        Phase::Won => Some(("You win!", Color::hex(0x8ac926))),
        Phase::Lost => Some(("Time's up", Color::hex(0xff595e))),
    };
    if let Some((text, color)) = banner {
        ctx.draw_text(text, vec2(ARENA.x / 2.0 - 90.0, ARENA.y / 2.0 - 40.0), 40.0, color);
        ctx.draw_text(
            "R or Enter: play again",
            vec2(ARENA.x / 2.0 - 100.0, ARENA.y / 2.0 + 10.0),
            20.0,
            Color::WHITE,
        );
    }
}
