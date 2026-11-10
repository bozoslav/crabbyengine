### Added
- Instance discovery now includes GPU adapter/backend, limits and fallback data from the renderer's capability report.
- Explicit schemas for built-in reflected components and additive C/C++ APIs for describing component and resource schemas, with native and offscreen tests.
- `Ctx::capabilities_json`, `ponko_ctx_capabilities_json` and C++ `ctx.capabilities_json()`: the `capabilities` answer for the running game, read in-process.
- Built-in schemas for `ModelMaterial` and `ModelMaterials`.
- Discovery JSON Schemas list `required` fields, and objects that refuse unknown fields (sprites, models, LOD groups, enum variants) say `"additionalProperties": false`, so a validator can tell the variants of `Shape`, `Shape3D` and `LodShadow` apart.

### Changed
- `ValueSchema`'s constructors match its variants: `enum_of` builds `Enum` (it was `one_of`), `any_of` builds `AnyOf` (the choice between schemas, formerly `OneOf`), and the new `strict_object` builds `StrictObject`. None of these names shipped in a release.
- A mesh collider's file form in the `Collider3D` schema marks `scale` and `convex` optional, as scenes may leave them out.

### Fixed
- The polygon and vehicle-wheel schemas follow `MAX_POLYGON_POINTS` and `Vehicle3D::MAX_WHEELS` instead of a hard-coded 8.
