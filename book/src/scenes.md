# Scenes and prefabs

A **scene** is a whole world kept in a file: a level, a menu, a test
setup. A **prefab** is one entity's components, kept in a file and spawned
as many times as you like: a tree, an enemy, a building. Both live in
`assets/`, both are JSON (so they diff well in git), and the editor,
agents and your own code write them like any other file.

<div class="lang-pair">
<div>

```rust
{{#include ../../tests/scenes.rs:scenes}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:scenes}}
```

</div>
</div>

- `load_scene` replaces the world with the scene's entities, which keep
  their ids, so components that refer to other entities still point at the
  right ones. Resources that are [reflected](reflection.md) load too.
- `spawn_prefab` puts the prefab's position at the point you give and keeps
  its saved rotation and scale. A prefab without a `Transform2D` gets one.
  The file is read once, then kept in memory, so spawning a thousand trees
  reads it once.
- Only [reflected](reflection.md) components go in files: reflect your
  types before loading or saving. `Transform2D`, `Sprite` and the physics
  components are reflected already. A sprite names its image
  (`"texture": "crab.png"`), and loading the scene or prefab loads it.
  With 3D on, `Transform3D` and `Model` are reflected too: a model names
  its mesh's glTF file (`"mesh": "tree.glb"`), loaded the same way. Scenes
  can also come from Unity ([Importing from Unity](unity.md)).
- Errors say what's wrong and change nothing: a missing file lists the
  folders searched, and a component that isn't reflected is named.

## Saving

The [inspector's editor](inspector.md#editing-build-levels-in-the-game)
saves and loads scenes and places prefabs with the mouse. From code:

`save_scene(path)` writes the world (its reflected components and
resources) and `save_prefab(entity, path)` writes one entity, without its
position. Both write over the file in `assets/` if it's there, else create
it (and its folders). Web builds can't save files.

```rust
# let palm = ctx.world.spawn((Transform2D::default(),));
ctx.assets.save_scene(&ctx.world, "levels/beach.json")?;
ctx.assets.save_prefab(&ctx.world, palm, "prefabs/palm.json")?;
```

C++: `ctx.save_scene(path)` and `ctx.save_prefab(e, path)`, which throw on
errors like the rest of the wrapper. C: `ponko_ctx_load_scene`,
`ponko_ctx_save_scene`, `ponko_ctx_spawn_prefab` and
`ponko_ctx_save_prefab`.

## What a load replaces and what it keeps

A scene is a snapshot of the world, not a whole save of the game: it holds
entities with reflected components and reflected resources. Everything
else belongs to the game and lives outside the world, so a load treats it
by a fixed rule (the full table is on `Ctx::load_scene` in the API docs):

| State | After `load_scene` |
|---|---|
| Entities | The scene's, with the scene's ids. An old `Entity` may now name a different entity of the new scene. |
| Reflected resources | The scene's values; reflected ones it doesn't name, and every other resource (scores, tuning, a baked navigation mesh), stay. |
| Commands and events | Commands queued earlier in the running system are dropped; events stay readable (their entity ids are the old world's). |
| 2D and 3D physics | Old bodies leave at once; the scene's are built at the next physics step, from rest. Gravity and settings stay. |
| Crowd agents, 3D UI panels, skeletons, joint attachments | Components that aren't saved: gone with their entities. Old crowd slots are freed at the next navigation step, and the keyboard focus of a 3D panel is forgotten. |
| `Attached` | Reflected: the scene's attachments follow the scene's entities. |
| Assets and prefabs | Stay cached; a load never unloads anything. |
| Positional sounds | A sound playing from a replaced entity stops; music and other sounds play on. |
| Camera, screen UI, lighting, input, time, random seed | Kept. Temporal effects (TAA) forget their history, and the inspector its selection. |
| Lockstep networking | Not synchronized: in a session, load in setup or a simulation system; other loads are refused (see below). |

A failed load (a missing file, a component that isn't reflected, an image
that can't be loaded) changes none of this; only images and meshes that
did load before the failure stay cached.

### Loading scenes in lockstep

In a [lockstep session](networking.md) every peer runs the same
simulation, so a load must happen on every peer at the same tick. Two
places guarantee that: setup, which every peer runs before the first tick,
and simulation systems, which every peer runs at the same ticks from the
same inputs. Load there with `load_scene`, or with `load_scene_lockstep`,
the same call named so the call site says it is part of the shared
simulation:

```rust
// In a simulation system: every peer reaches this tick with the same world.
if ctx.time.sim_tick() == 600 {
    ctx.load_scene_lockstep("levels/two.json")?;
}
```

Everywhere else a load would change one peer only, so while a session
connects or runs it is refused with an error and changes nothing: loads
from frame, late and draw systems, the inspector's Load button, the
editor's Stop, and the control endpoint's `scene.load` and `world.load`.
Offline, and after a session is lost, every load works as usual. The peers
still compare their worlds, so a load that only some peers make (one
behind an `if` on local state) shows up as a desync. C++:
`ctx.load_scene_lockstep(path)`; C: `ponko_ctx_load_scene_lockstep`, with
the same rules as `ponko_ctx_load_scene`.

## The files

A scene is the [save format](determinism.md) of `world.save()`:

```json
{{#include ../../assets/levels/beach.json}}
```

A prefab lists one entity's components:

```json
{{#include ../../assets/prefabs/palm.json}}
```

Fields left out take their defaults (here the palm's position and scale).
Prefabs are single entities for now: a prefab can't hold children.
