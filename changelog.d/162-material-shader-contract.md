### Added
- Explicit unlit surface shading with `Surface.unlit`, supported across material alpha and face modes and skinned/instanced geometry.
- Public `MaterialVariantKey`, `MaterialSides`, `MaterialInstancing` and `MaterialDesc::variant_key` reuse the renderer's pipeline-set selection without exposing GPU internals.
- Tested guide samples, material feature and pass-data matrices, native contract notes, and GPU feature-combination regressions.

### Changed
- WGSL `Surface` gains an `unlit: bool` field after `emissive`. Custom material shaders that build a `Surface(...)` with every field listed in order must add the new boolean; hooks that copy their input (`var out = s`) are unaffected.
- Custom material shaders that declare engine-owned `@group`/`@binding` resources, or add their own stage-entry attributes, are now rejected when loaded, with a located error naming the rule, instead of failing later on the GPU.

### Fixed
- Material hook signature and engine-owned binding errors now name the contract and fix at the user's WGSL line; comments no longer count as hook definitions.
- Persistent batches and GPU-buffer instances reject skinned meshes with an actionable instruction to use posed model entities.
- GPU-buffer instance setup reports the vertex-storage requirement and the CPU-backed alternative instead of silently drawing nothing on unsupported devices.
- Material shaders reading `items` fail with a located storage-capability diagnostic on unsupported GPUs instead of silently replacing the entire custom shader; comments no longer trigger that fallback.
