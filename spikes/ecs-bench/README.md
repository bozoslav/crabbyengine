# ECS spike (M3)

Compares a minimal archetype ECS of our own (`src/own.rs`) with `bevy_ecs` 0.20 and `hecs` 0.11. It answers two questions:

1. Which ECS do we build the engine on?
2. **D12 condition:** does routing every component through runtime registration (needed for the C/C++ API) cost the Rust path anything?

```sh
cargo run --release -p ecs-bench            # everything
cargo run --release -p ecs-bench -- iter    # filter by name
cargo +nightly miri test -p ecs-bench       # check the unsafe code
```

## Results (2026-11-02, Apple M4 Max, release, single thread)

1M entities with `(Pos, Vel, Spr)`; add/remove uses 100k entities. Median of 10 runs (200 for iteration).

| Benchmark | own | hecs | bevy_ecs |
|---|---:|---:|---:|
| spawn 1M (loop) | **12.6 ms** | 16.3 ms | 18.5 ms |
| spawn 1M (batch) | **10.9 ms** | 12.8 ms | 12.6 ms |
| iterate `pos += vel*dt`, 1M | **0.15 ms** | 1.77 ms | 0.36 ms |
| iterate, 1M over 16 archetypes | **0.16 ms** | 1.75 ms | 0.35 ms |
| get by entity, 1M random | **9.7 ms** | 15.8 ms | 13.8 ms |
| add + remove component, 100k | **5.0 ms** | 6.6 ms | 9.1 ms |
| despawn 1M | **7.0 ms** | 9.9 ms | 24.7 ms |
| clean debug build of the dependency | ~0 s (600 lines) | 0.8 s, 5 crates | 9.0 s, 80 crates |

Baselines for the iteration loop:

| | median | min |
|---|---:|---:|
| plain `Vec` loop, buffers grown with `push` | 0.17 ms | 0.15 ms |
| own, typed query (`for_each::<(&mut Pos, &Vel)>`) | 0.15 ms | 0.15 ms |
| own, C-style chunk system on components registered by name/size/align | 0.16 ms | 0.15 ms |

**D12:** typed queries and the C-style column path both run as fast as a hand-written loop, so runtime registration costs the Rust path nothing. C++ support stays.

## Caveats

- The spike has no change detection, query filters, parallel iteration or scheduler. `bevy_ecs` pays for change ticks on every `&mut` access, which accounts for part of its 2.4× gap on iteration. When we add change detection, it must be opt-in or column-level so the plain loop stays this fast.
- A plain `Vec` allocated in one go measured *slower* (0.23 ms median) than one grown by `push`. The difference is memory placement, not code: the minimums match. Only compare steady-state numbers.
- `hecs` iteration was ~0.8 ms when timed on a freshly built world and ~1.75 ms in steady state. It's slowest either way, and it has no runtime-defined components, so it was out because of D12 anyway.
- The unsafe code passes its tests under Miri.
