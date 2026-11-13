### Breaking changes
- Each 3D type is documented at one path, its home module: `ponko::render`
  (meshes, materials, models, shaders, LOD, render settings), `ponko::anim`,
  `ponko::terrain`, `ponko::physics3d`, `ponko::audio`, `ponko::nav`;
  `ponko::d3` keeps re-exporting them for `use ponko::d3::*;` and is the home
  of cameras, lights, decals, 3D text, lines, particles, probes and trails.
  Paths that named a type in a module other than these are gone:
  `ponko::render::ParticleCurve` / `ParticleGradient` → `ponko::d3::ParticleCurve` /
  `ParticleGradient`; `ponko::render::two_bone_ik` / `look_at_joint` →
  `ponko::anim::two_bone_ik` / `look_at_joint`.
- `StreamedLod` is documented in `ponko::render` with the other level-of-detail
  types (`ponko::d3::StreamedLod` still names it).
- `BodyKind`, `CollisionStarted` and `CollisionStopped` have a public home,
  the new `ponko::collision` module (with `physics` or `physics3d`); `d2`, `d3`
  and the prelude re-export them as before.
- `ponko::d3::Light` (a second name for `PointLight`) is removed → `ponko::d3::PointLight`.
- Kept: `ponko::d2::Animation` (sprite-sheet animation) keeps its name; it
  doesn't clash with the skeletal `Animation*` types, which all have longer
  names.
- The material blend mode moved out of the animation module:
  `ponko::anim::Blend` → `ponko::render::Blend` (still in `ponko::d3`).
- Renderer internals are no longer public: `MaterialVariantKey`,
  `MaterialSides`, `MaterialInstancing` and `MaterialDesc::variant_key` are
  removed (no replacement; the six pipeline sets are described in the guide).
- The benchmark drivers `ponko::render::{SceneBench, LodBench, RecordBench,
  ExtractionBench, ExtractionSample, ...}` exist only with the new unstable
  `bench-internals` cargo feature, which may change in any release.
- `MaterialDesc`, `ParticleSettings3D` (`d3::ParticleSettings`) and
  `TerrainMaterialDesc` are `#[non_exhaustive]`: struct literals outside the
  crate no longer compile. `MaterialDesc { roughness: 0.9, ..MaterialDesc::color(c) }`
  → `MaterialDesc::color(c).with_roughness(0.9)`;
  `ParticleSettings { rate: 40.0, lifetime: (1.0, 2.0), ..Default::default() }`
  → `ParticleSettings::default().with_rate(40.0).with_lifetime(1.0, 2.0)`;
  `TerrainMaterialDesc { roughness: 0.8, ..TerrainMaterialDesc::new(..) }`
  → `TerrainMaterialDesc::new(..).with_roughness(0.8)`. Every field has a
  `with_*` method (optional fields take the value or `None`:
  `.with_texture(image)`), and fields stay public for reading and assigning.
- `Blend`, `Interpolation`, `TextureKind`, `AnimationParameter`, `Motion`,
  `AnimationCondition`, `LayerBlend`, `ParticleShape3D` and `ParticleBlend3D`
  are `#[non_exhaustive]`: a `match` on them needs a `_` arm.
- Animation graphs use typed ids instead of `usize`: `StateId`, `ParamId` and
  `LayerId` (in `ponko::anim` and `ponko::d3`). Transitions take `StateId`s;
  `AnimationCondition::Greater(0, v)` → `Greater(ParamId(0), v)`;
  `Motion::Blend1D { parameter: 0, .. }` → `parameter: ParamId(0)` (likewise
  `Blend2D { x, y }` and `AnimationState::speed_parameter`);
  `animator.set_float(0, v)` → `set_float(ParamId(0), v)` (likewise
  `set_bool`, `set_trigger`, `parameter`); `crossfade(1, ..)` / `play(1, ..)`
  → `crossfade(StateId(1), ..)` / `play(StateId(1), ..)`; `state()` returns a
  `StateId`; `add_layer` returns and `layer_mut` takes a `LayerId`.
- `AnimationTransition` is `#[non_exhaustive]`:
  `AnimationTransition { from: Some(0), to: 1, conditions: vec![c], exit_time: None, duration: 0.12, offset: 0.0 }`
  → `AnimationTransition::new(StateId(0), StateId(1)).with_condition(c).with_duration(0.12)`
  (`from: None` → `AnimationTransition::from_any(to)`; also `with_exit_time`,
  `with_offset`).
- `AnimationGraph::new(params, states, transitions, 0)` panicked on an
  invalid graph; it now takes the initial `StateId` and returns
  `Result<Arc<AnimationGraph>, AnimationGraphError>`, an error saying which
  state, transition or parameter is wrong:
  `AnimationGraph::new(.., StateId(0))?` (or `.expect(..)`). The C API keeps
  its integer ids and reports the error message.
