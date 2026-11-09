# Asset sources: assets that don't live in `assets/`

By default a named asset (`"sprites/crab.png"`, `"rules.json"`) is a file in
the `assets/` folder (on the web, a file listed in `assets/index.json`). An
**asset source** is another place to look first: bytes compiled into the
executable, a pack file, or files made up in a test. A game adds one in
setup; after that, the same loads by name find the asset there, and the rest of
the code doesn't change.

Rust only for now: there is no C or C++ API for asset sources (see
[Still Rust-only](c-cpp.md#still-rust-only)).

## Embedding files in the executable

`MemorySource` holds named files in memory. Fill it with `include_bytes!` and
the game needs nothing next to the executable:

```rust
{{#include ../../tests/asset_sources.rs:embedded}}
```

`ctx.add_asset_source(source)` takes any type that implements `AssetSource`,
and `MemorySource` is the ready-made one. Your own is two methods: `id` (a
short name, unique among the sources you add) and `read(name)` (the bytes, or
an error of kind `NotFound` when the source doesn't have that name). Use it
for a pack file or a download cache.

## The rules

- **Lookup order.** Added sources are asked in the order you added them, then
  the `assets/` folders. The first one that has the name wins, so a source can
  override a file in `assets/`. A source that lacks a name returns a
  `NotFound` error and the next one is asked; any other error (a disk read that
  failed, say) stops the search and is reported.
- **Names.** Names are the same logical names the default source uses:
  `/`-separated, relative, `"sprites/crab.png"`. Scenes and prefabs refer to
  assets by these names too, so a scene stored in a source finds the textures
  stored in it.
- **Caching.** A loaded asset is cached by its source's `id` and its name.
  Loading a name again returns the first handle without reading the source
  again. The same name in two sources is two assets.
- **Add sources in setup**, before loading what they hold: an asset loaded
  earlier stays cached where it was found.
- **No hot reload.** Only files found in `assets/` on a desktop build are
  [watched](drawing.md#hot-reload). Assets from added sources load once and
  never change while the game runs.
- **Errors name the source.** A missing name lists the sources and folders
  searched; a corrupt file says which source it came from.

## Which loads use sources

These loads on `Ctx` look in added sources first: `load_texture`,
`load_font` (and `try_load_texture` / `try_load_font`, which return an error
instead of panicking), `load_resource`, `load_scene`, `spawn_prefab`, and the
textures that scenes and prefabs name.

Sounds, 3D models and other 3D files, and shaders still come only from
`assets/`.

A source also makes headless tests self-contained: a test that adds a
`MemorySource` with a generated image or a JSON file doesn't depend on the
repository's `assets/` folder (see [Headless runs and
testing](headless.md)). The engine's own tests do this in
[`tests/asset_sources.rs`](https://github.com/ponkoengine/ponko/blob/main/tests/asset_sources.rs).
