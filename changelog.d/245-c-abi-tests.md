### Added
- Behavior tests for the C functions that no test called: headless input,
  cameras, screen rays, the spatial grid, texture arrays and decal textures,
  2D gravity, 3D value helpers and mesh data builders, frame/late systems and
  window settings, shader/effect/compute file loading and reflection probes.
  Native C and C++ tests cover animation graphs, animator layers and
  parameters, blend trees, 3D queries and joint kinds. The control endpoint is tested over a loopback socket. Only 13 of the 627
  exports (drawing, window-only settings, opening a window, a live Steam lobby) stay untested, each
  with its reason in `scripts/ffi-coverage-allowlist.txt`.
- `scripts/check-ffi-coverage.py`, run by CI, fails when a C export has no
  test and no allowlist entry, or when an allowlist entry gets a test.

### Changed
- `Animator::parameter` and `Animator::state_count` read a parameter by index and
  the graph's state count, so callers can check before a setter or crossfade
  that panics on purpose.

### Fixed
- The C animator calls (`animator_set_float/bool/trigger`, `animator_crossfade`
  and the layer versions) returned a panic status, with a panic message on
  stderr, for a parameter of the wrong kind or index, a non-finite value, an
  unknown state or a negative or non-finite crossfade duration or offset. They
  now return `INVALID_ARGUMENT` and leave the animator unchanged.
- `animator_update` from C returned a panic status for a `dt` that is not
  finite or is negative; it now returns `INVALID_ARGUMENT` and leaves the
  animator and its skeleton unchanged.
