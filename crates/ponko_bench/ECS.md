# ECS benchmarks: methodology and baselines

The ECS matrix compares `ponko_ecs` with `bevy_ecs` and `hecs` on the same
operations at 1k, 100k and 1M entities, plus many small archetypes, query
construction and one game-like tick. The code is
[`src/ecs_suite.rs`](src/ecs_suite.rs) and [`src/archetypes.rs`](src/archetypes.rs).

## The command

```sh
PONKO_BENCH_JSON=crates/ponko_bench/baselines/ecs.json \
  cargo run --release -p ponko_bench -- 'ecs/|archetypes-1024|query-build'
```

It prints one line per result and writes every result, with the build and
machine context, to [`baselines/ecs.json`](baselines/ecs.json) (schema in the
[README](README.md#machine-readable-results)). Re-recording the baselines is
this one command on a clean, committed checkout (so `git_dirty` is `false`),
followed by updating the tables below; then `rm -rf target/release`. It takes
about a minute on an M4 Max. A filter is a substring of the names to run, and
`a|b` runs names containing either, so `-- ecs/par-` runs only the parallel
rows.

The workloads' correctness is checked apart from timing by
`cargo test -p ponko_bench`: for every case, the tests build the same world
in all three libraries at small sizes, run the operation and compare the
results (sorted positions bit for bit, live entities, component counts,
the game tick's whole state), and check the parallel loops against the
serial ones. CI runs those tests; it never runs the timings and has no
wall-clock thresholds.

## Cases

Names are `ecs/<case>-<size>/<library>[-<variant>]`. Entities are
`(Pos, Vel, Spr)` (8 + 8 + 20 bytes) unless stated.

| Case | What one timed run does | Kind |
|---|---|---|
| `iter` | `pos += vel * dt` over every entity, one archetype | steady |
| `par` | steering per entity (`sin`, `sin_cos`, `rem_euclid`), serial and parallel | steady |
| `spawn-loop` | spawn `n` entities into an empty world, one call each | fresh |
| `spawn-batch` | the same with `spawn_batch` | fresh |
| `spawn-despawn` | despawn the oldest `n/10` entities and spawn `n/10` new ones | steady |
| `add-remove` | insert a marker into all `n` entities, then remove it from all | steady |
| `optional` | `(&mut Pos, &Vel, Option<&Boost>)`, four archetypes, half with `Boost` | steady |
| `with` / `without` | `(&mut Pos, &Vel)` filtered by a marker half the entities have | steady |
| `game` | one tick: 4 layouts, 5 systems, projectiles expire and respawn | steady |
| `archetypes-1024` | `pos += vel * dt` over 1024 archetypes of 16 entities | steady |
| `query-build-x1000` | build the `(&mut Pos, &Vel)` query 1000 times | steady |
| `query-build+count-x1000` | build it and count its matches, 1000 times | steady |
| `query-build+count-cold` | build it and count its matches once, in a fresh world | fresh |

The game world is, out of every 20 entities: 10 mobs `(Pos, Vel, Hp, Ai)`,
4 projectiles `(Pos, Vel, Life)`, 5 props `(Pos, Spr)` and 1 pickup
`(Pos, Spr, Tag)`. Its systems run in order: AI steering, movement, health,
lifetime (collect the expired, despawn them, spawn as many replacements) and
a read-only draw pass over `(Pos, Spr)` without `Tag`.

## Method

- **Timing**: wall clock (`Instant`) around each run of the operation.
  *Steady* cases build their state once, run the operation 5 times untimed
  (warm-up), then time every run; *fresh* cases rebuild the state untimed
  before each run and drop 1 warm-up run. Runs per case: 1000/200/100
  (iteration and queries at 1k/100k/1M), 500/100/30 (parallel, game),
  500/50/10 (churn), 200/20/10 (spawning).
- **Statistics**: min, p50, p95, max and mean per case, nearest-rank
  percentiles. The tables show p50 / p95.
- **Allocations**: the runner installs a counting global allocator
  ([`src/alloc.rs`](src/alloc.rs)); every result reports allocations and
  bytes requested per timed run (`allocs_per_run`, `alloc_bytes_per_run`),
  for the whole process, so allocations on worker threads count.
- **Memory**: `world_bytes` is `World::allocated_bytes()` (component columns
  and entity handles) after the timed runs, for ponko only. It is not process
  memory; bevy_ecs and hecs report no comparable number.
- **Load**: `load_average` in the JSON is read when the runner starts.
- **Build**: release profile of the workspace, `ponko_ecs` with its default
  features (`parallel`), `bevy_ecs` 0.20.1 with `default` and
  `multi_threaded`, `hecs` 0.11.2 with its defaults. All versions are pinned
  by the workspace `Cargo.lock`; the JSON's `env` records them, the
  toolchain, CPU, core count, rayon threads and the load average.
- **Threads**: everything runs on the calling thread except the `par` rows.
  ponko's `t<k>` rows run `par_for_each` inside a rayon pool of `k` threads
  (1, 2, 4, 8 and the core count), entered with `ThreadPool::install`.
  ponko's `par` row calls `par_for_each` straight from the benchmark thread
  on rayon's global pool (one thread per logical core), as bevy's `par` row
  calls `par_iter_mut` on its compute task pool, started once with its
  default size (every logical core).

## Limits of each comparison

- Each library runs its own idiomatic loop: ponko `Query::for_each`, bevy a
  kept `QueryState` with `iter_mut(..).for_each`, hecs
  `query_mut().into_iter().for_each` (its `fold` fast path; the older
  `iter/hecs` row in the README uses a `for` loop, which is slower).
- bevy pays for change ticks on every `&mut` access; ponko only tracks
  changes for components that opted in, and hecs has none. Rows that write
  components include that difference.
- Query matching differs per library. bevy matches archetypes when it builds
  a `QueryState` and keeps them in it. ponko matches when a query runs; its
  per-world archetype index ([QUERY_CACHE.md](QUERY_CACHE.md)) stays off for
  `(&mut Pos, &Vel)` here, which matches every archetype, so every ponko
  query scans. hecs matches when a query runs too, but keeps a per-world
  cache of each query type's matching archetypes, rebuilt only when the
  archetype set changes. So `query-build-x1000` measures different work per
  library (hecs only borrows the world), and in `query-build+count-x1000`
  ponko and bevy match all 1024 archetypes every time while hecs hits its
  cache. `query-build+count-cold` is the row where all three really match:
  the first query of a freshly built world, with each library's bookkeeping
  for it (a few allocations, listed below). It is one query per run, so it
  is short and noisy; compare minimums.
- No scheduler is measured: the game tick calls its systems in a row, in
  every library. System scheduling is in [SCHEDULE.md](SCHEDULE.md).
- hecs has no parallel iteration, so its `par` row is serial only. bevy's
  pool size is fixed once per process, so it has no scaling curve.
- Dispatch differs. ponko's `par_for_each` splits rows into chunks of
  `total / (4 × threads)`, clamped to 512–8192 rows, and runs serially only
  when everything fits one chunk; the "stay on the calling thread if the
  loop would take under about 50 µs" check applies only when every matched
  archetype has at most 512 rows. The `par` worlds are one archetype of `n`
  rows, so even at 1k ponko splits them into two chunks. Every ponko row
  hands the work to rayon from a thread outside the pool: with
  `ThreadPool::install` (`t<k>`) the whole loop is sent to a sleeping pool
  thread and the caller blocks; on the global pool (`par`) rayon injects the
  parallel part and the caller blocks without running any of it. bevy's
  `par_iter_mut` splits by its own batch size and runs tasks on the calling
  thread too while it waits. Small worlds measure mostly this dispatch
  cost.
- bevy's structural changes (insert, remove, spawn, despawn) also run its
  component hooks and check for observers, even when none are registered;
  the `spawn-*`, `add-remove` and `spawn-despawn` rows include that.
  ponko and hecs have neither.
- Wall-clock numbers on a shared machine move with its load. Compare runs
  from the same machine, build and load, and prefer min and p50 to p95.

## Baselines (2026-11-12)

Apple M4 Max (14 logical cores), macOS, rustc 1.99.1, release, revision
`90bfe8e`. The machine was idle apart from a browser and an editor: the
load average (1, 5, 15 minutes) was 1.91, 3.47, 2.75 when the run started
(the JSON's figure) and 2.85, 3.60, 2.81 when it ended. One run of the
command above produced every row here. p50 / p95 per timed run:

| Case | Size | ponko_ecs | bevy_ecs | hecs |
|---|---|---:|---:|---:|
| iter | 1k | 0.12 µs / 0.12 µs | 0.12 µs / 0.12 µs | 0.17 µs / 0.21 µs |
| iter | 100k | 16.3 µs / 16.4 µs | 36.0 µs / 39.5 µs | 16.7 µs / 18.6 µs |
| iter | 1M | 158 µs / 174 µs | 371 µs / 416 µs | 185 µs / 195 µs |
| spawn-loop | 1k | 13.2 µs / 13.4 µs | 21.4 µs / 23.2 µs | 16.2 µs / 16.5 µs |
| spawn-loop | 100k | 1.15 ms / 1.16 ms | 1.79 ms / 1.80 ms | 1.53 ms / 1.55 ms |
| spawn-loop | 1M | 13.14 ms / 13.37 ms | 20.54 ms / 21.02 ms | 15.73 ms / 15.97 ms |
| spawn-batch | 1k | 10.2 µs / 10.4 µs | 12.5 µs / 12.7 µs | 11.6 µs / 11.7 µs |
| spawn-batch | 100k | 940 µs / 1.01 ms | 1.12 ms / 1.16 ms | 1.13 ms / 1.14 ms |
| spawn-batch | 1M | 9.79 ms / 9.86 ms | 12.41 ms / 12.45 ms | 11.45 ms / 11.85 ms |
| spawn-despawn | 1k | 2.08 µs / 2.12 µs | 3.96 µs / 5.42 µs | 3.00 µs / 3.12 µs |
| spawn-despawn | 100k | 196 µs / 219 µs | 430 µs / 478 µs | 296 µs / 309 µs |
| spawn-despawn | 1M | 2.06 ms / 2.12 ms | 4.51 ms / 4.64 ms | 3.04 ms / 3.06 ms |
| add-remove | 1k | 42.6 µs / 45.0 µs | 63.4 µs / 69.1 µs | 57.1 µs / 63.2 µs |
| add-remove | 100k | 4.26 ms / 4.30 ms | 8.33 ms / 8.57 ms | 5.82 ms / 5.89 ms |
| add-remove | 1M | 42.5 ms / 43.4 ms | 99.7 ms / 103.6 ms | 57.4 ms / 57.9 ms |
| optional | 1k | 0.50 µs / 0.54 µs | 0.17 µs / 0.17 µs | 0.50 µs / 0.50 µs |
| optional | 100k | 44.7 µs / 47.8 µs | 34.0 µs / 39.1 µs | 44.1 µs / 52.2 µs |
| optional | 1M | 470 µs / 480 µs | 368 µs / 389 µs | 492 µs / 506 µs |
| with | 1k | 0.08 µs / 0.12 µs | 0.08 µs / 0.08 µs | 0.12 µs / 0.12 µs |
| with | 100k | 8.21 µs / 8.29 µs | 16.8 µs / 23.0 µs | 9.38 µs / 9.46 µs |
| with | 1M | 87.8 µs / 93.2 µs | 180 µs / 188 µs | 98.2 µs / 103 µs |
| without | 1k | 0.08 µs / 0.12 µs | 0.04 µs / 0.08 µs | 0.12 µs / 0.12 µs |
| without | 100k | 8.25 µs / 8.38 µs | 16.7 µs / 19.8 µs | 8.50 µs / 9.46 µs |
| without | 1M | 80.8 µs / 82.7 µs | 180 µs / 187 µs | 98.4 µs / 101 µs |
| game | 10k | 14.4 µs / 17.9 µs | 15.5 µs / 23.6 µs | 15.4 µs / 20.2 µs |
| game | 100k | 140 µs / 175 µs | 161 µs / 244 µs | 145 µs / 191 µs |
| archetypes-1024 | 16k | 5.17 µs / 5.42 µs | 8.75 µs / 9.75 µs | 7.08 µs / 8.29 µs |
| query-build-x1000 | 16k | 16.7 µs / 18.6 µs | 7.80 ms / 8.04 ms | 0.25 µs / 0.25 µs |
| query-build+count-x1000 | 16k | 1.96 ms / 1.98 ms | 8.41 ms / 8.66 ms | 217 µs / 237 µs (cache hit) |
| query-build+count-cold | 16k | 2.17 µs / 3.21 µs | 12.4 µs / 33.1 µs | 12.8 µs / 16.0 µs |
| query-build+count-cold, min | 16k | 1.92 µs | 8.25 µs | 11.7 µs |

On a fresh world, with nothing cached anywhere, matching the 1024 archetypes
once takes ponko about 2 µs, bevy 8–12 µs and hecs 12–13 µs.

Parallel scaling, the `par` steering workload, p50 / p95. `ponko par` is
ponko on the global pool without `install`, the setup that matches bevy's:

| Size | ponko serial | ponko t1 | t2 | t4 | t8 | t14 | ponko par | bevy serial | bevy par | hecs serial |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1k | 5.67 µs / 5.92 µs | 10.5 µs / 12.1 µs | 11.0 µs / 13.5 µs | 8.79 µs / 13.4 µs | 19.0 µs / 33.4 µs | 28.4 µs / 54.4 µs | 26.2 µs / 52.6 µs | 5.29 µs / 5.46 µs | 10.7 µs / 17.6 µs | 5.17 µs / 5.71 µs |
| 100k | 558 µs / 572 µs | 526 µs / 550 µs | 289 µs / 306 µs | 162 µs / 174 µs | 131 µs / 162 µs | 101 µs / 133 µs | 104 µs / 171 µs | 566 µs / 586 µs | 132 µs / 140 µs | 556 µs / 566 µs |
| 1M | 4.80 ms / 5.47 ms | 4.76 ms / 5.34 ms | 2.50 ms / 2.81 ms | 1.31 ms / 1.49 ms | 736 µs / 861 µs | 610 µs / 773 µs | 596 µs / 715 µs | 4.91 ms / 5.62 ms | 750 µs / 918 µs | 4.87 ms / 5.51 ms |

- At 1M the curve is near linear up to 4 threads (3.7×) and reaches 6.5× on
  8 and 7.9× on 14; at 100k it reaches 3.4× on 4 and 5.5× on 14. bevy's
  pool reaches 6.5× at 1M and 4.3× at 100k.
- At 100k and 1M, `ponko par` matches the `t14` row (5.4× and 8.1× over
  serial); the `install` hop costs nothing measurable there.
- At 1k every parallel row is slower than the serial loop (5.7 µs): ponko
  9–11 µs with 1–4 threads and 19–28 µs with 8, 14 or the global pool; bevy
  11 µs. The cost grows with the number of threads woken, not with
  `install`: `ponko par` (no `install`, 14 threads) is as slow as `t14`.

Allocations per timed run, where any library made one (all other rows make
none in all three libraries):

| Case | ponko_ecs | bevy_ecs | hecs |
|---|---:|---:|---:|
| spawn-loop 1k / 100k / 1M | 57 / 92 / 107 | 131 / 215 / 251 | 41 / 83 / 101 |
| spawn-batch 1k / 100k / 1M | 29 / 36 / 39 | 43 / 50 / 53 | 13 / 13 / 13 |
| spawn-despawn 1k / 100k / 1M | 0 | 0.04 / 0.24 / 0.6 | 0 |
| add-remove 1k / 100k / 1M | 0 | 0.01 / 0.08 / 0.1 | 0 |
| par (parallel rows) | 0–0.03 (rayon) | 17.5 (task pool) | — |
| game 10k / 100k | 0.02 / 0.11 | 0.11 / 0.6 | 0.02 / 0.11 |
| query-build(+count)-x1000 | 0 | 29,000 (29 per build) | 0 |
| query-build+count-cold | 1 | 29 | 11 |

The fractional counts are buffers that still grow now and then (bevy's
entity lists, the game's list of expired projectiles, which all three share).
`world_bytes` for ponko: 43 KiB / 4.2 MiB / 42 MiB for `iter` at 1k / 100k / 1M
(44 bytes per entity: 36 bytes of components and 8 of entity handle),
1056 KiB for `archetypes-1024`, 474 KiB / 4.3 MiB for `game` at 10k / 100k.

What the numbers point at, for later ECS work (each needs its own measured
change): an `Option<&T>` query in ponko takes 2.7–3× as long as `iter` over
the same number of entities and is about 30% slower than bevy's at 100k and
1M; spawning one entity at a time costs a third more than `spawn_batch`.
