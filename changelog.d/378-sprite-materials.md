### Added
- Sprite blend modes: `BlendMode` (`Alpha`, `Additive`, `Multiply`,
  `Premultiplied`) as a component next to a `Sprite`, and
  `DrawParams::with_blend` for immediate draws. They work on every GPU,
  WebGL2 included.
- Sprite shaders and materials: `Graphics::load_sprite_shader` (hot
  reloaded) and `create_sprite_shader` take WGSL with a
  `fn fragment(p: SpritePixel) -> vec4<f32>` hook; `SpriteMaterial`
  (from `create_sprite_material`, changed with `update_sprite_material`)
  gives it four `vec4` parameters and two extra textures. Add it as a
  component, or use `DrawParams::with_material`. Sprites are batched by
  texture, blend mode and material. Sprite shader errors carry the line in
  the shader's own file and are listed in `shader_status` with
  `ShaderKind::Sprite`; shaders are checked against what WebGL2 supports.
- `LiveCounts::sprite_shaders` and `sprite_materials`; `Ctx::free` and
  `is_live` take the new handles.
- The `sprite_effects` example: an additive glow, an outline and a dissolve.
- C and C++: `PonkoSpriteShader`, `PonkoSpriteMaterial`,
  `PonkoSpriteMaterialDesc`, `PONKO_BLEND_*`,
  `ponko_ctx_load_sprite_shader`, `ponko_ctx_create_sprite_shader`,
  `ponko_ctx_create_sprite_material`, `ponko_ctx_update_sprite_material`,
  `ponko_ctx_sprite_material_desc`, `ponko_ctx_draw_styled`, the
  `ponko_blend_mode_component` and `ponko_sprite_material_component`
  components, the `PONKO_HANDLE_SPRITE_SHADER` and
  `PONKO_HANDLE_SPRITE_MATERIAL` handle kinds and `PONKO_SHADER_SPRITE`;
  in C++ `ponko::sprite_blend`, `sprite_material_of` and the matching `ctx`
  methods.
- A headless sprite drawing benchmark (`ponko_bench sprites/`).

### Changed
- `Graphics::shader_status` and the shader status types
  (`ShaderStatus`, `ShaderOutcome`, `ShaderDiagnostic`, `ShaderKind`) no
  longer need the `3d` feature, and are also exported from `ponko::render`
  without it.
- `Vec4` and `vec4` are in the prelude without the `3d` feature.
