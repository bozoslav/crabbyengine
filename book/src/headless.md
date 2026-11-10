# Headless runs and testing

`run_headless(ticks)` after [`Game::setup`](getting-started.md#setup-on-the-builder),
or `Game::run_headless(ticks, setup)` in one call (C++: `ponko::game::run_headless`,
both forms), runs the simulation with no window and no
GPU, as fast as the CPU allows, then hands you the `World`.

Use it for:

- **tests** of game logic in plain `cargo test`, including on CI machines
  without a GPU;
- **dedicated servers** and batch runs;
- **benchmarks** of the simulation alone.

<div class="lang-pair">
<div>

```rust
use ponko::*;

struct Pos(f32);

fn fall(ctx: &mut Ctx) {
    let dt = ctx.dt();
    ctx.world.query::<&mut Pos>().for_each(|p| p.0 -= 10.0 * dt);
}

#[test]
fn things_fall_ten_units_per_second() {
    let mut world = Game::new("test", 800, 600)
        .sim_system(fall)
        .setup(|ctx| {
            ctx.world.spawn((Pos(100.0),));
        })
        .run_headless(60);
    let y = world.query::<&Pos>().iter().next().unwrap().0;
    assert!((y - 90.0).abs() < 1e-3);
}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:headless}}
```

</div>
</div>

## What runs

- The setup callbacks run once, in the order they were added, also when
  `ticks` is 0. If the `setup` passed to `run_headless(ticks, setup)`
  returns a frame closure, that closure is dropped.
- Each tick runs every **sim system** once. Frame systems and drawing don't
  run, because there are no frames.
- Pause and sim speed are ignored; `ctx.quit()` stops early.
- Textures load for their sizes but nothing is uploaded.

The same game can run windowed or headless: the boids example does both
(`HEADLESS=1 cargo run --release --example boids`), and
[`examples/headless.rs`](https://github.com/bozoslav/ponko/blob/main/examples/headless.rs)
simulates a million agents this way.

## A complete game, tested headlessly

[`examples/coin_dash`](https://github.com/ponkoengine/ponko/tree/main/examples/coin_dash)
is a small complete game (title, settings, pause and result screens, win, lose,
restart, saved progress, keyboard and gamepad; one picture file, everything else
made in code) built so that a test can play it. Packaging it, the settings and the
web build are in [A complete game, packaged](packaging.md). It has three layers,
each a [plugin](systems.md#plugins):

1. `Controls`: `read_input`, a frame system, turns the named
   [actions](actions.md) into an `Intent` resource (below).
2. `Rules`: `rules`, a sim system, is the whole game. It reads only the
   `Intent` and the world, never the keyboard.
3. `Drawing`: the built-in sprite drawing plus a `hud` frame system.

```rust
{{#include ../../examples/coin_dash/game.rs:plugins}}
```

`read_input` resolves the action names once, at startup, into `ActionId`s, so
a name that `bindings` doesn't define is logged once instead of reading false on
every frame:

```rust
{{#include ../../examples/coin_dash/game.rs:controls}}
```

```rust
{{#include ../../examples/coin_dash/game.rs:read_input}}
```

Because `rules` reads only the `Intent`, [`tests/coin_dash.rs`](https://github.com/ponkoengine/ponko/blob/main/tests/coin_dash.rs)
replaces step 1 with a scripted player: a sim system added before the game's
own, which writes the `Intent` each tick. The game's plugin is added
unchanged:

```rust
{{#include ../../tests/coin_dash.rs:script}}
```

```rust
{{#include ../../tests/coin_dash.rs:chase}}
```

A whole round then runs in a few milliseconds, with the same result every time
because the game uses the context's seeded random numbers (see
[Randomness, determinism and saving](determinism.md)):

```rust
{{#include ../../tests/coin_dash.rs:win}}
```

The same file tests a loss, restarts, reproducible layouts, and the input
mapping itself: `feed_input` presses keys and moves sticks, and the test calls
`read_input` and checks the `Intent` it produced.

## Builds without a window

A simulation server or a CI job that only runs headless doesn't need winit
and wgpu. Turn the `window` feature off and keep the rest:

<div class="lang-pair">
<div>

```toml
# Cargo.toml
[dependencies]
ponko = { version = "0.0.1", default-features = false, features = ["audio"] }
```

</div>
<div>

```sh
# Build the C library without a window
cargo build --release -p ponko_ffi \
  --no-default-features --features audio
```

</div>
</div>

That C build needs 168 crates instead of 360. It always keeps 3D models and
navigation, because the C API exports those functions in every build. The
Rust engine alone is smaller still: 72 crates with no default features, 143
with only `audio`, and 312 with the defaults. CI checks that these reduced
builds never pull in the renderer, an audio device, the inspector, glTF,
physics or navigation by accident (`scripts/feature-deps.sh`). Everything in
this chapter works the same. Drawing calls do nothing, and `Game::run`
(C: `ponko_game_run`) stops with a message that says to use `run_headless`.
