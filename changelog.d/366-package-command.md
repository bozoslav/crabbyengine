### Added
- `ponko package` (`ponko_cli`) turns any game into what players download,
  with one command in its folder: a release build, its assets and license
  files, `THIRD-PARTY-NOTICES.txt` for every linked library, laid out and
  archived for the platform (a Windows zip, a macOS `.app` in a dmg, a Linux
  tar.gz, or with `--web` a folder for any static host), then a headless smoke
  run of the packaged copy from an unrelated folder. The game's
  `[package.metadata.ponko]` table (name, id, icon, assets, files; per example
  under `examples.<name>`) says what goes in; projects from `ponko new` have
  one. `--sign <program>` is the hook for signing and notarization.
  `ponko notices` writes the notices for a library alone.
- `PONKO_SMOKE=<ticks>`: `Game::run` opens no window, runs that many ticks
  headless (frame systems included), prints `ponko: smoke run passed` and
  returns, so a packaged game can be checked on a machine without a display.
- A game started from a macOS app bundle also finds its assets in the
  bundle's `Contents/Resources/assets`.

### Changed
- `scripts/package-coin-dash.py package` and `notices` run `ponko package`
  and `ponko notices`; Coin Dash's package is described in the root
  `Cargo.toml`. Its `smoke` walk-through and command line are unchanged.
