### Added
- Symmetric `d2`/`d3` type names, filtered ray and ball-overlap queries in both dimensions, and 3D prefab placement. Existing names, query signatures and serialized component names remain available.
- Additive native filtered 2D queries, C++ common filtered query spellings, and position/full-transform prefab placement APIs; existing flat native names and brace-initialized 2D calls remain compatible.

### Changed
- `Ctx::spawn_prefab` takes `impl Into<Placement>` (a `Vec2`, `Vec3`, `d2::Transform` or `d3::Transform`); `Placement` is `#[non_exhaustive]`. Arguments whose type used to be inferred as `Vec2` (`Default::default()`, `(x, y).into()`) must now name it (`Vec2::ZERO`, `vec2(x, y)`).
- 2D `overlap_circle`/`overlap_ball` treat a radius below `f32::EPSILON` (or NaN) as that tiny circle, and 2D ray casts from a non-finite origin hit nothing, as in 3D.

### Deprecated
- The `3D`-suffixed root names (`RigidBody3D`, `Collider3D`, `Physics3D`, `Hit3D`, `PointLight3D`, `Decal3D`, `Text3D`, ...) and the bare 2D physics names at the root (`RigidBody`, `Collider`, `Velocity`, `Shape`, `Physics`, `RayHit`), in favour of `d3::*` and `d2::*`. `Transform2D`, `Transform3D`, `Camera2D` and `Camera3D` stay in the prelude, undeprecated. The full list is in the #200 fragment and the naming chapter; rustc does not warn on these re-exports.
