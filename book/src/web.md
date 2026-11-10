# Web builds: games in the browser

The same game code runs in a browser tab. Try the demos, built from the
examples on every change to `main`:

- [The battle](play/battle/): two armies of 10,000 units on flow fields.
  Click to rally, `A` to attack, `D` to defend. Add
  [`?inspector=open`](play/battle/?inspector=open) to the address to open
  the [inspector](inspector.md).
- [Flow fields](play/flow/): 20,000 agents find their way to where you click.
- [Ant colonies](play/ants/): ants forage and lay pheromone trails.

Games draw with WebGPU where the browser has it, and WebGL2 where it
doesn't, so they run in every current desktop browser.

## Building your game

Once:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130   # the wasm-bindgen version in your Cargo.lock
```

Then, from your game's folder, with
[`web/build.sh`](https://github.com/bozoslav/ponko/blob/main/web/build.sh)
and [`web/index.html`](https://github.com/bozoslav/ponko/blob/main/web/index.html)
copied from the engine's repository:

```sh
web/build.sh --bin my_game               # -> target/web/my_game/
python3 -m http.server -d target/web/my_game 8000
```

and open <http://localhost:8000>. In the engine's repository,
`web/build.sh battle` builds an example.

The output folder is a static site: put it on any web host (GitHub Pages,
itch.io, your own server). It holds the page, the game (`game_bg.wasm`, about
3 MB compressed) and a copy of `assets/`.

## What's different in a browser

**Assets are fetched before setup.** There's no file system, so the build
script lists your `assets/` folder in `assets/index.json`, and the engine
downloads every file in it before running your setup. `load_texture` and
the other loading functions then work as usual.

**One thread.** Browsers can run WebAssembly on several threads only on
specially configured servers, so web builds use one. Parallel queries
(`par_for_each`) still work, just on a single core: the battle's 20,000
units take about 4.5 ms per tick in Chrome, against 0.5 ms native on all
cores of the same machine. Plan for tens of thousands of units on the web,
rather than millions.

**Measure time with the engine's `Instant`.** `std::time::Instant::now()`
panics in browsers. `ponko::Instant` is the same type on desktop and
works on the web:

```rust
# use ponko::*;
let start = Instant::now();
// ... work ...
let ms = start.elapsed().as_secs_f64() * 1000.0;
# let _ = ms;
```

**Settings come from the address, not the environment.** There are no
environment variables in a browser. The inspector reads `?inspector=1`
(on), `?inspector=open` (on and open) or `?inspector=0` instead of
`PONKO_INSPECTOR`.

**The page is yours.** The engine draws into `<canvas id="crabby">` if the
page has one, and adds a canvas otherwise. `web/index.html` makes the canvas
fill the page; change it to put the game anywhere. Browsers only allow
sound once the player has clicked or pressed a key, so sounds played before
that (music at startup, say) may stay silent.

**Not available on the web:** the [control endpoint](control.md) and
[MCP](agents.md) tools, [hot reload](data-files.md) of files, [hot
patching](hot-patching.md) of code, and anything that writes files
(`ctx.save_screenshot`, saving the world to a path). `println!` output goes
nowhere; panics appear in the browser's developer console.

## Testing in a browser

`web/smoke/` in the engine's repository runs a web build in a headless
browser (Chromium, Firefox or WebKit) and checks that it starts, fetches its
assets, keeps simulating, reacts to a click and draws something. It also reads
the engine's own report of the GPU it opened (`ctx.gfx.gpu_capabilities()`) and
checks that it agrees with the canvas, so a page that fell back to WebGL2
can't pass as WebGPU. A 2D fixture, a 3D fixture (built with `--features 3d`:
a glTF model fetched and parsed in the browser, with a sun and shadows) and
the complete-game reference `coin_dash` ([A complete game,
packaged](packaging.md#the-web-build)) are built and checked there. CI runs
WebGL2 in all three browsers in the merge queue; WebGPU is tried too, and a
browser that can't show a WebGPU canvas is reported as unsupported, with the
reason, never as a pass. Each run leaves a report, a trace and a screenshot.
See its
[README](https://github.com/ponkoengine/ponko/blob/main/web/smoke/README.md)
for running it, adding a fixture for another example, and what runs where.

## C and C++

Not yet: web builds work for Rust games. C and C++ games would need to be
compiled with Emscripten, which the engine doesn't support so far.
