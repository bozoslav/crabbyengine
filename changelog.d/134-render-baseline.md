### Added
- A reproducible rendering baseline in `ponko_bench`
  (`cargo run --release -p ponko_bench --features render -- render/scene`):
  seven generated offscreen 3D scenes (static world, dynamic world, many
  materials, heavy shadows, see-through models, skinned figures, a small
  town) reporting the CPU steps of a frame, the GPU time of each pass, draw
  calls, uploads, allocations, countable GPU memory and frame-time
  percentiles, together with the adapter, driver, limits, features,
  resolution, sample count and quality settings. `PONKO_BENCH_JSON` and
  `PONKO_RENDER_CSV` export it; metrics a device can't provide are written
  as missing with the reason. `render_compare` puts two runs side by side
  after checking that they are comparable. See
  `crates/ponko_bench/RENDER_SCENES.md`.
- `bench/bevy_scenes`: three of those scenes built again with Bevy, as a
  separate unpublished crate outside the workspace, for a side-by-side run
  (`PONKO_RENDER_MATCHED=1` on the Ponko side). The document lists what is
  matched and what is not.
- Benchmark records in the JSON export have a 99th percentile (`p99_ms`).
