# Reflection: components as JSON

Reflection lets code that doesn't know your types read and change them by
name: the agent bridge (MCP), the inspector, and save/load all use it. It's
opt-in per type and built on [serde](https://serde.rs).

```rust
use crabbyengine::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Health { current: f32, max: f32 }

Game::new("Arena", 1280, 720).run(|ctx| {
    ctx.world.reflect::<Health>();
    let e = ctx.world.spawn((Transform2D::default(), Health { current: 10.0, max: 10.0 }));

    // Read: {"current": 10.0, "max": 10.0}
    let hp = ctx.world.get_json(e, "Health").unwrap();

    // Write only some fields; the rest keep their values.
    ctx.world.set_json(e, "Health", json!({ "current": 3.0 })).unwrap();
});
```

Add `serde = { version = "1", features = ["derive"] }` to your `Cargo.toml`.

## C and C++ components

C++ has no serde, so a C++ component lists its fields when it is
registered. From then on it is a reflected component like any other:
agents read and write it through `crabby-mcp`, and saves include it.

```cpp
{{#include ../../examples/cpp/guide.cpp:reflect}}
```

Fields can be `float`, `double`, the fixed-width integers, `bool`,
`crabby::vec2` (`[x, y]` in JSON) and `crabby::color` (`{"r", "g", "b", "a"}`).
Entities are `uint64_t`, which is how Rust saves an `Entity` too. Bytes
no field covers (padding, fields you leave out) are zero in values made
from JSON, and editing a field leaves them alone.

In C, `crabby_component_reflect(world, id, fields, count)` takes the same
list as `CrabbyField { name, kind, offset }`, with `offsetof` for the
offset. The Rust side of this is `World::reflect_fields`, for components
that are registered at runtime and have no Rust type.

## Names

A component's name is its type name without the module path:
`my_game::units::Health` is `"Health"`. Reflecting two different types with
the same short name panics and names both, so a name always means one type.

`Transform2D` is reflected in every game already.

## The methods

All of them come from `WorldReflect`, which `use crabbyengine::*;` brings in.

| Method | Does |
|---|---|
| `reflect::<T>()` | makes `T` visible to reflection |
| `reflected_components()` | names of all reflected components |
| `get_json(e, name)` | one component as JSON |
| `set_json(e, name, value)` | writes it; adds it if missing; objects merge field by field |
| `remove_json(e, name)` | removes it; `Ok(true)` if it was there |
| `entity_json(e)` | all reflected components of `e`, keyed by name |
| `spawn_json(value)` | spawns from the `entity_json` format; on error spawns nothing |

`entity_json` and `spawn_json` round-trip, so a snapshot of an entity can be
copied, saved, or sent over the wire:

```rust
let snapshot = world.entity_json(e)?;   // {"Transform2D": {...}, "Health": {...}}
let copy = world.spawn_json(snapshot)?;
```

Components that aren't reflected are left out of `entity_json`.

## Errors

Every call returns a `ReflectError` that says what went wrong:
`UnknownComponent`, `NoSuchEntity`, `MissingComponent`, or `BadValue` (with
serde's message, such as `invalid type: string "seven", expected struct Health`).

## Tips

- `#[serde(default)]` on a struct lets `set_json` add the component from a
  partial value. `Transform2D` has it.
- Reflection is for tools, not hot loops: it goes through JSON. Use queries
  inside systems.
