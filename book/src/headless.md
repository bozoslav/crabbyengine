# Headless runs and testing

`Game::run_headless(ticks, setup)` (C++: `ponko::game::run_headless`) runs the simulation with no window and no
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
        .run_headless(60, |ctx| {
            ctx.world.spawn((Pos(100.0),));
        });
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

- `setup` runs once. If it returns a frame closure, that closure is dropped.
- Each tick runs every **sim system** once. Frame systems and drawing don't
  run, because there are no frames.
- Pause and sim speed are ignored; `ctx.quit()` stops early.
- Textures load for their sizes but nothing is uploaded.

The same game can run windowed or headless: the boids example does both
(`HEADLESS=1 cargo run --release --example boids`), and
[`examples/headless.rs`](https://github.com/bozoslav/ponko/blob/main/examples/headless.rs)
simulates a million agents this way.

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

That build has 82 crates instead of 142, builds from clean in 7 s instead of
15 s (release, M-series Mac) and leaves half the build output. Everything in
this chapter works the same. Drawing calls do nothing, and `Game::run`
(C: `ponko_game_run`) stops with a message that says to use `run_headless`.
