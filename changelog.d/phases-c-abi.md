### Added
- C: `ponko_game_add_system(game, phase, name, every, fn, user)` with the
  `PonkoPhase` enum registers a system in any execution phase; C++:
  `game::add(ponko::phase::..., f, name, every)`. Native games can again run
  input and steering before the physics step (`PONKO_PHASE_BEFORE_PHYSICS`).

### Fixed
- The guide no longer says the physics step runs where `.physics()` is
  called; `systems.md` documents the phases. The vehicle example and the C++
  guide drive their wheels from `BeforePhysics`, so inputs act in the same tick.
