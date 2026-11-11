### Fixed
- The renderer lifetime test submits its pending queue writes before it
  reads the allocator report, so it no longer reports growth on backends
  that count allocations (it failed on Windows software GPUs).
