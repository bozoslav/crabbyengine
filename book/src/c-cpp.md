# C and C++

Games can be written in C++ (or plain C) as well as Rust. They use the same
engine: the same ECS, game loop, renderer, spatial grid and control endpoint
for agents. Every chapter of this guide shows the C++ version of its examples
next to the Rust one.

- `include/ponko.hpp`: the C++17 wrapper, header-only. `ponko::game`,
  `ponko::ctx` and `ponko::world_view` mirror Rust's `Game`, `Ctx` and
  `World`.
- `include/ponko.h`: the C API underneath, generated from the `ponko_ffi`
  crate with cbindgen.
- One shared library, `ponko_ffi` (`libponko_ffi.dylib`/`.so`,
  `ponko_ffi.dll`).

Loops over entities run in your C++ code, over **whole component arrays**,
so a million entities never mean a million calls across the language
boundary, and `each_par` runs them on all cores.

## Build and run the examples

From the repository root, with Rust, CMake, a C11 compiler and a C++17 compiler:

```sh
cargo build -p ponko_ffi --release
cmake -S examples/cpp -B target/native -DCMAKE_BUILD_TYPE=Release -DPONKO_LIBRARY_DIR="$PWD/target/release"
cmake --build target/native --config Release
ctest --test-dir target/native -C Release --output-on-failure
./target/native/ponko_cpp_boids                                  # window
HEADLESS=1 BOIDS=1000000 TICKS=300 ./target/native/ponko_cpp_boids
./target/native/ponko_cpp_battle                                 # the battle demo
HEADLESS=1 UNITS=1000000 TICKS=600 ./target/native/ponko_cpp_battle
```

In PowerShell, replace `$PWD` with the repository's absolute path, using
forward slashes; Visual Studio builds use `--config Release` and put programs
in `target/native/Release/`.

For your own game, copy the recipe in `examples/cpp/CMakeLists.txt`: include
the two headers, link `ponko_ffi`, ship its shared library next to your
executable, and put your textures in an `assets/` folder next to it (the
example copies the repository's `assets/`). `ponko_abi_version()` returns `2`;
rebuild against the matching header when you update the engine.

## A C++ game

```cpp
{{#include ../../examples/cpp/guide.cpp:first_game_full}}
```

- Systems are anything callable as `void(ponko::ctx&)`. Lambdas can capture
  your game state by reference: the game runs inside `run()`, so it outlives
  nothing it captures.
- `run(setup)` opens the window; `run_headless(ticks, setup)` runs the
  simulation alone and returns the world (see [Headless runs](headless.md)).
- Errors become exceptions. An exception thrown by your system stops the game
  and comes out of `run()`; it never crosses into the engine.

## Components

C++ components are plain structs: standard-layout, trivially copyable and
trivially destructible. No `std::string`, `std::vector` or owning pointers in
components; store ids or indices and keep other objects outside the ECS.

```cpp
{{#include ../../examples/cpp/main.cpp:cpp_components}}
```

There's nothing to register: calls find a component by its C++ type, and
the engine learns about the type the first time it sees one. It's named
after the type, namespaces included (`game::Health`), which is how tools and
saves show it; `world.reflect<T>("Name", fields)` picks another name.

The built-ins are C++ types too: `ponko::transform2d` is `Transform2D`
(the same bytes as Rust), `ponko::attached` is `Attached`,
`ponko::animation` is `Animation`, `ponko::velocity` is `Velocity`, and
`ponko::sprite` is `Sprite` (spawn, get and set, but not a query parameter,
since the engine stores sprites in a Rust layout; the compiler tells you).
[Physics](physics.md) bodies and colliders, `ponko::rigid_body` and
`ponko::collider`, work like sprites.

```cpp
{{#include ../../examples/cpp/main.cpp:cpp_world}}
```

## Queries and threads

`world.each(fn)` calls your function for every entity with the components
it takes (see [Queries](ecs.md#queries)); `world.query<A, B>(fn)` hands you
whole arrays instead. While a loop runs, the world and context are busy:
other world or context calls throw (C: return `PONKO_BUSY`), except

- commands: `despawn_later`, `spawn_later`, `set_later`, `remove_later`,
  applied when the system returns (in `each` and `query`, not in the `_par`
  versions);
- events: `send` and `read` (also not in the `_par` versions; see
  [Events](events.md));
- spatial queries, and the clock values `dt()`, `sim_tick()`, `random()`.

`each_par` and `query_par` split archetypes into slices and run your function
on all cores at once. It must only touch its own entity and read shared
data; that is the same rule as Rust's `par_for_each`.

To visit only the entities whose component changed, name a filter:
`world.each<ponko::changed<Health>>(fn)`. A parameter taken as `T&` counts
as a change, `const T&` doesn't; see
[Change detection](ecs.md#change-detection).

`each` is as fast as a hand-written loop over the arrays: 1M entities,
`position += velocity`, take 0.15 ms either way
(`examples/cpp/bench.cpp`).

Everything else stays on the thread that runs the game.

## Speed

The battle demo is a whole game, written twice: `examples/battle.rs` and
`examples/cpp/battle.cpp`. Both have flow fields, per-team grids, combat on
all cores, reinforcements and rounds. Same seed, same map, same armies
(the C++ `rng` gives the same numbers as Rust's), release builds,
`HEADLESS=1 UNITS=… TICKS=1200`:

| Units | Rust | C++ |
|---:|---:|---:|
| 20k | 0.57 ms/tick | 0.57 ms/tick |
| 200k | 4.04 ms/tick | 4.24 ms/tick |
| 1M | 18.85 ms/tick | 19.97 ms/tick |

The hot loops use views (`grid.view()`, `flow.view()`) that the header reads
inline, so nothing crosses into the engine per unit. The gap for big armies
is in rebuilding the per-team grids: Rust feeds them straight from the
units, C++ copies each team's units into an array first.

`HEADLESS=1 BOIDS=… ./ponko_cpp_boids` and `HEADLESS=1 BOIDS=… cargo run
--release --example boids` run the same flocking, release builds, on the
development Mac (arm64, 2026-11-03):

| Boids | Rust | C++ |
|---:|---:|---:|
| 100k | 3.5 ms/tick | 3.9 ms/tick |
| 1M | 31.3 ms/tick | 34.5 ms/tick |

The remaining gap is mostly the C++ version copying velocities into its own
array each tick before rebuilding its grid. Plain column updates (one million
`p += v`, `cargo run --release --example ffi_bench` vs `ponko_cpp_bench`,
which times `each` and `query`) run at the same speed in both: about 0.15 ms.

Grid queries in hot loops should use views (`ctx.spatial_view()`,
`grid.view()`): the header walks the grid inline, as Rust does. Through the
query functions, C++ boids took 41.5 ms/tick at 1M.

## Plain C

C uses the same functions without the wrapper: status codes instead of
exceptions, a `user` pointer instead of captures.

```c
{{#include ../../examples/cpp/main.c:c_world}}
```

```c
{{#include ../../examples/cpp/main.c:c_columns}}
```

A headless game in C:

```c
{{#include ../../examples/cpp/main.c:c_game}}
```

```c
{{#include ../../examples/cpp/main.c:c_run}}
```

Every call returns `PonkoStatus`; on failure `ponko_last_error()` has the
message (per thread, until the next failed call). Null arguments, wrong
sizes, unknown ids, duplicate query columns and busy worlds are rejected.
Pointers you pass must still be valid: the library can't recognize a
dangling pointer or a buffer smaller than you say. Callbacks must return
normally; never let a C++ exception or `longjmp` cross them.

## Still Rust-only

C and C++ resources and components are plain structs (no `std::string`
or `std::vector` inside), copied in and out.

Most chapters show Rust and C++ side by side. These features exist only in
Rust today; the chapters say so where they use them, and
[#262](https://github.com/ponkoengine/ponko/issues/262) tracks the bindings:

| Rust feature | Where | From C and C++ |
|---|---|---|
| `ActionId` and the `actions!` macro | [Named input actions](actions.md) | Query actions by name |
| `InputScenario`, `Game::input_scenario`, `run_headless_scenario`, scenario checks and reports | [Input scenarios](actions.md#input-scenarios) | Feed events yourself with `feed_input`; tools can still load the JSON scenario file through the control endpoint |
| `AssetSource`, `MemorySource`, `Assets::add_source`, `Audio::try_load_stream` | [Asset sources](asset-sources.md) | Files in `assets/` only |
| `Assets::load_with`, `Assets::scene_asset_field` | [Asset sources](asset-sources.md) | Read the file and decode it yourself; load what a scene names before `load_scene` |
| `UiTheme`, `ui.set_theme` | [Retained game UI](ui.md) | Set each node's paint |
| Named grid spans in UI layouts | [Retained game UI](ui.md) | Named lines and areas, no named spans |
| `World::resource_scope` | [Entities, components, queries](ecs.md) | The existing resource calls |
| `ponko::Error` and the `try_` loaders | [Getting started](getting-started.md) | `ponko_last_error()` |

System phases (`Phase`, `Game::add`) are not on this list: see
[Systems and the frame](systems.md#phases) for the C and C++ calls.
