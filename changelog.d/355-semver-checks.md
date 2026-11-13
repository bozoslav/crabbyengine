### Added
- CI checks the public API of every published crate against its last crates.io
  release (`cargo-semver-checks`, the `Semver checks` job). A break fails any
  change to a `release-*` branch, and on `main` passes only with the `breaking`
  label and a `### Breaking changes` fragment. CONTRIBUTING.md explains how to
  read a failure and what the tool does not catch.
