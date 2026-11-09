### Changed

- The 3D renderer describes its frame passes as data: what each runs after,
  reads (this frame's or last frame's contents) and writes. Debug builds check
  the table for cycles and unordered hazards, and every frame's pass order
  against it, with messages naming the passes and resource. Release builds and
  the recorded frame are unchanged.
