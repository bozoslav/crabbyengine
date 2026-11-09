### Added
- Behavior tests for the C API's skeletons and skinning, ragdolls, 3D
  particles, joints, capsule casts and box/capsule overlaps, heightfields,
  the context clock, deferred commands and profiling calls (59 more exported
  functions now have a test that calls them).

### Fixed
- C API: setting a negative simulation speed, giving a particle or trail
  curve or color key outside 0..=1 or out of order, and loading a skeleton from
  a missing file or a file without one now return the invalid-argument status
  with a message instead of the panic status.
