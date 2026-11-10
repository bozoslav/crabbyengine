### Added
- `Graphics::pipeline_stats()` and `PipelineStats`: the render pipeline cache's
  material shaders, variant sets and pipelines alive, hits and misses,
  shaders that shared an identical one's pipelines, build and compile counts
  with total, slowest and last times, sets invalidated by reloads, and the
  WGSL bytes kept as keys. `perf.get`'s `render_inspector` (the
  `perf_report` MCP tool) has them as `pipelines`.

### Changed
- Material shader pipelines are kept in a cache keyed by the shader's code,
  the GPU features composed into it, the target formats and the sample
  count. Shaders with the same code compile once and share their pipelines,
  a hot reload drops only the reloaded shader's pipelines (other shaders,
  including ones sharing the old code, keep theirs), and freeing a shader
  releases its pipelines. Drawing is unchanged.
