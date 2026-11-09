### Added
- GPU tiers: `GpuTier` (`webgl2`, `baseline`, `desktop`) with documented
  floors, the features each may lack (`unavailable_features`) and the report
  of a tier's floor built without a GPU (`GpuTier::report`). The capability
  report gains `tier` and `forced_tier`.
- `PONKO_GPU_TIER=webgl2|baseline` opens the GPU with that tier's limits and
  without timestamp queries, to try a game on a weaker GPU; the offscreen
  render tests take it too, and CI runs them once per forced tier.
- `GraphicsQuality::for_tier`, `GraphicsQuality::recommended` and
  `GraphicsQuality::fitted_to`: quality presets that ask for nothing the GPU
  lacks.
- C/C++: `ponko_ctx_gpu_capabilities` (the report as JSON),
  `ponko_ctx_gpu_tier`, `ponko_ctx_gpu_feature`,
  `ponko_ctx_recommended_graphics_quality`, `ponko_gpu_tier_report`,
  `ponko_gpu_tier_feature`, `ponko_graphics_quality_for_tier`, the
  `PonkoGpuTier` and `PonkoGpuFeature` enums, and the
  C++ `gpu_tier`, `gpu_feature` and `ctx` wrappers.

### Fixed
- Offscreen render suites that built frames without the device's texture
  limit asked for a light shadow atlas larger than WebGL2-class devices
  allow (a validation error); the skinning test now checks the compute
  pass's six storage buffers, as the renderer does.
