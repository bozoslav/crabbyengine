# Systems and the frame

A **system** is a function `fn(&mut Ctx)` (or a closure); in C++, anything
callable as `void(ponko::ctx&)`, with an optional name for timings and tools. You add systems to
the `Game` and they run in the order you added them.

<div class="lang-pair">
<div>

```rust
Game::new("Sim", 1280, 720)
    .sim_system(steer)      // fixed rate: game logic
    .sim_system(movement)
    .system(camera)         // once per frame: input, camera, UI
    .run(|ctx| { /* spawn things */ });
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:systems}}
```

</div>
</div>

## Two kinds of systems

| | `sim_system` | `system` |
|---|---|---|
| Runs | at the fixed rate, 0 or more times per frame | once per rendered frame |
| `ctx.dt()` | the fixed step (1/60 s by default) | real time since last frame |
| Use for | gameplay, physics, AI: anything that must behave the same at any FPS | input, camera, debug UI |
| In headless runs | runs | doesn't run |

Change the rate with `.fixed_hz(120)`. In C++, `sim_system(f, name, every)`
also takes the interval of `sim_system_every`.

## One frame, in order

1. Simulation systems, `ctx.fixed_steps()` times (each tick runs all of them once).
2. Frame systems.
3. The closure returned from `setup`, if any.
4. With `3d`, animator and particle updates.
5. Late systems (`Game::late_system`), then built-in draw systems such as `draw_sprites`.
6. Spatial audio update, then rendering.

Commands are applied after each system. The setup-returned closure is
appended to the frame-system list, so it runs before animator updates and
late systems. Headless execution runs simulation systems only.

## Parallel queries and future scheduling

Systems currently execute sequentially, taking exclusive `&mut Ctx` access.
A system can use the ECS's parallel query methods to divide its entity work
across threads; this is not automatic parallel execution of different systems.
Optional declared-access scheduling is planned in
[#137](https://github.com/ponkoengine/ponko/issues/137) and
[#138](https://github.com/ponkoengine/ponko/issues/138), after safety and
performance baselines. Event visibility, deferred commands and barriers must
be explicit ([#146](https://github.com/ponkoengine/ponko/issues/146)). Existing
unannotated `sim_system` callbacks must remain usable and exclusive.
See [the architecture](https://github.com/ponkoengine/ponko/blob/main/ARCHITECTURE.md)
for current subsystem boundaries.

## Systems with state

A system can be a closure that owns data. Build it with a function that
returns the closure:

<div class="lang-pair">
<div>

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

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:stateful}}
```

</div>
</div>

## Replacing built-ins

Everything the engine does by itself is an ordinary system, and
`remove_system` takes it out (in C++, by name):

<div class="lang-pair">
<div>

```rust
Game::new("t", 800, 600)
    .remove_system(draw_sprites)   // draw your own way
    .system(my_draw);
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:remove_system}}
```

</div>
</div>

## Timings

`ctx.system_timings()` lists every system with its last and average time in
milliseconds, so a game can show its own profiler overlay. For a full
timeline, build with `--features profiling` and open
[puffin_viewer](https://github.com/EmbarkStudios/puffin) on `127.0.0.1:8585`.
