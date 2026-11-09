### Added
- One lifecycle for every resource handle: the `Handle` trait (`id`,
  `from_id`, `NONE`), `ctx.is_live(h)`, `ctx.free(h)` and
  `ctx.live_counts()` for textures (render targets included), sounds,
  fonts, meshes, materials, shaders, compute passes, effects, GPU buffers,
  instance batches and texture arrays. Freeing something still in use is
  defined: models draw nothing for a freed mesh, white for a freed texture,
  the default material for a freed material; freed effects and compute
  passes are skipped, sounds stop and text falls back to the built-in font.
  `Handle::NONE` is every kind's empty handle (`Texture::NONE`,
  `Material::NONE`, ...), never live. Ids are never reused, except for
  meshes and GPU buffers (as before). The per-kind `free_*`, `has_*` and
  `is_*_live` calls keep working. In C as `ponko_ctx_handle_is_live`,
  `ponko_ctx_handle_free`, `ponko_ctx_live_counts`, `PonkoHandleKind`
  and `PONKO_HANDLE_NONE`; in C++ as `ctx.is_live`, `ctx.free`,
  `ctx.live_counts` and `ponko::none<H>()`.
