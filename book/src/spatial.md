# Spatial queries

"Who is near me?" is the core question of most simulations. Checking every
pair is O(n²); a **spatial grid** answers it by looking only at nearby cells.

## The built-in grid

Turn it on with a cell size near your usual query radius:

```rust
Game::new("Boids", 1280, 720)
    .spatial_grid(24.0)
    .sim_system(flock);
```

At the start of every tick, every entity with a `Transform2D` is indexed into
`ctx.spatial`. Query it from any sim system:

```rust
struct Neighbors(usize);

fn count_neighbors(ctx: &mut Ctx) {
    let grid = &ctx.spatial;
    ctx.world
        .query::<(Entity, &Transform2D, &mut Neighbors)>()
        .par_for_each(|(me, t, n)| {
            n.0 = 0;
            grid.for_each_in_radius(t.position, 10.0, |o| {
                if o.entity != me { n.0 += 1; }
            });
        });
}
```

The grid is read-only while you query the world, so this runs on all cores.

| Method | Visits |
|---|---|
| `for_each_in_radius(center, r, f)` | entries within `r` of `center` |
| `for_each_in_rect(min, max, f)` | entries inside the rectangle |
| `count_in_radius(center, r)` | how many entries are within `r` |
| `nearest(center, max_r, accept)` | the closest entry `accept` agrees to |

Each visit gives a `GridEntry` with `entity` and `position`.

Every 32 ticks (`SPATIAL_SORT_EVERY`) the built-in system also reorders
entities by grid cell (`World::sort_by` with `spatial_key`), so neighbors in
space are neighbors in memory. With a million boids this cut a tick from
62 ms to 41 ms.

## Your own grid, with data

The built-in grid stores only entity and position. When neighbors need more
data, for example a boid's velocity, keep your own `SpatialGrid<T>` inside
the system and fill it with `rebuild_with_data`:

```rust
{{#include ../../examples/boids.rs:flocking}}
```

## Choosing a cell size

- About your query radius is a good start: a radius query then reads a 3×3
  block of cells.
- Much smaller cells mean more cells to visit; much larger means more
  entries per cell to reject.
- Queries much larger than the world fall back to a plain scan, so huge
  radii stay correct and don't get slow.
