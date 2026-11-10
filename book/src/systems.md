# Systems and the frame

A **system** is a function `fn(&mut Ctx)` (or a closure); in C++, anything
callable as `void(ponko::ctx&)`, with an optional name for timings and tools. You add systems to
the `Game`; each runs in a [phase](#phases), and systems in the same phase run
in the order you added them.

<div class="lang-pair">
<div>

```rust
# fn steer(_: &mut Ctx) {} fn movement(_: &mut Ctx) {} fn camera(_: &mut Ctx) {}
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

## Phases

Every system belongs to a `Phase`. Phases always run in this order, whatever
order the builder calls come in; within a phase, systems run in the order
you added them. `Game::add(phase, f)` (C++: `game::add(ponko::phase::..., f)`,
C: `ponko_game_add_system`) puts a system in any phase.

| Phase | Runs | Filled by |
|---|---|---|
| `Input` | each fixed tick, first | `spatial_grid`'s rebuild |
| `BeforePhysics` | each fixed tick | your systems that feed the physics step (forces, velocities, wheel inputs) |
| `Physics` | each fixed tick | `physics` / `physics3d`'s step |
| `AfterPhysics` | each fixed tick | `sim_system`, `sim_system_every` |
| `SimLast` | each fixed tick, last | |
| `Frame` | each rendered frame | `system`, the closure returned from setup |
| `Animate` | each rendered frame | |
| `Late` | each rendered frame | `late_system` |
| `Draw` | each rendered frame | built-in draw systems such as `draw_sprites` |

Headless runs (`run_headless`) run the fixed-tick phases only.

The physics step always runs in `Physics`, so a `sim_system` runs *after*
it even when you add it before calling `.physics()` or `.physics3d()`.
Logic that must affect the current tick's step goes in `BeforePhysics`:

```rust
# fn steer(_: &mut Ctx) {} fn score(_: &mut Ctx) {}
Game::new("Drive", 1280, 720)
    .add(Phase::BeforePhysics, steer) // the step below sees this tick's input
    .physics3d(vec3(0.0, -9.81, 0.0))
    .sim_system(score)                // sees the result of the step
    .run(|ctx| { /* spawn things */ });
```

Code written when the step ran where `.physics()` was called, with
simulation logic added before it, should move that logic to
`.add(Phase::BeforePhysics, logic)`; otherwise it acts one tick late.

## One frame, in order

1. Simulation systems, `ctx.time.fixed_steps()` times (each tick runs all of them
   once, in phase order: `Input`, `BeforePhysics`, `Physics`,
   `AfterPhysics`, `SimLast`).
2. Frame systems (`Phase::Frame`), then the closure returned from `setup`,
   if any.
3. `Phase::Animate` systems, then, with `3d`, the built-in animator and
   particle updates.
4. Late systems (`Phase::Late`, `Game::late_system`), then draw systems
   (`Phase::Draw`) such as the built-in `draw_sprites`.
5. Spatial audio update, then rendering.

## When changes become visible

Systems run one at a time, each with all of `ctx`. Between two systems
the engine applies the first one's commands, so **every system is a
barrier**: within a phase, across phases, and between ticks.

| What | Becomes visible |
|---|---|
| A change through `ctx.world` | immediately |
| A command in `ctx.commands` | when the recording system returns, before the next system; never to the recording system itself |
| Commands recorded in `setup` | before the first tick |
| A sent event | immediately, to every later `read` (also the sender's own) |
| An event, to a system earlier in the order | on that system's next run (the next tick for simulation systems) |
| Events from simulation ticks, to frame systems | in the same frame, all ticks of it |
| Events from frame systems, to simulation systems | on the next tick that runs |
| A change to a tracked component, to `Changed<T>`, `Added<T>` and `removed::<T>()` | to every other system, the next time it runs; never to the system that made it (see [Change detection](ecs.md#change-detection)) |

Commands apply in the order they were recorded: a closure from
`commands.add` sees the ones before it, and an `insert` after a `despawn` of
the same entity does nothing. Engine steps such as the physics step are
ordinary systems, so commands recorded in `Phase::BeforePhysics` are in the
world when physics runs. A system skipped by `sim_system_every` doesn't run
and records nothing. C and C++ systems follow the same rules: inside a
C query callback recorded commands are still pending, and parallel
queries refuse commands.

So the next system sees an event together with the commands its sender
recorded: a `Died(e)` sent with `commands.despawn(e)` arrives when `e` is
already gone.

Events are dropped at the start of a frame (each tick in headless runs) once
they are at least one whole frame old *and* older than the slowest
`sim_system_every` interval. Headless, with every system running every
tick, an event from tick `t` is kept through tick `t + 1`. While the
simulation is paused no ticks pass, so events sent by frame systems (a pause
menu's "restart") wait for the simulation readers.

`tests/schedule_barriers.rs` pins these rules.

### Writing systems that keep working

The flush after every system is the contract for ordinary systems, and it
stays that way. If parallel scheduling arrives
([#138](https://github.com/ponkoengine/ponko/issues/138)), systems that opt
in may get coarser barriers, documented there. To be ready:

- Depend on another system's commands or events through explicit order: put
  the reader in a later phase, rather than relying on registration order
  inside one phase.
- Read results of your own commands on your next run, not in the same call.
- Keep work that needs the world exactly as it is in one system, through
  `ctx.world` (immediate) or one `commands.add` closure.

## Parallel queries and future scheduling

Systems currently execute sequentially. A system can use the ECS's parallel
query methods to divide its entity work across threads; this is not
automatic parallel execution of different systems. Systems may declare what
they read and write, and view systems (`Game::add_view`) get only what they
declared; see [Declared system access](system-access.md). The parallel
scheduler that will use those declarations is planned in
[#138](https://github.com/ponkoengine/ponko/issues/138); the batch barrier it
must keep is documented in that chapter. Existing unannotated `sim_system`
callbacks remain usable and exclusive.
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
        if ctx.time.sim_tick() % every == 0 {
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
# fn my_draw(_: &mut Ctx) {}
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

`ctx.perf.system_timings()` lists every system with its last and average time in
milliseconds, so a game can show its own profiler overlay. For a full
timeline, build with `--features profiling` and open
[puffin_viewer](https://github.com/EmbarkStudios/puffin) on `127.0.0.1:8585`.
