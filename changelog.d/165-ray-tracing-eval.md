### Added
- A decision note on hardware ray tracing (`crates/ponko_bench/RAY_TRACING.md`):
  what wgpu's ray queries and acceleration structures offer per backend, the
  measured blind spots of screen-space reflections and probe global
  illumination, an isolated ray-query reflection prototype with its GPU cost
  and memory, and the fallback design. The recommendation is to defer; the
  renderer is unchanged.
- Offscreen tests behind it: an adapter probe that prints the ray tracing
  features and limits, reflections losing geometry above the screen or
  behind a wall, bounced light leaking through a thin partition on a probe
  plane, and the prototype (skipped without `EXPERIMENTAL_RAY_QUERY`).
