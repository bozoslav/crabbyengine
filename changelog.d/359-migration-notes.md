### Added
- A `### Breaking changes` fragment entry says what changed, what to write
  instead and shows the code before and after; `scripts/changelog.py --check`
  refuses one that doesn't. At release time `scripts/changelog.py --version
  X.Y.Z` folds the entries into the CHANGELOG's Breaking changes block and
  generates `book/src/migrating-to-X.Y.md` from the same entries, so the
  migration chapter of a minor release is no longer written by hand.
- The Semver checks job also fails a pull request that carries the `breaking`
  label without such a fragment, and `scripts/release-check.py` refuses a minor
  release with breaking changes that lacks its migration chapter.
