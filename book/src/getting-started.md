# Getting started

You need a recent stable Rust (`rustup update stable`). On Linux, also the
headers for sound and gamepads: `sudo apt-get install libasound2-dev libudev-dev`
(Debian, Ubuntu).

## Run the examples

Clone the repository and run any example:

```sh
git clone https://github.com/ponkoengine/ponko
cd ponko
cargo run --release --example boids
```

| Example | What it shows |
|---|---|
| `window` | The smallest game: a window, input, one sprite |
| `sprites` | Drawing many sprites with tint, rotation and scale |
| `ecs` | Entities, components and systems, with sim controls |
| `bunnymark` / `bunnymark_ecs` | Sprite throughput, immediate mode vs ECS |
| `boids` | Flocking with a spatial grid on all cores |
| `ants` | Ant colonies: pheromone trails, foraging, growth; up to millions of ants |
| `headless` | A million agents with no window or GPU |
| `coin_dash` | A small complete game (title, settings with UI scale and rebinding, rounds, pause, saved progress, sound), tested headlessly, packaged for the desktop and built for the web; see [A complete game, packaged](packaging.md) and [Headless runs and testing](headless.md#a-complete-game-tested-headlessly) |
| `examples/cpp/boids.cpp` | The boids example in C++ (build with CMake, see [C and C++](c-cpp.md)) |

Always use `--release` for anything with many entities; debug builds are
10–50× slower.

## Your first game

Add the engine to a new project:

```sh
cargo new my_game && cd my_game
cargo add ponko
```

Then in `src/main.rs` (Rust) or `main.cpp` (C++, see [C and C++](c-cpp.md)
for building):

<div class="lang-pair">
<div>

```rust
use ponko::*;

fn main() {
    Game::new("My Game", 800, 600).run(|ctx| {
        let crab = ctx.load_texture("crab.png");
        let mut pos = vec2(400.0, 300.0);

        move |ctx: &mut Ctx| {
            if ctx.key_down(Key::Right) { pos.x += 200.0 * ctx.dt(); }
            if ctx.key_down(Key::Left)  { pos.x -= 200.0 * ctx.dt(); }
            if ctx.key_pressed(Key::Escape) { ctx.quit(); }
            ctx.draw(&crab, pos);
        }
    });
}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:first_game_full}}
```

</div>
</div>

`setup` runs once and returns a closure that runs every frame. In C++, the
per-frame code is a frame `system` and the state lives in variables it
captures. Variables you
create in `setup` and move into the closure are your game state. For small
games that's all you need; when things grow, move to
[entities and systems](ecs.md).

Everything comes in with `use ponko::*;`: `Game`, `Ctx`, `Vec2`,
`Color`, `Key`, the ECS types and the built-in components.

## Setup on the builder

Once a game has systems, its startup work goes on the builder too:
`Game::setup` takes the startup code, and `run()` takes nothing.

<div class="lang-pair">
<div>

```rust
# fn movement(_: &mut Ctx) {} fn spawn_enemies(_: &mut Ctx) {}
# let with_enemies = true;
let game = Game::new("My Game", 800, 600)
    .sim_system(movement)
    .setup(|ctx| {
        let crab = ctx.load_texture("crab.png");
        ctx.world.spawn((Transform2D::at(vec2(400.0, 300.0)), Sprite::new(crab)));
    });
// Startup chosen at run time: each call adds one more callback.
let game = if with_enemies { game.setup(spawn_enemies) } else { game };
game.run(); // or run_headless(ticks)
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:builder_setup}}
```

</div>
</div>

- Setup callbacks run once, after the window and the renderer exist and
  before any system, in the order they were added. A
  [plugin](systems.md#plugins)'s `setup` is in the same queue, at the place
  the plugin was added.
- Commands recorded in a callback are applied when it returns, so the next
  callback and the first system see what it spawned.
- Add systems and options first: after the first `setup`, the builder only
  takes more `setup` and `plugin` calls and the `run` methods.
- A game whose startup is all in its [plugins](systems.md#plugins) and
  systems has no `setup` to add: `.configured()` gives it the same
  `run()` and `run_headless(ticks)` (C++ already has `run()` and
  `run_headless(ticks)` on any game; in C, pass a null setup).
- Nothing is returned, so two branches can pick different startup code
  (above). The state a frame closure would have captured goes in a
  resource, or in a [system that owns its state](systems.md#systems-with-state).

`run(|ctx| ...)` and `run_headless(ticks, |ctx| ...)` stay for the small
cases: the one-closure program at the top of this page, and a headless run
whose setup borrows a local variable (a timer, a counter in a test), which a
stored `setup` callback can't do.

## When loading fails

`load_texture`, `load_mesh`, `load_sound`, `load_font` and the other
`load_*` / `create_*` calls are for setup code, where a missing file is a
bug: they panic with a message that says what's wrong and where the engine
looked. Each has a `try_` twin that returns `Result<_, ponko::Error>`
instead, for assets that may legitimately be missing (a mod, an optional
skin, a user's file):

```rust
# use ponko::*;
# fn setup(ctx: &mut Ctx) {
match ctx.gfx.try_load_texture("skins/gold.png") {
    Ok(skin) => { /* use it */ let _ = skin; }
    Err(e) if e.kind() == ErrorKind::NotFound => { /* no skin installed */ }
    Err(e) => eprintln!("{e}"), // "ponko: ..." with the reason
}
# }
```

Match on `e.kind()` (`NotFound`, `Io`, `Decode`, `Invalid`, `BadHandle`,
...), not on the message text, which may change. `ponko::Error` implements
`std::error::Error`, so `?` passes it on into a `Box<dyn Error>` or your own
error type. The C and C++ APIs keep their existing error reporting for now.
