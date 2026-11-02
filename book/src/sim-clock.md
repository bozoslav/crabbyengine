# The simulation clock

Simulation systems run on their own clock, separate from the frame rate. The
game can pause, step and fast-forward the sim while rendering and input keep
running normally.

```rust
fn controls(ctx: &mut Ctx) {
    if ctx.key_pressed(Key::Space) {
        if ctx.is_paused() { ctx.resume() } else { ctx.pause() }
    }
    if ctx.key_pressed(Key::Period) {
        ctx.step();              // one tick while paused
    }
    if ctx.key_pressed(Key::Num4) {
        ctx.set_sim_speed(64.0); // fast-forward
    }
}

Game::new("Colony", 1280, 720).sim_system(ants).system(controls);
```

Put controls in a frame `system`: frame systems keep running while the sim is
paused.

## What the clock guarantees

- **Ticks are always the same length.** `ctx.dt()` in a sim system is the
  fixed step (`1 / fixed_hz`), at every speed. Speed only changes how many
  ticks run per frame, so a run at 64× gives the same result as one at 1×.
- **The game stays responsive.** At most 64 ticks run per frame. If the sim
  can't keep up, it runs as fast as it can instead of freezing the window.
- `ctx.sim_tick()` counts ticks; `ctx.sim_time()` is simulated seconds as an
  `f64` (no drift after hours of simulated time).

## Doing work less often

Not everything needs to run 60 times a second:

```rust
fn slow_ai(ctx: &mut Ctx) {
    if ctx.sim_tick() % 10 != 0 {
        return; // think 6 times per second
    }
    // ...
}
```

## Smooth motion between ticks

If the sim rate is lower than the frame rate, `ctx.fixed_alpha()` (0..1)
tells a frame system how far it is between the last tick and the next, for
interpolating positions when drawing.
