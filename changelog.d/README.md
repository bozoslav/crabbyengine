# Changelog fragments

Pull requests don't edit `CHANGELOG.md`: every merge would make the other
open pull requests conflict in it. Add a fragment here instead, one file per
change, and the release folds them in.

- Name it after the pull request or topic: `changelog.d/<pr-or-topic>.md`,
  for example `changelog.d/1234.md` or `changelog.d/action-ids.md`.
- Write one or more `### Heading` sections, each starting with `- ` bullets.
  Use `Breaking changes` (each bullet names the replacement) and the
  [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) headings
  (`Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`); a
  larger change may use its own heading. Continuation lines are indented
  by two spaces, as in `CHANGELOG.md`.
- A change that breaks a published crate's API needs a `### Breaking changes`
  section, and the pull request the `breaking` label: the Semver checks job
  fails a break it finds without both (CONTRIBUTING.md, "Semver checks").

```md
### Added
- `World::example()` does the thing, with the details users need.

### Fixed
- Saving a world no longer drops empty archetypes.
```

CI validates the fragments with `python3 scripts/changelog.py --check`.

## At release time

```sh
python3 scripts/changelog.py --dry-run  # preview the Unreleased section
python3 scripts/changelog.py            # fold the fragments in, delete them
```

Bullets go to the end of the Unreleased subsection with the same heading
(created if missing, `Breaking changes` first), in file-name order. Then rename `[Unreleased]` to the
version as usual.
