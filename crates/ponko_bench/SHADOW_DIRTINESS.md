# Shadow dirtiness: stable projections and resource dependencies (#159)

Measured on Apple M4 Pro (12 logical CPU cores), macOS, Metal integrated GPU,
driver reported as unknown, Rust 1.96.1. Release builds, default features plus
`ponko_bench/render`; no window or display pacing. These are workload results,
not a claim about other GPUs or games.

The before renderer is `07bde19775b3d4c1ec04dce6d44935bf6b5987b4`; the after
renderer is `aa18c017`. Both use the workload controls introduced with this
change. To build before, the five modified renderer implementation files
(`src/render/gpu.rs`, `src/render/gpu3d.rs`, `src/render/gpu3d/shadow_cache.rs`,
`src/render/gpu3d/shader_checks.rs`, `src/three/collect.rs`) were temporarily
read from the baseline revision in this checkout; the common scene harness
was retained. The edited files were restored immediately after building.
Each executable was saved separately before measuring. No checkout or
worktree was created for this comparison.

Every row uses 1280×720, 4× MSAA, the high preset with `matched` enabled
(plain sky, FXAA, no bloom/AO/SSR), four 2048² sun cascades,
`shadow_refresh = 1`, scale 0.1, 60 warmup frames and 300 measured frames.
Three runs alternate executable order; the tables report the median of the
three run statistics. The static scene has 20,000 batched boxes/spheres and
a ground; the actor scene has 2,000 ordinary models of 16 meshes and a ground.
The sun-shadow distance is respectively 80 and 200 world units.

## Results

Shadow GPU cost includes zero-work frames: sum of timed shadow passes divided
by measured frames. Counting passes measures retained cascades directly;
`SceneFrame`'s ordinary-model triangle counts do not include static batches.

| Workload | Shadow passes/frame before → after | Shadow GPU ms/frame before → after | CPU prepare median ms before → after |
|---|---:|---:|---:|
| Static camera and casters | 0 → 0 | 0 → 0 | 0.0290 → 0.0293 |
| Slow camera, static casters | 0.1767 → 0.1467 | 0.00620 → 0.00531 | 0.0931 → 0.0929 |
| Moving actors, fixed camera | 3 → 3 | 0.1273 → 0.1308 | 0.1926 → 0.1967 |
| All actors, camera and sun moving | 4 → 4 | 0.4336 → 0.4355 | 0.2277 → 0.2243 |

Slow camera motion eliminates 17.0% of shadow passes and 14.4% of shadow GPU
work. The existing baseline already skips all stationary shadow work: this
change preserves that result. Moving actors still invalidate the maps that
see them. The reduction is small in whole-frame terms because the opaque
pass dominates this workload; it is not a whole-frame speedup claim.

| Workload | Total CPU median ms before → after | Frame median ms before → after | Frame p95 ms before → after | Frame p99 ms before → after |
|---|---:|---:|---:|---:|
| Static | 0.1407 → 0.1437 | 1.2210 → 1.2210 | 1.3582 → 1.3660 | 1.4173 → 1.4179 |
| Slow camera | 0.1990 → 0.2010 | 1.2853 → 1.2823 | 1.4395 → 1.4323 | 1.5298 → 1.5368 |
| Moving actors | 0.5287 → 0.5370 | 1.6788 → 1.6988 | 1.8673 → 1.8616 | 1.9013 → 1.9303 |
| Full invalidation | 0.6003 → 0.5987 | 2.1737 → 2.1652 | 2.3880 → 2.3752 | 2.4566 → 2.4785 |

The full-invalidation case changes every model transform, the camera and the
sun every frame. All four cascades redraw; there is no eviction/repopulation
policy or retry loop. Total CPU median changes by -0.3%, frame median by
-0.4%, frame p99 by +0.9%. This shows no meaningful worst-case cost increase
on this workload. Moving-actor CPU median is +1.6% (8.3 µs), reported rather
than treated as a speedup.

| Workload | Allocations/frame before → after | Allocated bytes/frame before → after | Resource reallocations/run before → after |
|---|---:|---:|---:|
| Static | 201.04 → 201.04 | 57,284 → 57,291 | 0 → 0 |
| Slow camera | 231.35 → 229.84 | 63,062 → 62,584 | 0 → 0 |
| Moving actors | 399.02 → 399.02 | 150,729 → 150,729 | 1 → 1 |
| Full invalidation | 531.21 → 531.18 | 174,123 → 174,107 | 0 → 0 |

These allocation counters include the whole renderer and GPU driver command
recording; they are not allocations made solely by the cache. Material-ID
snapshots use retained vectors alongside the pre-existing retained instance
snapshots. No additional allocations/frame appear in full invalidation.
CPU-uploaded image textures now retain one alpha byte per texel (a 1024²
texture uses 1 MiB); region updates reuse that storage. These generated
benchmarks bypass the window renderer's texture upload wrapper, so that
specific memory cost is stated separately rather than inferred from their
GPU residency counts. The JSON records the available counted GPU memory;
driver-reserved memory and cache-only CPU memory were not measured.

## Reproduce

Set `CARGO_TARGET_DIR` to the desired build cache for every Cargo invocation.
For each revision/executable, repeat three times, reversing order in round 2:

```sh
PONKO_SHADOW_WORKLOAD=slow-camera PONKO_RENDER_MATCHED=1 \
PONKO_RENDER_FRAMES=300 PONKO_RENDER_WARMUP=60 PONKO_RENDER_SCALE=0.1 \
PONKO_BENCH_JSON=slow-camera.json PONKO_RENDER_FRAMES_CSV=slow-camera.csv \
  cargo run --release -p ponko_bench --features render -- 'render/scene static-world'
```

Use `static` with `static-world`, and `actors`/`thrash` with `dynamic-world`
for the other rows. Settings and all three rounds' distributions, allocations,
work counts and available memory are in
[`baselines/shadow-dirtiness-metal.json`](baselines/shadow-dirtiness-metal.json).
Scratch executables, frame CSVs and logs are not committed.

## Correctness and compatibility

The sphere radius is calculated in camera space; the snapped center is
translated directly in light space. Equality remains exact, without an
approximate matrix tolerance. Casters are still compared every scheduled
refresh; the collector's configured far-cascade refresh schedule is unchanged.

Naga's transitive function dependency information distinguishes tracked
material uniforms/2D textures from time, frame state, storage and texture
arrays. Input instance indices and surface view inputs remain conservative.
Distance-banded static batches retain their exact camera-eye key, so they
remain conservative under camera movement even if their band selection stays
the same.
Material dependencies are recorded per view, including already-recorded
pending frames. Built-in cutouts depend on base alpha only; custom functions
may depend on other channels and retain conservative texture invalidation.
Shader reloads affecting only an opaque surface do not invalidate depth.
Freeing a cached vertex shader or rebinding a freed cutout texture invalidates
its dependent views. These cold lifecycle fixes and additional tests landed
after the measured revision; the measured steady-frame paths are unchanged.

GPU tests compare cached images with forced redraws during sub-texel camera
motion, actor movement, camera cuts, shader changes and cutout texture changes.
They assert actual reuse and include visible negative controls. Existing
refresh-turn and static-batch band tests remain in place. Rust and C/C++ public
APIs and layouts are unchanged. Metal with default, baseline and webgl2 limits
is covered; actual browser backends and other hardware GPUs are not measured.

Comparable before/after release images of the generated static scene were
inspected (same setup and 60-frame warmup). Cached/forced-redraw GPU-test
image pairs were also inspected; reference fixtures were not modified.

## Final local verification

- GPU render suite with `PONKO_REQUIRE_GPU=1`: 369 harness passes and 11 ignored
  benchmarks on each tier. Default had no capability skips; baseline had 10,
  webgl2 had 84. The new shadow regressions executed on all tiers.
- `RUST_TEST_THREADS=1 cargo test --workspace`: 4,881 tests/doctests passed,
  16 ignored. An earlier parallel run passed; a later parallel rerun hit the
  existing validation-bundle fixture collision in the shared Cargo target
  (an invalid project was reported built successfully alongside a valid fixture
  of the same package). That test passed alone and in the serial workspace run.
- `cargo build --workspace --examples`,
  `cargo clippy --workspace --all-targets`, `cargo fmt --all --check`,
  `git diff --check` and the changelog checker passed. Clippy reports existing
  warnings in untouched ECS/physics/import/FFI code; none in changed code.
- Commit leak hooks and the PR-body leak check passed.
