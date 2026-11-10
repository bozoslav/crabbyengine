### Added
- Change detection for components, opt-in per component: the query filters
  `Changed<T>` and `Added<T>` (on `query.filter::<F>()`, also inside `Or`
  and tuples) match only the entities whose `T` changed or was added, and
  `world.removed::<T>()` lists the entities that lost it. In a game, each
  system sees what happened since it last ran, so every system sees each
  change once. A mutable borrow (`get_mut`, `&mut T` in a query) counts as a
  change. See "Change detection" in the ECS chapter of the guide for what
  counts on every path: spawning, inserting, replacing, removing, commands,
  dynamic components, saving and loading.
- `World::track_changes`, `track_changes_dynamic`, `mark_changed`,
  `mark_changed_dynamic`, `removed`, `removed_since`, `removed_dynamic`,
  `forget_removed`, `change_tick`, `advance_change_tick`, `changes_since`,
  `set_changes_since`, `Query::since`, `Tick`,
  `ComponentInfo::tracks_changes`, and `World::query_chunks` with
  `ChunkQuery` (a chunk query that says which columns it writes and which
  must have changed).
- `World::id()`: a number unique per world. The game loop uses it to start
  every system's change window over when `ctx.world` is replaced, so a
  system that runs rarely still sees everything in the new world.
- C: `ponko_query_with` with `PonkoQueryOptions`, `ponko_component_track_changes`,
  `ponko_mark_changed`, `ponko_removed`, `ponko_world_change_tick`,
  `ponko_world_advance_change_tick`, `ponko_world_changes_since` and
  `ponko_world_set_changes_since`. C++: the filters `ponko::changed<T>`,
  `ponko::added<T>` and `ponko::any_of<...>` for `each`, `each_par`,
  `query`, `query_par` and `count`, `const` columns in `query`, and
  `track_changes`, `mark_changed`, `removed`, `removed_since`,
  `change_tick`, `advance_change_tick`, `changes_since`,
  `set_changes_since` on the world.
- `ponko_bench` measures tracked and untracked worlds side by side
  (`cargo run --release -p ponko_bench -- changes`).

### Changed
- Tracking costs 8 bytes per entity for each tracked component and nothing
  for the others: a world that tracks nothing keeps no stamps, its columns
  and archetypes keep their size, and its query loops are the ones from
  before. `Archetype::bytes_per_entity` and `World::allocated_bytes` count
  the stamps of tracked components.
- In C++, an `each` parameter taken as `const T&` or by value, and a
  `const` column of `query`, read a tracked component without marking it
  changed; `T&` marks it. `ponko_query` and `ponko_query_par` mark every
  tracked column they hand out.
- For a tracked component, every mutable fetch marks it changed, not only
  `for_each`: `Query::get`, `Query::iter` and the parallel loops with
  `&mut T` or `Option<&mut T>` mark the entities they reach, written or
  not, and so do a view query and `WorldView::get_mut`. Fetch `&T` where
  you only read. Untracked components are not affected.
- For declared-access checks, an `Added<T>` or `Changed<T>` filter counts
  as reading `T`.
