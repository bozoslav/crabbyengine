### Added
- The release workflow accepts release candidates (`v0.2.0-rc.1`): the crates
  publish as pre-releases that Cargo never selects unless the version is
  written out, the GitHub release is marked pre-release, and the notes may
  still be under `[Unreleased]`. It also accepts `vX.Y.Z` on a `release-X.Y`
  branch as well as on `main`, so patch releases can be cut while `main` holds
  breaking changes. CONTRIBUTING.md's "Releasing" describes both flows and the
  `backport` label's cherry-pick procedure.
