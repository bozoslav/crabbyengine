### Added
- `Ctx::load_scene_lockstep`: `load_scene` named for a lockstep game, with
  the same rules, so the call site says the load is part of the shared
  simulation. C: `ponko_ctx_load_scene_lockstep`; C++:
  `ctx.load_scene_lockstep`.
- The guide's "Scenes and prefabs" chapter has a table of what a load
  replaces and keeps, and a section on loading scenes in lockstep.

### Fixed
- In a lockstep session, loads that would change only one peer are now
  refused with an error and change nothing: `load_scene` from frame, late
  and draw systems, and tools' loads (the inspector's Load, the editor's
  Stop, the control endpoint's `scene.load` and `world.load`). Loads in
  setup and in simulation systems, which every peer runs, work as before.
- The editor's Stop keeps its Play snapshot and undo history when the world
  can't be put back, so it can be retried, and it no longer pauses the
  simulation when it is refused.
- A scene load clears the inspector's selection, which could otherwise show
  a different entity that reused the id.
- A scene load forgets which 3D UI panel had the keyboard, so a new panel
  with the old one's entity id no longer gets key input without a click.
- A scene load resets temporal history (TAA, temporal AO, reflections), so
  models of the new scene that reuse old entity ids get no motion from the
  old ones.
