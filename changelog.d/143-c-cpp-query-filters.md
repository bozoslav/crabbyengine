### Added
- Query filters from C and C++. C builds a filter from component ids
  (`ponko_filter_with`, `ponko_filter_without`, `ponko_filter_all` for AND
  groups, `ponko_filter_any` for OR over arms, `ponko_filter_always`,
  `ponko_filter_never`, `ponko_filter_free`) and runs a query with it through
  `ponko_query_filtered`, which takes the options of `ponko_query_with`
  (read-only columns, change filters, `parallel`). It visits the entities the
  Rust query with the same `With` / `Without` / `Or` filter visits and has the
  same limits; a filter over them, or one naming a component the world
  doesn't know, returns `PONKO_INVALID_ARGUMENT` (`ponko_filter_check` tests
  one ahead). A component named twice in a filter is one slot and `with` plus
  `without` of one component match nothing, as for a Rust tuple. A filter is
  compiled when a query uses it, into inline storage, so repeated filtered
  queries allocate nothing from C, and the C++ wrapper builds each filter list
  once per world. C++ gets `ponko::with<T>`, `ponko::without<T>`,
  `ponko::or_<...>` and `ponko::all_of<...>` in the list of `each`,
  `each_par`, `query`, `query_par` and `count`. Existing C functions and
  struct layouts are unchanged.
- `DynFilter` in `ponko_ecs` describes the same filters by component id,
  `DynFilter::compile` expands it into the inline form the typed filters use
  (a `FilterError` where the typed path panics), and
  `World::query_chunks_filtered` runs a `ChunkQuery` over the archetypes that
  pass it, with the same matching code as typed queries. `compile_tree`
  compiles a binding's own filter tree without allocating.
- The guide pairs the typed-filter example with a C++ one and describes the C
  calls.

### Performance
- Plain and typed-filter queries are unchanged: `ponko_bench query-filter`
  (1M rows, 16 archetypes, release, median of one run each, alternating the
  `main` build and this branch's on the same machine): plain 0.169 ms on
  `main` and 0.169 ms here, typed `With<Vel>` 0.169 ms and 0.169 ms.
- The C path (`ponko_query_with`, which is `World::query_chunks`) is unchanged
  too, and so is a filtered one, with the filter compiled on every call as
  `ponko_query_filtered` does (a walk of the filter's component numbers into
  inline storage, one `component_at` lookup each). `ponko_bench
  query-filter/c-plain` (raw column pointers per archetype, same workload)
  takes 0.170 / 0.170 / 0.167 / 0.167 ms on `origin/main` and 0.169 / 0.169 /
  0.167 / 0.167 ms on this branch; `c-filtered` (`With`) takes 0.170 / 0.168 /
  0.167 / 0.168 ms and `c-filtered-nested` (`(Pos AND Vel) OR NOT M0`, an AND
  group in an OR) 0.171 / 0.167 / 0.170 / 0.169 ms (four alternating headless runs,
  median). The per-call compile is below what the benchmark can resolve (the
  runs differ by less than 0.003 ms either way).
