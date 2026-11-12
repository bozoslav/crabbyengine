### Added
- Explicit unlit surface shading with `Surface.unlit`, supported across material alpha and face modes and skinned/instanced geometry.
- Public `MaterialVariantKey`, `MaterialSides`, `MaterialInstancing` and `MaterialDesc::variant_key` reuse the renderer's pipeline-set selection without exposing GPU internals.
- Tested guide samples, material feature and pass-data matrices, native contract notes, and GPU feature-combination regressions.

### Fixed
- Material hook signature and engine-owned binding errors now name the contract and fix at the user's WGSL line; comments no longer count as hook definitions.
- Persistent batches and GPU-buffer instances reject skinned meshes with an actionable instruction to use posed model entities.
