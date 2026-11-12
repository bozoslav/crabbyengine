### Added
- `LodGroup::instance_levels`: a level-of-detail group as the distance
  levels of a static instance batch, for many copies that never move. The
  batch then picks each copy's level where it is culled (in the GPU's
  culling pass on the GPU-driven path, cell by cell on a device without
  compute culling) instead of the CPU picking every entity's level each
  frame. Crossfades, `LodHysteresis` and per-camera levels stay with
  entities.
- `render/lod` in `ponko_bench`: an offscreen benchmark of the split
  between picking levels on the CPU and culling static batches on the GPU
  under a moving camera (`crates/ponko_bench/LOD_SPLIT.md`).

### Changed
- `BatchCulling::Auto` keeps a static batch of several distance levels on
  the CPU culling path at any size; `PreferGpu` still sends it to the GPU.
  Picking levels in the GPU's culling pass measured slower than the CPU's
  cell-by-cell picking at every size up to a million instances, on the one
  GPU measured. Single-level batches take the same path as before.

### Fixed
- A `LodGroup` with a `Skeleton` on its entity draws its skinned renderers
  posed by that skeleton, at every level and through crossfades, instead of
  in their modelled (rest) pose. Only the levels a view draws are skinned
  on the GPU.
- `RenderStats::occluded` counts a `LodGroup` renderer hidden by
  occlusion culling once, also when its group is still seen another way
  (its shadow, another renderer), and no longer counts shadow-only
  casters, which the camera never draws.
