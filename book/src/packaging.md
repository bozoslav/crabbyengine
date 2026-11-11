# A complete game, packaged

[`examples/coin_dash`](https://github.com/ponkoengine/ponko/tree/main/examples/coin_dash)
is the engine's reference for a small *finished* game: title screen, settings,
rounds with a win and a loss, pause, restart, result screens, sound, saved
progress, keyboard and gamepad, and a download you can unzip anywhere. It uses
only the public `Game`/`Ctx` API and content made for it: one 32 x 32 picture
(`assets/coin_dash/coin.png`), a disc and the sound effects generated in code,
and the engine's built-in fonts. Rules and tests are described in
[Headless runs and testing](headless.md#a-complete-game-tested-headlessly);
this chapter is about getting it to players and what it shows.

## One way to build and run it

From a fresh checkout, with a Rust toolchain (on Linux also the ALSA and udev
development headers, as for any example):

```sh
git clone https://github.com/ponkoengine/ponko && cd ponko
cargo run --release --example coin_dash      # play it
cargo test --test coin_dash                  # the whole game, headlessly
```

Move with the arrow keys, WASD or the left stick; Enter starts; Esc pauses; `O`
opens the settings. A round is eight coins in twenty seconds. The best time and
the high score are saved (see [Your files](#your-files)).

## The package

```sh
python3 scripts/package-coin-dash.py package              # release build -> target/package/coin-dash-<version>-<os>-<arch>/
python3 scripts/package-coin-dash.py smoke --package target/package
```

`package` builds the example and lays out a folder you can zip, with the same
files on every desktop:

| File | What it is |
|---|---|
| `coin_dash` (`coin_dash.exe`) | The program |
| `assets/coin_dash/coin.png` | The game's one asset, found next to the program |
| `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE` | The engine's and the game's license |
| `CONTENT-NOTICES.txt` | What the game ships: the picture's author and license, "no sound files", "no font file" |
| `THIRD-PARTY-NOTICES.txt` | Every library linked into the program, with its declared license and the license texts its source ships |
| `README.txt` | How to run it, the controls, where it saves |

The asset lookup is the engine's own (`asset_roots`: next to `Cargo.toml` under
`cargo run`, next to the executable, then the current folder), so an unpacked
game finds its files wherever it is started from. The third-party notice is made
from `cargo tree -p ponko` for the platform it is built on (so it lists what
that binary links, not every optional feature) and the license files in each
crate's source, including the fonts' (`OFL.txt`, `UFL.txt`) in
`epaint_default_fonts`. Identical texts are printed once, with the libraries
that use them. `tests/coin_dash.rs` fails if a file under `assets/coin_dash/` is
not named in `CONTENT-NOTICES.txt`, and the smoke test fails if it is not in the
package's copy either. A library that declares no `license` (or `license_file`)
fails the packaging by name instead of being listed as "unknown", and so does a
dev-dependency of `ponko` that this platform would link without the notice
listing it; `python3 scripts/package-coin-dash.py selftest` checks both with
made-up crates, plus the exit-code decisions below.

## The smoke test

`coin_dash --smoke headless|window [--report file]` plays the game by itself:
title, settings (open, close), a round won by chasing coins, back to the title.
It is a simulation system like the scripted players in the tests
(`examples/coin_dash/smoke.rs`), and writes a JSON report of the screens it saw,
the rounds saved, how the progress and settings files were found, and the size of
the loaded coin picture. In `headless` mode there is no window or GPU; in
`window` mode a window opens and the report also counts the frames drawn.

`package-coin-dash.py smoke` unpacks the package to a new folder whose name has
a space and a non-ASCII letter (outside the checkout, with `CARGO_*` removed
from the environment), starts the program from a different, empty folder, and
checks:

1. the package's files, licenses and notices (including that the notices name
   the libraries and the shipped asset);
2. a first run: the walk-through finishes, the asset was found and decoded, and
   `progress.json` and `settings.json` exist;
3. a second run starts from them (`Loaded`) and adds a round;
4. a damaged `progress.json` is recovered from (`Recovered`, kept as
   `progress.json.bad`) and written again;
5. with no `COIN_DASH_SAVE`, files go to the operating system's data folder
   (a home folder of the test's own: `%APPDATA%`, `~/Library/Application
   Support`, `$XDG_DATA_HOME`);
6. a copy of the package *without* `assets/` fails, and says which file it
   could not find.

`--mode window` repeats step 2 in a real window. Exit codes: 0 passed, 1 a
check failed, 2 unsupported (no display or GPU, only without
`--allow-unsupported`), 3 no package. "Unsupported" means the engine's own
message that it found no GPU or could not open a window ("ponko: no compatible
GPU found" and the like); any other crash, such as a platform error, is a
failure, and under GitHub Actions a skipped window run prints a warning
annotation.

CI (`package-smoke`, on `main` and with the `full-ci` label) runs both
modes on Linux, macOS and Windows, and the window run must open a window on
all three: Linux under a virtual display with software Vulkan, Windows with
DirectX 12 only (WARP, the software renderer) where the runner has no GPU. It
packages the dev
profile (same layout and notices, and the dependencies the other jobs already
built); a release package is the command above. Locally, the window mode needs
a display, and the headless mode is what runs anywhere. On Linux under X11
a window also needs the system's X11 libraries at run time, among them
`libxkbcommon-x11` (a minimal install, like CI's runner, may lack it).

## Your files

Progress (best time, high score, rounds) and settings live in the player's data
folder: `%APPDATA%\coin-dash\`, `~/Library/Application Support/coin-dash/`, or
`$XDG_DATA_HOME/coin-dash/` (`~/.local/share/coin-dash/`).
`COIN_DASH_SAVE=<file>` moves the progress file, and the settings go beside it.
Files are written through a temporary file and renamed, so a crash leaves the
old one. A missing file is a first run. An unreadable one, one that doesn't
parse, one with impossible numbers or another version is moved to `<name>.bad`,
the game starts from the defaults and says so on the title screen. A write that
fails (a read-only disk) is shown and doesn't stop the game. The tests cover
every one of these cases for both files.

## The web build

```sh
web/build.sh coin_dash && python3 -m http.server -d target/web/coin_dash 8000
```

builds the same game for the browser: the coin picture is fetched with the
asset list, the retained settings screen and the keys work as on the desktop.
It keeps **no progress or settings** in a browser (there are no files; see
[Web builds](web.md)) and has no quit command. The `web-smoke` CI job (the
browser runner of `web/smoke`, [issue
#173](https://github.com/ponkoengine/ponko/issues/173)) builds it after
the fixture and runs it in headless Chromium on WebGL2: it checks that the
asset was fetched and decoded as 32 x 32, that the simulation advances, that a
click and keys arrive, and that the screen the game reports follows them
(the `coin_dash` entry of `web/smoke/fixtures.json`: `--press Enter=screen:Playing
--press Escape=screen:Paused --press Q=screen:Title --press O=screen:Settings
--press Escape=screen:Title`), then that the canvas is not blank. The game
publishes its state on `<html data-smoke>` in the fixture's format plus the
screen name, and the GPU the engine opened on `<html data-smoke-gpu>`
(`publish_web_state`).

## UI scaling, rebinding and navigation

The settings screen (`examples/coin_dash/settings.rs`) is the engine's
[retained UI](ui.md), and shows three things the engine already has:

**Navigation.** Tab and Shift-Tab, the arrow keys or the d-pad move the focus;
Enter, Space or South press the focused control; Escape or East closes the
screen; the list of controls scrolls to keep the focus in view; the slider
changes by 0.05 on left and right. The game writes no navigation code, only
the reaction to the events. The tests drive it with the input the engine would
build, and find controls by their accessible names (so they also check that
every control has a useful one: the key buttons read "Confirm key: Enter", not
just "Enter"):

```rust
{{#include ../../tests/coin_dash.rs:ui_drive}}
```

**UI scale.** `Ui::set_scale` scales the screen's geometry, text and hit
testing together; the scale buttons step through 75%, 100%, 125% and 150%. The
text the game draws itself (`hud`) multiplies its sizes by the same factor, and a
test measures every screen's lines at every scale and fails if one is wider or
taller than the window. The scale is saved.

**Rebinding.** The installed `ActionMap` is the one record of what is bound.
At startup `bindings_for` turns the saved `Controls` (a key and a gamepad
button per control) into a map and `ctx.input.set_actions` installs it. The
settings screen shows the keys it reads back from `ctx.input.actions()` with
`Controls::from_bindings`, so a binding installed some other way (a
user-edited actions file, say) shows up as it is. Pressing a control's button
opens a small modal prompt (a screen with nothing to focus, so no key can press
a button behind it), and the next key or button pressed is the answer. The
check runs against what the map binds now, `ctx.input.set_action` replaces
only the action that control feeds (its `ActionId` stays valid), and the copy
saved with the other settings is read back from the map. The hints on the
title, pause and result screens show the current keys.

```rust
{{#include ../../examples/coin_dash/settings.rs:set_key}}
```

```rust
{{#include ../../examples/coin_dash/settings.rs:bindings_for}}
```

```rust
{{#include ../../examples/coin_dash/settings.rs:from_bindings}}
```

```rust
{{#include ../../examples/coin_dash/settings.rs:rebind}}
```

```rust
{{#include ../../tests/coin_dash.rs:rebind_test}}
```

### Limitations

- **Rebinding**: one key and one gamepad button per control, read by the
  game from the first gamepad slot (the prompt listens on every slot, but the
  binding is made for slot 0); the left stick, the arrow keys, Space and P are
  fixed; there are no mouse bindings; a key or button another control uses is
  refused (swap in two steps) rather than swapped.
  **Reserved for the menus**: Escape and Tab on the keyboard, and East and the
  four d-pad directions on the gamepad. They can't be bound to a control
  (the prompt, the model and a hand-edited `settings.json` all refuse them),
  because the retained UI navigates with them and a screen that rebinds its own
  navigation keys could lock the player out. The one exception is Pause: it
  starts on Escape, and once moved elsewhere **Reset to defaults** is the only
  way to give it Escape back. Key names are physical positions on a US layout
  (`Key::Q` is where Q is on QWERTY, whatever is printed on the key). The
  gamepad prompt was exercised only with synthetic events: there is no
  controller on the build machines.
- **Scale**: four steps, 75 to 150%; the window isn't resized, and the largest
  step is what the title screen's text still fits. The engine scales the
  retained UI; the immediate-mode calls (`draw_text`, `ui_label`) aren't scaled
  by it, so the game scales its own.
- **Navigation** is tested with the `UiInput` the engine builds from keys and
  pad buttons, not with operating-system events. The game is built without
  the `accessibility` feature, so no screen reader reads it: its accessible
  names and roles are checked through the platform-neutral snapshot only.
- **Tests** of the retained UI drive `Ui::frame` by hand, because
  `run_headless` runs simulation systems only (no frame systems, no UI frame).
- **The window walk-through** can't be pixel-checked: it proves the program
  opens a window, loads its asset and draws frames.

## Gap list

What this exercise found that the engine or the project doesn't do yet, with
the issue that owns it (an entry says "no issue" where none does; they belong
in [#207](https://github.com/ponkoengine/ponko/issues/207), the API friction
index, or a new one):

| Gap | Where it is tracked |
|---|---|
| No query for "which key or button was pressed this frame": the rebinding prompt lists the keys it offers itself, because the engine can only answer "is this one pressed". | Input service of [#195](https://github.com/ponkoengine/ponko/issues/195) |
| `run_headless` runs no frame systems and no UI frame, so screens built on the retained UI are tested by hand-feeding `Ui::frame`. | Phases: [#198](https://github.com/ponkoengine/ponko/issues/198); scripted input: [#179](https://github.com/ponkoengine/ponko/issues/179) |
| No native screen-reader adapter for the retained UI. | [#183](https://github.com/ponkoengine/ponko/issues/183) |
| Immediate-mode text and UI don't follow the retained UI's scale; the theme and builders are the retained UI's only. | [#203](https://github.com/ponkoengine/ponko/issues/203) |
| Saves and settings are plain files in the data folder; what a scene replacement does to them and to runtime services is undefined. | [#178](https://github.com/ponkoengine/ponko/issues/178); save-file compatibility fixtures [#174](https://github.com/ponkoengine/ponko/issues/174) |
| The web build keeps nothing (no storage backend), and the browser matrix is Chromium on SwiftShader WebGL2 only (no real-GPU WebGPU, Firefox, Safari or mobile). | [#173](https://github.com/ponkoengine/ponko/issues/173) |
| The window smoke proves "a window opens and frames are drawn", not what they look like; no image regression for a whole game. | [#135](https://github.com/ponkoengine/ponko/issues/135) |
| The C and C++ bindings have no counterpart of this game (it is Rust only). | [#262](https://github.com/ponkoengine/ponko/issues/262) |
| Packages aren't attached to a release, signed or notarized, and there is no installer; the third-party notice is generated by a short script (on macOS, 33 of the 188 linked libraries ship no license file in their published source, so only their declared license and authors are listed; the package says which) rather than by a license-policy tool. | [#282](https://github.com/ponkoengine/ponko/issues/282) |
