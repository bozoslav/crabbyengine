### Changed
- The comparison chapter no longer says systems only run in registration
  order, describes shipped work (the parallel scheduler, declared access,
  custom render passes, GPU tiers, diagnostics) as shipped, links the pages
  other engines now publish, and drops statements about other engines it
  could not cite.
- The "Still Rust-only" table in the C and C++ chapter lists every Rust API
  the guide names as missing from C and C++ (action read-back, built-in step
  declarations, `SystemView`, LOD hysteresis, scene compute passes and target
  effects, keyed audio curves, storage helpers and others).
  `scripts/check-ffi-coverage.py` now fails when the table and
  `scripts/rust-only.txt` disagree, when a listed item disappears from the
  Rust source, or when a C function appears for it.
