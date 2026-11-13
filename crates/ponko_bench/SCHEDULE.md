# The parallel system scheduler (#138)

Run `cargo run --release -p ponko_bench --bin ponko_bench -- schedule`
(`src/schedule.rs`; `-- schedule/` for the scheduler rows only,
`-- schedule-off/` for the before/after rows). Every benchmark is a
headless game. The time is one whole simulation tick, measured inside the
game from before its first system to after its last, so it includes the
scheduler's own work (clocks, event slots, the barrier after each batch).
60 warm-up ticks, then 400 timed ticks (100 or 200 for the biggest worlds).

The systems are view systems (functions of `&mut SystemView`) over `Val<N>` components,
two floats each. *Light* is `v += w * dt` (well under a nanosecond per
entity); *heavy* is a few transcendental calls (40 to 60 ns per entity).
`SxE` means `S` systems, each over its own component of `E` entities.

| Mode | What it is |
|---|---|
| `seq` | scheduler off (the default): every system runs alone, in order |
| `auto` | `parallel_systems(true)`, default threshold (50 µs) |
| `threads` | `parallel_systems(true)`, threshold zero: every batch goes to the thread pool |

Apple M4 Max (10 performance + 4 efficiency cores), macOS 27.0.1, rustc
1.99.1, release profile, default features, 2026-11-11. Medians of one run
of the final code. The machine was in use for other work (load average 7
to 15), so treat the numbers as indicative and rerun before comparing. Two
earlier runs of the same benchmarks agreed on every row up to
`sweep/8x4000/heavy` within 10%; in those, some larger rows were disturbed
by other load (one read 51.6 ms instead of 28.7 ms), which this run did
not show.

## Scheduler off: before and after

The acceptance asks that simple games don't get slower. With the scheduler
off (never mentioned by the game), against `origin/main` at `f3bef8ef`,
which has view systems but no scheduler; `ctx` are `&mut Ctx` systems,
`views` are view systems:

| Benchmark | Before (`f3bef8ef`) | After |
|---|---|---|
| schedule-off/ctx/64x100/light | 8 µs | 8 µs |
| schedule-off/views/64x100/light | 8 µs | 8 µs |
| schedule-off/ctx/8x16000/light | 21 µs | 21 µs |
| schedule-off/views/8x16000/light | 21 µs | 21 µs |
| schedule-off/ctx/8x250000/light | 321 µs | 323 µs |
| schedule-off/views/8x250000/light | 321 µs | 322 µs |

No difference. A caution about these small rows: on this machine a
benchmark process that starts cold (right after a build, or while other
work holds the fast cores) reads the first rows at 11 to 15 µs and 24 to
27 µs instead of 8 and 21 µs. That happened in three of five runs of the
after code, and the same rows measured with the scheduler explicitly off
(`schedule/tiny/64x100/light/seq`) went from 15 µs to 8 µs between two
runs of one binary. The table gives the settled runs (two in a row agreed
to the microsecond); `origin/main` was measured once and was settled. With
the scheduler off the engine also never starts rayon's thread pool
(`tests/system_schedule_off_pool.rs`).

## Overhead: many tiny systems

| Benchmark | `seq` | `auto` | `threads` |
|---|---|---|---|
| tiny/4x100/light | <1 µs | <1 µs | 22 µs |
| tiny/16x100/light | 2 µs | 2 µs | 34 µs |
| tiny/64x100/light | 8 µs | 8 µs | 49 µs |
| sweep/8x100/heavy | 14 µs | 14 µs | 32 µs |
| chain/8x16000/light | 29 µs | 28 µs | 28 µs |

Handing a batch to the thread pool costs about 20 to 45 µs at the median
(9 to 24 µs at best, 50 to 90 µs at the 95th percentile): waking the
workers and waiting for them. For systems this small that is many times
the work itself, which is why `auto` keeps such batches on the game's
thread. There, a batch costs the same as running its systems one at a time
(the `seq` and `auto` columns agree to the microsecond), and a batch
allocates nothing in steady state (`tests/system_schedule_alloc.rs`).

In `chain`, every system reads what the one before wrote, so the plan has
eight stages of one system and nothing ever runs together: the scheduler
costs nothing there in any mode.

## The crossover

Eight independent systems, growing worlds:

| Benchmark | `seq` | `auto` | `threads` | `auto` vs `seq` |
|---|---|---|---|---|
| sweep/8x100/light | 1 µs | 1 µs | 26 µs | same |
| sweep/8x1000/light | 2 µs | 2 µs | 32 µs | same |
| sweep/8x4000/light | 6 µs | 6 µs | 33 µs | same |
| sweep/8x16000/light | 21 µs | 21 µs | 39 µs | same |
| sweep/8x64000/light | 81 µs | 59 µs | 58 µs | 1.4× |
| sweep/8x250000/light | 322 µs | 113 µs | 114 µs | 2.8× |
| sweep/8x1000/heavy | 291 µs | 98 µs | 97 µs | 3.0× |
| sweep/8x4000/heavy | 1.66 ms | 0.31 ms | 0.31 ms | 5.3× |
| sweep/8x16000/heavy | 7.15 ms | 1.04 ms | 1.04 ms | 6.9× |
| sweep/8x64000/heavy | 28.74 ms | 3.82 ms | 3.83 ms | 7.5× |

Threads lose up to 21 µs of total work (8x16000 light: 39 µs instead of
21 µs) and win from 81 µs (8x64000 light). What other threads can save is
the batch's work without its slowest system, which someone has to run
anyway: 18 µs in the first case, 71 µs in the second. The default
threshold sits between them:

**A batch goes to other threads when the usual run time of its systems,
not counting the slowest, is at least 50 µs** (`DEFAULT_PARALLEL_THRESHOLD`,
changed with `Game::parallel_threshold`). "Usual" is a moving average of
each system's measured run time, so a batch moves to threads, or back, as
its systems get slower or faster. Below the threshold it runs on the
game's thread in registration order. Either way the result is the same;
see the guide's "Parallel systems" chapter.

In every row `auto` matches the better of `seq` and `threads`.

## Representative schedules

| Benchmark | `seq` | `auto` | `threads` | `auto` vs `seq` |
|---|---|---|---|---|
| pair/2x64000/heavy | 7.16 ms | 3.65 ms | 3.66 ms | 2.0× |
| mixed/50000 | 8.45 ms | 5.71 ms | 5.75 ms | 1.5× |
| uneven/1x64000+7x100 | 3.63 ms | 3.62 ms | 3.62 ms | same |

- `pair`: two big systems is the smallest batch there is, and it halves.
- `mixed`: one heavy system alone in its phase, then two medium systems
  that read what it wrote, two heavy ones, a `&mut Ctx` system and two
  cheap ones. Only the batch of four before the `Ctx` system gains; the
  heavy system in its own phase and the `Ctx` system bound the speedup
  (1.5×, not 4×).
- `uneven`: one big system next to seven tiny ones has nothing to gain,
  since the big one sets the time. `auto` sees that (the work besides the
  slowest is far under 50 µs) and stays on the game's thread. Forcing it
  to threads gains nothing either; in one earlier, disturbed run it was
  slower (5.9 ms median, 14.5 ms at the 95th percentile).

## When it doesn't pay off

- Systems that take microseconds: nothing to gain, and `auto` doesn't try.
- Schedules that are chains (each system reads the previous one's output),
  or where `&mut Ctx` systems sit between the declared ones: there are no
  batches. `Game::schedule_plan()` shows the stages and why each starts.
- One dominant system: split its work with `par_for_each` inside the
  system instead (see [PAR_ARCHETYPES.md](PAR_ARCHETYPES.md)).
- Light, memory-bound systems gain less than compute-bound ones (2.8×
  against 7.5× with eight systems on ten performance cores).
- While profiling (`Game::profile_headless`, `Perf::set_profiling(true)`),
  batches stay on the game's thread so each system can be charged its own
  allocations; and on the web there are no threads to use.
