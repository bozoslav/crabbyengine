### Added
- `Ctx::load_scene_lockstep`: loads a scene as a step of a lockstep session,
  from a simulation system or before the session starts, so every peer
  makes the same load at the same tick. C: `ponko_ctx_load_scene_lockstep`;
  C++: `ctx.load_scene_lockstep`.
- The guide's "Scenes and prefabs" chapter has a table of what a load
  replaces and keeps, and a section on loading scenes in lockstep.

### Changed
- `Ctx::load_scene`, and the editor's and control endpoint's scene loads,
  are refused while a lockstep session connects or runs: they would change
  only this peer. Use `load_scene_lockstep`.

### Fixed
- A scene load forgets which 3D UI panel had the keyboard, so a new panel
  with the old one's entity id no longer gets key input without a click.
- A scene load resets temporal history (TAA, temporal AO, reflections), so
  models of the new scene that reuse old entity ids get no motion from the
  old ones.
