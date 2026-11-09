### Changed
- `Character3D`'s step reuses its per-character buffers (the dynamic bodies a
  character pushed) instead of allocating a new `Vec` for every character
  every tick. What remains is rapier's own: its character controller builds
  fresh contact lists on each move.
