### Breaking changes
- **Components opt in.** A type is no longer a component just because it is
  `Send + Sync + 'static`: `struct Health(f32);` →
  `#[derive(Component)] struct Health(f32);` (or `impl Component for Health {}`).
  The derive comes with `ponko::prelude::*`, `ponko::Component` and
  `ponko_ecs::Component` (trait and derive share the name). Using a type that
  isn't a component in `spawn`, `insert`, a query or a filter is a compile
  error: "`T` is not a component", with the fix in the note.
- **Built-in types are not components.** A tuple inserted as one component,
  a bare number, `String`, `&str`, `Vec<..>`, `Arc<..>` or `Entity` no longer
  compiles as a component: `world.spawn((5u32,))` →
  `#[derive(Component)] struct Score(u32);` and `world.spawn((Score(5),))`;
  `world.insert(e, target_entity)` → `world.insert(e, Target(target_entity))`
  with `#[derive(Component)] struct Target(Entity);`.
- **`ponko_ecs` has a `derive` feature**, on by default, which brings in the
  new `ponko_macros` crate. With `default-features = false`, add
  `features = ["derive"]` to keep `#[derive(Component)]`, or implement the
  trait by hand.

### Added
- `ponko_macros`: `#[derive(Component)]`, re-exported by `ponko` and
  `ponko_ecs`. It works through either crate, also under a renamed
  dependency, and for generic types (a component for every argument that is
  `Send + Sync + 'static`). `#[component(..)]` is reserved for future options
  and rejected for now.

### Changed
- Resources, events and the C/C++ API are unchanged: resources and events
  need no derive, and C/C++ components are still registered by name and
  size, not as Rust types.
