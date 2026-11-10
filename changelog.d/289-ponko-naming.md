### Changed

- Export real `ponko_*` C symbols and make `Ponko*` Rust types, `PONKO_*`
  constants, `ponko.h` and `ponko.hpp` the primary interface. ABI version 2,
  layouts and calling conventions are unchanged. Default builds preserve old
  binary symbols through the optional `ponko_ffi/legacy-names` feature;
  deprecated old headers are thin shims and Rust aliases remain available.
- Write `ponko-world` / `ponko-prefab` save tags and Ponko glTF extras; read
  both spellings. Register new instances under the Ponko directory and discover
  both default registries. Prefer the Ponko web canvas, with legacy fallback.
  Benchmark output and current native examples use Ponko names.
- Warn once when a deprecated environment fallback is selected; retain primary
  presence precedence. CMake also warns on old cache settings. Keep versioned
  legacy Steam metadata for older peers; MCP tool/parameter contracts are unchanged.
- Deprecate legacy names on 2026-11-10. Removal requires at least one minor
  release and a future major release **no earlier than 2027-11-10**; there is
  no automatic removal on that date. See [the migration guide](book/src/naming.md)
  for old-server discovery and forward-reader compatibility limits.
