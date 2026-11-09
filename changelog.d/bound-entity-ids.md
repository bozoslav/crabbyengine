### Fixed
- `World::load` (and so scene loading, `ponko_world_load`,
  `ponko_ctx_load_scene` and the control endpoint's `scene.load`) rejects
  saves whose entity ids have an index far beyond what the save needs, with an
  error and the world unchanged, instead of allocating an entity table sized by
  the id. Indices must be below the larger of the world's current
  `entity_slots()` and 8 × the entities in the save, plus 2^22 spare slots.
  Restoring an entity id far past the slot table also grows it in one step.

### Added
- `World::entity_slots()`: the size of the world's entity slot table.
