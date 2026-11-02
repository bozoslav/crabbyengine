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
