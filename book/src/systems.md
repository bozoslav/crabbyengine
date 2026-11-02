# Systems and the frame

A **system** is a function `fn(&mut Ctx)` (or a closure). You add systems to
the `Game` and they run in the order you added them.

```rust
Game::new("Sim", 1280, 720)
    .sim_system(steer)      // fixed rate: game logic
    .sim_system(movement)
    .system(camera)         // once per frame: input, camera, UI
    .run(|ctx| { /* spawn things */ });
```

## Two kinds of systems

| | `sim_system` | `system` |
|---|---|---|
| Runs | at the fixed rate, 0 or more times per frame | once per rendered frame |
| `ctx.dt()` | the fixed step (1/60 s by default) | real time since last frame |
| Use for | gameplay, physics, AI: anything that must behave the same at any FPS | input, camera, debug UI |
| In headless runs | runs | doesn't run |

Change the rate with `.fixed_hz(120)`.

## One frame, in order

1. Simulation systems, `ctx.fixed_steps()` times (each tick runs all of them once).
2. Frame systems.
3. The closure returned from `setup`, if any.
4. Draw systems: the built-in `draw_sprites`.
5. Rendering.

Commands are applied after each system.

## Systems with state

A system can be a closure that owns data. Build it with a function that
returns the closure:

```rust
fn spawner(every: u64) -> impl FnMut(&mut Ctx) {
    let mut spawned = 0;
    move |ctx| {
        if ctx.sim_tick() % every == 0 {
            ctx.commands.spawn((Transform2D::default(),));
            spawned += 1;
        }
    }
}

Game::new("t", 800, 600).sim_system(spawner(30));
```

## Replacing built-ins

Everything the engine does by itself is an ordinary system, and
`remove_system` takes it out:

```rust
Game::new("t", 800, 600)
    .remove_system(draw_sprites)   // draw your own way
    .system(my_draw);
```

## Timings

`ctx.system_timings()` lists every system with its last and average time in
milliseconds, so a game can show its own profiler overlay. For a full
timeline, build with `--features profiling` and open
[puffin_viewer](https://github.com/EmbarkStudios/puffin) on `127.0.0.1:8585`.
