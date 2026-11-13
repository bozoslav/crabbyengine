### Added
- 2D particles, without the `3d` feature: `d2::ParticleEmitter` next to a
  `d2::Transform` spawns particles at a rate or in bursts from a point,
  circle, cone or rectangle, moves them with gravity and drag, and draws them
  in the sprite batch at the emitter's `z`, with a sprite `BlendMode` and an
  optional `SpriteMaterial`. Settings (`d2::ParticleSettings`) use the 3D
  emitter's vocabulary in pixels: lifetime, speed, size and rotation ranges,
  size curves, color gradients, sprite sheets, world or local space, a pool
  limit and a seed. About 100,000 living particles take 0.84 ms of CPU per
  frame on an Apple M4 Max (`ponko_bench particles2d`). The guide's Drawing
  chapter has a "Particles" section and the `particles2d` example.
- `ponko::particles`, the home of `ParticleCurve`, `ParticleGradient` and
  `ParticleSpace`, shared by the 2D and 3D emitters (still in `d3` too), with
  `ParticleCurve::keys` and `ParticleGradient::keys`.
- `Systems::DRAW_PARTICLES`, the built-in system that draws 2D particles
  after the sprites (`systems::draw_particles`; C `PONKO_SYSTEM_DRAW_PARTICLES`,
  C++ `systems::draw_particles`).
- 2D emitters are reflected as `"ParticleEmitter2D"` (inspector, MCP tools,
  scenes), the image saved by file name like a sprite's.
- C and C++: `PonkoParticleSettings2D`, `ponko_particle_settings2d_default`,
  `ponko_ctx_set_particle_emitter2d`, `ponko_ctx_particle_burst2d`,
  `ponko_ctx_particle_live_count2d`, `ponko_ctx_particle_update2d` and
  `ponko_ctx_remove_particle_emitter2d`; C++ `particle_settings2d` and the
  matching `ctx` methods.

### Changed
- `Systems::UPDATE_PARTICLES` (`systems::update_particles`) is installed
  without the `3d` feature too, and advances 2D emitters as well as the 3D
  ones and trails.
