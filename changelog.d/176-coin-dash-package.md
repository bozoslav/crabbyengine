### Added
- `coin_dash`, the complete-game reference, is now packaged and smoke-tested
  (#176): `scripts/package-coin-dash.py package` lays out the program, its
  asset, the licenses, a content notice and a generated third-party notice;
  `smoke` unpacks it elsewhere and plays it from an unrelated folder, headless
  on every desktop and in a real window, checking saves, damaged-save recovery,
  the OS save folder and a missing-assets failure. The new `package-smoke` CI job
  runs it on Linux, macOS and Windows.
- `coin_dash` gains a settings screen on the retained UI: UI scale, volume and
  rebindable keys and gamepad buttons (Escape, Tab, East and the d-pad stay with
  the menus), saved beside the progress with the same
  recovery, plus sound effects made in code, a coin picture loaded from
  `assets/coin_dash/`, and `--smoke headless|window` to play itself.
- `coin_dash` builds for the web (`web/build.sh coin_dash`) and runs in the
  `web-smoke` job; `web/smoke/smoke.mjs` takes `--asset`, `--asset-size` and
  `--press <Key>=<field>:<value>` so it can check games other than its fixture.
- Guide chapter "A complete game, packaged": the build and run path, the
  package and its notices, the smoke test, the web build, UI scaling, rebinding
  and navigation with their limitations, and the gap list linking the issues
  that own the rest.
