# Shared geometry and draw ordering (#156)

This finishes the state-sorting work from #273/#279. The merged #220 and
subsequent override work are present: grouping uses each part's **resolved**
material, including whole-mesh, per-part, skinned, morph and LOD overrides.
No public Rust or C/C++ signatures or layouts change.

## Method

macOS, Apple M4 Pro (12 logical cores), Metal integrated GPU, driver not
reported by wgpu; rustc 1.96.1 (31fca3adb, 2026-06-26), release builds,
`ponko_bench --features render`, 12 Rayon threads. Base renderer:
`cb65e4534fb3fd4ab4a89da7e874d5afd9f2310b`; implementation:
`2398b5a13fdb44c59b89dda356dfc44261387548`. The base had only the added
`SingleMaterial` benchmark workload applied, with the renderer unchanged.
That workload-only patch is reproducible with:

```sh
git diff cb65e453 2398b5a1 -- src/render/gpu3d/scene_bench.rs
```

Build the base plus that patch and the implementation separately, retaining
the two executables, then run the same commands with each executable. The
commands below build/run the current implementation. Output names should
identify the build. There are no windows, display pacing or vsync. Every frame
waits for the GPU. The deterministic camera orbit is identical in both builds
(0.004 radians/frame); these are matched camera trajectories, not a stationary
camera. Each scene has 60 warm-up frames and 180 measured frames, scale 1,
High quality, 4x MSAA, Rgba8UnormSrgb, shadows and scene-default screen effects.
No other benchmark or build was deliberately run concurrently with this final
paired measurement. This is one paired run, not a claim of statistical
significance for small differences.

```sh
export CARGO_TARGET_DIR=$HOME/.cache/ponko-target
cargo build --release -p ponko_bench --features render
PONKO_REQUIRE_GPU=1 PONKO_RENDER_FRAMES=180 PONKO_RENDER_WARMUP=60 \
  PONKO_RENDER_PNG=images PONKO_BENCH_JSON=results-720.json \
  "$CARGO_TARGET_DIR/release/ponko_bench" \
  'render/scene single-material|multi-material|alpha-heavy'
PONKO_REQUIRE_GPU=1 PONKO_RENDER_FRAMES=180 PONKO_RENDER_WARMUP=60 \
  PONKO_RENDER_SIZE=3840x2160 PONKO_BENCH_JSON=results-4k.json \
  "$CARGO_TARGET_DIR/release/ponko_bench" 'render/scene multi-material'
```

[Machine-readable results](draw-order-156.json) retain environment, limits,
quality, CPU step percentiles, GPU pass percentiles, counts, uploads,
allocations and memory. The base executable's cached build metadata predates
the workload-only patch; the revisions and patch described above define the
comparison. GPU times come from timestamp queries, never CPU wait estimates.

## Results

Times below are milliseconds, before → after. CPU is collect + prepare +
encode + submit. Frame includes GPU completion; it is not pipelined FPS.

| Scene / resolution | CPU p50 | Encode p50 | GPU passes p50 | Frame p50 | Frame p95 | Frame p99 |
|---|---:|---:|---:|---:|---:|---:|
| Single material / 1280×720 | 0.459 → 0.453 | 0.325 → 0.314 | 0.798 → 0.792 | 1.660 → 1.629 | 1.885 → 1.800 | 1.967 → 1.849 |
| Many materials / 1280×720 | 1.844 → 1.531 | 1.260 → 0.948 | 2.674 → 2.685 | 5.296 → 5.027 | 5.729 → 5.248 | 5.839 → 5.379 |
| Many materials / 3840×2160 | 1.665 → 1.480 | 1.035 → 0.876 | 16.468 → 16.476 | 20.016 → 19.890 | 20.421 → 20.090 | 21.472 → 21.438 |
| Transparent heavy / 1280×720 | 1.514 → 3.926 | 0.356 → 2.598 | 12.403 → 13.657 | 14.661 → 18.502 | 15.332 → 19.119 | 18.188 → 20.374 |

The 4K many-material workload is GPU-bound: ~16.47 ms of measured GPU work
versus ~1.5–1.7 ms CPU. It demonstrates the limit of state ordering: recording
gets cheaper, but unchanged shading and pixel cost dominate. GPU throughput
is effectively unchanged; the small frame-time difference is not evidence of
a GPU speedup. At 720p recording improves ~25%, with unchanged geometry.
The single-material workload has unchanged state/draw counts and no measured
CPU, GPU or frame-time regression.

State changes in the **last measured frame**, over model passes (including
shadows). These are issued commands, before wgpu's redundancy filtering.
Recorded draws are p50 over all measured frames.

| Scene | Pipeline changes | Bind groups | Vertex buffers | Index buffers | Recorded draws p50 |
|---|---:|---:|---:|---:|---:|
| Single material | 3 → 3 | 11 → 11 | 11 → 11 | 3 → 3 | 4 → 4 |
| Many materials (both resolutions) | 13 → 13 | 1,554 → 1,554 | 1,554 → 19 | 1,542 → 7 | 1,541 → 1,541 |
| Transparent heavy | 10 → 10 | 28 → 3,527 | 28 → 18 | 17 → 7 | 17 → 3,513 |

A pass binds its shared vertex/index buffers once. Seven index binds in the
many-material frame span shadow chunks, prepass, coverage and opaque work;
the result does not promise one binding for the entire frame.

| Scene | GPU mesh bytes | Process allocations/frame | Allocated bytes/frame |
|---|---:|---:|---:|
| Single material | 1,200 → 2,304 | 661.33 → 661.35 | 133,226 → 133,246 |
| Many materials / 720p | 1,170,324 → 1,572,864 | 845.34 → 840.32 | 1,485,774 → 841,628 |
| Transparent heavy | 15,488 → 20,480 | 777.62 → 792.67 | 174,161 → 1,148,288 |

These process allocation counts include wgpu and the harness; they are not
engine-only steady-state allocations. New draw-list scratch buffers retain
capacity, and the arena flush does nothing without uploads. Mesh uploads are
zero in measured frames. Total uploads/frame are unchanged: ~4,618 bytes
single-material, ~116,766 many-material, ~2,077,701 transparent-heavy. Geometry
counts and visibility match in every paired workload.

## Instancing versus ordering

Opaque passes can reorder parts without changing blending. Each instanced
mesh run is expanded into part draws and sorted by pipeline, resolved
material and shared buffer identity. A full depth prepass needs no distance
sort. Without one, the nearest model origin in each run supplies a
front-to-back tie-break **inside a state group**. Instances within a run
retain their order, so no extra draw or instance copy is required. Dedicated
buffer groups stay together within a pipeline/material group; depth sorting
introduces no extra state binds.
This intentionally uses run-level distance rather than breaking opaque
instancing into per-model draws. GPU tests check near-first ordering and
unchanged draw/buffer counts, including multi-part materials.

Transparent models need a different compromise. Sorting only inside each
mesh run is incorrect when another mesh or material falls between its
models. All blended models are now sorted globally by squared distance from
the camera, with deterministic ties. Only adjacent models of the same run
mesh (including its material override) can coalesce into an instanced draw.
The dense heterogeneous alpha workload therefore goes from 17 to 3,513
recorded model draws at p50. Its added CPU cost and ~10% more GPU time are a
**correctness cost**, not a performance improvement. Re-grouping those draws
by material would restore the wrong compositing. Shared geometry still keeps
buffer binds low. Model-origin sorting is the usual object-level painter's
order, not triangle sorting or order-independent transparency; intersecting
surfaces and parts inside a model retain their existing geometric limits.

## Arena ownership and tier boundary

Immutable vertex/index data use reusable ranges in one shared pair of
buffers. CPU-side indices are rebased once at upload; part ranges carry the
first index. Every draw uses base vertex zero, including indirect static
batches. Thus the same implementation works on baseline and WebGL2 limits,
without assuming base-vertex support. Allocation is bounded by device buffer
limits and u32 ranges; if the shared arena cannot fit a mesh, it uses the
existing dedicated-buffer path. Skinned/morph rest geometry keeps dedicated
storage because skinning writes separate posed vertices and has different
storage usages/lifetimes. Immutable geometry used by GPU-instance buffers
can share arenas.

Freed ranges coalesce and are reused. Growth preserves live offsets and
uploads CPU mirrors into geometrically larger buffers at prepare, before
recording; consumers resolve the current buffer rather than keeping stale
handles. Old buffers live through wgpu command ownership. Freeing the last
shared mesh drops both GPU buffers and mirrors. With live meshes, capacity
and holes are retained for reuse, so GPU bytes can increase by nearly 2×.
CPU mirrors also retain geometry payload plus Vec capacity; their individual
bytes are not separately instrumented (process peak RSS is in the JSON).
No device-allocator memory total is available on Metal. This bounds the
optimization to immutable meshes and avoids adding animated-buffer copies
or changing public mesh APIs.

## Correctness and inspection

Default, baseline and WebGL2-limit render suites ran on Apple M4 Pro / Metal:
379 passed, 11 ignored per tier. Capability-specific returns are not proof of
unsupported hardware features. These limit tiers do not execute a browser
WebGL2 backend or test another vendor's GPU. Full CI is requested separately.

New regressions cover overlapping transparent models from different meshes,
a material override, a mesh split around another mesh in depth order,
movement that swaps the near color, and a deliberately material-grouped
negative control. Shared geometry is checked by GPU readback after upload,
free/reuse, growth and empty/regrow, and by rendered pixels after reuse and
growth, with a removed-mesh negative control. Multi-part grouping and opaque
depth order preserve draw/buffer counts. Existing material, skin, static
batch, image-reference and lifecycle suites pass; reference files are unchanged.

The final single-material and many-material screenshots are byte-identical
before/after. The alpha screenshots were inspected and intentionally differ
because global ordering restores previously obscured surfaces:
[before](draw-order-156-alpha-before.png), [after](draw-order-156-alpha-after.png).
They are benchmark artifacts, not regenerated image references.

`perf.get` keeps `render_inspector.schema = 1` and adds
`counts.state_changes = { pipelines, bind_groups, vertex_buffers,
index_buffers, draws }`. The counters describe the last recorded 3D frame,
include static batches, exclude non-model work, and are read without
consuming them. Headless counts stay null with an unavailable reason;
2D-only rendering has null state changes with its own reason. Native frame
benchmark JSON/text also includes `state_changes`; offscreen scene reports
already expose changes and `counts.recorded_draws`. That recorded counter is
authoritative for transparent splits; the older `draw_calls` total is an
estimate that does not enumerate the split list.

The engine's MCP `perf_report` description is updated additively. Reviewer-owned
plugin follow-up (no plugin repository changed): document schema-1
`render_inspector.counts.state_changes` and its five exact fields, last-frame
model-pass scope, non-consuming reads, and headless/2D null behavior; update
`perf_report` result examples and generated host instructions/descriptions to
match. Tool names, parameters and canonical MCP connection configuration do
not change.
