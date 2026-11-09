### Changed
- The 3D frame keeps its models between frames instead of collecting,
  sorting and uploading all of them again: each model keeps its place in
  the list of its mesh in every view that draws it, and a frame works out
  only the models that changed (found by value, however a component was
  written), culls again only when a model or the views changed, and
  uploads only the instances that differ. With 100,000 models, a tenth in
  view (`ponko_bench -- extract-100k`, 14 cores): a still scene collects
  in 0.16 ms instead of 0.61 ms, under a flying camera in 0.31 ms instead
  of 0.62 ms with 25 KiB of instances uploaded per frame instead of
  789 KiB, and with every model moving in 0.50 ms instead of 0.61 ms. The
  first frame builds what later ones keep (1.4 ms instead of 0.5 ms), and
  about 200 bytes are kept per model. See "Models between frames" in the
  performance chapter.
- Models of one mesh are drawn in no particular order; it changes as
  models enter and leave the view.
- The instance buffer is compared and uploaded per instance (changed
  instances with up to 15 unchanged ones between them go in one write),
  not in pages of 32.

### Added
- `ponko_bench`: `extract-100k/still`, `camera`, `mixed`, `dynamic`,
  `edits` and `rebuild` measure the 3D frame collection headless, with the
  instance bytes a frame would upload and the bytes kept between frames.
