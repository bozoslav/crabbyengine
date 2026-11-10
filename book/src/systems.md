# Systems and the frame

A **system** is a function `fn(&mut Ctx)` (or a closure); in C++, anything
callable as `void(ponko::ctx&)`, with an optional name for timings and tools.
A game is built from two things:

- **[Phases](#phases)** say *when* a system runs. Every system is in one
  phase; the phases run in a fixed order, and systems in the same phase run
  in the order you added them.
- **[Plugins](#plugins)** say *which* systems belong together. A plugin is
  one area of the game (the player, the HUD, the shop): its systems and its
  startup work, added with one call.

The engine's own work (physics, animation, drawing) is made of
[ordinary systems](#built-in-systems) in the same phases, so you can see,
time, remove and replace every step of the frame.

<div class="lang-pair">
<div>

```rust
# fn steer(_: &mut Ctx) {} fn movement(_: &mut Ctx) {} fn camera(_: &mut Ctx) {}
Game::new("Sim", 1280, 720)
    .sim_system(steer)      // fixed rate: game logic
    .sim_system(movement)
    .system(camera)         // once per frame: input, camera, UI
    .setup(|ctx| { /* spawn things */ })
    .run();
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
C: `ponko_game_add_system`) puts a system in any phase; `sim_system`,
`system` and `late_system` are short for the three most common ones.

| Phase | Runs | Your systems | The engine's systems |
|---|---|---|---|
| `Input` | each fixed tick, first | | the spatial grid's rebuild |
| `BeforePhysics` | each fixed tick | what feeds the physics step: forces, velocities, wheel inputs | |
| `Physics` | each fixed tick | | the `physics` / `physics3d` step |
| `AfterPhysics` | each fixed tick | `sim_system`, `sim_system_every` | |
| `SimLast` | each fixed tick, last | bookkeeping that must see the whole tick | |
| `Frame` | each rendered frame | `system`, the closure returned from `run`'s setup | interpolated physics transforms |
| `Animate` | each rendered frame | pose work, before the engine's | animators, then particles (`3d`) |
| `Late` | each rendered frame | `late_system`: cameras and anything that follows a final pose | |
| `Draw` | each rendered frame | your own drawing | sprite animation, tilemaps, sprites; positional audio last (`3d`) |

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
    .setup(|ctx| { /* spawn things */ })
    .run();
```

Code written when the step ran where `.physics()` was called, with
simulation logic added before it, should move that logic to
`.add(Phase::BeforePhysics, logic)`; otherwise it acts one tick late.

The animators, the particles and positional audio stay at the **end** of
their phase: a system you add to `Animate` runs before the engine poses
the skeletons, and one you add to `Late` or `Draw` runs before sounds are
placed, no matter when you add it. To run *after* the engine's pose, use the next
phase (`Late` for a camera that follows a bone).

## One frame, in order

1. The fixed-tick phases, `ctx.time.fixed_steps()` times: each tick runs
   `Input`, `BeforePhysics`, `Physics`, `AfterPhysics`, `SimLast`.
2. `Frame`.
3. `Animate`.
4. `Late`.
5. `Draw`.
6. The renderer draws what the frame queued.

There is nothing else: every step between the ticks and the renderer is a
system in one of these phases, and [`ctx.perf.system_timings()`](#timings) lists
all of them.

## Built-in systems

The functions are in `ponko::systems`; the ids in `Systems` name them for
`Game::remove` and `Game::system_count`.

| Function in `ponko::systems` | Id | Added by | Phase |
|---|---|---|---|
| `rebuild_spatial_grid` | `Systems::REBUILD_SPATIAL_GRID` | `Game::spatial_grid` | `Input` |
| `step_physics` | `Systems::PHYSICS` | `Game::physics` | `Physics` |
| `step_physics3d` | `Systems::PHYSICS3D` | `Game::physics3d` | `Physics` |
| `interpolate_physics3d` | `Systems::INTERPOLATE_PHYSICS3D` | `Game::physics3d` | `Frame` |
| `update_animators` | `Systems::UPDATE_ANIMATORS` | always, with `3d` | end of `Animate` |
| `update_particles` | `Systems::UPDATE_PARTICLES` | always, with `3d` | end of `Animate` |
| `update_audio3d` | `Systems::UPDATE_AUDIO3D` | always, with `3d` | end of `Draw` |
| `animate_sprites` | `Systems::ANIMATE_SPRITES` | always | `Draw` |
| `draw_tilemaps` | `Systems::DRAW_TILEMAPS` | always | `Draw` |
| `draw_sprites` | `Systems::DRAW_SPRITES` | always | `Draw` |
| `follow_attachments` | `Systems::FOLLOW_ATTACHMENTS` | you | where you add it |
| `step_navmesh3d` | `Systems::STEP_NAVMESH3D` | you | where you add it |
| `apply_net_poses` | | you | where you add it |

In C the ids are `PONKO_SYSTEM_DRAW_SPRITES` and so on; in C++,
`ponko::systems::draw_sprites`.

Before these functions had their own module they were at the crate root.
Those spellings still work: they are deprecated re-exports of the same
functions, so a system added under the old name is the same system, with the
same id ([Naming and compatibility](naming.md) lists them).

### Replacing a built-in

`Game::remove` takes a built-in out by its id. Add your own system, or the
same function in another place:

<div class="lang-pair">
<div>

```rust
# fn my_draw(_: &mut Ctx) {}
Game::new("t", 800, 600)
    .remove(Systems::DRAW_SPRITES)   // draw your own way
    .add(Phase::Draw, my_draw);

// Tilemaps over the sprites instead of under them.
Game::new("t", 800, 600)
    .remove(Systems::DRAW_TILEMAPS)
    .add(Phase::Draw, systems::draw_tilemaps);
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:remove_system}}
```

</div>
</div>

Removing an id that isn't installed does nothing, so a plugin can remove a
built-in without knowing whether the game turned that feature on.

Your own systems have ids too: `Game::add_system` returns one (C++:
`game::add_system`; C: `ponko_game_add_system_id`). Two systems made from
the same function get different ids, so each can be removed by itself:

```rust
# fn debug_overlay(_: &mut Ctx) {}
let mut game = Game::new("t", 800, 600);
let overlay = game.add_system(Phase::Draw, debug_overlay);
// later, while still building the game:
let game = game.remove(overlay);
# let _ = game;
```

`remove_system(function)` and `remove_named_system(name)` are the older
forms; they still work and are deprecated, because a function or a name
can't tell two registrations apart.

## Plugins

A `Plugin` is one module of a game: `build` adds its systems (each in the
phase it needs) and `setup` does its startup work once the game is
running. `Game::plugin` adds it:

<div class="lang-pair">
<div>

```rust
# #[derive(Default)] struct Score(u32);
struct Player { speed: f32 }

impl Plugin for Player {
    fn build(&self, game: Game) -> Game {
        let speed = self.speed;
        game.add(Phase::BeforePhysics, move |ctx| {
            let dt = ctx.dt();
            ctx.world.query::<&mut ponko::d2::Transform>().for_each(|t| t.position.x += speed * dt);
        })
    }
    fn setup(&self, ctx: &mut Ctx) {
        ctx.world.spawn((ponko::d2::Transform::default(),));
    }
}

struct Hud;

impl Plugin for Hud {
    fn build(&self, game: Game) -> Game {
        game.add(Phase::Draw, |ctx| {
            let score = ctx.world.resource::<Score>().0;
            ctx.draw_text(&format!("{score}"), vec2(16.0, 16.0), 24.0, Color::WHITE);
        })
    }
    fn setup(&self, ctx: &mut Ctx) {
        ctx.world.insert_resource(Score::default());
    }
}

Game::new("Modules", 1280, 720)
    .plugin(Player { speed: 4.0 })
    .plugin(Hud)
    .configured() // startup is in the plugins: no setup of its own
    .run();
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:plugin}}

{{#include ../../examples/cpp/guide.cpp:plugin_use}}
```

</div>
</div>

In C++ a plugin is any object with `build(ponko::game&)`; `setup(ponko::ctx&)`
is optional. In C, `ponko_game_plugin(game, build, setup, user)` takes the
two callbacks.

What to rely on:

- **`build` runs at once**, inside the `plugin` call. Everything it adds
  names its phase, so the order of the `plugin` calls only matters between
  two systems in the same phase.
- **`setup` runs once**, when the game starts and the window and the
  renderer exist (headless: before the first tick). Plugin setups and
  [`Game::setup`](getting-started.md#setup-on-the-builder) callbacks run in the order they were
  added; the setup passed to `run` comes last.
- **Commands recorded in any setup** are applied before the first system
  runs, so one plugin's systems can count on what another's setup spawned.
- A plugin is plain data: there is no registry, no dependency list and
  nothing to enable. A plugin that needs another one says so in its docs,
  or adds it itself in `build` with `game.plugin(Other)`.

A game built from several files usually ends up as one plugin per file
and a `main` that lists them; `examples/coin_dash` is built this way.

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
            ctx.commands.spawn((ponko::d2::Transform::default(),));
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

## Timings

`ctx.perf.system_timings()` lists every system with its last and average time in
milliseconds, so a game can show its own profiler overlay. The engine's
systems are in the list under their names (`ponko::systems::update_animators`,
`ponko::ecs::draw_sprites`), next to yours, in the order they run. For a full
timeline, build with `--features profiling` and open
[puffin_viewer](https://github.com/EmbarkStudios/puffin) on `127.0.0.1:8585`.
