# Performance and benchmarks

Numbers from a MacBook Pro (M4 Max), release builds. Reproduce them yourself;
every number here has a command next to it.

## ECS iteration

`cargo run --release -p crabby_bench`

| Benchmark (1M entities) | Crabby | bevy_ecs | hecs |
|---|---|---|---|
| iterate `(&mut Pos, &Vel)` | 0.15 ms | 0.36 ms | 1.77 ms |

`par_for_each` on a heavy per-entity workload, 10M entities: 8.1 ms on all
cores vs 57.6 ms on one.

## Headless simulation

`AGENTS=1000000 cargo run --release --example headless`

| Agents | ms per tick |
|---|---|
| 1M | 1.0 |
| 10M | 8.7 |

## Boids (neighbor search)

`HEADLESS=1 BOIDS=<n> cargo run --release --example boids`

| Boids | ms per tick |
|---|---|
| 20k | 0.9 |
| 100k | 3.7 |
| 1M | ~33 |

## Ant colonies (whole game loop)

`HEADLESS=1 ANTS=<n> cargo run --release --example ants`: steering with
pheromone sensing, trail laying, foraging, colony growth.

| Ants | ms per tick |
|---|---|
| 5k | 0.22 |
| 50k | 1.2 |
| 1M | 15 |

## Flow fields (pathfinding)

`cargo run --release -p crabby_bench -- flow` for the field itself;
`HEADLESS=1 AGENTS=<n> cargo run --release --example flow` for units
following it around walls and mud, with a new goal every 2 seconds.

| What | Time |
|---|---|
| `compute`, 256 × 256 cells | 1.25 ms |
| `compute`, 1024 × 1024 cells | 13.0 ms |
| 1M `direction_at` lookups, parallel | 0.29 ms |
| 20k units, per tick | 0.14 ms |
| 1M units, per tick | 3.2 ms |

## Battle (the flagship demo)

`HEADLESS=1 UNITS=<n> TICKS=1200 cargo run --release --example battle`:
two armies march along flow fields, keep their spacing, find enemies with
per-team spatial grids, fight, and rebuild from their bases. The map grows
with the army, so the crowd density stays the same.

| Units | Map (cells) | ms per tick |
|---|---|---|
| 20k | 400 × 225 | 0.57 |
| 200k | 1264 × 711 | 4.4 |
| 1M | 2828 × 1590 | 20.8 |

With a window, 20k units run at the display's 120 FPS with 1.2 ms of
simulation per frame.

How it stays fast, in case your game needs the same tricks:
- Marching units look for enemies only on their turn (`ctx.turns(4)`),
  units in a fight every tick.
- Separation uses a fine grid of friends, every other tick; enemy search
  a coarse grid of enemies, at melee reach first and full sight only if
  nothing is in reach.
- The four grids are rebuilt at the same time (`rayon::scope`).
- Once a second, units are sorted by `spatial_key`, so neighbors on the
  map are neighbors in memory.

## Events and attachments

`cargo run --release -p crabby_bench -- events` and `-- attach`:

| What | Per tick |
|---|---|
| 100k [events](events.md) sent, read by another system and dropped | 0.42 ms (4.2 ns per event) |
| 100k [attachments](attachments.md) following 100k moving entities | 0.81 ms |

Both allocate nothing once running: events reuse their buffers, and
attachments are placed in parallel into scratch kept between ticks.

## Rendering a million sprites

`CRABS=1000000 SCALE=<s> cargo run --release --example bunnymark_ecs`:
1M bouncing crabs, drawn by the built-in `draw_sprites`. The example prints
the render stats described below.

| Crab size on screen | FPS | render CPU | GPU |
|---|---|---|---|
| 1.6 px (`SCALE=0.05`) | 120 (display cap) | 3.3 ms | 2.4 ms |
| 8 px (`SCALE=0.25`) | 99 | 3.3 ms | 8.6 ms |
| 32 px (`SCALE=1`) | 15 | 2.8 ms | 71 ms |

The 32 px row is limited by the GPU filling pixels: on a Retina screen a
million 32 px crabs are about 4 billion blended pixels per frame. Fewer or
smaller sprites on screen is the only fix for that; the cost per sprite is
small (about 1.6 ms of GPU time per million).

What keeps the CPU side small:
- `draw_sprites` builds the instance data on all cores (1M sprites: 1.1 ms,
  was 3.4 ms on one core).
- Each sprite is 36 bytes on the GPU (was 52): the per-frame upload for 1M
  sprites is 36 MB. (A 28-byte version without a texture region measured
  the same render CPU time; the region is needed for text and atlases.)

### Is it the CPU or the GPU?

`ctx.render_stats()` tells you what drawing the last frame cost:

```rust
# use crabbyengine::*;
fn report(ctx: &mut Ctx) {
    let r = ctx.render_stats();
    println!(
        "{} sprites in {} draw calls: {:.2} ms CPU, GPU {:?} ms",
        r.sprites, r.draw_calls, r.cpu_ms, r.gpu_ms
    );
}
```

`gpu_ms` is measured on the GPU with timestamp queries, so the frame rate
the window shows (which the compositor can cap or smooth) can't fool it.
If `gpu_ms` is close to the frame time, the GPU is the limit: draw fewer
or smaller sprites. If `cpu_ms` plus your systems' time
(`ctx.system_timings()`) is close to it, the CPU is. Agents
get the same numbers from the control endpoint's `info` method.

## Memory

Speed is half of scale; the other half is what each entity costs. The
headless example prints it: 1M agents with `Transform2D`, `Sprite` and a
small `Agent` component take **76 bytes per entity** allocated, spare
capacity included.

From code, `world.allocated_bytes()` is the total for all component data,
and `Archetype::bytes_per_entity()` is what one more entity of that exact
component set costs. Agents get both from the control endpoint's
`world.summary` (the MCP `world_summary` tool).

## Rules the engine follows

- No heap allocation per frame in steady state on hot paths.
- Components live in tightly packed columns per archetype, so `for_each`
  compiles to plain loops the compiler can vectorize.
- Performance claims in pull requests come with numbers from release builds.

## Tips

- Use `--release`. Always.
- Prefer `for_each`/`par_for_each` over `iter()` for big queries.
- Spawn with `spawn_batch`.
- Run expensive logic at a reduced rate with `sim_system_every(n, system)`,
  or spread per-entity work over ticks with `ctx.turns(n)` (see
  [The simulation clock](sim-clock.md)).

## C++

The [battle demo in C++](c-cpp.md#speed) runs at the same speed as the Rust
one: 21.04 against 20.99 ms/tick for a million units.
[C++ boids](c-cpp.md#speed) run 1M boids at 34.5 ms/tick headless, against
31.3 ms/tick for the Rust version; plain column updates are the same speed in
both (about 0.15 ms per million).
