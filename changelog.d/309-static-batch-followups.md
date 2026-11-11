### Added
- Per-batch culling setters in Rust, C and C++, preserving the existing C options layout.

### Fixed
- GPU static batches retain source order for depth ties; target camera parity no longer permits pixel outliers.
- Empty static batch pools and pages retain buffers for eight frames, including entirely empty scenes, and replacement buffers keep advancing generations.
- Cached shadow maps recover on the first frame after batch view limits stop applying.
- Automatic batch routing responds immediately to material blend changes.

### Changed
- CPU distance boundary checks evaluate four packed points at a time while preserving scalar distance semantics.
