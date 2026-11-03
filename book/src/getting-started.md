# Getting started

You need a recent stable Rust (`rustup update stable`). On Linux, also the
headers for sound and gamepads: `sudo apt-get install libasound2-dev libudev-dev`
(Debian, Ubuntu).

## Run the examples

Clone the repository and run any example:

```sh
git clone https://github.com/bozoslav/crabbyengine
cd crabbyengine
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
| `examples/cpp/boids.cpp` | The boids example in C++ (build with CMake, see [C and C++](c-cpp.md)) |

Always use `--release` for anything with many entities; debug builds are
10–50× slower.

## Your first game

Add the engine to a new project:

```sh
cargo new my_game && cd my_game
cargo add crabbyengine --git https://github.com/bozoslav/crabbyengine
```

Then in `src/main.rs` (Rust) or `main.cpp` (C++, see [C and C++](c-cpp.md)
for building):

<div class="lang-pair">
<div>

```rust
use crabbyengine::*;

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

Everything comes in with `use crabbyengine::*;`: `Game`, `Ctx`, `Vec2`,
`Color`, `Key`, the ECS types and the built-in components.
