### Added
- Behavior tests for the C functions that no test called: headless input,
  cameras, screen rays, the spatial grid, texture arrays and decal textures,
  2D gravity, 3D value helpers and mesh data builders, frame/late systems and
  window settings, shader/effect/compute file loading and reflection probes.
  Native C and C++ tests cover animation graphs, animator layers and
  parameters, blend trees, 3D queries and joint kinds. Only 10 of the 627
  exports (drawing, opening a window, a live Steam lobby) stay untested, each
  with its reason in `scripts/ffi-coverage-allowlist.txt`.
- `scripts/check-ffi-coverage.py`, run by CI, fails when a C export has no
  test and no allowlist entry, or when an allowlist entry gets a test.
