### Changed
- [STABILITY.md](STABILITY.md) is a 0.x policy: 0.1.x releases are
  compatible with each other, a 0.y.0 release may break things and lists
  every break with its replacement, and the MSRV and the C ABI version only
  change in a 0.y.0. It names the unstable surfaces: `#[doc(hidden)]` items
  (including `ponko::__ffi`), the `bench-internals` feature,
  `ponko::ui::unstable_taffy` and the re-exported `glam`, `serde_json` and
  `steamworks`.
- The C library archives of a release carry `THIRD-PARTY-NOTICES.txt`, the
  licenses of every library linked into them, and `NOTICE` names the
  vendored SMAA shader and tables. Every published crate carries the license
  files and a README.
- The guide lives at https://ponko.dev/guide/ and links the API reference on
  docs.rs; the GitHub Pages workflow and its per-release guide copies are
  gone.

### Removed
- `ponko_bench` is no longer published to crates.io (it is a workspace tool).
