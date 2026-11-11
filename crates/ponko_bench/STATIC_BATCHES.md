# Static instance batches: GPU-driven against CPU culling (#149)

How the two paths of static instance batches (`Graphics::create_instance_batch`,
`create_instance_batch_levels`) cost a frame, before and after #149, and
where the renderer switches between them.

The measurements below describe #149's original atomic append path. For the
stable compaction and CPU distance follow-ups, see
[STATIC_BATCH_FOLLOWUPS.md](STATIC_BATCH_FOLLOWUPS.md); the original numbers
are retained as historical evidence, not measurements of the current shader.

## How to run it

```sh
cargo test --release --lib --features 3d static_batch_benchmark -- --ignored --nocapture
PONKO_GPU_TIER=baseline cargo test --release --lib --features 3d static_batch_benchmark -- --ignored --nocapture
PONKO_GPU_TIER=webgl2   cargo test --release --lib --features 3d static_batch_benchmark -- --ignored --nocapture
PONKO_STATIC_BENCH="one batch" cargo test ...   # only the scenarios whose name contains the text
```

The benchmark (`src/render/static_batch_bench.rs`) opens no window: it
draws into an offscreen texture with the real 3D renderer and waits for
the GPU after every frame. It is an ignored test, so `cargo test` skips it.

## What was measured, and on what

- Apple M4 Max, Metal (wgpu 30.0.1), macOS; rustc 1.99.1, release profile,
  features `3d` plus the defaults. 2026-11-11.
- Target 1280x720 `Rgba8UnormSrgb`, 4x MSAA, depth prepass, no post effects,
  atmosphere off; sun shadows (four cascades) only in the rows that say so.
- Cubes (12 triangles) or spheres (224 triangles) on a square field of
  regions 32 m wide, one batch per region, cells 8 m wide, a maximum
  distance of 0.75 of the field. The camera starts at a corner, looks along
  the diagonal and flies 0.02 m every frame, so view uniforms, visible sets
  and shadow maps change every frame as in a game. The "camera at rest"
  rows keep it still.
- The "spread" rows are one batch of instances 1 m apart with the default
  32 m cells (`InstanceBatchOptions::default().cell_size`) and a maximum
  distance of at most 150 m, of cubes and of spheres of 2,024 and 5,040
  triangles: the heavy meshes a game draws in batches, at the density a
  game places them.
- 20 warm-up frames, then 120 measured frames; a row is the median of
  those, and the best of 5 such runs (3 for the "spread" rows). Rows were
  measured in one to four sessions; each cell is the lowest of them.
- *Before* is revision `d0752442` (the same benchmark file, built against
  the unchanged renderer: its GPU path is a device with compute culling,
  its CPU path a device with WebGL2's limits). *After* is this change.
- Columns: **prepare** is `Renderer3D::prepare` (CPU culling, uniforms,
  the culling table), **record** is recording every pass, **CPU** is
  prepare + record + submit, **cull GPU** is the culling compute pass's
  GPU time (timestamp queries), **GPU** is all passes' GPU time, **frame**
  is the wall time from before prepare until the GPU finished, **draws**
  is draw calls in the model passes.
- **auto** is what a game gets: the path the renderer picks per batch
  (before, that was the GPU-driven path wherever the device has compute
  culling).

The machine was shared with other builds while this ran (load average
between 5 and 40). CPU columns repeat within about 10%; the **GPU ms** and
**frame ms** columns move by up to a factor of two between runs of the
same row on this GPU (it changes its clock speed with load), so compare
them only where the difference is large, or where the rows were run back
to back (said where it matters). The **cull GPU** column and the counts
are steady.

**Every number here is from this one Apple GPU on Metal.** The thresholds
below are fitted to it. Other GPUs, drivers and backends were not
measured, and may cross over elsewhere.

## What changed

Before, every batch, view and distance level had its own buffers and bind
groups, and the culling pass ran one compute dispatch per batch and view:
about 2 to 8 µs of GPU time and 1.3 µs of CPU time each. The CPU path
drew one draw per visible cell and per mesh part, and chose a distance
level per cell.

Now:

- All GPU-driven batches share buffer pools, and **one dispatch per page
  of views** (one page for most scenes, see the guide's limits) culls every
  batch for every view and level, from a table written once a frame. 4,096 batches: 7.8 ms of GPU time for culling became 0.26 ms, and
  the renderer's CPU time 9.4 ms became 1.1 ms.
- Draws are grouped by mesh across batches, so the mesh's buffers and the
  material are bound once per run of batches instead of once per batch.
- The CPU path joins neighbouring visible instances into one draw (4,096
  batches: 58,342 draws became 3,920, CPU time 3.9 ms became 1.0 ms) and
  tests distances per instance in the cells a band's edge crosses, so it
  draws the same instances as the GPU-driven path.
- The renderer picks the path per batch from the crossover below, and a
  game can overrule it per batch (`InstanceBatchOptions::culling`).

## The crossover: which path a batch takes

The GPU-driven path has a fixed cost: the compute pass (12 to 17 µs of GPU
time for one small batch), its table and bind groups (about 25 µs more CPU
time per frame at one batch: 244 against 215 µs), and one bind group and
indirect draw per batch level and pass. CPU culling costs by the cell: it
is the cheaper path until a batch has so many cells that walking them costs
more than that, or until the triangles it draws for nothing cost more.

**By instance count** (cubes, 8 m cells). One batch, after (CPU µs, GPU
path against CPU path): 256 instances 244 / 215; 4,096: 247 / 222; 65,536:
245 / 243; 131,072: 246 / 266; 262,144: 251 / 310; 524,288: 260 / 417;
1,000,000: 261 / 590. Frame times follow (1,000,000: 3.31 against
3.78 ms). Splitting the same instances into more batches doesn't favour
the GPU path: 65,536 cubes in 16 to 4,096 batches cost the CPU path 249 to
962 µs and the GPU path 284 to 1,098 µs, and 1,024 regions of 1,024 cubes
(a million instances) cost the CPU path 424 µs and the GPU path 454 µs.

**By triangles** (one batch, 32 m cells, instances 1 m apart). Cells cull
coarsely: every instance of a cell the view touches is drawn. The compute
pass tests each instance, so it draws fewer triangles, and with a heavy
mesh that outweighs its fixed cost long before 131,072 instances (GPU ms /
frame ms, GPU path against CPU path):

| mesh | instances | instances x triangles | GPU path | CPU path |
|---|---|---|---|---|
| cube, 12 | 10,000 | 0.1 million | 0.31 / 0.83 | 0.24 / 0.76 |
| cube, 12 | 131,072 | 1.6 million | 0.40 / 0.99 | 0.37 / 0.90 |
| sphere, 2,024 | 1,024 | 2 million | 0.42 / 0.92 | 0.44 / 0.89 |
| sphere, 5,040 | 1,024 | 5 million | 0.70 / 1.24 | 0.75 / 1.22 |
| sphere, 2,024 | 4,096 | 8 million | 1.12 / 1.70 | 1.18 / 1.73 |
| sphere, 2,024 | 10,000 | 20 million | 2.28 / 2.87 | 2.46 / 2.96 |
| sphere, 5,040 | 4,096 | 21 million | 2.09 / 2.69 | 2.30 / 2.81 |
| sphere, 5,040 | 10,000 | 50 million | 4.82 / 5.43 | 5.02 / 5.54 |
| sphere, 2,024 | 32,768 | 66 million | 6.84 / 7.52 | 7.03 / 7.58 |
| sphere, 5,040 | 32,768 | 165 million | 14.95 / 15.64 | 15.17 / 15.71 |
| sphere, 2,024 | 131,072 | 265 million | 7.72 / 8.29 | 8.45 / 8.99 |
| sphere, 5,040 | 131,072 | 661 million | 16.85 / 17.49 | 18.29 / 19.00 |

Up to 8 million instance-triangles the two paths are within the noise of
one another (the GPU path draws less, the CPU path records less). From 20
million up the GPU path wins every row, by 0.1 to 1.5 ms a frame.

So the switch is per batch: **a batch is GPU-driven when it has at least
131,072 instances, or when its instances times the triangles of its
heaviest level reach 16,777,216** (2^24, between the last tie and the
first win above), where the device has compute culling. Other batches are
culled on the CPU. `GpuCapabilities::instance_batches` reports both
numbers (`min_gpu_instances`, `min_gpu_triangles`).

Two more rules:

- A batch with a see-through (blended) material stays on the CPU path
  under the automatic choice. The original compute pass had no fixed order.
  Current compaction preserves source order, but does not depth-sort blending.
- `InstanceBatchOptions::culling` overrules the choice per batch:
  `BatchCulling::PreferGpu` or `PreferCpu` (a device without compute
  culling, and a batch too big for the GPU path, stay on the CPU path).
  That is the way out where this one machine's thresholds are wrong for a
  game's GPU or scene.

What the **auto** rows say about small scenes. Before, every batch took
the GPU path on a device with compute culling. Run back to back in one
session (two runs each, the lower): one batch of 256 instances 1.01 ms a
frame before and 0.79 ms after, 4,096: 0.97 and 0.78, 65,536: 1.04 and
0.88, 131,072: 1.14 and 1.04. Between sessions the same rows moved by
more than those differences (an earlier pair of sessions gave 1.23 ms
before and 1.52 ms after at 65,536, with the after run on a busier
machine), so this says no more than: small scenes cost what the CPU path
costs, and in the one back-to-back comparison that was not slower than
before. The CPU columns are the steady ones: 283 µs before and 243 µs
after at 65,536.

## What culling costs against the geometry it saves

65,536 spheres of 224 triangles in 64 batches, with an occluder over part
of the screen, GPU path, after: nothing hidden 1.76 ms of GPU time; half
hidden 0.96 ms; 90% hidden 0.38 ms. The culling pass that does it costs
0.04 ms of GPU time (0.29 to 0.31 ms before). The CPU path saves the same
(0.97 and 0.41 ms) at cell accuracy with no compute pass, which is why it
stays the choice for batches of this size.

The compute pass pays for itself when per-instance culling of one large
batch replaces walking its cells on the CPU (at a million cubes it costs
0.11 ms of GPU time and saves 0.33 ms of CPU time), or when it saves
heavy triangles (the table above).

## What exact distances cost the CPU path

The CPU path now tests the distance of every instance in the cells that a
distance band's edge crosses, so it draws the same instances as the
GPU-driven path (before, it kept or dropped whole cells). That is paid in
`prepare`, only on frames where the view moved:

| one batch, CPU path | prepare before | prepare after, camera moving | prepare after, camera at rest | frame before | frame after, moving |
|---|---|---|---|---|---|
| 65,536 cubes | 38 µs | 56 µs | 26 µs | 0.94 ms | 0.88 ms |
| 262,144 cubes | 46 µs | 121 µs | – | 1.69 ms | 1.45 ms |
| 1,000,000 cubes | 48 µs | 386 µs | 26 µs | 4.77 ms | 3.78 ms |

The `webgl2` tier, which has only this path, measured the same: 365 µs of
prepare at a million instances with the camera moving and 29 µs at rest
(2.94 ms a frame at rest, 3.60 ms moving).

The benchmark's million instances stand in a field 32 m wide with 8 m
cells, so the edge of the maximum distance crosses most cells: about the
worst case. The frame is still shorter than before, because joined draws
and exact distances save more GPU time (4.21 ms became 2.84 ms) than the
tests cost. What was recovered: a view's lists are reused while the view
and the batch have not changed (a camera at rest, and the shadow views of
lights that did not move). What was not: a moving camera pays the tests
every frame. Reusing cells that stayed inside one band needs a per-cell
distance bound kept across frames; it was not built, and a batch that big
takes the GPU path wherever the device has compute culling.

## The tables

Times are best-of medians as described above. `–`: the path has no
culling pass.

### one batch

| batches | per batch | occluded | path | prepare µs | record µs | CPU µs | cull GPU µs | GPU ms | frame ms | draws |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 256 | 0% | gpu, before | 38 | 178 | 258 | 7 | 0.21 | 0.90 | 2 |
| 1 | 256 | 0% | gpu, after | 35 | 169 | 244 | 12 | 0.28 | 0.92 | 2 |
| 1 | 256 | 0% | cpu, before | 37 | 156 | 231 | – | 0.25 | 0.81 | 24 |
| 1 | 256 | 0% | cpu, after | 33 | 148 | 215 | – | 0.24 | 0.82 | 20 |
| 1 | 256 | 0% | auto, before | 39 | 190 | 274 | 9 | 0.30 | 1.01 | 2 |
| 1 | 256 | 0% | auto, after | 34 | 149 | 217 | – | 0.25 | 0.79 | 20 |
| 1 | 4,096 | 0% | gpu, before | 38 | 190 | 273 | 13 | 0.34 | 1.00 | 2 |
| 1 | 4,096 | 0% | gpu, after | 36 | 171 | 247 | 17 | 0.33 | 0.95 | 2 |
| 1 | 4,096 | 0% | cpu, before | 38 | 157 | 231 | – | 0.30 | 0.85 | 28 |
| 1 | 4,096 | 0% | cpu, after | 37 | 150 | 222 | – | 0.27 | 0.81 | 86 |
| 1 | 4,096 | 0% | auto, before | 39 | 198 | 282 | 15 | 0.32 | 0.97 | 2 |
| 1 | 4,096 | 0% | auto, after | 35 | 149 | 218 | – | 0.27 | 0.78 | 86 |
| 1 | 65,536 | 0% | gpu, before | 39 | 201 | 286 | 12 | 0.44 | 1.04 | 2 |
| 1 | 65,536 | 0% | gpu, after | 34 | 171 | 245 | 13 | 0.40 | 0.97 | 2 |
| 1 | 65,536 | 0% | cpu, before | 38 | 161 | 238 | – | 0.45 | 0.94 | 294 |
| 1 | 65,536 | 0% | cpu, after | 56 | 151 | 243 | – | 0.39 | 0.88 | 382 |
| 1 | 65,536 | 0% | auto, before | 38 | 199 | 283 | 12 | 0.45 | 1.04 | 2 |
| 1 | 65,536 | 0% | auto, after | 57 | 151 | 243 | – | 0.39 | 0.88 | 382 |
| 1 | 131,072 | 0% | gpu, before | 39 | 196 | 279 | 13 | 0.53 | 1.13 | 2 |
| 1 | 131,072 | 0% | gpu, after | 34 | 173 | 246 | 15 | 0.52 | 1.04 | 2 |
| 1 | 131,072 | 0% | cpu, before | 39 | 166 | 243 | – | 0.68 | 1.19 | 592 |
| 1 | 131,072 | 0% | cpu, after | 80 | 150 | 266 | – | 0.56 | 1.07 | 542 |
| 1 | 131,072 | 0% | auto, before | 40 | 201 | 287 | 13 | 0.54 | 1.14 | 2 |
| 1 | 131,072 | 0% | auto, after | 34 | 168 | 241 | 15 | 0.52 | 1.04 | 2 |
| 1 | 262,144 | 0% | gpu, before | 41 | 207 | 295 | 21 | 0.86 | 1.44 | 2 |
| 1 | 262,144 | 0% | gpu, after | 34 | 175 | 251 | 22 | 0.84 | 1.37 | 2 |
| 1 | 262,144 | 0% | cpu, before | 46 | 171 | 261 | – | 1.15 | 1.69 | 1158 |
| 1 | 262,144 | 0% | cpu, after | 121 | 153 | 310 | – | 0.88 | 1.45 | 762 |
| 1 | 262,144 | 0% | auto, before | 40 | 200 | 287 | 21 | 0.85 | 1.45 | 2 |
| 1 | 262,144 | 0% | auto, after | 34 | 174 | 246 | 22 | 0.84 | 1.37 | 2 |
| 1 | 524,288 | 0% | gpu, before | 41 | 198 | 284 | 51 | 1.56 | 2.17 | 2 |
| 1 | 524,288 | 0% | gpu, after | 36 | 183 | 260 | 49 | 1.50 | 2.05 | 2 |
| 1 | 524,288 | 0% | cpu, before | 40 | 150 | 227 | – | 2.25 | 2.77 | 2320 |
| 1 | 524,288 | 0% | cpu, after | 224 | 156 | 417 | – | 1.56 | 2.26 | 1080 |
| 1 | 524,288 | 0% | auto, before | 44 | 202 | 292 | 56 | 1.57 | 2.18 | 2 |
| 1 | 524,288 | 0% | auto, after | 36 | 180 | 257 | 50 | 1.50 | 2.03 | 2 |
| 1 | 1,000,000 | 0% | gpu, before | 35 | 185 | 261 | 105 | 2.80 | 3.31 | 2 |
| 1 | 1,000,000 | 0% | gpu, after | 36 | 184 | 261 | 106 | 2.79 | 3.31 | 2 |
| 1 | 1,000,000 | 0% | cpu, before | 48 | 221 | 306 | – | 4.21 | 4.77 | 4412 |
| 1 | 1,000,000 | 0% | cpu, after | 386 | 164 | 590 | – | 2.84 | 3.78 | 1488 |
| 1 | 1,000,000 | 0% | auto, before | 52 | 232 | 332 | 114 | 2.94 | 3.68 | 2 |
| 1 | 1,000,000 | 0% | auto, after | 35 | 182 | 259 | 107 | 2.80 | 3.42 | 2 |

### one batch, camera at rest

| batches | per batch | occluded | path | prepare µs | record µs | CPU µs | cull GPU µs | GPU ms | frame ms | draws |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 65,536 | 0% | gpu, before | 28 | 182 | 250 | 10 | 0.33 | 0.88 | 2 |
| 1 | 65,536 | 0% | gpu, after | 26 | 170 | 232 | 13 | 0.32 | 0.82 | 2 |
| 1 | 65,536 | 0% | cpu, before | 30 | 152 | 218 | – | 0.41 | 0.88 | 254 |
| 1 | 65,536 | 0% | cpu, after | 26 | 147 | 204 | – | 0.37 | 0.82 | 346 |
| 1 | 65,536 | 0% | auto, before | 29 | 191 | 262 | 13 | 0.40 | 0.99 | 2 |
| 1 | 65,536 | 0% | auto, after | 25 | 146 | 206 | – | 0.36 | 0.82 | 346 |
| 1 | 1,000,000 | 0% | gpu, before | 34 | 210 | 288 | 101 | 2.55 | 3.21 | 2 |
| 1 | 1,000,000 | 0% | gpu, after | 26 | 175 | 241 | 96 | 2.48 | 2.96 | 2 |
| 1 | 1,000,000 | 0% | cpu, before | 54 | 213 | 308 | – | 4.03 | 4.71 | 3812 |
| 1 | 1,000,000 | 0% | cpu, after | 26 | 151 | 214 | – | 2.52 | 3.02 | 1354 |
| 1 | 1,000,000 | 0% | auto, before | 33 | 209 | 286 | 98 | 2.50 | 3.07 | 2 |
| 1 | 1,000,000 | 0% | auto, after | 26 | 178 | 245 | 96 | 2.48 | 3.01 | 2 |

### 65,536 cubes in regions

| batches | per batch | occluded | path | prepare µs | record µs | CPU µs | cull GPU µs | GPU ms | frame ms | draws |
|---|---|---|---|---|---|---|---|---|---|---|
| 16 | 4,096 | 0% | gpu, before | 58 | 205 | 306 | 78 | 0.52 | 1.06 | 22 |
| 16 | 4,096 | 0% | gpu, after | 42 | 195 | 284 | 15 | 0.51 | 1.07 | 22 |
| 16 | 4,096 | 0% | cpu, before | 39 | 160 | 234 | – | 0.53 | 1.01 | 328 |
| 16 | 4,096 | 0% | cpu, after | 52 | 158 | 249 | – | 0.46 | 0.96 | 374 |
| 16 | 4,096 | 0% | auto, after | 50 | 153 | 241 | – | 0.46 | 0.97 | 374 |
| 32 | 2,048 | 0% | gpu, before | 84 | 230 | 364 | 255 | 1.12 | 1.92 | 44 |
| 32 | 2,048 | 0% | gpu, after | 38 | 178 | 256 | 15 | 0.49 | 1.00 | 44 |
| 32 | 2,048 | 0% | cpu, before | 43 | 159 | 242 | – | 0.86 | 1.41 | 710 |
| 32 | 2,048 | 0% | cpu, after | 48 | 145 | 228 | – | 0.47 | 0.94 | 416 |
| 32 | 2,048 | 0% | auto, after | 48 | 148 | 232 | – | 0.47 | 0.95 | 416 |
| 64 | 1,024 | 0% | gpu, before | 119 | 233 | 408 | 263 | 1.02 | 1.81 | 74 |
| 64 | 1,024 | 0% | gpu, after | 39 | 190 | 270 | 14 | 0.48 | 1.04 | 74 |
| 64 | 1,024 | 0% | cpu, before | 51 | 200 | 298 | – | 0.77 | 1.33 | 1218 |
| 64 | 1,024 | 0% | cpu, after | 49 | 155 | 242 | – | 0.43 | 0.94 | 400 |
| 64 | 1,024 | 0% | auto, after | 48 | 156 | 241 | – | 0.42 | 0.94 | 400 |
| 128 | 512 | 0% | gpu, before | 232 | 292 | 586 | 663 | 1.62 | 2.71 | 150 |
| 128 | 512 | 0% | gpu, after | 39 | 179 | 261 | 19 | 0.46 | 1.03 | 150 |
| 128 | 512 | 0% | cpu, before | 54 | 200 | 294 | – | 1.17 | 1.86 | 2670 |
| 128 | 512 | 0% | cpu, after | 57 | 164 | 263 | – | 0.43 | 1.00 | 488 |
| 128 | 512 | 0% | auto, after | 52 | 149 | 238 | – | 0.41 | 0.91 | 488 |
| 256 | 256 | 0% | gpu, before | 370 | 433 | 886 | 1175 | 2.11 | 4.05 | 250 |
| 256 | 256 | 0% | gpu, after | 40 | 177 | 258 | 28 | 0.45 | 0.98 | 250 |
| 256 | 256 | 0% | cpu, before | 61 | 303 | 404 | – | 1.70 | 2.57 | 4678 |
| 256 | 256 | 0% | cpu, after | 61 | 154 | 254 | – | 0.40 | 0.93 | 578 |
| 256 | 256 | 0% | auto, after | 60 | 155 | 255 | – | 0.41 | 0.95 | 578 |
| 512 | 128 | 0% | gpu, before | 648 | 543 | 1256 | 1064 | 1.45 | 3.55 | 510 |
| 512 | 128 | 0% | gpu, after | 45 | 188 | 276 | 46 | 0.43 | 1.05 | 510 |
| 512 | 128 | 0% | cpu, before | 66 | 537 | 643 | – | 0.52 | 1.74 | 9118 |
| 512 | 128 | 0% | cpu, after | 80 | 160 | 279 | – | 0.40 | 1.01 | 814 |
| 512 | 128 | 0% | auto, after | 79 | 160 | 279 | – | 0.39 | 0.99 | 814 |
| 1,024 | 64 | 0% | gpu, before | 1316 | 945 | 2344 | 2074 | 2.44 | 6.62 | 958 |
| 1,024 | 64 | 0% | gpu, after | 55 | 290 | 391 | 97 | 0.53 | 1.18 | 958 |
| 1,024 | 64 | 0% | cpu, before | 122 | 1037 | 1209 | – | 1.85 | 4.04 | 18362 |
| 1,024 | 64 | 0% | cpu, after | 119 | 190 | 350 | – | 0.41 | 1.10 | 1266 |
| 1,024 | 64 | 0% | auto, after | 117 | 190 | 345 | – | 0.40 | 1.08 | 1266 |
| 2,048 | 32 | 0% | gpu, before | 2813 | 2048 | 5020 | 4553 | 4.94 | 13.29 | 1950 |
| 2,048 | 32 | 0% | gpu, after | 71 | 501 | 620 | 168 | 0.56 | 1.48 | 1950 |
| 2,048 | 32 | 0% | cpu, before | 162 | 1853 | 2070 | – | 0.98 | 4.76 | 32182 |
| 2,048 | 32 | 0% | cpu, after | 185 | 325 | 557 | – | 0.45 | 1.41 | 2240 |
| 2,048 | 32 | 0% | auto, after | 199 | 335 | 584 | – | 0.44 | 1.52 | 2240 |
| 4,096 | 16 | 0% | gpu, before | 5284 | 3849 | 9386 | 7811 | 8.24 | 22.98 | 3728 |
| 4,096 | 16 | 0% | gpu, after | 116 | 918 | 1098 | 259 | 0.67 | 2.17 | 3728 |
| 4,096 | 16 | 0% | cpu, before | 300 | 3470 | 3855 | – | 1.38 | 8.01 | 58342 |
| 4,096 | 16 | 0% | cpu, after | 331 | 570 | 962 | – | 0.51 | 1.90 | 3920 |
| 4,096 | 16 | 0% | auto, after | 344 | 582 | 987 | – | 0.56 | 2.11 | 3920 |

### 65,536 spheres in regions

| batches | per batch | occluded | path | prepare µs | record µs | CPU µs | cull GPU µs | GPU ms | frame ms | draws |
|---|---|---|---|---|---|---|---|---|---|---|
| 64 | 1,024 | 0% | gpu, before | 110 | 221 | 383 | 156 | 1.89 | 2.66 | 74 |
| 64 | 1,024 | 0% | gpu, after | 37 | 181 | 260 | 13 | 1.76 | 2.33 | 74 |
| 64 | 1,024 | 0% | cpu, before | 48 | 176 | 270 | – | 1.94 | 2.50 | 1216 |
| 64 | 1,024 | 0% | cpu, after | 48 | 157 | 245 | – | 1.75 | 2.26 | 408 |
| 64 | 1,024 | 0% | auto, after | 46 | 154 | 238 | – | 1.73 | 2.25 | 408 |
| 256 | 256 | 0% | gpu, before | 352 | 360 | 780 | 530 | 2.20 | 3.63 | 250 |
| 256 | 256 | 0% | gpu, after | 39 | 186 | 268 | 25 | 1.69 | 2.27 | 250 |
| 256 | 256 | 0% | cpu, before | 67 | 310 | 425 | – | 1.91 | 2.71 | 4674 |
| 256 | 256 | 0% | cpu, after | 59 | 156 | 255 | – | 1.67 | 2.23 | 578 |
| 256 | 256 | 0% | auto, after | 59 | 155 | 254 | – | 1.66 | 2.21 | 578 |
| 1,024 | 64 | 0% | gpu, before | 1380 | 1004 | 2488 | 2085 | 3.81 | 8.29 | 958 |
| 1,024 | 64 | 0% | gpu, after | 56 | 291 | 393 | 81 | 1.74 | 2.38 | 958 |
| 1,024 | 64 | 0% | cpu, before | 130 | 1050 | 1242 | – | 1.93 | 3.63 | 18358 |
| 1,024 | 64 | 0% | cpu, after | 116 | 193 | 353 | – | 1.65 | 2.32 | 1256 |
| 1,024 | 64 | 0% | auto, after | 115 | 192 | 352 | – | 1.67 | 2.32 | 1256 |

### regions of 1,024 cubes

| batches | per batch | occluded | path | prepare µs | record µs | CPU µs | cull GPU µs | GPU ms | frame ms | draws |
|---|---|---|---|---|---|---|---|---|---|---|
| 128 | 1,024 | 0% | gpu, before | 235 | 295 | 607 | 357 | 1.08 | 2.21 | 150 |
| 128 | 1,024 | 0% | gpu, after | 42 | 188 | 274 | 20 | 0.65 | 1.20 | 150 |
| 128 | 1,024 | 0% | cpu, before | 227 | 1008 | 1366 | – | 1.20 | 4.17 | 2678 |
| 128 | 1,024 | 0% | cpu, after | 57 | 162 | 261 | – | 0.64 | 1.17 | 656 |
| 128 | 1,024 | 0% | auto, after | 59 | 164 | 265 | – | 0.64 | 1.19 | 656 |
| 256 | 1,024 | 0% | gpu, before | 1257 | 1882 | 3379 | 1000 | 2.74 | 9.70 | 250 |
| 256 | 1,024 | 0% | gpu, after | 45 | 202 | 295 | 29 | 0.96 | 1.57 | 250 |
| 256 | 1,024 | 0% | cpu, before | 93 | 356 | 524 | – | 1.29 | 2.12 | 4698 |
| 256 | 1,024 | 0% | cpu, after | 67 | 166 | 276 | – | 0.95 | 1.52 | 938 |
| 256 | 1,024 | 0% | auto, after | 66 | 161 | 268 | – | 0.95 | 1.50 | 938 |
| 512 | 1,024 | 0% | gpu, before | 767 | 648 | 1502 | 1254 | 2.91 | 5.89 | 510 |
| 512 | 1,024 | 0% | gpu, after | 49 | 209 | 303 | 53 | 1.70 | 2.26 | 510 |
| 512 | 1,024 | 0% | cpu, before | 96 | 587 | 737 | – | 1.92 | 3.02 | 9610 |
| 512 | 1,024 | 0% | cpu, after | 92 | 166 | 300 | – | 1.69 | 2.31 | 1518 |
| 512 | 1,024 | 0% | auto, after | 96 | 169 | 308 | – | 1.69 | 2.30 | 1518 |
| 1,024 | 1,024 | 0% | gpu, before | 1357 | 957 | 2398 | 2332 | 5.36 | 9.60 | 958 |
| 1,024 | 1,024 | 0% | gpu, after | 78 | 322 | 454 | 103 | 3.05 | 3.92 | 958 |
| 1,024 | 1,024 | 0% | cpu, before | 104 | 1009 | 1157 | – | 3.42 | 5.02 | 18444 |
| 1,024 | 1,024 | 0% | cpu, after | 140 | 238 | 424 | – | 2.90 | 3.73 | 2346 |
| 1,024 | 1,024 | 0% | auto, after | 145 | 238 | 430 | – | 2.89 | 3.74 | 2346 |

### occluded spheres

| batches | per batch | occluded | path | prepare µs | record µs | CPU µs | cull GPU µs | GPU ms | frame ms | draws |
|---|---|---|---|---|---|---|---|---|---|---|
| 64 | 1,024 | 50% | gpu, before | 109 | 187 | 339 | 289 | 1.42 | 2.17 | 74 |
| 64 | 1,024 | 50% | gpu, after | 47 | 186 | 280 | 38 | 0.96 | 1.56 | 74 |
| 64 | 1,024 | 50% | cpu, before | 52 | 151 | 241 | – | 1.06 | 1.57 | 642 |
| 64 | 1,024 | 50% | cpu, after | 72 | 167 | 283 | – | 0.97 | 1.53 | 284 |
| 64 | 1,024 | 50% | auto, after | 71 | 176 | 295 | – | 0.97 | 1.54 | 284 |
| 64 | 1,024 | 90% | gpu, before | 121 | 231 | 407 | 309 | 0.70 | 1.49 | 74 |
| 64 | 1,024 | 90% | gpu, after | 56 | 233 | 349 | 40 | 0.38 | 1.02 | 74 |
| 64 | 1,024 | 90% | cpu, before | 56 | 156 | 250 | – | 0.29 | 0.77 | 132 |
| 64 | 1,024 | 90% | cpu, after | 62 | 160 | 262 | – | 0.41 | 0.92 | 74 |
| 64 | 1,024 | 90% | auto, after | 63 | 158 | 260 | – | 0.41 | 0.92 | 74 |

### sun shadows

| batches | per batch | occluded | path | prepare µs | record µs | CPU µs | cull GPU µs | GPU ms | frame ms | draws |
|---|---|---|---|---|---|---|---|---|---|---|
| 64 | 1,024 | 0% | gpu, before | 252 | 281 | 604 | 292 | 0.88 | 1.85 | 115 |
| 64 | 1,024 | 0% | gpu, after | 59 | 277 | 394 | 19 | 0.72 | 1.49 | 115 |
| 64 | 1,024 | 0% | cpu, before | 59 | 268 | 379 | – | 0.66 | 1.31 | 1867 |
| 64 | 1,024 | 0% | cpu, after | 73 | 248 | 376 | – | 0.63 | 1.29 | 609 |
| 64 | 1,024 | 0% | auto, after | 73 | 255 | 384 | – | 0.64 | 1.29 | 609 |

### three levels

| batches | per batch | occluded | path | prepare µs | record µs | CPU µs | cull GPU µs | GPU ms | frame ms | draws |
|---|---|---|---|---|---|---|---|---|---|---|
| 64 | 1,024 | 0% | gpu, before | 127 | 188 | 358 | 197 | 0.59 | 1.29 | 98 |
| 64 | 1,024 | 0% | gpu, after | 36 | 173 | 250 | 16 | 0.47 | 1.00 | 98 |
| 64 | 1,024 | 0% | cpu, before | 40 | 155 | 233 | – | 0.44 | 0.95 | 1148 |
| 64 | 1,024 | 0% | cpu, after | 55 | 154 | 247 | – | 0.43 | 0.95 | 1040 |
| 64 | 1,024 | 0% | auto, after | 54 | 152 | 244 | – | 0.43 | 0.94 | 1040 |

### three levels, shadows

| batches | per batch | occluded | path | prepare µs | record µs | CPU µs | cull GPU µs | GPU ms | frame ms | draws |
|---|---|---|---|---|---|---|---|---|---|---|
| 64 | 1,024 | 0% | gpu, before | 332 | 317 | 724 | 396 | 0.98 | 2.04 | 153 |
| 64 | 1,024 | 0% | gpu, after | 59 | 267 | 383 | 22 | 0.70 | 1.43 | 153 |
| 64 | 1,024 | 0% | cpu, before | 68 | 263 | 380 | – | 0.66 | 1.31 | 1762 |
| 64 | 1,024 | 0% | cpu, after | 92 | 258 | 403 | – | 0.65 | 1.33 | 1569 |
| 64 | 1,024 | 0% | auto, after | 90 | 254 | 397 | – | 0.65 | 1.31 | 1569 |

### spread cubes, 32 m cells

| batches | per batch | occluded | path | prepare µs | record µs | CPU µs | cull GPU µs | GPU ms | frame ms | draws |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 1,024 | 0% | gpu | 34 | 170 | 243 | 21 | 0.32 | 0.93 | 2 |
| 1 | 1,024 | 0% | cpu | 33 | 151 | 221 | – | 0.23 | 0.79 | 46 |
| 1 | 4,096 | 0% | gpu | 34 | 169 | 243 | 19 | 0.34 | 0.94 | 2 |
| 1 | 4,096 | 0% | cpu | 35 | 151 | 223 | – | 0.25 | 0.78 | 96 |
| 1 | 10,000 | 0% | gpu | 33 | 166 | 238 | 18 | 0.31 | 0.83 | 2 |
| 1 | 10,000 | 0% | cpu | 36 | 148 | 221 | – | 0.24 | 0.76 | 152 |
| 1 | 32,768 | 0% | gpu | 34 | 164 | 238 | 14 | 0.31 | 0.85 | 2 |
| 1 | 32,768 | 0% | cpu | 41 | 152 | 231 | – | 0.35 | 0.80 | 278 |
| 1 | 131,072 | 0% | gpu | 39 | 196 | 281 | 16 | 0.40 | 0.99 | 2 |
| 1 | 131,072 | 0% | cpu | 50 | 160 | 249 | – | 0.37 | 0.90 | 290 |

### spread 2,024-triangle meshes, 32 m cells

| batches | per batch | occluded | path | prepare µs | record µs | CPU µs | cull GPU µs | GPU ms | frame ms | draws |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 1,024 | 0% | gpu | 33 | 164 | 236 | 12 | 0.42 | 0.92 | 2 |
| 1 | 1,024 | 0% | cpu | 32 | 149 | 217 | – | 0.44 | 0.89 | 46 |
| 1 | 4,096 | 0% | gpu | 41 | 200 | 287 | 12 | 1.12 | 1.70 | 2 |
| 1 | 4,096 | 0% | cpu | 43 | 168 | 254 | – | 1.18 | 1.73 | 94 |
| 1 | 10,000 | 0% | gpu | 34 | 186 | 261 | 12 | 2.28 | 2.87 | 2 |
| 1 | 10,000 | 0% | cpu | 36 | 148 | 221 | – | 2.46 | 2.96 | 152 |
| 1 | 32,768 | 0% | gpu | 39 | 196 | 278 | 13 | 6.84 | 7.52 | 2 |
| 1 | 32,768 | 0% | cpu | 42 | 151 | 232 | – | 7.03 | 7.58 | 274 |
| 1 | 131,072 | 0% | gpu | 37 | 191 | 268 | 13 | 7.72 | 8.29 | 2 |
| 1 | 131,072 | 0% | cpu | 49 | 153 | 242 | – | 8.45 | 8.99 | 284 |

### spread 5,040-triangle meshes, 32 m cells

| batches | per batch | occluded | path | prepare µs | record µs | CPU µs | cull GPU µs | GPU ms | frame ms | draws |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 1,024 | 0% | gpu | 37 | 180 | 258 | 12 | 0.70 | 1.24 | 2 |
| 1 | 1,024 | 0% | cpu | 38 | 152 | 225 | – | 0.75 | 1.22 | 46 |
| 1 | 4,096 | 0% | gpu | 37 | 184 | 262 | 12 | 2.09 | 2.69 | 2 |
| 1 | 4,096 | 0% | cpu | 39 | 152 | 227 | – | 2.30 | 2.81 | 94 |
| 1 | 10,000 | 0% | gpu | 38 | 192 | 269 | 12 | 4.82 | 5.43 | 2 |
| 1 | 10,000 | 0% | cpu | 41 | 149 | 225 | – | 5.02 | 5.54 | 152 |
| 1 | 32,768 | 0% | gpu | 40 | 214 | 291 | 13 | 14.95 | 15.64 | 2 |
| 1 | 32,768 | 0% | cpu | 46 | 153 | 236 | – | 15.17 | 15.71 | 274 |
| 1 | 131,072 | 0% | gpu | 52 | 221 | 322 | 14 | 16.85 | 17.49 | 2 |
| 1 | 131,072 | 0% | cpu | 52 | 163 | 254 | – | 18.29 | 19.00 | 286 |

### Other tiers

`PONKO_GPU_TIER=baseline` (wgpu's `downlevel_defaults` limits on the same
adapter), one batch: 65,536 instances, auto 226 µs CPU and 0.85 ms a
frame; 1,000,000, GPU path 245 µs and 3.28 ms, CPU path 576 µs and
3.77 ms, auto 248 µs and 3.31 ms. `PONKO_GPU_TIER=webgl2`
(`downlevel_webgl2_defaults`: no compute) runs the CPU path only: one
batch of 256 203 µs CPU and 0.79 ms a frame, 65,536 231 µs and 0.86 ms,
1,000,000 548 µs and 3.60 ms. A device opened with a tier's limits has no
timestamp queries here, so those runs have no GPU columns.

## Ideas not taken

The first two were **not built**: what they could save is an estimate
worked out from the measurements named. Only the third was measured as an
alternative.

- **One multi-draw for many batches** (estimate). Instance ranges would
  come from the indirect arguments' first instance, so batches need no
  bind group each. It needs `INDIRECT_FIRST_INSTANCE`, which WebGL2 and
  part of GLES lack, so the per-batch path would have to stay as well.
  What it could save is part of recording a batch level: 0.20 µs each on
  the GPU path now ((918 - 195) µs over 3,706 more draws, measured),
  against 0.11 µs for a direct draw. At the 100 to 300 batches a streamed
  scene keeps in view that would be 10 to 30 µs a frame at best, less than
  the run-to-run noise of a frame here. Not built.
- **A table from workgroup to job instead of a binary search** in the
  culling shader (estimate). The whole pass costs 0.26 ms for 4,096
  batches (0.06 µs a job, measured) and 0.02 ms for 64; the search is a
  few steps of it. Not built.
- **Turning off wgpu's indirect-draw validation**
  (`WGPU_VALIDATION_INDIRECT_CALL=0`; measured, GPU path, 65,536 cubes,
  two runs each back to back, the quieter pair). 4,096 batches (3,728
  indirect draws): recording 891 µs with validation, 826 µs without
  (about 0.02 µs a draw), frame 2.11 and 2.01 ms. 2,048 batches: 495 and
  470 µs. 1,024 batches and fewer: within noise (283 and 262 µs at 1,024,
  176 and 169 µs at 16). The check keeps a bad indirect buffer from
  reading past the mesh; 0.07 ms at four thousand GPU-driven batches,
  which the automatic choice does not produce, is not worth dropping it.
  The engine leaves wgpu's default.

## Allocations

Preparing a steady frame (culling on either path, the table, uniforms)
allocates nothing: `steady_frames_allocate_nothing_preparing_batches`
counts zero in the batch code of every frame of a flight, shadows and
occlusion on, where every view's lists are built again each frame, and
zero again when the same frame is prepared a second time (the camera at
rest, the lists reused). wgpu's own allocations inside a buffer write (a
staging buffer each, 24 to 28 allocations a moving frame on the GPU path)
are not the batch code's and are left out, as in the recording count
below.

Recording does allocate, inside wgpu: its command lists grow, and its
tracker and Metal backend allocate as bind groups and buffers are set
(640 allocations for the test scene's frame on the GPU path, 476 on the
CPU path, about one per batch and pass on both). None is made by the batch
code: `recording_batches_allocates_only_inside_wgpu` counts the allocations
made while the batch drawing code runs outside a wgpu call (the test
build marks every wgpu call in it), and finds none. It needs no debug
information, so it holds in CI's builds too. The engine cannot remove
wgpu's own.

## Draw order on the GPU-driven path

Before #309, the compute pass wrote a batch's visible instances in no fixed order, so
the order of a batch's instances in a draw changes from run to run and
frame to frame. For opaque instances that shows only where two meet at
exactly the same depth in a sample: the later one wins. The parity tests
met it once: one edge pixel of a 128 x 128 picture off by 3 of 255 in 12
of 150 runs of the GPU path against itself on the `baseline` tier (the
CPU path against itself: 0 of 150). Blended instances depend on the order
everywhere they overlap, which is why the automatic choice keeps them on
the CPU path.

## Not measured

- Any adapter other than this Apple GPU on Metal: no Vulkan, D3D12, GL or
  mobile numbers. Both thresholds are fitted to this one machine;
  `InstanceBatchOptions::culling` overrules them per batch.
- A real WebGL2 or WebGPU browser. The `webgl2` and `baseline` runs are
  this adapter held to those tiers' limits; they prove the fallback paths
  run and what they record, not what a browser's driver costs.
- GPU memory in use, and frame-time percentiles (the benchmark keeps
  medians; frames are not paced by a display).
