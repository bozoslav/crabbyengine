# Headless runs and testing

`Game::run_headless(ticks, setup)` runs the simulation with no window and no
GPU, as fast as the CPU allows, then hands you the `World`.

Use it for:

- **tests** of game logic in plain `cargo test`, including on CI machines
  without a GPU;
- **dedicated servers** and batch runs;
- **benchmarks** of the simulation alone.

```rust
use crabbyengine::*;

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

## What runs

- `setup` runs once. If it returns a frame closure, that closure is dropped.
- Each tick runs every **sim system** once. Frame systems and drawing don't
  run, because there are no frames.
- Pause and sim speed are ignored; `ctx.quit()` stops early.
- Textures load for their sizes but nothing is uploaded.

The same game can run windowed or headless: the boids example does both
(`HEADLESS=1 cargo run --release --example boids`), and
[`examples/headless.rs`](https://github.com/bozoslav/crabbyengine/blob/main/examples/headless.rs)
simulates a million agents this way.
