### Added
- `RenderStats::uploads` (`GpuUploads`): the bytes the renderer uploaded to
  the GPU last frame, by kind (model transforms, sprites, instance
  batches, uniforms, lights, materials, skins, meshes, textures, other),
  and the buffers it created. Benchmark reports and the control
  endpoint's `perf_report` include them.

### Changed
- The 3D renderer uploads only the changed parts of the model list each
  frame, and skips light and decal data that didn't change: a still scene
  of 256 shadowed models went from about 103 KiB uploaded per frame to
  15 KiB of uniforms.
