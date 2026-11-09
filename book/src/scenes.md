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
ctx.save_scene("levels/beach.json")?;
ctx.save_prefab(palm, "prefabs/palm.json")?;
```

C++: `ctx.save_scene(path)` and `ctx.save_prefab(e, path)`, which throw on
errors like the rest of the wrapper. C: `ponko_ctx_load_scene`,
`ponko_ctx_save_scene`, `ponko_ctx_spawn_prefab` and
`ponko_ctx_save_prefab`.

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
