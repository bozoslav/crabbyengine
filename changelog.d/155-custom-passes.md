### Added
- `ctx.scene3d.passes.scene_compute`: compute passes at a named scene stage, after
  the opaque and transparent models and temporal antialiasing and before
  target effects, screen effects, bloom and tone mapping. Their shaders read
  this frame's lit image and depth (`scene_color`, `scene_position`,
  `scene_distance`); target and screen effects see their results in the
  same frame. Texture and GPU buffer passes both work; on a GPU without
  compute shaders they are refused at creation (texture) or do nothing
  (buffer), and the frame draws as without them. Rust only for now.
- The game's passes (compute passes of both stages, target cameras and
  target effects) are checked every frame for resource hazards: a target
  effect sampling its own target, or a target camera or target effect
  clearing a render target another pass wrote earlier in the frame, is
  skipped with a warning naming the entry (its list, index and shader) and
  the render target. Frames without custom passes skip the check.
- `examples/scene_compute.rs`: eye adaptation from a scene compute pass
  read by a screen effect in the same frame; and a "Custom passes" section
  in the 3D guide with the stages, ordering, resource rules and fallbacks.

### Changed
- A target effect whose target another pass wrote earlier in the frame
  is now skipped (it used to silently replace that result); the warning
  for a target effect sampling its own target names its list and index.
- A `ctx.scene3d.passes.compute` entry whose render target a target camera
  clears later in the same frame is now skipped with a warning naming the
  entry and the target (it used to run, and its result was lost).

### Fixed
- Compute pass shaders are built for the frame's sample count, so one that
  reads the scene depth works with multisampling off.
