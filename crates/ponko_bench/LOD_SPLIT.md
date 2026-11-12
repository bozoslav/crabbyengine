# Levels of detail: CPU picking against GPU batch culling (#160)

Where level-of-detail work goes under a moving camera, and whether picking
levels on the GPU pays. The same field of identical three-level groups is
drawn three ways:

- **entities**: one `LodGroup` entity per copy. Every frame the CPU picks
  every group's level (`Frame3D::collect`, in parallel), culls the level's
  renderers and lists the instances drawn; the renderer uploads the
  instances that changed.
- **gpu-batch**: the group as `LodGroup::instance_levels`, one static batch
  of every copy with `BatchCulling::PreferGpu`. The GPU's culling pass tests
  every copy for every level band and view and compacts what is drawn: the
  GPU-side level selection.
- **cpu-batch**: the same batch with `BatchCulling::PreferCpu`: culled and
  picked cell by cell on the CPU (exact distances in the cells a band's
  edge crosses). This is also what a device without compute culling does.

## How to run it

```sh
cargo run --release -p ponko_bench --features render -- render/lod
cargo run --release -p ponko_bench --features render -- "render/lod 100k drive"
PONKO_GPU_TIER=webgl2 cargo run --release -p ponko_bench --features render -- "render/lod 100k drive "
```

`PONKO_LOD_FRAMES` (default 240) and `PONKO_LOD_WARMUP` (default 60) set
the frame counts. The harness is `src/render/gpu3d/lod_bench.rs`; the
runner is `crates/ponko_bench/src/lod_split.rs`. It opens no window: frames are drawn into an
offscreen texture with the real `Renderer3D`, and each frame waits for the
GPU. Each scenario prints its steps in the usual record format, then one
table row (the columns below).

## The scene

- 1280x720 `Rgba8UnormSrgb`, 4x MSAA, no solid depth prepass (Metal, as a
  game's window), default post effects, atmosphere off, no occlusion
  culling. Sun shadows (four cascades) only in the "shadows" rows.
- Copies on a square grid 4 m apart with a fixed jitter and turn. The group
  is 2 m: a sphere of 16 segments (480 triangles) while it covers 15% of the
  screen, 6 segments (72 triangles) down to 4%, a box down to 1%, then
  nothing: 11, 43 and 173 m at the default field of view. So about 1,500
  copies are drawn whatever the field's size; the rest are picked or culled
  away every frame.
- The camera flies low (8 m) across the field and back, looking ahead and
  down and swaying: **drive** is 0.5 m a frame (30 m/s at 60 fps), **fly**
  is 4 m a frame (240 m/s). Thousands of copies change level every frame.

## What was measured, and on what

- Apple M4 Max (14 cores, 36 GiB), Metal, integrated GPU; macOS; rustc
  1.99.1, release profile; `ponko_bench --features render` (engine features
  `3d`, `window`). Revision `0d11012c` (this branch on `5a7bae0b`).
  2026-11-12.
- **The machine was very busy**: other agents compiled throughout, with a
  load average of 89 to 191 on 14 cores during the runs. CPU times moved by
  up to 20 times between runs of the same row (entities' collect at 100k:
  a median of 5.1 ms in one run, 22 ms in the next). The 10k and 100k rows
  ran three times; each row below is one run of 240 measured frames, the
  quietest of the three (load 112 to 128). The 1M rows ran once. The
  **min** columns, the fastest frame of 240, are the closest to an idle
  machine. GPU times (timestamp queries)
  repeated within 0.03 ms between three runs, except the shadow row's
  GPU-driven total (2.60 to 2.98 ms). Only differences far larger than this
  noise are claimed below.
- Columns: **collect** is `Frame3D::collect` (for entities: the CPU's level
  picking); **prepare** is `Renderer3D::prepare` (for cpu-batch: its
  culling and picking); **cull GPU** is the static batch culling pass's GPU
  time (the GPU's level picking); **GPU** is all passes; **frame** is from
  before collect until the GPU finished. Times in ms.

| copies | camera | path | collect median / min | prepare median | cull GPU | GPU | frame median / min |
|---|---|---|---|---|---|---|---|
| 10k | drive | entities | 4.95 / 0.54 | 0.29 | – | 0.83 | 11.9 / 2.82 |
| 10k | drive | gpu-batch | 0.033 / 0.026 | 0.17 | 0.16 | 0.98 | 6.56 / 2.03 |
| 10k | drive | cpu-batch | 0.032 / 0.023 | 0.18 | – | 0.81 | 4.43 / 1.89 |
| 10k | fly | entities | 2.08 / 0.45 | 0.30 | – | 0.81 | 6.54 / 2.81 |
| 10k | fly | gpu-batch | 0.033 / 0.021 | 0.17 | 0.16 | 0.95 | 4.56 / 2.08 |
| 10k | fly | cpu-batch | 0.032 / 0.026 | 0.19 | – | 0.80 | 4.00 / 1.96 |
| 100k | drive | entities | 5.06 / 1.99 | 0.27 | – | 0.83 | 10.2 / 4.52 |
| 100k | drive | gpu-batch | 0.032 / 0.024 | 0.17 | 0.44 | 1.25 | 4.47 / 2.37 |
| 100k | drive | cpu-batch | 0.032 / 0.026 | 0.21 | – | 0.81 | 4.15 / 1.96 |
| 100k | fly | entities | 5.67 / 2.05 | 0.33 | – | 0.79 | 9.70 / 4.50 |
| 100k | fly | gpu-batch | 0.033 / 0.027 | 0.18 | 0.43 | 1.27 | 4.71 / 2.52 |
| 100k | fly | cpu-batch | 0.032 / 0.023 | 0.22 | – | 0.80 | 3.67 / 2.08 |
| 100k | drive, shadows | entities | 8.98 / 2.09 | 0.38 | – | 1.17 | 18.7 / 4.95 |
| 100k | drive, shadows | gpu-batch | 0.035 / 0.030 | 0.22 | 1.42 | 2.98 | 7.66 / 4.07 |
| 100k | drive, shadows | cpu-batch | 0.035 / 0.026 | 0.42 | – | 1.24 | 6.22 / 2.86 |
| 1M | drive | entities | 38.1 / 17.4 | 0.30 | – | 0.85 | 50.4 / 20.2 |
| 1M | drive | gpu-batch | 0.034 / 0.028 | 0.19 | 3.64 | 4.70 | 14.6 / 5.83 |
| 1M | drive | cpu-batch | 0.033 / 0.023 | 0.50 | – | 0.80 | 12.4 / 2.38 |

(The 1M rows are from a separate run at a load average of about 110.)

Without compute culling (`PONKO_GPU_TIER=webgl2`, 100k, drive; no
timestamps on that tier): the `PreferGpu` batch takes the CPU path by
itself (route "cpu: no compute culling"), prepare 0.22 ms against 0.21 ms
for `PreferCpu`. (Frame times on that run are left out: one run on the busy
machine, inside the noise above.)

## The split

Picking levels on the CPU for every entity is what costs: 0.5 to 2 ms a
frame at best (10k to 100k copies; 2 to 9 ms medians on the busy machine),
17 to 38 ms at a million. Culling the same copies as a batch on the GPU
costs 0.16 ms (10k), 0.43 ms (100k), 1.4 ms (100k with four shadow
cascades) and 3.6 ms (1M) of GPU time, and next to nothing on the CPU
(collect 0.03 ms, prepare 0.17 to 0.22 ms). The entities' per-frame uploads
are small (2 KiB a frame driving, 54 to 61 KiB flying): the renderer
re-uploads only changed instances, so the cost is the picking, not the
upload.

## Is GPU-side level selection a net win? Not here

Against the same batch culled on the CPU, the GPU-driven path saves
0.04 ms of `prepare` at 100k, 0.2 ms with shadows and 0.32 ms at a million,
and costs 0.15 to 0.47 ms more GPU time at 10k to 100k, 1.7 ms with shadows
and 3.9 ms at a million (its culling pass tests every copy for every level
band and view; the CPU path skips whole cells that are out of view or out
of a band). The fastest frames agree: 2.37 against 1.96 ms at 100k, 5.83
against 2.38 ms at a million. On this machine the GPU path never won.

The decision:

- **`LodGroup::instance_levels` is the prototype, and it is opt-in**: a
  game makes a batch of a group's copies itself. The win it measures is
  from batching (no per-entity picking), on either culling path: about 0.5
  to 2 ms of CPU a frame at 10k to 100k copies, 17 ms or more at a million.
- **GPU-side picking stays out of the default path.** `BatchCulling::Auto`
  keeps a batch of several distance levels on the CPU path at any size
  (the crossover in STATIC_BATCHES.md, 131,072 instances, was measured for
  single-level batches and still applies to them). Before this, `Auto`
  sent the million-copy level batch to the GPU, the slower choice here by
  far more than the noise: GPU 4.70 against 0.80 ms, fastest frame 5.83
  against 2.38 ms. `PreferGpu` still sends a level batch to the GPU. This
  is one machine, one scene and one GPU: other GPUs, or much heavier
  meshes than these (480 triangles at the top level), may make the GPU
  pass pay, and are a reason to measure this again.
- Entities stay the way to get crossfades, `LodHysteresis`, skinned levels,
  a level per render-target camera and groups that move.

## Memory

Computed from the sizes of what is held (not read from the allocator):

| copies | entities (CPU) | batch copy (CPU) | GPU-driven pools | CPU-path instance buffer (GPU) |
|---|---|---|---|---|
| 10k | 4.7 MiB | 0.9 MiB | 1.3 MiB | about 0.8 MiB |
| 100k | 47.3 MiB | 9.2 MiB | 20.1 MiB (32.1 with shadows) | about 7.6 MiB |
| 1M | 473 MiB | 92.1 MiB | 160.1 MiB | about 76 MiB |

- An entity holds its transform and a `LodGroup` with its own level and
  renderer lists: about 496 bytes per copy here. The frame's instance list
  holds only what is drawn (0.2 to 0.9 MiB).
- A batch keeps a CPU copy of every instance (80 bytes), its bounding
  sphere (16) and its cells: about 97 bytes per copy. Every copy is also on
  the GPU: 80 bytes per copy on the CPU path (not in the pools column,
  which counts the GPU-driven pools only), and on the GPU-driven path the
  pools add the cells, and per level band and view the visible lists and
  indirect arguments: 136 to 211 bytes per copy here, 337 with the four
  shadow cascades.

So batching trades the entities' 496 bytes of CPU memory per copy for
about 177 bytes (CPU path) or 233 to 308 bytes (GPU-driven; 434 with
shadows) split between
CPU and GPU, and a fixed level choice: the bands follow the field of view
and bias the batch was made with.

## What posing skinned groups costs groups that aren't

`Frame3D::collect` for 100,000 three-level `LodGroup` entities with no
skeleton (the "entities" scene without the GPU: a headless timing of
`collect` alone, 240 frames after 60 of warm-up, camera moving 0.5 m a
frame), this branch against `5a7bae0b`, built and run in turn three times
each, twice:

| build | load average | fastest frame | median |
|---|---|---|---|
| `5a7bae0b` | 128 | 0.71, 0.72, 0.71 ms | 0.79, 0.79, 0.79 ms |
| this branch | 88 to 95 | 0.79, 0.79, 0.76 ms | 0.85, 0.85, 0.85 ms |
| `5a7bae0b` | 22 to 28 | 0.73, 1.11, 0.86 ms | 0.83, 3.55, 0.95 ms |
| this branch | 22 to 36 | 0.79 to 0.82 ms | 0.88 to 0.95 ms |

(The last branch rows are before and after skipping the per-group skin
lookup on frames without posed groups: 0.93 to 0.95 ms and 0.88 to 0.89 ms
medians.) The first pair shows a steady gap: fastest frames 0.71 to
0.72 ms against 0.76 to 0.79 ms, medians 0.79 against 0.85 ms. So the
branch costs about 0.05 to 0.08 ms (7 to 10%) a frame at 100,000 groups,
under 0.1 ms in absolute terms, measured on a busy machine. Renderer
ordinals are counted only for groups with posed renderers, and a frame
without any looks nothing up per group.

## Correctness

The offscreen tests in `src/render/lod_motion_render_test.rs` check that a
batch of `instance_levels` draws the same levels (pixels of each level's
colour) as the entities along a fast flight, on the GPU-driven path and on
a device with WebGL2's limits. Each also builds a control batch with a bias
of 0.5 and asserts that the comparison reports it, so the test can tell
levels apart. `auto_keeps_a_batch_of_lod_levels_on_the_cpu_at_any_size`
checks the `Auto` route above. `LodGroup::instance_levels` unit tests
compare the bands with `LodGroup::pick` over 4,000 distances at four
scales and biases, and at a bias of 0 or less.
