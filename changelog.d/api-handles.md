### Breaking changes
- Handles can no longer be made from numbers or read as numbers: `Handle::id`
  and `Handle::from_id`, the hidden `id()`/`from_id()` on `Shader`, `Compute`,
  `Effect`, `GpuBuffer` and `InstanceBatch`, and `CollisionMesh::to_bits` /
  `from_bits` are removed. Keep the handle itself (it is `Copy`, `Eq` and
  `Hash`, so it works as a map key), compare handles directly, and use
  `ctx.is_live(h)` to check one. `Handle::NONE` replaces `from_id(u32::MAX)`.
  Asset inventory records (`AssetRecord::id`, `DanglingHandle::id`) still
  report the raw id, for reports only.
- Textures and sounds are passed by value like every other handle:
  `ctx.draw(&t, pos)` → `ctx.draw(t, pos)`, and the same for
  `ctx.gfx.draw`, `draw_ex`, `set_texture_filter` and `update_texture`;
  `ctx.play_sound(&s)` → `ctx.play_sound(s)`, and the same for
  `ctx.audio.play`, `play_ex`, `play_on`, `play_at` and
  `SystemView::play_sound` / `play_sound_ex`.
- `TerrainTile` is opaque: `tile.lods` → `tile.lods()`, `tile.lod_distance` →
  `tile.lod_distance()`, and setting the field → `tile.with_lod_distance(d)`.
  It is a `Handle`: `ctx.gfx.free_terrain_tile(tile)` → `ctx.free(tile)` (or
  `ctx.gfx.free(tile)`), and `ctx.is_live(tile)` is true while all its
  meshes are. Like other handles it is `Eq` and `Hash`, by its meshes: two
  tiles that differ only in `lod_distance` are now equal.
- `CollisionMesh` is a `Handle` like the others:
  `ctx.physics3d.free_mesh(m)` → `ctx.free(m)` (or `ctx.physics3d.free(m)`),
  `ctx.physics3d.is_mesh_live(m)` → `ctx.is_live(m)` (or
  `ctx.physics3d.is_live(m)`), and `CollisionMesh::default()` →
  `CollisionMesh::NONE` (the default was the first mesh made).
- `CollisionMesh` no longer implements `Serialize` / `Deserialize`, so no
  saved number becomes a handle: serde writes the mesh of a `Collider3D` or
  `TerrainCollider` as `null`, reads `null` back as `CollisionMesh::NONE`
  and refuses numbers. Reflection (saves, the editor's copies and undo,
  agents) leaves out a `Collider3D` of a mesh made in code, as it does a
  `Sprite` of a texture made in code, and a reflected mesh collider must
  name its file. Before, such a collider was saved as
  `{"Mesh": {"index": i, "generation": g}}`, and loading it named whatever
  mesh held that slot. Colliders of meshes loaded from files are still saved
  by their file.
- `LiveCounts` has a `collision_meshes` field.

### Fixed
- A freed mesh or GPU buffer handle could name a new resource again once its
  table slot had been reused 256 times, which streaming reaches quickly.
  Meshes, GPU buffers and collision meshes now share one id scheme: a 20-bit
  slot and a 12-bit count of its reuses, and a slot is retired after 4095
  reuses instead of wrapping, so no handle is ever given out twice in a run.
  The C ABI is unchanged: handles are still opaque `uint32_t` ids
  (`PonkoCollisionMesh` keeps its 64-bit field); code that read a slot out
  of an id's bits must stop doing so.
