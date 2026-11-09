# Browser smoke test

Checks that a web build actually runs in a browser, which compiling to
wasm can't: `smoke.mjs` serves a site built by `web/build.sh`, opens it in
headless Chromium (no window) and checks, in order:

1. **Startup**: the game reaches its first frame (no panic, no page error).
2. **Assets**: `assets/index.json` and `assets/crab.png` were fetched (HTTP
   200) and the texture decoded at its real size; an image and streamed
   music from the game's in-memory asset source loaded by name without
   being fetched.
3. **Progress**: the fixed-step simulation keeps ticking (at least 60 ticks
   in 3 seconds, a third of the 60 Hz rate; a coarse bound, not a frame-rate
   assertion, kept above the `stall` self-test's 20 ticks).
4. **Input**: a click on the canvas reaches the game.
5. **Rendering**: the canvas is not blank (screenshot, or a copy of the
   canvas for WebGPU).

The fixture is `examples/web_smoke.rs`. It writes its state as JSON to
`<html data-smoke="...">` each frame, which is how the runner reads ticks,
frames, clicks and the asset's size.

## Running it

```sh
web/build.sh web_smoke                       # -> target/web/web_smoke/
cd web/smoke && npm ci
# Only Chromium's headless shell (about 200 MB); here inside this folder:
PLAYWRIGHT_BROWSERS_PATH=$PWD/.browsers node node_modules/playwright-core/cli.js install --only-shell chromium
cd ../..
PLAYWRIGHT_BROWSERS_PATH=$PWD/web/smoke/.browsers \
  node web/smoke/smoke.mjs --dir target/web/web_smoke --backend webgl2
```

Without `PLAYWRIGHT_BROWSERS_PATH`, Playwright uses its usual cache folder.
Remove `web/smoke/.browsers` and `web/smoke/node_modules` when done; both are
git-ignored.

Options:

| Option | Meaning |
|---|---|
| `--backend webgl2` | Hide WebGPU from the page, so the engine takes its WebGL2 fallback. |
| `--backend webgpu` | Require WebGPU; "unsupported" if the browser has no adapter or can't show a WebGPU canvas. |
| `--backend auto` | Let the engine choose; the report says which it used. |
| `--case missing-asset` / `panic` / `stall` | Self-tests: answer 404 for the texture, panic during setup (`?smoke_case=panic`), or pause the simulation (`?smoke_case=stall`). The run passes only if that failure is detected. |
| `--timeout <s>` | Seconds to wait for the first frame (default 90). |
| `--allow-unsupported` | Exit 0 instead of 2 when the browser lacks the backend. |
| `--out <dir>` | Where artifacts go (default `target/web-smoke/<backend>-<case>`). |
| `--browser-arg <flag>` | Extra Chromium flag (repeatable). |

## Results

Exit codes: **0** passed, **1** the game failed, **2** unsupported (the
browser couldn't start, or lacks the backend asked for), **3** bad arguments
or no build. "Unsupported" is decided without the engine: WebGPU counts as
available only if a plain WebGPU canvas, cleared to red by the runner itself,
reads back red.

Each run writes to its `--out` folder:

- `report.json`: browser version, platform, Chromium flags, WebGPU adapter
  and WebGL2 renderer, the backend the game's canvas used, startup time,
  total elapsed time, the state before and after the progress check, canvas
  statistics, every request with its status, console messages, page errors
  and the outcome (`kind`: `pass`, `asset`, `panic`, `page-error`, `stall`,
  `startup-timeout`, `input`, `blank`, `backend`, `unsupported`, ...).
- `console.log`: the page's console.
- `canvas.png`: the canvas after the click; `page.png`: the page on failure.

## What runs where

| Where | Browser | WebGL2 (forced) | WebGPU |
|---|---|---|---|
| CI (`web-smoke` job, ubuntu-latest, no GPU) | Chromium headless shell from Playwright | Runs on SwiftShader; must pass | Tried; passes as "unsupported" when the headless browser can't present a WebGPU canvas |
| Apple Silicon Mac, headless shell | same | Passes (ANGLE on SwiftShader) | Unsupported: a SwiftShader fallback adapter exists, but a WebGPU canvas can't be shown |

Not covered yet: WebGPU on real GPU hardware, Firefox and Safari, mobile
browsers, 3D (`--features 3d`) web builds, and the hosted demo examples.
