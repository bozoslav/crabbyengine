### Deprecated
- `ponko::stream` merged into `ponko::streaming`: `stream::Job` and
  `stream::CellStreamer` are `streaming::Job` and `streaming::CellStreamer`.
  The old names stay as deprecated aliases until 0.3.0.

### Breaking changes
- The `ponko::physics` module is gone: 2D physics lives in `ponko::d2`.
  `physics::{RigidBody, Collider, Velocity, Shape, Physics, QueryFilter}`
  were also public as `d2::*`; `physics::RayHit` is `d2::Hit`,
  `physics::MAX_POLYGON_POINTS` is `d2::Shape::MAX_POLYGON_POINTS`, and the
  step systems are in `ponko::systems`. The types are the same; saved
  component names (`RigidBody`, `Collider`, ...) don't change.
  Instead: import from `ponko::d2` (or `ponko::systems` for the systems).
  Before:
  ```rust
  use ponko::physics::{Collider, RayHit, RigidBody, MAX_POLYGON_POINTS};
  ```
  After:
  ```rust
  use ponko::d2::{Collider, Hit, RigidBody, Shape};
  let most = Shape::MAX_POLYGON_POINTS;
  ```
- The `ponko::physics3d` module is gone: 3D physics lives in `ponko::d3`,
  under the names `d3` already gave it (`Collider3D` is `d3::Collider`,
  `RigidBody3D` is `d3::RigidBody`, `Physics3D` is `d3::Physics`, and so on
  for `Character3D`, `ContactModifier3D`, `ContactPair3D`, `Hit3D`,
  `Joint3D`, `JointAxis3D`, `JointDrive3D`, `JointMotion3D`,
  `QueryFilter3D`, `Ragdoll3D`, `Shape3D`, `Vehicle3D`, `Velocity3D` and
  `Wheel3D`). `CollisionGroups`, `CollisionMesh`, `CollisionMeshData`,
  `CollisionMeshOptions`, `ColliderFocus`, `FrictionCurve`, `RagdollBone`
  and `TerrainCollider` keep their names in `d3`; `PHYSICS3D_HZ` is
  `d3::Physics::HZ`; the step and interpolation systems are in
  `ponko::systems`. Saved component names (`RigidBody3D`, ...) don't change.
  Instead: import from `ponko::d3`.
  Before:
  ```rust
  use ponko::physics3d::{Collider3D, CollisionGroups, PHYSICS3D_HZ};
  ```
  After:
  ```rust
  use ponko::d3::{Collider, CollisionGroups, Physics};
  let hz = Physics::HZ;
  ```
- `ponko::d3` no longer re-exports the types of topic modules, so each type
  has one path: render types (`Material`, `MaterialDesc`, `Mesh`,
  `MeshData`, `Model`, `ModelMaterial(s)`, `Shader*`, `Lod*`, `Post`, `Sky`,
  `ComputePass`, `ScreenEffect`, `TargetCamera`, `StreamedLod`, ...) are in
  `ponko::render`, animation types (`Animator`, `AnimationClip`,
  `AnimationGraph`, `Skeleton`, `SkinOf`, `MorphWeights`, ...) in
  `ponko::anim`, terrain types (`Heightfield`, `TerrainTile`,
  `TerrainTileData`, `TerrainMaterialDesc`, `TextureArray`, `TextureKind`,
  `FarTerrain`) in `ponko::terrain`, `ParticleCurve`, `ParticleGradient` and
  `ParticleSpace` in `ponko::particles`, and `BodyKind`, `CollisionStarted`
  and `CollisionStopped` in `ponko::collision`. `d3` keeps the types named
  after the dimension (`Transform`, `Camera`, lights, decals, text, trails,
  particle emitters, positional audio, navigation agents, `UiPanel`) and the
  3D physics. `Model` stays in the prelude.
  Instead: import these types from their module next to `use ponko::d3::*;`.
  Before:
  ```rust
  use ponko::d3::*;
  let desc = MaterialDesc::color(Color::WHITE);
  let mesh = ctx.gfx.create_mesh(&MeshData::cube(1.0));
  ```
  After:
  ```rust
  use ponko::d3::*;
  use ponko::render::{MaterialDesc, MeshData};
  let desc = MaterialDesc::color(Color::WHITE);
  let mesh = ctx.gfx.create_mesh(&MeshData::cube(1.0));
  ```
- `ponko::d2` no longer re-exports the types of topic modules: tile maps and
  levels (`TileMap`, `Tileset`, `Level`, `LevelOptions`, `LevelObject`,
  `AutoTile`, ...) are in `ponko::tilemap`, `Animation` and `SpriteSheet` in
  `ponko::sheet` (`SpriteSheet` stays in the prelude), `Attached` in
  `ponko::attach`, `SpatialGrid` in `ponko::spatial`, `NavGrid` and
  `FlowField` in `ponko::flow`, `BlendMode`, `SpriteMaterial`,
  `SpriteMaterialDesc` and `SpriteShader` in `ponko::render`, the particle
  curves in `ponko::particles` and the collision types in
  `ponko::collision`. `d2` keeps `Transform`, `Camera`, `Sprite`, the
  particle emitter types and the 2D physics.
  Instead: import these types from their module.
  Before:
  ```rust
  use ponko::d2::*;
  let child = (Transform::default(), Attached::to(parent));
  let map = TileMap::new(tiles, 32, 32);
  ```
  After:
  ```rust
  use ponko::d2::*;
  use ponko::{attach::Attached, tilemap::TileMap};
  let child = (Transform::default(), Attached::to(parent));
  let map = TileMap::new(tiles, 32, 32);
  ```
