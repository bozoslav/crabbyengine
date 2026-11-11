### Added
- `InstanceBatchStats` (`RenderStats::instance_batches`) counts the static
  instance batches on each path (GPU-driven, CPU because small, CPU because
  the device has no compute culling, CPU because past a limit, dropped),
  the shared buffer pools and their pages, the frame's culling dispatches
  and jobs, and the batch views a limit kept from being drawn. The render
  inspector has the same under `counts.instance_batches`.
- `GpuBatchLimits` (`GpuCapabilities::instance_batches`) says whether
  batches can be GPU-driven, from how many instances or instance-triangles
  they are, and the most instances and cells the device takes.
- `BatchCulling` (`InstanceBatchOptions::culling`): `Auto` (the default),
  `PreferGpu` or `PreferCpu` overrules the automatic choice of path for
  one batch.
- An offscreen benchmark of static instance batches, with its results in
  `crates/ponko_bench/STATIC_BATCHES.md`.

### Changed
- New public fields on structs that are not `#[non_exhaustive]`:
  `RenderStats::instance_batches`, `GpuCapabilities::instance_batches`,
  `GpuLimits::max_buffer_size` and `InstanceBatchOptions::culling`.
  Reading fields and building with `..Default::default()` keep working;
  code that writes one of these structs out field by field, or matches it
  without `..`, has to add the field. Capability reports saved before
  still load (the new fields default). The C `PonkoInstanceBatchOptions`
  layout is unchanged; batches made from C and C++ take the automatic
  choice.
- GPU-driven instance batches are culled from shared buffers in one
  compute dispatch per page of views (one page in most scenes) instead of
  one dispatch per batch and view (4,096 batches: 7.8 ms of GPU time for
  culling became 0.26 ms, and the renderer's CPU time 9.4 ms became
  1.1 ms).
- A batch is GPU-driven when it has at least 131,072 instances, or when
  its instances times the triangles of its heaviest level reach
  16,777,216; other batches are culled on the CPU, also where the device
  has compute culling, and so are batches with a blended material. Both
  thresholds are crossovers measured on one Apple GPU on Metal. Draws of
  batches that share a mesh share its state.
- The CPU path draws neighbouring visible instances together (4,096
  batches: 58,342 draws became 3,920) and tests distance bands per
  instance where a cell straddles one, so it draws the same picture as
  the GPU-driven path, overlapping bands included. Those tests cost a
  moving camera up to 0.34 ms of CPU time at a million instances in one
  batch; a view that has not moved reuses its lists.
- Renderer warnings about instance batches past a device limit go to the
  browser's console on the web (they were printed nowhere there), and
  each is given once until the batch fits again.

### Fixed
- An instance batch larger than a storage buffer binding or a GPU buffer,
  more culling cells than one dispatch takes, or an occlusion grid larger
  than the culling pass's buffer no longer fails GPU validation: the batch
  falls back to CPU culling, or the views past the limit are left out,
  counted in `InstanceBatchStats::views_skipped` and warned about once.
  The culling pass's per-frame buffer stays within one storage binding
  too (the occlusion grid gives way to the views).
- GPU memory of GPU-driven instance batches goes back when the batches
  are freed, instead of staying at its peak.
