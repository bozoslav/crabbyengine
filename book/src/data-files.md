# Data files: tune without restarting

Keep the numbers you tweak (speeds, damage, costs, wave sizes) in a JSON
file in `assets/` instead of in code. With [hot reload](drawing.md#hot-reload)
on (the default in debug builds), saving the file changes them in the
running game, within a quarter of a second, so you can tune a
fight while watching it.

<div class="lang-pair">
<div>

```rust
{{#include ../../tests/hot_reload.rs:data_files}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:data_files}}
```

</div>
</div>

In Rust the file becomes a resource: any type that implements
`serde::Deserialize`, replaced whole on each reload (use `#[serde(default)]`
for optional fields). In C and C++ the resource is a plain struct described
with `reflect_resource`, and the file's fields merge into it, so the file
may list only some of them (C: `ponko_ctx_load_resource_json`). C++ can
also load a file into a component of one entity: `ctx.load_json<T>(path, e)`.
Rust has the same merging forms for reflected types, with the name taken from
the type: `ctx.assets.load_resource_json::<T>(&mut ctx.world, path)` and
`ctx.assets.load_component_json::<T>(&mut ctx.world, path, entity)`.

- Loading fails with a clear message: a missing file lists the folders
  searched, and bad JSON says the line and column.
- A file that doesn't parse while the game runs (often one caught halfway
  through saving) keeps the old values and prints why; the next good save
  applies.
- `ctx.assets.reloaded_files()` names the files reloaded this frame, textures
  included, for work that depends on them (rebuilding a map, say). In C++,
  `ctx.was_reloaded("map.json")` (C: `ponko_ctx_was_reloaded`).
- Headless runs reload too, so an agent or a test can change a file
  between ticks.
- Files can also come from somewhere other than `assets/` (embedded in the
  executable, say); see [Asset sources](asset-sources.md). Those are not
  hot reloaded.
