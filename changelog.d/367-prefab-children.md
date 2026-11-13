### Added
- Prefabs can hold children: a version-2 prefab file lists a tree of
  entities, root first, with ids its components use to name each other.
  `Ctx::spawn_prefab` spawns the tree relative to the root, returns the
  root, and remaps every `Entity` value that names an entity of the prefab
  (attachments, joints, skin targets, game components) to the new copies.
  Version-1 files still load and are still written for single entities.
- `PrefabParts` (reflected, in the prelude) lists the parts a prefab root
  was spawned with; `Ctx::despawn_prefab` removes a root with its parts.
- A prefab entity may include another prefab (`"prefab": "<file>"`), with
  its `"components"` changing the included root; loops are refused.
- `Assets::save_prefab` saves an entity's prefab parts and what is attached
  to them (`Attached`, `SkinOf`, `AttachToJoint`, 3D joints between two of
  them), with transforms relative to the root.
- `Entity::with_remap` and `Entity::remapped` in `ponko_ecs`: read saved
  entity ids as other ones while deserializing.
- Control endpoint `prefab.despawn` and MCP tool `despawn_prefab`;
  `prefab.spawn` / `spawn_prefab` also return the root's `parts`.
- C: `ponko_ctx_prefab_parts` and `ponko_ctx_despawn_prefab`; C++:
  `ctx.prefab_parts(root)` and `ctx.despawn_prefab(root)`.
- The `prefabs` example (carts with included wheels), runnable headless.

### Changed
- `SkinOf`, `AttachToJoint` and (with 3D physics) `Joint3D` are reflected,
  so scenes, saved worlds and prefabs keep them; their types implement
  serde's `Serialize` and `Deserialize`.
- The editor's undo of a placed prefab removes its parts too.
