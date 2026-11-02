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

## C++ column iteration

The [C/C++ comparison](c-cpp.md#release-comparison) measures matching Rust typed
queries and C++ whole-column callbacks over one million entities. Three local
release runs measured 0.151–0.159 ms per update in Rust and 0.151–0.156 ms in C++,
with identical checksums. The guide includes the workload and build commands.
