### Added
- `TargetEffect` and `Ctx::target_effects`: an effect shader run over the
  lit 3D frame (after the transparent surfaces and TAA, before screen
  effects) that writes a render target of its own and leaves the frame
  unchanged. Screen effects in the same frame can sample the result. An
  entry whose material samples its own target is skipped with a warning.
  See `examples/target_effect.rs`.
