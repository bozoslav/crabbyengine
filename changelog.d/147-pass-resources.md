### Changed

- The 3D renderer's description of its frame passes is finer: each pass lists
  its steps (the render and compute passes and copies inside it), the frames
  each step is recorded in, and how it uses each resource (as an attachment,
  a resolve target, a sampled texture, a copy's side). Resources have a
  format, a sample count and a lifetime. Debug builds check that attachments
  of a step agree on the sample count, resolves and copies match in format,
  nothing is sampled while it is rendered to, every conditional read has a
  writer in every kind of frame, textures have their declared format and
  sample count, resources that share memory are never alive together, each
  part records exactly the steps its pass declares for the frame, and no
  attachment is discarded while a later step reads it. Release builds do
  none of it; command recording costs the same
  (`crates/ponko_bench/RECORDING.md`).

### Added

- `ponko_bench` has an offscreen 3D command-recording benchmark:
  `cargo run --release -p ponko_bench --features render -- render/record`.

### Fixed

- With ambient occlusion, screen-space reflections or motion vectors on, on a
  GPU without a solid depth prepass, the opaque pass no longer discards the
  depth that the occlusion-culling readback reduces right after it.
