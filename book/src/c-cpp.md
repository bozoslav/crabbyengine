# C and C++

Crabby's C ABI and C++17 wrapper use the same archetype ECS as Rust. Systems
receive whole component columns, so a million entities do not require a million
calls across the language boundary. This API currently covers entities, plain-data
components, queries, and headless fixed-step simulations. Rendering, audio,
reflection, resources, and the control protocol are not exposed through this ABI.

The C interface is generated from `crabby_ffi` with cbindgen and ships as
`include/crabby.h`. C++ games include `include/crabby.hpp`, a header-only wrapper
with scoped world ownership and typed components. Both link the same Rust library.

## Build and run the examples

From the repository root, with Rust, CMake, a C11 compiler and a C++17 compiler:

```sh
cargo build -p crabby_ffi --release
cmake -S examples/cpp -B target/native -DCMAKE_BUILD_TYPE=Release -DCRABBY_LIBRARY_DIR="$PWD/target/release"
cmake --build target/native --config Release
ctest --test-dir target/native -C Release --output-on-failure
```

In PowerShell, replace `$PWD` in the configure command with the repository's
absolute path, using forward slashes. CMake selects the compiler for the host;
Visual Studio builds use `--config Release`. The example project handles the
platform's shared-library name and runtime lookup.

For your own game, use the CMake example as the linking recipe: include the two
headers, link `crabby_ffi`, and ship its shared library beside your executable
(or configure your platform's library search path). The crate also emits a static
library; static linking requires the Rust target's native system libraries and is
not the CMake example's default. `crabby_abi_version()` returns `1` for this API;
rebuild the game against the matching header when changing engine versions.

## C: register data and iterate columns

Component names and IDs belong to one world. Register the size and alignment of
each component before spawning values. Values are copied into the ECS: the
caller's local variables remain theirs.

```c
{{#include ../../examples/cpp/main.c:c_world}}
```

A query calls your function once for each matching archetype. Columns are in the
same order as the component IDs you requested, even when registration order
was different. Each column has `chunk->count` contiguous values.

```c
{{#include ../../examples/cpp/main.c:c_columns}}
```

Use `crabby_chunk_entity` if a callback needs an entity handle; the chunk's
`entities` field is opaque. Entity handles include a generation, so despawned
handles stay dead when a slot is reused. `crabby_get` copies a component into a
caller-provided buffer; it does not expose a long-lived storage pointer.

C functions return `CrabbyStatus`. On failure, read `crabby_last_error()` before
another failed API call on that thread replaces the message. Copy the message if you
need to keep it. Null arguments, inconsistent layouts, incorrect copy sizes,
unknown component IDs and duplicate query components are rejected. Raw pointers
must still refer to valid memory: the library cannot recognize a dangling handle
or a buffer smaller than the size its caller claims.

## C++: scoped ownership and typed components

Components must be standard-layout, trivially copyable and trivially destructible.
Do not put `std::string`, `std::vector`, owning pointers or references into ECS
components. Store plain numeric IDs and manage other objects outside the ECS.

```cpp
{{#include ../../examples/cpp/main.cpp:cpp_components}}
```

A `crabby::world` owns the native world and frees it on destruction. It can be
moved but cannot be copied. Component tokens carry their world identity; using a
token with a different world is an error. Reusing a component name for a
different C++ type is also rejected, even when both types have the same layout.

```cpp
{{#include ../../examples/cpp/main.cpp:cpp_world}}
```

The wrapper turns C errors into exceptions. If a query callback throws, its
trampoline catches the exception before returning through Rust, then rethrows it
on the C++ side. No C++ exception may cross a C callback boundary directly.

## Fixed-step headless games

The headless runner calls the existing engine loop. Setup runs once, including
when the requested tick count is zero. The step callback receives a zero-based
tick and `dt = 1 / hz` seconds. Returning `false` stops the run; the resulting
world is returned to the caller. The control endpoint is disabled for these runs,
so environment variables cannot leave a foreign caller waiting for an agent.

```cpp
{{#include ../../examples/cpp/main.cpp:cpp_headless}}
```

The callback receives a borrowed `world_view`, valid only for that callback.
Use it to spawn entities or query components, but do not retain the view or free
its native handle. The owning world returned by the runner lives until its C++
owner is destroyed. C programs can use the corresponding `crabby_run_headless`
function and free its output with `crabby_world_free`.

## Borrowing and threads

A chunk and its column pointers are valid only until the query callback returns.
While that callback is running, access to the same world through other API calls
returns `CRABBY_BUSY`. Read and write the supplied columns directly; collect
structural changes outside the query and apply them afterward. Query component
IDs must be distinct, and a query accepts at most 32 components. A query with no
components visits every nonempty archetype.

A world and its callbacks stay on the thread that created it. Do not share a world or borrowed columns
across concurrent calls, retain chunk pointers, or use a world after freeing it.
Rust panics are contained at the ABI boundary where unwinding is enabled;
allocation failures or builds configured to abort cannot be recovered this way.

## Release comparison

The matching benchmarks create one million `Pos`/`Vel` entities, warm up ten
iterations, then measure 200 updates of `p.x += v.x; p.y += v.y`. Both check a
final checksum of `630000000`; allocation and spawning are outside the timed
region. No test asserts a timing threshold.

```sh
cargo run --release --example ffi_bench
cmake --build target/native --config Release --target crabby_cpp_bench
# Run target/native/crabby_cpp_bench (Release/crabby_cpp_bench.exe on Visual Studio).
```

The Rust benchmark uses typed ECS queries; the C++ benchmark uses the C ABI's
whole-column query callback. Use release builds for both and repeat locally:
compiler, hardware, thermal state and background load affect the comparison.

On the development Mac (arm64, macOS 27.0.1, Rust 1.99.1 and Apple Clang 21),
three alternating runs on 2026-11-02 produced these medians:

| Run | Rust typed query | C++ through C ABI |
|---|---:|---:|
| 1 | 0.151 ms | 0.151 ms |
| 2 | 0.157 ms | 0.156 ms |
| 3 | 0.159 ms | 0.155 ms |

Both paths produced the expected checksum in every run. They perform similarly
on this column-update workload; this does not measure spawning, complex game
logic, rendering, or other platforms.
