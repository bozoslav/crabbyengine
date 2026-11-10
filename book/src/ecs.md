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

## Change detection

Some work only needs the entities that changed: resize a health bar when
`Health` changes, rebuild a path when a `Target` moves, upload an instance
when its transform changes. Ask for them with the filters `Changed<T>` and
`Added<T>`:

<div class="lang-pair">
<div>

```rust
# struct Health(f32);
# struct HealthBar { width: f32 }
fn resize_health_bars(ctx: &mut Ctx) {
    // Only the entities whose Health changed since this system last ran.
    ctx.world
        .query::<(&Health, &mut HealthBar)>()
        .filter::<Changed<Health>>()
        .for_each(|(hp, bar)| bar.width = hp.0);
    // The ones that are new, and the ones that lost it (or despawned).
    let spawned = ctx.world.query::<Entity>().filter::<Added<Health>>().count();
    let gone: &[Entity] = ctx.world.removed::<Health>();
#   let _ = (spawned, gone);
}
fn regenerate(ctx: &mut Ctx) {
    // `&mut Health` can write, so every entity visited counts as changed.
    // Ask for `&Health` when you only read.
    ctx.world.query::<&mut Health>().for_each(|hp| hp.0 += 1.0);
}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:ecs_changes}}
```

</div>
</div>

**It is opt-in, per component.** Nothing is tracked until a component is
asked for: by a `Changed<T>` or `Added<T>` filter (C++: `ponko::changed<T>`,
`ponko::added<T>`), or by `world.track_changes::<T>()` (C++:
`world.track_changes<T>()`). Call `track_changes` in `setup` for components
whose removals you read, because `removed::<T>()` panics for a component that
isn't tracked. A game that tracks nothing keeps no stamps and runs the same
loops as before.

**Each system has its own view.** In a system, the filters and `removed`
report what happened since that system last ran, so every system sees each
change exactly once, whether it runs before or after the system that made
it, and a system that runs every tenth tick sees ten ticks of changes. A
system isn't shown its own changes (or its own commands) again. The first
time a system runs, and in code outside systems such as `setup`, everything
counts as added and changed.

What counts, for a tracked component:

| Operation | Added | Changed |
|---|---|---|
| `spawn`, `spawn_batch`, `spawn_at`, and their `_dynamic` forms | yes | yes |
| `insert` on an entity without one | yes | yes |
| `insert` replacing a value | no | yes |
| `get_mut` | no | yes |
| `&mut T` or `Option<&mut T>` in a query | no | yes, for every entity the query visits |
| the same in a view system (`sys.world`), and its `get_mut` | no | yes |
| `mark_changed::<T>(e)` | no | yes |
| `get`, `has`, `&T` in a query, `count()`, `sort_by` | no | no |
| `remove`, `despawn`, `clear` | goes to `removed::<T>()`; the entity's other components keep their state |

- **A mutable borrow is a change, written or not.** The world can't see a
  write through `&mut T`, so it assumes one. A query with `&mut T` marks
  every entity it visits: all it matches, or only the ones that pass its
  `Added`/`Changed` filters. To change only a few entities, query `&T` and
  call `get_mut` (or record a command) for those. A system that both reads
  `Changed<T>` and writes `T` to every entity makes every other reader of
  `T` see everything, every tick.
- **`Changed` includes `Added`**: a new value is also a changed one. Use
  `Added<T>` alone for "new here". Inserting `T` again after removing it
  counts as added again.
- **Filters combine** with the others: `(Changed<A>, Without<Dead>)` needs
  both, `Or<(Changed<A>, Changed<B>)>` needs either. A filter on `T` also
  requires `T`, like `With<T>`. Up to 4 `Added`/`Changed` filters per query
  outside `Or`.
- **Commands** are stamped when they are applied, which is when the
  recording system returns: the other systems see them as changes of that
  system.
- **Removals** list each entity that lost `T` since the system last ran,
  in the order it happened, despawned entities included (check
  `world.is_alive(e)` before using one). The engine drops a removal once
  every system has been shown it, so a system that doesn't run holds
  removals back: while the simulation is paused, the removals that frame
  systems make stay in the logs until the simulation systems run again.
- **Saving and loading**: stamps and removals are not part of a saved
  world. Loading a save removes every entity and spawns the saved ones, so
  every system sees them as removed and then added.
- **Replacing the world** (`ctx.world = World::new()`) starts every system
  over: the new world has its own history, so on its next run each system
  sees everything in the new world as added and changed, and the removals
  logged there so far, however rarely it runs. Nothing is reported for the
  old world's entities; they were not removed, their world is gone. A
  `Tick` you kept yourself belongs to the world it came from: keep
  `world.id()` next to it and start again from `Tick::default()` when the
  id differs.
- **The tick counter doesn't run out.** The world counts ticks in 64 bits
  and stores 32 bits per stamp. A stamp older than two billion ticks is
  kept at that age, so a very old change is never mistaken for a new one;
  a reader that slept that long is shown everything.

C and C++ components work the same way. In C++, an `each` parameter taken
as `T&` can write and counts as a change, while `const T&` or `T` only
reads; `world.query<const A, B>(fn)` does the same for whole arrays, and
`ponko::any_of<ponko::changed<A>, ponko::changed<B>>` is the `Or`. In C,
`ponko_query` marks every column it hands out, and `ponko_query_with` takes
a `PonkoQueryOptions` saying which columns it writes and which must have
changed. `ponko_set` (C++ `world.set`) counts like `insert`, and
`ponko_mark_changed` (C++ `world.mark_changed<T>(e)`) marks one entity by
hand, for a value that depends on something the world can't see.

Outside the game loop (a plain `World` in a tool or a test) there are no
systems, so keep the tick yourself: `let seen = world.advance_change_tick()`
after looking, then `.since(seen)` on the next query and
`world.removed_since::<T>(seen)`; call `world.forget_removed(seen)` so the
removal logs don't grow.

**What it costs.** For a component that isn't tracked: nothing. For a
tracked one: 8 bytes per entity, and one stamp per entity for each
mutable visit. With 1M entities on an Apple M-series laptop, a `&mut`
loop takes 0.20 ms instead of 0.17 ms, a `&T` loop is unchanged, and a
`Changed<T>` loop that finds nothing takes 0.5 ms (it reads one stamp per
entity). `cargo run --release -p ponko_bench -- changes` prints the
numbers for your machine.

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
