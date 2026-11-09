### Added
- `membership/*` benchmark group (`cargo run --release -p ponko_bench -- membership`): archetype
  membership and column lookup across 2-64 components, and structural-change costs.

### Changed
- Adding or removing a component is faster on entities with many components: the column mapping is
  one merge pass over the sorted component ids instead of a scan per column, and the transition-edge
  maps no longer use SipHash (2.3x at 32 components, 3.7x at 64).
