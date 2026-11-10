### Added
- `ctx.gfx.is_live(h)`, `ctx.gfx.free(h)` and `ctx.gfx.live_counts()` for
  every graphics handle (the new `GfxHandle` trait: textures, render
  targets, fonts, meshes, materials, shaders, compute passes, effects, GPU
  buffers, instance batches, texture arrays), and `ctx.audio.is_live(s)` /
  `ctx.audio.free(s)` for sounds. `ctx.is_live` and `ctx.free` forward to them.
- `AssetInventory::dangling` (and `dangling` in `perf.get`'s `assets`): the
  sprites, models, model materials and shadow meshes that still show a
  freed texture, mesh or material.

### Changed
- `Graphics::has_material`, `has_shader`, `has_compute`, `has_effect`,
  `has_font`, `is_mesh_live`, `free_mesh` and `free_instance_batch` are
  deprecated in favor of `ctx.gfx.is_live` and `ctx.gfx.free`; the flat
  `Ctx` versions now point there too. `has_texture_array` stays: it also
  checks the array's size and layer count.
- A freed GPU buffer's id goes back to the pool only after the compute
  passes writing to it are removed (right away through `ctx.free`, at the
  next frame through `ctx.gfx.free`), so a stale pass never writes into the
  next buffer given that id.

### Fixed
- `AssetClass` docs no longer say textures, materials, shaders, effects,
  compute passes and sounds are never freed.
- The headless test of models whose resources were freed now runs its
  checks (they were in a frame system, which headless runs skip).
