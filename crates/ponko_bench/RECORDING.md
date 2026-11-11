# 3D command recording (#147)

What turning a prepared 3D frame's passes into GPU command buffers costs
(`Renderer3D::draw`), before and after the frame's passes were described by
step, resource use and frame features (`src/render/gpu3d/passes.rs`).

The description is checked in debug builds only: the frame's order and
features, the steps each part records, texture formats and sample counts, and
attachments discarded while a later step reads them. Release builds compile
the checks out (`cfg!(debug_assertions)`), so recording should cost the same.
This measures that it does.

## Workload

`render/record` in `ponko_bench` (`src/record.rs`, behind the `render`
feature), through the hidden `ponko::render::RecordBench`: an offscreen
renderer drawing into a 1280×720 `Rgba8UnormSrgb` texture with 4× multisampling.
4,096 models of 64 meshes (one material each) on a grid, a camera that drifts
every frame, default sun shadows.

- `plain`: sun shadows, depth prepass, opaque, bloom, tone mapping.
- `effects`: the same with an atmosphere, temporal antialiasing (motion
  vectors), ambient occlusion and screen-space reflections.

Each run draws 60 warm-up frames, then 600 measured ones. Only
`Renderer3D::draw` is timed: every part recorded into its own command buffer
(7 per frame here) on the rayon pool, and `finish`. Preparing the frame,
submitting and the GPU's work are outside the timer; the run waits for the
GPU after each frame.

```sh
cargo run --release -p ponko_bench --features render -- render/record
```

## Results

Measured 2026-11-11 on an Apple M4 Max (14 cores, 36 GB), macOS 27.0.1,
Metal, wgpu 30.0.1, Rust 1.99.1, release profile, default features plus
`physics`, `physics3d`, `navmesh` and `window`. Before: `1dde9b8b` (main at
`1eb35347` plus this benchmark). After: `517dcfae`.

Other builds were running on the machine (load average about 30), which shows
in the 95th percentiles and in whole runs. The two binaries were therefore run
in turn, six times each. Every round is listed, in the order it ran (the
before binary first in each):

| Workload, round | Median µs before → after | 95th percentile µs before → after | Minimum µs before → after |
|---|---:|---:|---:|
| plain, 1 | 239 → 927 | 345 → 21,053 | 132 → 234 |
| plain, 2 | 364 → 244 | 18,331 → 368 | 171 → 146 |
| plain, 3 | 423 → 281 | 8,263 → 520 | 176 → 171 |
| plain, 4 | 245 → 242 | 315 → 308 | 143 → 128 |
| plain, 5 | 246 → 247 | 354 → 350 | 139 → 139 |
| plain, 6 | 234 → 235 | 333 → 306 | 137 → 141 |
| effects, 1 | 578 → 364 | 9,060 → 4,746 | 156 → 184 |
| effects, 2 | 573 → 415 | 14,994 → 2,332 | 208 → 221 |
| effects, 3 | 332 → 377 | 605 → 1,443 | 197 → 185 |
| effects, 4 | 325 → 310 | 423 → 416 | 191 → 168 |
| effects, 5 | 342 → 305 | 438 → 450 | 168 → 181 |
| effects, 6 | 296 → 305 | 395 → 392 | 163 → 145 |

Rounds 1 to 3 were disturbed by the other builds: in each, one binary or the
other has a 95th percentile of milliseconds, and the medians move with it in
both directions (927 µs after in round 1, 364 and 423 µs before in rounds 2
and 3). Rounds 4 to 6 are the ones where both binaries' 95th percentile
stayed under 0.5 ms.

Over all six rounds the lowest median was 234 → 235 µs (plain) and
296 → 305 µs (effects), and the lowest minimum 132 → 128 µs and
156 → 145 µs. In the undisturbed rounds the differences between rounds of one
binary (234 to 246 µs, 296 to 342 µs) are larger than any difference between
the two. No regression in command recording is measurable; with this much
other load, a difference of a few percent would not be visible either.

Not measured: debug builds (where the checks run), other adapters and
backends, the browser, GPU time, memory, allocations.
