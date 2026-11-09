### Added
- Typed query filters: `query.filter::<F>()` takes `With<T>`, `Without<T>`,
  `Or<(...)>` and tuples of them, so a query can match "players or enemies"
  and a filter can be named once (`type Living = ...`) and reused. Filters
  stay inline, so filtered queries still don't allocate. `With`, `Without`,
  `Or` and `QueryFilter` are re-exported from `ponko`, and `.cached()`
  queries cache typed filters too.
- Queries fetch tuples of up to 12 items (was 8), matching bundles; nest
  tuples for more.

### Changed
- A query takes up to 8 `with` and 8 `without` components (was 4 each);
  repeating the same component no longer uses up a slot.
