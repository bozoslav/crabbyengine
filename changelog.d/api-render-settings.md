### Breaking changes
- The 3D render settings moved from separate `Ctx` fields into one,
  `ctx.scene3d` (`render::SceneSettings`): `ctx.camera3d` →
  `ctx.scene3d.camera`, `ctx.lighting` → `ctx.scene3d.lighting`, `ctx.sky` →
  `ctx.scene3d.sky`, `ctx.post` → `ctx.scene3d.post`, `ctx.lod` →
  `ctx.scene3d.lod`. Its fields borrow separately, so two can be used at once.
- The extra 3D passes moved to `ctx.scene3d.passes` (`render::RenderPasses`):
  `ctx.compute_passes` → `ctx.scene3d.passes.compute`,
  `ctx.scene_compute_passes` → `ctx.scene3d.passes.scene_compute`,
  `ctx.target_cameras` → `ctx.scene3d.passes.target_cameras`,
  `ctx.target_effects` → `ctx.scene3d.passes.target_effects`, `ctx.effects` →
  `ctx.scene3d.passes.effects`.
- Graphics quality is a set of presets that write into the settings above,
  not a struct of copies: `GraphicsQuality` is an enum (`Low`, `Medium`,
  `High`, `Ultra`). `ctx.gfx.set_quality(&mut ctx.post, &mut ctx.lighting,
  GraphicsQuality::low())` → `ctx.scene3d.set_quality(GraphicsQuality::Low)`;
  `ctx.gfx.quality(&ctx.post, &ctx.lighting).level()` →
  `ctx.scene3d.quality()`, `None` for custom settings. `QualityLevel` is
  removed (`QualityLevel::Custom` → `None`); to change single settings, write
  the fields of `ctx.scene3d.post` and `ctx.scene3d.lighting`.
- `GraphicsQuality::for_tier(tier)` and `GraphicsQuality::recommended(caps)`
  → `ctx.scene3d.set_recommended_quality(caps)` (`tier.report()` for a
  tier's floor); `quality.fitted_to(caps)` →
  `ctx.scene3d.fit_quality_to(caps)`; `quality.clamped()` →
  `ctx.scene3d.clamp_quality()` (`quality()` compares clamped values);
  `GraphicsQuality::MIN_RENDER_SCALE` / `MAX_RENDER_SCALE` →
  `Post::MIN_RENDER_SCALE` / `Post::MAX_RENDER_SCALE`.
- Automatic quality's settings moved from `Graphics` to the settings it
  changes: `ctx.gfx.set_quality_autotune(..)` →
  `ctx.scene3d.set_quality_autotune(..)`, `ctx.gfx.freeze_quality_autotune`
  → `ctx.scene3d.freeze_quality_autotune`,
  `ctx.gfx.pin_render_scale(&mut ctx.post, s)` →
  `ctx.scene3d.pin_render_scale(s)`,
  `ctx.gfx.pin_shadow_resolution(&mut ctx.lighting, r)` →
  `ctx.scene3d.pin_shadow_resolution(r)`. Its live state is the context's:
  `ctx.gfx.quality_autotune_status(..)` → `ctx.scene3d.quality_autotune_status(&ctx.gfx)`.
  `SceneSettings` holds only plain values, so replacing it
  (`ctx.scene3d = SceneSettings::default()`) turns automatic quality off and
  restoring a copy restores its policy; the engine keeps the 3D camera's
  view at the window's size either way.
- MSAA has one home, `Post::msaa` (4 or 2, default 4): `Game::msaa(n)` →
  `ctx.scene3d.post.msaa = n` in a setup callback (applied right after it,
  before the first frame), and `ctx.gfx.set_msaa(n)` →
  `ctx.scene3d.post.msaa = n` at any time (applied at the next frame).
  `Game::msaa` panicked on a count other than 2 or 4; `Post::msaa` keeps the
  current count instead, says so once on stderr, and `GpuCapabilities`
  shows it as the Multisampling fallback, as for a count the GPU can't
  take.
- The flat sky's ambient colors moved next to the switch that uses them:
  `Lighting::sky_color`, `ground_color` and `equator_color` →
  `Sky::sky_color`, `ground_color` and `equator_color` (used only with
  `Sky::atmosphere` off).
- `Lighting::shadow_filter` is a `ShadowFilter` enum instead of a `u32`:
  `1` → `ShadowFilter::Sharp`, `2` → `Soft`, `3` → `Softest` (the default).
  `Lighting::shadow_cascades` stays a `u32` (1 to 4): it is a count of
  shadow maps (the presets write 2 to 4), not a choice between modes.
- `Lighting`, `Post`, `Sky`, `LodSettings`, `RenderPasses`, `GraphicsQuality`,
  `AntiAliasing`, `Tonemap` and `ShadowFilter` are `#[non_exhaustive]`: build
  the structs from `Default` and set fields (`let mut post = Post::default();
  post.bloom = 0.1;`), and give matches a wildcard arm.
- `d3` re-exports `SceneSettings`, `RenderPasses` and `ShadowFilter` next to
  `Lighting`, `Post`, `Sky` and `GraphicsQuality`.
- The C ABI is unchanged; `PonkoLighting` still carries the ambient colors
  and its `shadow_filter` stays 1 to 3. `ponko_ctx_graphics_quality` now
  reads the values as the renderer draws them (clamped), so
  `ponko_ctx_set_graphics_quality` always takes them back.
