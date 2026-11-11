### Added

- Validation bundles, complete: the `validation_bundle` example can launch a
  game as its own process and drive it through the MCP tools (`--launch`,
  or `--launch-program` for a game that is already built), records the
  build step, the launch and every tool call, tells build failures, launch
  failures, launch timeouts and failed checks apart, finds the commit and
  compiler itself, keeps performance reports and (for `--windowed` runs) a
  screenshot, and records a person's inspection of an image
  (`inspect`). `compare A B` reports setup differences, results, checks
  that came out differently and which files are not comparable; `verify`
  checks a bundle against its manifest.
- `--scenario FILE` puts a launched game of your own (Rust, C or C++)
  through an input-scenario file; its check steps are the assertions.
  `examples/cpp/battle-validation.json` does it for the C++ battle.
- The MCP `launch` tool takes `program`: start an already built executable
  instead of building with cargo.
- `scripts/validation-bundles.sh` and `scripts/validation-bundle-native.sh`;
  CI publishes the reference bundles as the `validation-bundles` artifact
  and runs the C++ battle through the tool in the native job.
- Guide: a "Validation bundles" chapter.

### Changed

- Bundle comparison: how a recorded command went (a tool call answered or
  not, how a build ended, how much output was cut) is no longer a setup
  difference. It is listed as "commands that went differently".

### Fixed

- A game that exits waits (at most 2 seconds) until the control replies it
  owes are written, so the tool that asked it to quit always gets its
  answer; requests it never got to are refused instead of left waiting.
- In a window, a running input scenario now gets one tick per frame, as a
  headless run does. Ticks a tool asked for used to run inside one frame,
  where a scenario's key press stayed "just pressed" for every tick.
