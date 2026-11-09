### Added
- Residency budgets: `Ctx::set_residency_budget(ResidencyKind::Mesh, Some(bytes))`
  caps the estimated GPU bytes of meshes loaded with `Ctx::resident_mesh` (and
  `try_resident_mesh`). Over budget, the least recently used meshes are freed
  first; pinned ones (`pin_resident_mesh`), ones entities show and ones asked
  for in the current frame and tick are kept. Evicted meshes load again when
  asked for. `Ctx::residency` returns `ResidencyStats` (budget, resident and
  pinned bytes, loads, reloads, evictions), also in the control endpoint's
  `perf.get` answer under `residency`.
