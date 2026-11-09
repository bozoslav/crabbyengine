### Changed
- 3D shadow maps, depth prepasses, the opaque pass and target cameras draw
  their models sorted by pipeline, then material, then mesh, instead of in
  mesh order, so neighbouring draws share state. The pixels are unchanged;
  transparent models keep their order. In a 2000-mesh, 24-material scene
  this took pipeline changes per frame from 5393 to 57 and bind group
  changes from 10834 to 125.
- The plain instanced draw path no longer rebinds the instance buffer and
  the storage placeholder before every draw.
