# Handles: freeing what you made

Textures, meshes, materials, sounds and the other things the engine keeps
for your game come back as small handles: plain `Copy` values holding an id.
Every kind works the same way:

| Question | Rust | C++ |
|---|---|---|
| Is it still valid? | `ctx.is_live(h)` | `ctx.is_live(h)` |
| Give the memory back | `ctx.free(h)` | `ctx.free(h)` |
| An empty handle | `Texture::NONE`, `Mesh::NONE`, ... | `ponko::none<ponko::texture>()` |
| How many are live? | `ctx.live_counts()` | `ctx.live_counts()` |

The kinds are `Texture` (render targets included), `Sound`, `Font`, and with
the `3d` feature `Mesh`, `Material`, `Shader`, `Compute`, `Effect`,
`GpuBuffer`, `InstanceBatch` and `TextureArray`. They implement the `Handle`
trait, so a helper can take any of them.

The service that holds a handle answers the same questions:
`ctx.gfx.is_live(h)`, `ctx.gfx.free(h)` and `ctx.gfx.live_counts()` for
everything drawn (every kind except `Sound`, the `GfxHandle` trait), and
`ctx.audio.is_live(s)` and `ctx.audio.free(s)` for sounds. `ctx.is_live`
and `ctx.free` take any kind and forward to them.

A game that streams its world frees what each cell made when the cell goes
out of range:

<div class="lang-pair">
<div>

```rust
{{#include ../../tests/handles.rs:handles}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:handles}}
```

</div>
</div>

## The rules

- **Ids are never reused within a run**, so a freed handle stays dead: it
  never starts naming something made later, and a stale copy kept somewhere
  can't quietly point at the wrong thing. **Except meshes and GPU buffers:**
  streamed worlds make and free those all the time, so the next one made
  takes a freed id and the engine's tables (and per-frame work over every
  mesh) don't keep growing. Drop a mesh or buffer handle once you free it.
- **Freeing twice, or freeing `NONE`, does nothing.** `NONE` is never live.
- **Only what the game made can be freed.** The built-in font, the default
  material's internals and the engine's own textures (shadow maps, the font
  atlas) aren't the game's to free; freeing `Font::DEFAULT` does nothing.
- **Nothing is reference counted.** Freeing is explicit, and the handle you
  hold doesn't keep anything alive.

## Freeing something that is still used

Freeing a resource others still use is allowed, and what they do then is
defined. Nothing panics and nothing reads freed memory:

| Freed | What still uses it does |
|---|---|
| Mesh | A model showing it draws nothing, like `Mesh::NONE`. |
| Texture | Materials and sprites sample white, as if they had no texture. A freed normal, ORM or emissive map is ignored. |
| Material | Meshes draw with the default material. |
| Shader | Materials using it draw with the built-in shader. |
| Texture array | Materials sample a white array; decals using it are turned off. |
| Render target | Cameras, compute passes and 3D panels drawing into it are skipped. |
| Effect, compute pass | They are left out of the frame. |
| GPU buffer | Its compute passes are dropped; meshes drawing from it draw nothing. |
| Sound | Its playing voices stop; playing it again does nothing. |
| Font | Text in it uses the built-in font. |

Updating a freed handle (`update_texture`, `update_material`) does nothing,
and a hot-reloaded file no longer touches it. Loading a freed file again
(`load_mesh`, `load_texture`) loads a fresh copy with a new id.

To hide something for a while, show `Mesh::NONE` instead of freeing it, and
show the real mesh again later. Free when you won't need it again.

## Counting

`ctx.live_counts()` returns how many handles of each kind are live. Compare
two counts to find a leak: a level that loads and unloads should end where it
started.

```rust
let before = ctx.live_counts();
// ...load and unload a level...
assert_eq!(ctx.live_counts(), before);
```

`ctx.asset_inventory()` counts the same handles with estimated bytes (its
texture count also includes the engine's own textures), and its `dangling`
list names the sprites and models that still show a freed texture, mesh or
material. The control endpoint's `perf.get` includes it as `assets`.

## The older names

The per-kind calls (`free_mesh`, `free_instance_batch`, `has_material`,
`has_shader`, `has_font`, `is_mesh_live`, ...) still work and do the same as
`free` and `is_live`, but are deprecated: use `ctx.gfx.free(h)` and
`ctx.gfx.is_live(h)`. `gpu_buffer_len`, `instance_batch_len` and
`is_render_target` stay: they answer more than whether a handle is live.

## Not handles

- A `TerrainTile` holds one mesh per detail level, not an id; free it with
  `free_terrain_tile` (or `ctx.free` each of its meshes).
- 3D physics collision meshes have their own generation-checked handles; see
  [3D physics](physics3d.md).

## In C

C passes the kind and the id:

```c
bool live = false;
ponko_ctx_handle_is_live(ctx, PONKO_HANDLE_MESH, mesh.id, &live);
ponko_ctx_handle_free(ctx, PONKO_HANDLE_MESH, mesh.id);
PonkoLiveCounts counts;
ponko_ctx_live_counts(ctx, &counts);
```

An unknown kind is an invalid argument. `PONKO_HANDLE_NONE` is the empty id
for every kind. C API calls that take a handle report a freed one as an
invalid argument.
