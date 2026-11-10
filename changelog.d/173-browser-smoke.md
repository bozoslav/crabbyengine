### Added
- Browser smoke tests cover more (#173). `web/smoke/smoke.mjs` runs in headless
  Chromium, Firefox and WebKit (`--browser`), can use Chromium's full browser in
  its new headless mode (`--chromium-mode new`), and a WebGPU run now presents
  and checks the canvas where the browser can: a canvas copy is taken in the
  presenting task, and the Chromium run was verified drawing through WebGPU on
  a real adapter. A browser that can't run a backend is reported as
  "unsupported" with the reason (in `report.json`, the console and the CI job
  summary), and one that is installed but won't start fails.
- A 3D web fixture, `web_smoke_3d` (a glTF model fetched and parsed in the
  browser, lit and shadowed, with the models and triangles drawn checked), built
  with `--features 3d`. `web/build.sh` takes `--features`.
- A fixture per example: `web/smoke/fixtures.json` gives each example its Cargo
  features, asset, expected state and key presses (`--fixture <name> [--build]`),
  and `examples/common/web_smoke.rs` is the shared helper a fixture uses to
  publish its state and the runner's self-test failure cases.
- Richer artifacts: the engine's own adapter report (`Graphics::gpu_capabilities`,
  published on `<html data-smoke-gpu>`) is checked against the canvas's context
  and recorded with the limits and fallbacks; a Playwright `trace.zip` for every
  run; an optional Chromium GPU trace (`--chrome-trace`); a Markdown summary of
  every run (`web/smoke/summary.mjs`).
- CI: the `web-smoke` job now runs WebGL2 in Chromium, Firefox and WebKit for the
  2D and 3D fixtures, tries WebGPU in two Chromium modes, and uploads the
  artifacts; the runner's own tests (`node --test web/smoke`) run on every pull
  request in `web-smoke-runner`; the `web` job lints the 3D examples for wasm.

### Fixed
- 3D games start in a browser that has only WebGL2: the 3D frame drew into
  multisampled textures, which WebGL2 can't make (it panicked at startup), and
  read sprite textures through a linear view, which it can't make either. On
  WebGL2 the frame now has one sample (the capabilities report's
  `multisampling` entry says so: its `fallback` text, which is part of the
  serialised JSON, is now "no multisampling: edges are smoothed only by FXAA or
  SMAA, if chosen" instead of "Nx multisampling"; use FXAA or SMAA for smooth
  edges), and normal and ORM maps are remade as linear textures with the same
  bytes when a material first uses them, so they are not decoded as sRGB. A
  game's own shader must read the scene depth at index 0 or through
  `scene_position`. Found by the new 3D smoke fixture.
- `GpuAdapter::backend` is documented as `webgpu`, not `browser_webgpu`, for the
  browser's WebGPU: that is what the engine reports.
