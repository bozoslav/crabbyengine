### Fixed
- Skip the static instance page-generation GPU test with a logged capability
  reason on tiers without compute culling, including forced WebGL2 on Metal.
  Those tiers render instance batches through the CPU fallback.
