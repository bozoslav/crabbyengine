### Changed
- The 3D frame keeps its models between frames instead of collecting,
  sorting and uploading all of them again: each model keeps its place in
  the list of its mesh in every view that draws it, and a frame works out
  only the models that changed (found by value, however a component was
  written), culls again only when a model or the views changed, and
  uploads only the instances that differ. With 100,000 models, a tenth in
  view (`ponko_bench -- extract-100k`, 14 cores, and one core in
  brackets): a still scene collects in 0.20 ms instead of 0.45 ms (0.41
  instead of 1.21), under a flying camera in 0.29 ms instead of 0.44 ms
  (0.85 instead of 1.20) with 25 KiB of instances uploaded per frame
  instead of 789 KiB, and with 100 models despawned and spawned per frame
  in 0.47 ms instead of 0.48 ms (1.13 instead of 1.40).
- What that costs. With every model moving every frame, collecting takes
  as long as before on 14 cores (0.45 ms) and about 9% longer on one core
  (1.31 ms instead of 1.20 ms). The first frame builds what later ones
  keep: 1.33 ms instead of 0.49 ms (2.66 instead of 1.25 on one core).
  About 150 bytes are kept per model in the world on one core and 210 on
  several, and again for every target camera with a 3D view and for global
  illumination's capture, which keep their own. See "Models between
  frames" in the performance chapter.
- Models of one mesh are kept in no particular order; it changes as
  models enter and leave the view. `Blend::Transparent` models of one mesh
  are now drawn far to near (by their origins) every frame, so overlapping
  ones blend in the right order whatever order they are kept or were
  spawned in. Different meshes are still not sorted against each other.
- `ctx.gfx.models_drawn()` counts each model once: those the camera could
  see and those out of view whose shadow can fall into it, including
  models in a shadow cascade whose map is kept this frame. It used to
  count an instance for every view that drew a model (the camera and each
  shadow map), so a model that was seen and cast a shadow counted more
  than once.
- The instance buffer is compared and uploaded per instance (changed
  instances with up to 15 unchanged ones between them go in one write),
  not in pages of 32.

### Added
- `ponko_bench`: `extract-100k/still`, `camera`, `mixed`, `dynamic`,
  `edits` and `rebuild` measure the 3D frame collection headless, with the
  instance bytes a frame would upload and the bytes kept between frames.
