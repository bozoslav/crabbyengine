### Added
- `membership/query/*` benchmarks (`cargo run --release -p ponko_bench -- membership/query`): a
  query over 4096 archetypes of 4-64 components, hot and after the world left the cache, with
  lookup candidates side by side. Results, the alternatives tried and why archetype lookups keep
  their linear scan: `crates/ponko_bench/MEMBERSHIP.md`.

### Changed
- Removing a component that has a destructor through `RawWorld::remove_dynamic` (the C/C++ path)
  no longer allocates for values up to 64 bytes at up to 16-byte alignment, which was one heap
  allocation per remove; its extra cost over a plain remove drops from about 15 ns to about 3 ns.
