### Added
- View systems: `Game::add_view(phase, access, system)` registers a system
  that gets a `SystemView` instead of `&mut Ctx`: a `WorldView` of its
  declared components and resources, and checked events, commands and
  services. Anything undeclared panics in every build, naming the system.
- `Service` (input, time, random, gfx, audio, assets, ui, perf, camera,
  spatial, physics, physics3d, net) and `SystemAccess::read_service` /
  `write_service`; services conflict like data.
- `SystemAccess::conflicts_before(later)`: finer, ordered command and event
  rules under a documented batch barrier contract (readers before senders,
  two senders, and earlier world users before command recorders don't
  conflict). The access report uses them, marks enforced (view) systems and
  lists `parallel_pairs()`.
- Declarations by registered name for C and C++ systems:
  `SystemAccess::read_named` / `write_named` / `read_resource_named` /
  `write_resource_named` / `read_events_named` / `send_events_named`.
- C API: `crabby_access_*` (`ponko_access_*`), `CrabbyService`,
  `crabby_game_add_system_with_access`, `crabby_access_conflicts` and
  `crabby_game_access_report`; C++ `crabby::access`, `crabby::service`,
  `game::add(phase, access, fn)` and `game::access_report()`.
- `ponko_ecs`: `ViewAccess`, `ViewState` and `WorldView`, restricted world
  views that non-overlapping systems can share across threads.
- Guide chapter "Declared system access", with the soundness strategy.

### Changed
- `AccessConflict` and `Service` are `#[non_exhaustive]`; `AccessConflict`
  has a `Service` variant. `ReportedSystem` has an `enforced` field.
- The debug access recorder counts replacing a component in place with
  `World::insert_dynamic` as a write, not a structural change.
