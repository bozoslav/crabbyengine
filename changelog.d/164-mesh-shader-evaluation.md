### Added
- An evaluation of mesh shaders and modern GPU geometry paths
  (`crates/ponko_bench/MESH_SHADERS.md`): the support matrix from wgpu and
  naga, a headless probe test that reports which mesh-shader features an
  adapter has and skips cleanly without them, an offscreen meshlet
  prototype kept outside the default renderer, a benchmark of it against
  indexed indirect draws with distance levels (GPU time, CPU time, memory),
  the maintenance and portability cost, and the recommendation (defer) with
  the conditions that would change it. The default renderer and its
  fallbacks are unchanged.
