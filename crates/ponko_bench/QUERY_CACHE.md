# The archetype index for queries (#139)

A query has to find the archetypes that hold what it asks for. The world
keeps an index of that per query signature, and decides by a policy
(`QueryCachePolicy`) which signatures are worth indexing. This file holds
the measurements behind the policy's defaults, and how to repeat them.

## What is measured

All groups are headless and live in `crates/ponko_bench`
(`src/query_policy.rs`, `src/query_cache.rs`). Every world has one layout
per subset of some zero-sized markers, so a query that requires `k` markers
matches `layouts / 2^k` of them. Three strategies:

- `scan`: the index off (`QueryCachePolicy::off()` or `.uncached()`), which
  is what every query did before.
- `auto`: the default policy.
- `index` / `cached`: every signature indexed, whatever it matches
  (`.cached()`, or a policy without thresholds).

| Group | Workload |
| --- | --- |
| `query-policy/select/{layouts}/{share}/{count,each}/{strategy}` | 1,000 reconstructed queries per batch matching all, 1/2, 1/4, 1/16 or 1/64 of 8 to 4,096 layouts, one entity per layout. 5 warm-up batches, 100 measured. |
| `query-policy/churn/{kind}/{strategy}` | 1,024 layouts, four repeated signatures (1/4, 1/16, an `Or` matching 3/4, 1/8) run after every structural change: new layouts, moves between layouts, spawn and despawn (emptying layouts), late component registration. |
| `query-policy/frame/{layouts}/{strategy}` | A game-like frame over 50,000 entities (half in 8 layouts, the rest spread): 20 queries of mixed selectivity (dense movement, selective queries, `Or` filters, counts, an iterator, a parallel loop, a runtime-filtered chunk query), then 64 moves, 32 despawns and 32 spawns. 300 frames. |
| `query-policy/mem/{layouts}/{strategy}` | The same frame, 100 times, then the index's own bytes, the world's storage and the process's peak resident memory. One case per process: the peak never goes down. |
| `query-policy/promote/uses-{n}/after-{k}` | 45 signatures in a fresh world of 1,024 layouts, each used `n` times and indexed on its `k`th use. |
| `query-cache/...` | The earlier prototype group: one 1/16 signature and one dense one, 1 to 1,024 layouts, 1/100/1,000 queries per batch, cold and warm. |
| `query-filter/`, `iter/`, `query-build`, `archetypes-1024`, `par-arch/` | Existing groups, for the simple-query fast path before and after. |

Commands (each prints median, p95 and min in milliseconds):

```sh
cargo build --release -p ponko_bench
B=target/release/ponko_bench
for f in query-filter iter/ query-build archetypes-1024 par-arch; do $B $f; done
$B query-policy/select
$B query-policy/churn
$B query-policy/frame
for l in 64 1024 4096 16384; do for s in scan auto index; do $B query-policy/mem/$l/$s; done; done
$B query-policy/promote
$B query-cache
```

## Machine and revision (2026-11-11)

Apple M4 Max (14 cores, 36 GB), macOS 27.0.1, aarch64-apple-darwin, rustc
1.99.1 (58cf8f9c3), Cargo release profile, default benchmark features (ECS
parallel support on). Revision `46aa9906` (this branch with `main` at
`28295d14` merged); "before" numbers are `main` at `f3bef8ef`, measured the
same day. The six re-runs of the 64-layout frame row and the "Promotion"
table were measured later the same day, on `7cbf17a9` plus the promotion
benchmark (the index fixes after `46aa9906` don't touch the measured
paths).

**The machine was shared**: other builds and tests ran during every run
(load average 7 to 14). Medians are stable to a few percent between runs;
p95 values include scheduling outliers and are not CPU latency. One run
each, no GPU involved, no claim about whole-engine performance.

## Selectivity and the crossover

p50, milliseconds per 1,000 queries (`count`; so the number is also
microseconds per query). `scan / auto / index`:

| Layouts | all | 1/2 | 1/4 | 1/16 | 1/64 |
| --- | --- | --- | --- | --- | --- |
| 8 | 0.030 / 0.030 / 0.078 | 0.035 / 0.035 / 0.079 | 0.040 / 0.040 / 0.084 | | |
| 16 | 0.043 / 0.043 / 0.088 | 0.053 / 0.053 / 0.094 | 0.059 / 0.059 / 0.092 | 0.075 / 0.074 / 0.092 | |
| 32 | 0.071 / 0.070 / 0.119 | 0.082 / 0.082 / 0.117 | 0.090 / 0.090 / 0.103 | 0.109 / 0.100 / 0.097 | |
| 64 | 0.118 / 0.169 / 0.153 | 0.156 / 0.160 / 0.163 | 0.158 / 0.133 / 0.130 | 0.184 / 0.106 / 0.107 | 0.184 / 0.108 / 0.106 |
| 128 | 0.215 / 0.260 / 0.263 | 0.277 / 0.245 / 0.243 | 0.298 / 0.181 / 0.183 | 0.329 / 0.132 / 0.130 | 0.344 / 0.118 / 0.117 |
| 256 | 0.406 / 0.389 / 0.390 | 0.556 / 0.408 / 0.402 | 0.607 / 0.295 / 0.299 | 0.655 / 0.173 / 0.173 | 0.665 / 0.129 / 0.132 |
| 1,024 | 1.588 / 1.636 / 1.628 | 2.365 / 1.535 / 1.559 | 2.627 / 1.043 / 1.029 | 3.040 / 0.418 / 0.412 | 3.122 / 0.227 / 0.228 |
| 4,096 | 6.212 / 6.457 / 8.550 | 17.956 / 10.506 / 10.909 | 19.110 / 4.383 / 4.364 | 17.713 / 1.423 / 1.389 | 12.169 / 0.770 / 0.705 |

`for_each` gives the same picture (64 layouts, 1/16: 0.184 / 0.109 / 0.106;
1,024 layouts, 1/16: 2.942 / 0.447 / 0.457).

What this decided:

- **`min_archetypes = 64`.** Using the index costs a signature build and a
  hash lookup, about 45 to 50 ns per query here. Below 32 layouts a forced
  index loses for every selectivity (8 layouts: 0.078 against 0.030 ms). At
  32 it breaks even only from 1/16 down. At 64 it wins from 1/4 down (0.133
  against 0.158 ms) and ties at 1/2. So worlds under 64 archetypes scan and
  pay nothing: `auto` equals `scan` there to the last digit shown.
- **`max_match_percent = 50`.** A list of every archetype saves no work:
  at 128 layouts `all` is 0.263 ms indexed against 0.215 ms scanned, and at
  4,096 the forced dense list was the slowest of the three. At 1/2 the
  index ties (64) or wins (128 and up), so the limit is "at most half".
- **The cost that remains.** In a world of 64 archetypes or more, a query
  whose signature is decided as dense still looks its signature up before
  it scans: 0.169 against 0.118 ms per 1,000 queries at 64 layouts, 0.260
  against 0.215 at 128, within noise from 256 up, where the scan itself
  dominates. That is about 50 ns per query, once per query and not per
  entity. `.uncached()` removes it for a query known to be dense.
- **`promote_after = 2`.** A signature seen once isn't remembered beyond a
  counter; one-off queries don't build lists. Measured below
  ("Promotion").

## Churn (1,024 layouts)

p50 / p95 in milliseconds per batch:

| Structural change between queries | scan | auto |
| --- | --- | --- |
| New layouts | 3.754 / 3.909 | 2.047 / 4.137 |
| Moves between layouts | 2.952 / 3.033 | 1.414 / 3.872 |
| Spawn and despawn (layouts empty and refill) | 5.927 / 10.533 | 2.773 / 6.894 |
| Late component registration | 3.673 / 8.980 | 1.890 / 4.579 |

Layouts are append-only and never change their component set, so a list is
extended by matching only the layouts created since it was last used; moves,
despawns, `clear` and registration don't invalidate anything. The index
stays ahead under every kind of change measured.

## A game-like frame (50,000 entities)

p50 / p95 in milliseconds per frame:

| Layouts | scan | auto |
| --- | --- | --- |
| 16 | 0.048 / 0.049 | 0.048 / 0.049 |
| 64 | 0.087 / 0.460 | 0.088 / 1.271 |
| 256 | 0.097 / 0.134 | 0.089 / 0.092 |
| 1,024 | 0.158 / 0.162 | 0.118 / 0.121 |
| 4,096 | 0.446 / 0.564 | 0.266 / 0.553 |

With few layouts the frame is the same (the policy scans). From 256
layouts on the default policy is faster, by 25% at 1,024 and 40% at 4,096.

The 64-layout row is where the index starts, and its p95 in the table
(1.271 ms for auto against 0.460 ms for scan) came from one run. Six more
runs of that row alone (`$B query-policy/frame/64/`, the machine busier
than for the table: load average 9 to 12), p50 / p95 / min in
milliseconds:

| Run | scan | auto |
| --- | --- | --- |
| 1 | 0.143 / 0.246 / 0.070 | 0.152 / 0.248 / 0.070 |
| 2 | 0.155 / 0.256 / 0.070 | 0.125 / 0.245 / 0.072 |
| 3 | 0.145 / 0.244 / 0.073 | 0.119 / 0.241 / 0.069 |
| 4 | 0.132 / 0.235 / 0.070 | 0.123 / 0.223 / 0.070 |
| 5 | 0.146 / 0.246 / 0.071 | 0.141 / 0.267 / 0.069 |
| 6 | 0.134 / 0.238 / 0.070 | 0.135 / 0.242 / 0.069 |

Mean of the medians: 0.143 scan, 0.133 auto; the p95 values agree to
within 10% in every run and the minima are equal. The 1.271 ms did not
come back: it was the machine, not the policy, and auto is not slower at
64 layouts.

## Promotion

`query-policy/promote/uses-{n}/after-{k}`: a fresh world of 1,024 layouts
per sample, 45 different signatures (each 1/4 of the layouts), every one
used `n` times, indexed on its `k`th use. p50 / p95 in milliseconds for
all 45 × `n` queries, 40 samples:

| Uses of each signature | `promote_after = 1` | `= 2` (default) | `= 8` |
| --- | --- | --- | --- |
| 1 | 0.482 / 0.581 | 0.233 / 0.258 | 0.221 / 0.245 |
| 2 | 0.551 / 0.577 | 0.707 / 0.730 | 0.461 / 0.490 |
| 4 | 0.706 / 0.781 | 0.852 / 0.862 | 0.908 / 0.925 |
| 16 | 1.615 / 1.668 | 1.742 / 1.844 | 2.754 / 2.879 |

Indexing on the first use doubles the cost of a signature that is never
used again (0.482 against 0.233 ms), which is what one-off queries are.
Waiting for the second use costs one extra scan per signature, once: 8%
at 16 uses and less with every further use. Waiting for the eighth use
is 70% slower at 16 uses. A signature used exactly twice is the worst
case for 2 (it builds a list on its last use); `.cached()` is the hint
for a query known to repeat rarely but worth indexing at once.

## Memory

After 100 frames of the mixed workload (20 signatures):

| Layouts | Strategy | Signatures indexed | Indices | Index bytes | World storage | Peak RSS |
| --- | --- | --- | --- | --- | --- | --- |
| 64 | scan | 0 | 0 | 0 | 1,224 KiB | 85,952 KiB |
| 64 | auto | 17 of 20 | 237 | 82,708 | 1,224 KiB | 68,176 KiB |
| 64 | index | 20 | 389 | 83,316 | 1,224 KiB | 68,192 KiB |
| 1,024 | scan | 0 | 0 | 0 | 1,380 KiB | 87,040 KiB |
| 1,024 | auto | 17 of 20 | 3,792 | 96,928 | 1,380 KiB | 69,392 KiB |
| 1,024 | index | 20 | 6,224 | 106,656 | 1,380 KiB | 70,608 KiB |
| 4,096 | scan | 0 | 0 | 0 | 1,449 KiB | 88,176 KiB |
| 4,096 | auto | 17 of 20 | 15,168 | 142,432 | 1,449 KiB | 70,592 KiB |
| 4,096 | index | 20 | 24,896 | 181,344 | 1,449 KiB | 88,384 KiB |
| 16,384 | scan | 0 | 0 | 0 | 3,213 KiB | 103,184 KiB |
| 16,384 | auto | 17 of 20 | 60,672 | 324,448 | 3,213 KiB | 80,480 KiB |
| 16,384 | index | 20 | 99,584 | 480,096 | 3,213 KiB | 80,464 KiB |

- "Index bytes" is what `World::query_cache_stats().bytes` reports: the
  table (reserved once for 128 signatures, about 80 KiB) plus four bytes
  per listed archetype.
- Process peak RSS (`getrusage`) is dominated by the benchmark process
  itself (rayon's pool, setup) and varies by about 20 MiB between runs of
  the same case; the scan runs were not lower than the indexed ones. The
  index, at most a few hundred KiB here, **can't be told apart in RSS**.
  The table above is the measurement; RSS only bounds it.
- The default budget is 1 MiB of indices (`max_index_bytes`) and 128
  signatures (`max_signatures`). A signature that would pass the budget is
  scanned; when the table is full, signatures unused since the last sweep
  are dropped (after 32 refused new ones), so new signatures get in.
- The limit of that rule: "unused" means unused between two sweeps, and
  sweeps come every 32 refusals. A world that runs more than 128 different
  signatures, with more than 32 refusals between two uses of an indexed
  one, drops and rebuilds that one (an allocation each time). Growing
  `evictions` in `query_cache_stats()` shows it; the fix is a larger
  `max_signatures`, or `.uncached()` on the one-off queries.
- A world under `min_archetypes` never creates the index: 0 bytes.

## The simple-query fast path, before and after

1,000,000 entities in one or a few layouts; p50 / p95 in milliseconds.
"Before" is `main`, "after" is this branch with the default policy:

| Case | Before | After |
| --- | --- | --- |
| `query-filter/plain` | 0.178 / 0.218 | 0.167 / 0.176 |
| `query-filter/typed` | 0.170 / 0.202 | 0.167 / 0.175 |
| `query-filter/nested-or-and` | 0.179 / 0.220 | 0.167 / 0.170 |
| `query-filter/c-plain` | 0.180 / 0.215 | 0.167 / 0.171 |
| `query-filter/c-filtered` | 0.174 / 0.202 | 0.168 / 0.172 |
| `query-filter/c-filtered-nested` | 0.178 / 0.216 | 0.168 / 0.171 |
| `iter/ponko` | 0.176 / 0.206 | 0.165 / 0.169 |
| `iter/ponko-iter` | 0.905 / 0.999 | 0.865 / 0.893 |
| `query-build-x1000/ponko` | 0.020 / 0.023 | 0.018 / 0.019 |
| `query-build+count-x1000/ponko` | 2.173 / 2.704 | 2.144 / 2.156 |
| `archetypes-1024/ponko` | 0.005 / 0.006 | 0.005 / 0.005 |
| `par-arch/4096x16/light/seq` | 0.065 / 0.080 | 0.060 / 0.066 |
| `par-arch/4096x16/light/par` | 0.076 / 0.132 | 0.068 / 0.087 |

No case is slower; the differences are the machine's load at the time, not
a speed-up. On this path a query checks one integer (the archetype count
against `min_archetypes`) and builds no signature. Allocation counting
tests (`crates/ponko_ecs/tests/query_cache_alloc.rs`) show zero heap
allocations per query once warm: in small worlds, with indexed and scanned
signatures, with a full table and its sweeps, and from views.

## The earlier prototype group

`query-cache`, 1,000 reconstructed queries of one 1/16 signature, p50 in
milliseconds, `scan / auto / cached`:

| Layouts | count | for_each |
| --- | --- | --- |
| 64 | 0.192 / 0.130 / 0.134 | 0.190 / 0.117 / 0.118 |
| 1,024 | 3.157 / 0.519 / 0.501 | 3.234 / 0.529 / 0.482 |

The prototype needed `.cached()` and started at 128 layouts; the automatic
policy reaches the same numbers without the hint, from 64 layouts.

## Views and parallel batches

A world view shares the world, and a batch of views may run on several
threads, so a view never mutates the index:

- A view's lookup is a read of the table and the entry's list. The only
  write is a relaxed atomic "last used" stamp, stored only when its value
  changes (once per sweep period), so threads sharing an entry don't keep
  writing its cache line.
- **There is no lock on the hit path.** A signature the index doesn't hold
  (or whose list is behind the archetype count) is scanned, and noted in a
  small list behind a mutex taken with `try_lock` only: a busy list is
  skipped, never waited for. The world counts those notes as uses the next
  time it is held exclusively (when the next view is prepared), which is
  when signatures are promoted and lists extended.
- Because the lock is off the steady-state path, it has no numbers of its
  own: a view's indexed query costs what an exclusive one does. The miss
  path adds one uncontended `try_lock` (and a look through a list of at
  most 16 notes) to a scan. For a signature the table takes, that happens
  a few times in the life of a world: until it is counted and decided.
- A **full table** is the exception, and it is bounded differently: a
  signature the table has no room for can't be decided. Once the table
  has refused a new signature, view queries of signatures it doesn't hold
  stop noting (one relaxed atomic load each, no lock) until a sweep drops
  something. So that a world only views query still gets its sweeps, every
  64th time views are prepared on a full table they may note again for one
  round. A refused view signature therefore costs a `try_lock` in one round
  of 64, and one counted miss per round at most.
- A view's use stamps the entry with the index's clock, which only
  exclusive lookups advance; a sweep moves the clock on, so a use right
  after a sweep counts as after it (`a_signature_only_views_use_survives_every_sweep`).
- Tests: `crates/ponko_ecs/tests/query_cache_views.rs` (eight views on
  rayon threads against a scan, with structural changes between rounds),
  unit tests in `crates/ponko_ecs/src/query/cache.rs` (many threads read
  one index), and the Miri runs listed in `AGENTS.md`.

## What the key covers

The signature is the query's required components plus its filter in a
canonical conjunctive form: `with`, `without`, `Or`, AND groups inside
`Or`, nested tuples and runtime `DynFilter`s, with sorted and deduplicated
literals, tautologies and satisfied clauses removed, unit clauses folded
into the required and excluded sets, and subsumed clauses dropped. Typed and
runtime filters that mean the same share one entry. `Added` and `Changed`
are not in the key (they select rows, not archetypes), nor are optional
components or mutability. A filter over the inline limits is rejected by
the query before the index is asked. Every loop still applies its own
filter to the candidates, so an entry that is too wide could cost time but
never change a result; differential tests compare every path against a
world with the index off (`tests/query_cache_differential.rs`).

Only `u32` archetype indices are stored. No component or archetype pointer
outlives a query borrow, so growth of the archetype list can't leave
anything dangling.
