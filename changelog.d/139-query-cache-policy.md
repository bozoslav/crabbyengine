### Added
- Queries find their archetypes through a world-owned index automatically
  (#139). In a world with at least 64 archetypes, a query signature used
  twice that matches at most half of the archetypes is indexed and skips
  the rest; smaller worlds and denser queries scan as before. Results,
  order, change detection and determinism are the same either way. The
  index is bounded (128 signatures, 1 MiB by default), drops signatures no
  longer used, follows new archetypes, and allocates nothing once warm.
- The index key covers the whole filter: `with`, `without`, `Or`, AND
  groups inside `Or`, nested tuples and runtime `DynFilter`s, normalized so
  equivalent filters share one entry. Chunk queries
  (`World::query_chunks`, `query_chunks_filtered`, `raw().for_each_chunk`)
  and world views use it too; views read it without locks.
- `Query::uncached()` and `ViewQuery::cached()`/`uncached()`,
  `World::query_chunks_cached` with a `QueryCache` hint, and
  `World::set_query_cache_policy`, `query_cache_policy` and
  `query_cache_stats` (`QueryCachePolicy`, `QueryCacheStats`) to tune,
  switch off and observe the index per world. A `max_match_percent`
  above 100 is kept as 100, in Rust and from C.
- C: `ponko_query_cached` with `PONKO_QUERY_CACHE_AUTO`/`ALWAYS`/`NEVER`,
  `PonkoQueryCachePolicy`, `PonkoQueryCacheStats`,
  `ponko_query_cache_policy_default`/`_off`,
  `ponko_world_set_query_cache_policy`, `ponko_world_query_cache_policy`
  and `ponko_world_query_cache_stats`. C++: `ponko::cached` and
  `ponko::uncached` in the list of `each`, `query` and `count`, and
  `world.set_query_cache_policy`, `query_cache_policy` and
  `query_cache_stats`. Existing C queries use the index automatically.

### Changed
- `Query::cached()` now indexes a query on first use and keeps dense
  signatures indexed; it no longer is the only way to use the index, and
  the index starts at 64 archetypes instead of 128.
