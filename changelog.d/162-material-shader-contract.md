### Added
- Explicit unlit surface shading with `Surface.unlit`, supported across material alpha and face modes and skinned/instanced geometry.
- Public `MaterialVariantKey`, `MaterialSides`, `MaterialInstancing` and `MaterialDesc::variant_key` reuse the renderer's pipeline-set selection without exposing GPU internals.
- Tested guide samples, material feature and pass-data matrices, native contract notes, and GPU feature-combination regressions.

### Fixed
- Material hook signature and engine-owned binding errors now name the contract and fix at the user's WGSL line; comments no longer count as hook definitions.
- Persistent batches and GPU-buffer instances reject skinned meshes with an actionable instruction to use posed model entities.
- GPU-buffer instance setup reports the vertex-storage requirement and the CPU-backed alternative instead of silently drawing nothing on unsupported devices.
- Material shaders reading `items` fail with a located storage-capability diagnostic on unsupported GPUs instead of silently replacing the entire custom shader; comments no longer trigger that fallback.
