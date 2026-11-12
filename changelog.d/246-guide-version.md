### Changed
- The guide says which version it documents: every page opens with a note
  that it follows `main`, with the dependency line that compiles its
  examples (`ponko = { git = "https://github.com/ponkoengine/ponko" }`), and
  Getting started shows that line next to `cargo add ponko`. The note comes
  from a small mdBook preprocessor, `scripts/guide-versions.py`.
- The Pages workflow also publishes the guide of every release tag
  (`/ponko/vX.Y.Z/`), built from that tag's sources, next to the guide for
  `main`.
- Releases happen only when they agree: the release workflow's first job runs
  `scripts/release-check.py`, and nothing is built, released or published for
  a tag that is not on `main`, whose crate versions differ from it, or
  without a non-empty CHANGELOG section or with fragments left over. A weekly
  audit workflow fails if a crates.io version has no matching git tag. The
  release steps are in CONTRIBUTING.md ("Releasing").

### Fixed
- The "Named input actions" example in the guide shows the `use ponko::input::{...}`
  line it needs for gamepad bindings, so a copy compiles as it is.
