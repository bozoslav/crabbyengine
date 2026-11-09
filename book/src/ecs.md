# Entities, components, queries

The game world lives in `ctx.world`, an entity-component system (ECS).

- An **entity** is an id (`Entity`). It has no data of its own.
- A **component** is any `Send + Sync + 'static` Rust type. No derive needed.
  In C++ it is any plain struct (trivially copyable), found by its type:
  nothing to register.
- A **query** visits every entity that has a given set of components.

## Spawning

<div class="lang-pair">
<div>

```rust
struct Health(f32);
struct Enemy;

let e: Entity = ctx.world.spawn((Transform2D::at(vec2(10.0, 20.0)), Health(100.0), Enemy));

// Many at once is much faster than a loop of `spawn`:
ctx.world.spawn_batch((0..100_000).map(|i| (Transform2D::at(vec2(i as f32, 0.0)), Health(10.0))));
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:ecs_spawn}}
```

</div>
</div>

A tuple of components is a *bundle*; bundles go up to 12 components. C++
passes the components straight to `spawn`.

```cpp
{{#include ../../examples/cpp/guide.cpp:ecs_components}}
```

## Reading and changing one entity

<div class="lang-pair">
<div>

```rust
# struct Health(f32);
# struct Enemy;
# let e = ctx.world.spawn((Health(10.0), Enemy));
if let Some(h) = ctx.world.get_mut::<Health>(e) {
    h.0 -= 5.0;
}
ctx.world.insert(e, Enemy);        // add or replace a component
ctx.world.remove::<Enemy>(e);      // returns the removed value
ctx.world.despawn(e);
assert!(!ctx.world.is_alive(e));   // old handles stay dead, even if the slot is reused
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:ecs_one}}
```

</div>
</div>

## Queries

<div class="lang-pair">
<div>

```rust
# struct Health(f32);
# struct Enemy;
ctx.world
    .query::<(&mut Transform2D, &Health)>()
    .without::<Enemy>()
    .for_each(|(t, h)| t.position.y += h.0);
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:ecs_query}}
```

</div>
</div>

In C++, `each` reads the components to visit from your function's
parameters: `T&` to change one, `const T&` to read it, and a first
`ponko::entity` parameter gets the entity's id. `world.each_par` is the C++
`par_for_each`: your function runs on all cores at once, so it must only
touch its own entity.

For code that works on whole arrays, `world.query<A, B>(fn)` hands you one
*chunk* (archetype) at a time, with an array per component:

```cpp
{{#include ../../examples/cpp/guide.cpp:ecs_columns}}
```

`each` compiles to the same loop, so it's as fast. With `query_par`, size a
buffer with `world.count<A, B>()` and write row `i` of a chunk to slot
`chunk.first() + i` to fill it from all cores: rows are numbered the same
way in every query.

What you can ask for (Rust):

| In the query | You get |
|---|---|
| `&T` | read access to `T` (entity must have it) |
| `&mut T` | write access to `T` |
| `Option<&T>`, `Option<&mut T>` | the component if present; doesn't filter |
| `Entity` | the entity's id |
| a tuple of the above | all of them, up to 12 (nest tuples for more) |

Filters `with::<T>()` and `without::<T>()` narrow the match without fetching
data. For "either" matches, or a filter you reuse across systems, pass a typed
filter to `filter::<F>()`: `With<T>`, `Without<T>`, `Or<(...)>`, or a tuple of
them that must all hold:

```rust
# struct Player;
# struct Enemy;
# struct Dead;
# struct Health(f32);
type Living = (Or<(With<Player>, With<Enemy>)>, Without<Dead>);
world.query::<&mut Health>().filter::<Living>().for_each(|h| h.0 -= 1.0);
```

Asking for the same component twice with `&mut` (or `Option<&mut T>`) panics.

Ways to run a query:

- `for_each(f)`: the fastest loop; use this by default.
- `par_for_each(f)`: same, split over all CPU cores. Use it for per-entity
  work on many entities.
- `par_collect_into(&mut vec, f)`: fills `vec` with `f` of every match, on
  all cores, in `for_each` order. The vector keeps its memory, so doing it
  every tick allocates nothing.
- `iter()`: a normal iterator, for when you need `map`/`filter`/`collect`. It
  is several times slower than `for_each` on big worlds.
- `count()`, `get(entity)`.

## Commands: changing the world while you iterate

You can't spawn or despawn inside a query loop, because the query is borrowing
the world. Record the change in `ctx.commands` instead:

<div class="lang-pair">
<div>

```rust
# struct Health(f32);
fn die(ctx: &mut Ctx) {
    let cmds = &mut ctx.commands;
    ctx.world.query::<(Entity, &Health)>().for_each(|(e, h)| {
        if h.0 <= 0.0 { cmds.despawn(e); }
    });
}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:ecs_commands}}
```

</div>
</div>

The engine applies commands after every system, so the next system sees them.

## Resources

A resource is one global value of a type, stored in the world and found
by its type:

<div class="lang-pair">
<div>

```rust
struct Score(u32);
ctx.world.insert_resource(Score(0));
ctx.world.resource_mut::<Score>().0 += 1;
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:resources}}
```

</div>
</div>

C++ resources are plain structs, copied in and out: `resource<T>()` (throws
if the world has none), `try_resource<T>()`, `set_resource(value)` and
`remove_resource<T>()`. Read them before an `each` loop, not inside it (the
world is borrowed while the loop runs). In C: `ponko_resource_set`,
`ponko_resource_get` and `ponko_resource_remove`, by name.

For Rust, use `resource_scope` when a non-Copy resource must be mutated while
querying components or accessing other resources. This avoids manually removing
and reinserting it:

```rust
struct Total(u32);
ctx.world.insert_resource(Total(0));
ctx.world.resource_scope::<Total, _>(|world, total| {
    world.query::<&u32>().for_each(|value| total.0 += *value);
});
```

The scoped resource is temporarily absent from world lookups. Its original box
and any mutations are restored on return (including early returns) or panic
unwinding. Any same-type replacement inserted inside the callback is discarded
on exit. Different resource types can be scoped inside one another. Scoping
itself reuses storage without steady-state allocations. This API is Rust-only;
native bindings retain their existing resource operations.

## Sorting for cache locality

`world.sort_by::<T>(key)` reorders entities that have `T` by a `u64` key.
Entity ids stay valid. The built-in spatial grid uses it to keep entities
that are close in space close in memory; see [Spatial queries](spatial.md).
