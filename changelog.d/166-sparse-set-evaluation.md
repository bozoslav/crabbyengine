### Added
- Benchmarks for sparse, frequently toggled components (#166):
  `cargo run --release -p ponko_bench -- sparse-set/` measures 100,000
  entities with 16 independent optional components at 1, 5 and 20 %
  density, churned every tick, on archetype storage (query index on and
  off), on a benchmark-only sparse-set prototype next to the world, and
  with the components packed as bits of one dense component. It covers
  structural changes, dense and sparse queries, `with`/`without` filters, a
  whole frame, archetype counts and memory, plus a dense case where
  archetypes should win. The prototype is not part of the engine and no
  storage or API changes. Results and the decision to keep archetype
  storage only: `crates/ponko_bench/SPARSE_SET.md`.
