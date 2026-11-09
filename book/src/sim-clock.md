# The simulation clock

Simulation systems run on their own clock, separate from the frame rate. The
game can pause, step and fast-forward the sim while rendering and input keep
running normally.

<div class="lang-pair">
<div>

```rust
# fn ants(_: &mut Ctx) {}
fn controls(ctx: &mut Ctx) {
    if ctx.key_pressed(Key::Space) {
        if ctx.time.is_paused() { ctx.time.resume() } else { ctx.time.pause() }
    }
    if ctx.key_pressed(Key::Period) {
        ctx.time.step();              // one tick while paused
    }
    if ctx.key_pressed(Key::Num4) {
        ctx.time.set_speed(64.0); // fast-forward
    }
}

Game::new("Colony", 1280, 720).sim_system(ants).system(controls);
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:controls}}
```

</div>
</div>

Put controls in a frame `system`: frame systems keep running while the sim is
paused.

## What the clock guarantees

- **Ticks are always the same length.** `ctx.dt()` in a sim system is the
  fixed step (`1 / fixed_hz`), at every speed. Speed only changes how many
  ticks run per frame, so a run at 64× gives the same result as one at 1×.
- **The game stays responsive.** At most 64 ticks run per frame. If the sim
  can't keep up, it runs as fast as it can instead of freezing the window.
- `ctx.time.sim_tick()` counts ticks; `ctx.time.sim_time()` is simulated seconds as an
  `f64` (no drift after hours of simulated time).

## Doing work less often

Not everything needs to run 60 times a second. There are two tools, and they
combine.

**A whole system at a lower rate:**

<div class="lang-pair">
<div>

```rust
# fn movement(_: &mut Ctx) {} fn plan_routes(_: &mut Ctx) {} fn grow_food(_: &mut Ctx) {}
Game::new("Colony", 1280, 720)
    .sim_system(movement)               // every tick
    .sim_system_every(10, plan_routes)  // 6 times a second
    .sim_system_every(10, grow_food)    // also 6/s, but on other ticks
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:every}}
```

</div>
</div>

Inside `plan_routes`, `ctx.dt()` is 10 ticks' worth of time, so rates stay
right. Systems with the same interval are spread over different ticks, so
their cost doesn't pile up on one frame.

**Entities taking turns:** each tick, only 1/n of the entities do the
expensive part.

<div class="lang-pair">
<div>

```rust
# struct Brain;
# impl Brain { fn decide(&mut self, _dt: f32) {} }
fn think(ctx: &mut Ctx) {
    let turn = ctx.time.turns(8); // each ant thinks every 8th tick
    ctx.world.query::<(Entity, &mut Brain)>().par_for_each(|(e, brain)| {
        if turn.includes(e) {
            brain.decide(turn.dt()); // time since this ant's last turn
        }
    });
}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:turns}}
```

</div>
</div>

This keeps the per-tick cost flat instead of spiking every 8th tick, and
works in `par_for_each` (C++: `each_par`).

## Smooth motion between ticks

If the sim rate is lower than the frame rate, `ctx.time.fixed_alpha()` (0..1)
tells a frame system how far it is between the last tick and the next, for
interpolating positions when drawing.
