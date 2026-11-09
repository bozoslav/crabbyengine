### Fixed
- Background `Job`s no longer stall the frame's parallel work (`par_for_each`,
  3D scene collection and other rayon work). Jobs now run on their own pool of
  half as many threads as cores (at least one), so a frame never waits for a
  long job to free a thread. On the web, jobs still run when spawned.
