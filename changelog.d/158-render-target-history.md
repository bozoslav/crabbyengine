### Added
- Rust `Graphics::set_msaa` switches supported 2x/4x sample counts at runtime,
  rebuilding material/sky/particle pipelines and camera attachments while
  preserving assets. Unsupported counts and pending shader checks return false.

### Fixed
- Scene target freshness includes MSAA sample count, so sample changes reset
  TAA, temporal AO, SSR and motion through the central history decision.
- Successful shader reloads and asynchronous rollbacks reset shading history,
  preventing stale pixels from blending into newly compiled materials/effects.
- Occlusion, suspension/resumption and lost/outdated/suboptimal surface recovery
  invalidate temporal targets even at unchanged dimensions. Offscreen captures
  remain available while hidden or suspended, using the last nonzero size.
- GPU regressions cover repeated resize, render-scale/effect toggles and shader
  reload, validate retired-resource bindings, and verify that target cameras
  render current pixels without temporal history.
