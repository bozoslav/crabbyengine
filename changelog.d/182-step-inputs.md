### Added
- The physics and navigation steps declare what they touch:
  `systems::step_physics_access()`, `step_physics3d_access()`,
  `interpolate_physics3d_access()` and `step_navmesh3d_access()` return the
  `SystemAccess` of each step (its service, components, resource and
  events). `Game::physics` and `Game::physics3d` register their systems
  with these declarations, and so does `ponko_game_navmesh3d` /
  `game::navmesh3d()` for C and C++ games: `Game::access_report()` lists
  them, and debug builds check that the steps stay inside them. The steps
  keep `&mut Ctx`, their ids, phases and place in the order.
- `World::record_access_into(log)` starts access recording in the buffers
  of an earlier `AccessLog`.
- The guide's "Systems and the frame" chapter has a section on the order of
  physics, navigation, character motion and interpolation, with each rule
  held by a test in `tests/step_inputs.rs`.

### Changed
- The debug check of a declared `Ctx` system (`Game::add_with_access`)
  reuses its logs, so a checked system no longer allocates on every run.

### Fixed
- The 3D physics guide said every simulation system sees the stepped pose.
  In a window, a simulation system that runs before the physics step reads
  the last drawn blend from `d3::Transform`; `ctx.physics3d.position(e)` is
  the stepped pose everywhere.
