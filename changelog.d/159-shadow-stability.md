### Changed

- Keep sun shadow cascades across small camera movements when texel-snapped
  projections and caster content remain exactly unchanged, with stable
  camera-space bounds and direct light-space translation.
- Reuse shadows from custom material functions whose inputs are tracked;
  retain conservative redraws for time, view, GPU buffer, texture array and
  render-target inputs.
  Invalidate texture and shader changes only in views using affected materials.
- Compare uploaded texture alpha exactly so RGB-only changes and updates to
  normal, ORM and emissive maps retain built-in cutout shadows. CPU-uploaded
  textures retain one byte per texel for this comparison.

### Fixed

- Refresh retained shadow depth when a cached custom vertex shader is freed
  or a cutout texture is freed and rebound to its fallback.
