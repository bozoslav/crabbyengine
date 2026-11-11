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
- **Load**: `load_average` in the JSON is read when the runner starts (from
  revision `1a4fe84` on; the two runs below read it when the file was
  written, at the end, and the load before each run is quoted from `uptime`).
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

## Baselines (2026-11-11)

Apple M4 Max (14 logical cores), macOS, rustc 1.99.1, release, revision
`3f6e7df`. **The machine was not idle**: other builds and the user's work
ran at the same time. The load average (1, 5, 15 minutes) was 12.5, 12.9,
8.8 just before the run and 24.5, 16.8, 10.6 when it ended (the JSON's
figure). Serial rows were stable (p95 within a few percent of p50 for most);
parallel rows at 100k and the spawning rows' p95 show the load. Re-record on
an idle machine before using these for decisions (the command above).
p50 / p95 per timed run:

| Case | Size | ponko_ecs | bevy_ecs | hecs |
|---|---|---:|---:|---:|
| iter | 1k | 0.12 µs / 0.17 µs | 0.12 µs / 0.17 µs | 0.21 µs / 0.25 µs |
| iter | 100k | 17.0 µs / 17.3 µs | 42.5 µs / 47.2 µs | 19.5 µs / 20.3 µs |
| iter | 1M | 169 µs / 174 µs | 415 µs / 459 µs | 192 µs / 200 µs |
| spawn-loop | 1k | 14.2 µs / 14.6 µs | 23.3 µs / 24.2 µs | 17.6 µs / 34.5 µs |
| spawn-loop | 100k | 1.37 ms / 2.74 ms | 1.95 ms / 6.39 ms | 1.81 ms / 3.25 ms |
| spawn-loop | 1M | 13.39 ms / 13.45 ms | 20.82 ms / 21.00 ms | 15.97 ms / 16.08 ms |
| spawn-batch | 1k | 10.8 µs / 10.9 µs | 13.5 µs / 13.8 µs | 15.2 µs / 20.7 µs |
| spawn-batch | 100k | 995 µs / 1.62 ms | 1.28 ms / 4.30 ms | 1.21 ms / 3.18 ms |
| spawn-batch | 1M | 9.97 ms / 10.02 ms | 12.65 ms / 12.70 ms | 11.55 ms / 11.75 ms |
| spawn-despawn | 1k | 2.83 µs / 2.92 µs | 4.46 µs / 5.79 µs | 3.21 µs / 3.33 µs |
| spawn-despawn | 100k | 238 µs / 246 µs | 472 µs / 1.21 ms | 311 µs / 825 µs |
| spawn-despawn | 1M | 2.23 ms / 2.28 ms | 4.47 ms / 4.57 ms | 2.89 ms / 2.90 ms |
| add-remove | 1k | 45.6 µs / 49.7 µs | 66.9 µs / 80.6 µs | 63.5 µs / 71.1 µs |
| add-remove | 100k | 4.66 ms / 4.94 ms | 9.23 ms / 9.87 ms | 6.03 ms / 6.30 ms |
| add-remove | 1M | 43.4 ms / 47.1 ms | 102.8 ms / 107.5 ms | 59.3 ms / 62.0 ms |
| optional | 1k | 0.54 µs / 0.62 µs | 0.17 µs / 0.21 µs | 0.50 µs / 0.62 µs |
| optional | 100k | 44.9 µs / 48.3 µs | 38.1 µs / 39.2 µs | 45.2 µs / 48.5 µs |
| optional | 1M | 477 µs / 488 µs | 406 µs / 416 µs | 499 µs / 511 µs |
| with | 1k | 0.08 µs / 0.12 µs | 0.08 µs / 0.17 µs | 0.12 µs / 0.12 µs |
| with | 100k | 8.50 µs / 8.54 µs | 19.8 µs / 20.0 µs | 9.62 µs / 9.71 µs |
| with | 1M | 94.5 µs / 97.1 µs | 205 µs / 212 µs | 101 µs / 105 µs |
| without | 1k | 0.08 µs / 0.12 µs | 0.08 µs / 0.12 µs | 0.12 µs / 0.12 µs |
| without | 100k | 8.46 µs / 8.50 µs | 19.0 µs / 19.6 µs | 9.50 µs / 9.71 µs |
| without | 1M | 84.0 µs / 87.8 µs | 203 µs / 212 µs | 100 µs / 104 µs |
| game | 10k | 14.3 µs / 17.9 µs | 15.8 µs / 23.8 µs | 15.3 µs / 20.5 µs |
| game | 100k | 141 µs / 181 µs | 168 µs / 258 µs | 147 µs / 196 µs |
| archetypes-1024 | 16k | 5.12 µs / 5.21 µs | 10.8 µs / 11.2 µs | 7.88 µs / 8.50 µs |
| query-build-x1000 | 16k | 19.4 µs / 19.8 µs | 8.65 ms / 10.32 ms | 0.25 µs / 0.29 µs |
| query-build+count-x1000 | 16k | 2.19 ms / 2.68 ms | 9.74 ms / 15.15 ms | 225 µs / 266 µs (cache hit) |

### Second run: the cold query and the global-pool rows

After review, only the rows below ran again at revision `1a4fe84`
(`-- 'query-build+count|ecs/par-'`), with the load even higher: 36.2, 30.8,
20.5 just before the run, 33.9, 30.4, 20.4 at its end. The results are the
`supplement` document in [`baselines/ecs.json`](baselines/ecs.json); the
rows above are the first run. p50 / p95, and min for the short cold row:

| Case | ponko_ecs | bevy_ecs | hecs |
|---|---:|---:|---:|
| query-build+count-x1000 | 2.30 ms / 2.45 ms | 9.70 ms / 10.11 ms | 237 µs / 270 µs (cache hit) |
| query-build+count-cold, p50 / p95 | 3 µs / 4 µs | 29 µs / 40 µs | 16 µs / 22 µs |
| query-build+count-cold, min | 2 µs | 19 µs | 13 µs |

On a fresh world, with nothing cached anywhere, matching the 1024 archetypes
once takes ponko about 2–3 µs, hecs 13–16 µs and bevy 19–29 µs.

Parallel scaling, the `par` steering workload, p50 / p95:

| Size | ponko serial | ponko t1 | t2 | t4 | t8 | t14 | bevy serial | bevy par | hecs serial |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1k | 6.00 µs / 6.21 µs | 39.5 µs / 445 µs | 21.8 µs / 281 µs | 36.5 µs / 844 µs | 21.2 µs / 162 µs | 21.3 µs / 197 µs | 6.29 µs / 6.50 µs | 9.42 µs / 33.0 µs | 6.08 µs / 6.71 µs |
| 100k | 593 µs / 691 µs | 652 µs / 1.55 ms | 586 µs / 1.67 ms | 477 µs / 1.86 ms | 343 µs / 1.17 ms | 666 µs / 2.43 ms | 608 µs / 1.40 ms | 326 µs / 1.43 ms | 616 µs / 696 µs |
| 1M | 4.96 ms / 5.72 ms | 5.03 ms / 5.63 ms | 2.52 ms / 2.88 ms | 1.32 ms / 1.49 ms | 774 µs / 868 µs | 612 µs / 699 µs | 5.16 ms / 5.73 ms | 781 µs / 903 µs | 4.99 ms / 5.63 ms |

At 1M the curve is near linear up to 4 threads (3.8×) and reaches 8.1× on
14; bevy's pool reaches 6.6×. At 100k the minimums show the same scaling
(0.16 ms from 4 threads on, against 0.47 ms serial), but the medians were
dominated by the other load.

The second run (same load note as above) adds ponko on the global pool
without `install` (`ponko par`), the setup that matches bevy's:

| Size | ponko serial | ponko t1 | t2 | t4 | t8 | t14 | ponko par | bevy serial | bevy par | hecs serial |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1k | 6.0 µs / 7.0 µs | 11 µs / 27 µs | 11 µs / 23 µs | 11 µs / 22 µs | 22 µs / 40 µs | 27 µs / 60 µs | 28 µs / 64 µs | 6.0 µs / 8.0 µs | 11 µs / 24 µs | 6.0 µs / 8.0 µs |
| 100k | 596 µs / 668 µs | 621 µs / 685 µs | 313 µs / 358 µs | 172 µs / 214 µs | 130 µs / 181 µs | 120 µs / 179 µs | 118 µs / 200 µs | 625 µs / 683 µs | 134 µs / 186 µs | 613 µs / 669 µs |
| 1M | 5.41 ms / 6.15 ms | 5.40 ms / 6.21 ms | 2.70 ms / 3.27 ms | 1.39 ms / 1.57 ms | 792 µs / 1.10 ms | 624 µs / 789 µs | 613 µs / 836 µs | 5.55 ms / 6.40 ms | 828 µs / 941 µs | 5.36 ms / 6.29 ms |

- At 100k and 1M, `ponko par` matches the `t14` row (5.0× and 8.8× over
  serial in this run); the `install` hop costs nothing measurable there.
  bevy's pool reaches 4.7× and 6.7×.
- At 1k every parallel row is slower than the serial loop (6 µs): ponko
  11 µs with 1–4 threads and 22–28 µs with 8, 14 or the global pool; bevy
  11 µs. The cost grows with the number of threads woken, not with
  `install`: `ponko par` (no `install`, 14 threads) is as slow as `t14`.
  The first run's 1k medians were 21–40 µs for every pool size and are
  dominated by its load; the second run's are the better figure, still
  under load.

Allocations per timed run, where any library made one (all other rows make
none in all three libraries):

| Case | ponko_ecs | bevy_ecs | hecs |
|---|---:|---:|---:|
| spawn-loop 1k / 100k / 1M | 57 / 92 / 107 | 131 / 215 / 251 | 41 / 83 / 101 |
| spawn-batch 1k / 100k / 1M | 29 / 36 / 39 | 43 / 50 / 53 | 13 / 13 / 13 |
| spawn-despawn 1k / 100k / 1M | 0 | 0.04 / 0.24 / 0.6 | 0 |
| add-remove 1k / 100k / 1M | 0 | 0.01 / 0.08 / 0.1 | 0 |
| par (parallel row) | 0.02–0.09 (rayon) | 17.5 (task pool) | — |
| game 10k / 100k | 0.02 / 0.11 | 0.11 / 0.6 | 0.02 / 0.11 |
| query-build(+count)-x1000 | 0 | 29,000 (29 per build) | 0 |
| query-build+count-cold | 1 | 29 | 11 |

The fractional counts are buffers that still grow now and then (bevy's
entity lists, the game's list of expired projectiles, which all three share).
`world_bytes` for ponko: 43 KiB / 4.2 MiB / 42 MiB for `iter` at 1k / 100k / 1M
(44 bytes per entity: 36 bytes of components and 8 of entity handle),
1056 KiB for `archetypes-1024`, 474 KiB / 4.3 MiB for `game` at 10k / 100k.

What the numbers point at, for later ECS work (each needs its own measured
change): an `Option<&T>` query in ponko takes 2.8× as long as `iter` over
the same number of entities and is 15–20% slower than bevy's at 100k and 1M; spawning one entity at a
time costs a third more than `spawn_batch`.
