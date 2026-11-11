### Changed
- The 3D renderer's internals are split into subsystems that each own their
  pipelines, GPU objects and state (resources, material shaders, targets,
  shadow maps, pass preparation, pass encoding, temporal history, game
  effects, the post chain), with the ownership map in `ARCHITECTURE.md`.
  No public API, pass order or image changes; a whole-frame benchmark
  before and after is in `crates/ponko_bench/RENDERER_FRAME.md`.
