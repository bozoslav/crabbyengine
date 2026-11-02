# Entities, components, queries

The game world lives in `ctx.world`, an entity-component system (ECS).

- An **entity** is an id (`Entity`). It has no data of its own.
- A **component** is any `Send + Sync + 'static` Rust type. No derive needed.
- A **query** visits every entity that has a given set of components.

## Spawning

```rust
struct Health(f32);
struct Enemy;

let e: Entity = ctx.world.spawn((Transform2D::at(vec2(10.0, 20.0)), Health(100.0), Enemy));

// Many at once is much faster than a loop of `spawn`:
ctx.world.spawn_batch((0..100_000).map(|i| (Transform2D::at(vec2(i as f32, 0.0)), Health(10.0))));
```

A tuple of components is a *bundle*; bundles go up to 12 components.

## Reading and changing one entity

```rust
if let Some(h) = ctx.world.get_mut::<Health>(e) {
    h.0 -= 5.0;
}
ctx.world.insert(e, Enemy);        // add or replace a component
ctx.world.remove::<Enemy>(e);      // returns the removed value
ctx.world.despawn(e);
assert!(!ctx.world.is_alive(e));   // old handles stay dead, even if the slot is reused
```

## Queries

```rust
ctx.world
    .query::<(&mut Transform2D, &Health)>()
    .without::<Enemy>()
    .for_each(|(t, h)| t.position.y += h.0);
```

What you can ask for:

| In the query | You get |
|---|---|
| `&T` | read access to `T` (entity must have it) |
| `&mut T` | write access to `T` |
| `Option<&T>`, `Option<&mut T>` | the component if present; doesn't filter |
| `Entity` | the entity's id |
| a tuple of the above | all of them, up to 8 |

Filters `with::<T>()` and `without::<T>()` narrow the match without fetching
data. Asking for the same component twice with `&mut` panics.

Ways to run a query:

- `for_each(f)`: the fastest loop; use this by default.
- `par_for_each(f)`: same, split over all CPU cores. Use it for per-entity
  work on many entities.
- `iter()`: a normal iterator, for when you need `map`/`filter`/`collect`. It
  is several times slower than `for_each` on big worlds.
- `count()`, `get(entity)`.

## Commands: changing the world while you iterate

You can't spawn or despawn inside a query loop, because the query is borrowing
the world. Record the change in `ctx.commands` instead:

```rust
fn die(ctx: &mut Ctx) {
    let cmds = &mut ctx.commands;
    ctx.world.query::<(Entity, &Health)>().for_each(|(e, h)| {
        if h.0 <= 0.0 { cmds.despawn(e); }
    });
}
```

The engine applies commands after every system, so the next system sees them.

## Resources

A resource is one global value of a type, stored in the world:

```rust
struct Score(u32);
ctx.world.insert_resource(Score(0));
ctx.world.resource_mut::<Score>().0 += 1;
```

## Sorting for cache locality

`world.sort_by::<T>(key)` reorders entities that have `T` by a `u64` key.
Entity ids stay valid. The built-in spatial grid uses it to keep entities
that are close in space close in memory; see [Spatial queries](spatial.md).
