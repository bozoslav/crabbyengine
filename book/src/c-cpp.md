# C and C++

Games can be written in C++ (or plain C) as well as Rust. They use the same
engine: the same ECS, game loop, renderer, spatial grid and control endpoint
for agents. Every chapter of this guide shows the C++ version of its examples
next to the Rust one.

- `include/crabby.hpp`: the C++17 wrapper, header-only. `crabby::game`,
  `crabby::ctx` and `crabby::world_view` mirror Rust's `Game`, `Ctx` and
  `World`.
- `include/crabby.h`: the C API underneath, generated from the `crabby_ffi`
  crate with cbindgen.
- One shared library, `crabby_ffi` (`libcrabby_ffi.dylib`/`.so`,
  `crabby_ffi.dll`).

Systems get **whole component columns**, so a million entities never mean a
million calls across the language boundary, and `query_par` runs C++ code on
all cores.

## Build and run the examples

From the repository root, with Rust, CMake, a C11 compiler and a C++17 compiler:

```sh
cargo build -p crabby_ffi --release
cmake -S examples/cpp -B target/native -DCMAKE_BUILD_TYPE=Release -DCRABBY_LIBRARY_DIR="$PWD/target/release"
cmake --build target/native --config Release
ctest --test-dir target/native -C Release --output-on-failure
./target/native/crabby_cpp_boids                                  # window
HEADLESS=1 BOIDS=1000000 TICKS=300 ./target/native/crabby_cpp_boids
./target/native/crabby_cpp_battle                                 # the battle demo
HEADLESS=1 UNITS=1000000 TICKS=600 ./target/native/crabby_cpp_battle
```

In PowerShell, replace `$PWD` with the repository's absolute path, using
forward slashes; Visual Studio builds use `--config Release` and put programs
in `target/native/Release/`.

For your own game, copy the recipe in `examples/cpp/CMakeLists.txt`: include
the two headers, link `crabby_ffi`, ship its shared library next to your
executable, and put your textures in an `assets/` folder next to it (the
example copies the repository's `assets/`). `crabby_abi_version()` returns `2`;
rebuild against the matching header when you update the engine.

## A C++ game

```cpp
{{#include ../../examples/cpp/guide.cpp:first_game_full}}
```

- Systems are anything callable as `void(crabby::ctx&)`. Lambdas can capture
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

Register each type by name once; the token you get is how calls name it.
Tokens belong to one game or world, and reusing a name for a different C++
type is an error, even with the same layout.

The built-ins: `world.transforms()` is `Transform2D` (`crabby::transform2d`,
the same bytes as Rust, usable as a query column), and `world.sprites()` is
`Sprite` (`crabby::sprite`: spawn, get and set, but not a query column, since
the engine stores sprites in a Rust layout). Rust components without a C
layout are refused by every call.

```cpp
{{#include ../../examples/cpp/main.cpp:cpp_world}}
```

## Queries and threads

A query calls your function once per archetype (*chunk*) with columns in the
order you listed the tokens. While it runs, the world and context are busy:
other world or context calls throw (C: return `CRABBY_BUSY`), except

- commands: `despawn_later`, `spawn_later`, `set_later`, `remove_later`,
  applied when the system returns (in `query`, not in `query_par`);
- events: `send` and `read` (in `query`, not in `query_par`; see
  [Events](events.md));
- spatial queries, and the clock values `dt()`, `sim_tick()`, `random()`.

`query_par` splits archetypes into slices and runs your function on all
cores at once. It must only touch its own chunk and read shared data; that
is the same rule as Rust's `par_for_each`.

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
| 200k | 4.40 ms/tick | 4.41 ms/tick |
| 1M | 20.99 ms/tick | 21.04 ms/tick |

The hot loops use views (`grid.view()`, `flow.view()`) that the header reads
inline, so nothing crosses into the engine per unit.

`HEADLESS=1 BOIDS=… ./crabby_cpp_boids` and `HEADLESS=1 BOIDS=… cargo run
--release --example boids` run the same flocking, release builds, on the
development Mac (arm64, 2026-11-03):

| Boids | Rust | C++ |
|---:|---:|---:|
| 100k | 3.5 ms/tick | 3.9 ms/tick |
| 1M | 31.3 ms/tick | 34.5 ms/tick |

The remaining gap is mostly the C++ version copying velocities into its own
array each tick before rebuilding its grid. Plain column updates (one million
`p += v`, `cargo run --release --example ffi_bench` vs `crabby_cpp_bench`) run
at the same speed in both: about 0.15 ms.

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

Every call returns `CrabbyStatus`; on failure `crabby_last_error()` has the
message (per thread, until the next failed call). Null arguments, wrong
sizes, unknown ids, duplicate query columns and busy worlds are rejected.
Pointers you pass must still be valid: the library can't recognize a
dangling pointer or a buffer smaller than you say. Callbacks must return
normally; never let a C++ exception or `longjmp` cross them.

## Still Rust-only

Resources: keep game-wide data in your own objects, captured by your
systems (`[&]`), as the examples do. To let tools and agents see that
state, keep it in a reflected component on one entity. Everything else in this guide works
from C++, and each chapter shows both.
