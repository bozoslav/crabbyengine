# A whole 3D frame, before and after the renderer's split into subsystems (#136)

#136 moved `Renderer3D`'s code into subsystems (resources, material
shaders, targets, shadow maps, pass preparation, pass encoding, temporal
history, game effects, the post chain) without changing what a frame does.
This is the check that it also costs the same.

## How to run it

```sh
cargo test --release --lib --features 3d renderer_frame_benchmark -- --ignored --nocapture
```

The benchmark (`src/render/frame_bench.rs`) opens no window: it draws into
an offscreen texture with the real 3D renderer and waits for the GPU after
every frame. It is an ignored test, so `cargo test` skips it.

## What was measured, and on what

- Apple M4 Max, Metal (wgpu 30.0.1), macOS; rustc 1.99.1, release profile,
  features `3d` plus the defaults. 2026-11-11.
- Target 1280x720 `Rgba8UnormSrgb`, 4x MSAA, depth prepass, sun shadows
  (four cascades), atmosphere off.
- 4000 ordinary models (no instance batches) of 40 meshes with a material
  each: cubes and spheres, every eighth material blended, every fifth cut
  out. The camera flies 0.02 m a frame, so uniforms, the sorted
  transparent models and the shadow maps change every frame.
- Two rows: tone mapping only; and the same with temporal antialiasing,
  bloom, GTAO and screen-space reflections, which exercise the post chain
  and the temporal history.
- 30 warm-up frames, then 200 measured; each row is the best of five runs'
  medians. `prepare`: `Renderer3D::prepare` (uploads, lists, decisions).
  `record`: encoding every pass. `submit`: handing the command buffers to
  the GPU. CPU = their sum. GPU: the passes' timestamp queries, summed.
  `frame`: wall time including the wait for the GPU.
- Before: revision `1eb35347` plus the benchmark alone (`80bb5595`).
  After: `4e696eb2` (all five stages).
  The review fixes that followed (the post chain's stale check and draw
  count moved into it, the screen-space passes' state in a struct, two
  more named preparation steps) move code the same way and were not
  measured again.
- Not measured: memory and allocations (the refactor moves code and
  fields; it adds no per-frame allocation: the frame lists keep their
  capacity as before), frame-time percentiles, other adapters and the web.

## Results

The machine was shared with other builds (load average about 30), so the
two revisions were run back to back, and the rows below are that pair.

| revision | scene | prepare ms | record ms | submit ms | CPU ms | GPU ms | frame ms | model draws |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| before | models, shadows, tone mapping | 0.210 | 0.277 | 0.067 | 0.554 | 0.592 | 1.424 | 146 |
| after | models, shadows, tone mapping | 0.211 | 0.288 | 0.068 | 0.567 | 0.606 | 1.440 | 146 |
| before | the same with TAA, bloom, GTAO and reflections | 0.220 | 0.350 | 0.083 | 0.653 | 1.288 | 2.254 | 146 |
| after | the same with TAA, bloom, GTAO and reflections | 0.224 | 0.363 | 0.077 | 0.664 | 1.290 | 2.259 | 146 |

The after numbers are slightly higher in every CPU column but one: record
is 4.0% and 3.7% higher (0.011 and 0.013 ms), prepare 0.5% and 2%, CPU
time as a whole 2.3% and 1.7%; only submit in the second row is lower. GPU
time is 2.4% and 0.2% higher. The differences (at most 0.013 ms of CPU and
0.014 ms of GPU time a frame) are inside the run-to-run spread on this
machine, so they are not shown to be a cost of the refactor, and not
shown to be nothing either: other runs of the same
revisions, under heavier load, gave 0.58 to 0.60 ms and 0.66 to 0.77 ms of
CPU time for the two rows after the refactor, and 0.60 and 0.77 ms before.
GPU time moved with the load far more than with the revision (0.6 to
1.6 ms for the first row), so only the back-to-back pair compares it. The
draw count is the same: the frame records the same passes and draws.
