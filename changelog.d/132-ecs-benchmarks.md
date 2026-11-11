### Added
- ECS benchmark matrix in `ponko_bench` (`-- ecs/`): ponko_ecs, bevy_ecs
  and hecs at 1k/100k/1M entities for iteration, parallel scaling (1 to all
  threads), spawn loops and batches, spawn/despawn and add/remove churn,
  optional and filtered queries, and a game-like tick with four layouts and
  five systems. Methodology, limits and baselines in
  `crates/ponko_bench/ECS.md`; correctness is tested separately.
- Every benchmark result reports heap allocations per timed run (console and
  `PONKO_BENCH_JSON`), and the JSON records the load average and bevy_ecs
  features. hecs joins the many-archetype and query-construction cases.
- `query-build+count-cold/*`: the first query of a fresh world, the one row
  where ponko_ecs, bevy_ecs and hecs all match every archetype (hecs caches
  matches per world). `ecs/par-*/ponko-par` runs `par_for_each` on rayon's
  global pool without `ThreadPool::install`.
- The benchmark filter accepts alternatives: `-- 'iter|spawn'`.
