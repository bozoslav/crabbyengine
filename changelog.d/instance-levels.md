### Added
- Persistent instance batches support GPU-selected distance levels, per-instance
  sRGB colours, and independent camera and shadow meshes through Rust and C/C++.

### Changed
- Persistent batches share instance records across views, reject bounded batches
  before GPU work, and share one culling compute pass. Limited devices select
  one level per cell; the legacy single-mesh distance behaviour is preserved.

### Fixed
- Shadow distance bands preserve deferred far-cascade refresh; batch updates
  and mesh shadow flag changes invalidate cached sun cascades immediately.
