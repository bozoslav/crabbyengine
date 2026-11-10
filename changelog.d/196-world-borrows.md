### Added
- `World::resources_mut::<(A, B, C)>()` borrows up to 8 different resources
  mutably at once.
- `Query::with_resources::<(&A, &mut B)>()` carries resources into a query
  loop: `.for_each(|item, (a, b)| ...)`. No more copying values out before a
  query and writing them back after it.
- `Query::with_lookup::<L>()` gives a query loop a `Lookup` for other
  entities: `others.get(entity)` reads and `others.get_mut(entity)` changes
  components the query doesn't write, so "find in one query, change after
  it" no longer needs a collected `Vec`. Overlap between the query and the
  lookup panics once, when the query is built. Writes through `get_mut`
  count as changes for `Changed<T>`, as with `World::get_mut`, so a lookup
  can't write a component the query's own `Added`/`Changed` filter reads
  (that also panics when the query is built).
- `World::get_many_mut::<T, N>([a, b])` borrows one component of several
  different entities mutably.
- None of these allocates, and plain query iteration is unchanged. They are
  Rust-only; C and C++ keep their column and copy-in/copy-out calls.
