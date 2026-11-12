### Changed
- Share immutable 3D mesh geometry in reusable vertex/index arenas on every GPU
  tier. Sort opaque mesh parts by their resolved materials, and order equal-state
  runs front to back without a full depth prepass while retaining instancing.
- Include model-pass state changes in `perf.get`'s schema-1 render inspector and
  frame benchmark reports. Add a single-material rendering benchmark scene.

### Fixed
- Sort transparent models globally back to front across meshes and material
  overrides, splitting instance runs only where blending order requires it.
