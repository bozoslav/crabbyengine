# Stability

The compatibility contract inherited from Crabby Engine 1.0.

## Ponko rename and compatibility continuity

Ponko's package metadata starts a new version line at `0.0.1`. That naming
and numbering change does **not** revoke the compatibility commitments
below for inherited APIs, Cargo features, C/C++ layouts and symbols, saved
worlds, or MCP tools. The existing 1.x policy remains the reference for
compatible evolution. Any proposal that conflicts with it needs an explicit
migration and major-version decision; a 0.x number is not permission for an
undocumented break. Source directories now use `ponko_*`. The `ponko.h` / `ponko.hpp` headers
provide source aliases over the existing C symbols and C++ namespace. Binary
symbols and saved-format identifiers remain unchanged. See the
[naming migration](book/src/naming.md) for environment precedence and retained
external identifiers.

## Semantic versioning

From 1.0.0, the public API of these crates follows [semver](https://semver.org):

| Crate | What is covered |
|---|---|
| `ponko` | Every public item that is not `#[doc(hidden)]`, and the cargo features |
| `ponko_ecs` | Every public item that is not `#[doc(hidden)]` |
| `ponko_ffi` | The C ABI in `include/crabby.h` and the C++ wrapper `include/crabby.hpp` (see [C ABI](#c-abi)) |
| `ponko_mcp` | The MCP tool names and their parameters |

- **Patch** releases (1.0.x) fix bugs. They change no public signatures.
- **Minor** releases (1.x.0) add things: new functions, types, cargo
  features, enum variants on `#[non_exhaustive]` enums, MCP tools, optional
  parameters. Existing code keeps compiling.
- **Major** releases (2.0.0) are the only ones that remove or change things.

Enums that will grow are `#[non_exhaustive]` (`NetState`, `BodyKind`,
`Shape`, `FieldType`, `ReflectError`, `ErrorKind`, `Key`, `MouseButton`, `GamepadButton`,
`GamepadAxis`): matches on them need a `_` arm, so new variants can arrive in a minor release.

Not covered, and allowed to change in any release:

- `#[doc(hidden)]` items: test helpers such as `Net::loopback_lossy` and
  macro internals.
- `ponko_bench`, the published benchmark executable; its output is not a stable engine API.
- Error message text, log output, the inspector's look, and the exact
  frames rendered: tests should check behavior, not strings or pixels.
- Performance. Regressions are bugs, and benchmark numbers go in PRs, but
  timings are not part of the API.

## Errors and panics

Fallible calls follow one convention, introduced with `ponko::Error`:

- `load_x` / `create_x` are for setup code, where a missing file is a bug.
  They panic with the `Error`'s message (which starts with `ponko:`).
- `try_load_x` / `try_create_x` return `Result<T, ponko::Error>`. Match on
  `Error::kind()` (a `#[non_exhaustive]` `ErrorKind`), never on the message.
- Operations that fail in normal play (scene loads, prefab spawns, network
  joins) return a `Result` under their plain name and have no panicking twin.

Texture, mesh, skeleton, sound and font loading and material and mesh
creation follow it. Older calls that return `Result<_, String>` keep their
signatures on the `Ctx` services (`ctx.gfx`, `ctx.assets`); they move to
`ponko::Error` service by service, and `String: From<ponko::Error>` keeps
them interoperable meanwhile.

## Deprecation

Nothing is removed without warning. Before an item is removed in the next
major release, at least one minor release ships it marked `#[deprecated]`
(Rust) or with a note in the header (C), saying what to use instead. The
CHANGELOG lists every deprecation.

## Minimum Rust version (MSRV)

Ponko builds with **Rust 1.95** and newer (`rust-version` in Cargo.toml;
CI checks it). Raising it is allowed in a **minor** release, never in a
patch release, and the CHANGELOG says so. It follows the dependencies
(wgpu, egui, rapier) and stays at least a few releases behind stable.

## C ABI

The C ABI has its own version, `crabby_abi_version()` (currently 2), which
only changes when existing functions or structs change incompatibly. That
happens only in a major release of the engine. Minor releases add new
functions and enum values; existing struct layouts never change within an
ABI version. Programs check the version at startup if they load the
library dynamically:

```c
if (crabby_abi_version() != 2) { /* built against another version */ }
```

## Saved worlds

`World::save` writes JSON tagged `"format": "crabby-world"` and a save
version (currently 1). Every 1.x release loads saves from every earlier 1.x
release. Your own components are yours to migrate: give a component
`#[serde(default)]` and saves written before you added a field still load.

`World::load` refuses entity ids whose index is far beyond what the save
needs (the exact bound is in its docs), so a corrupt or hostile file can't
make the world allocate a huge entity table. Saves the engine writes stay
within it unless the world once had over 4 million entities alive and was
saved with under an eighth of them; such a save still loads back into the
world that wrote it.

## Compatibility checks

The promises above are tested against retained baselines, not only against
what the current revision generates. A baseline is a small fixture recorded at
a named engine revision with its provenance next to it; it is never edited to
make a test pass. The baselines come from the engine's `main` branch, not from
a published release, so they show that today's code honors what that revision
promised and do not claim anything about earlier binary releases.

| Contract | Baseline | Check |
|---|---|---|
| Saved worlds, scenes and prefabs | `tests/fixtures/saves/` | `tests/save_compat.rs`, `tests/prefab_compat.rs` |
| MCP tools and parameters | `crates/ponko_mcp/tests/fixtures/` | `crates/ponko_mcp/tests/contract.rs` |
| C ABI header and layouts | `crates/ponko_ffi/tests/fixtures/abi/` | `crates/ponko_ffi/tests/abi_compat.rs`, `examples/cpp/abi_baseline.c` |

Additions pass: new MCP tools and optional parameters, new C functions and enum
constants. A removed or changed tool, parameter, function or struct layout, a
new required MCP parameter, or a parameter that accepts fewer values fails with
a message naming it. Each fixture folder's README says how to retain a new
baseline after an intentional major-version change.

## Determinism

What gives the same world from the same seed and inputs:

| | Same machine, same build | Other OS or CPU, same build | Other engine version |
|---|---|---|---|
| ECS, scheduling, `ctx.random` | yes | yes, see below | not promised |
| Your sim code with `+ - * /` and `sqrt` on `f32`/`f64` | yes | yes | yes |
| `sin`, `cos`, `powf`, `exp`, ... | yes | **may differ** | — |
| Physics (`physics`) | yes | **no** | not promised |
| Physics (`physics-determinism`) | yes | yes | not promised |

- IEEE-754 makes basic arithmetic and `sqrt` bit-identical everywhere, but
  the standard library's `sin`, `cos`, `powf` and friends call the platform's
  math library, which may round differently. For lockstep between
  different platforms, keep them out of the simulation or use a pure-Rust
  implementation such as the `libm` crate.
- Lockstep peers, and replays, must use the same game build and engine
  version. Lockstep packets carry a protocol version, and a peer with
  another version is ignored and times out.
- Iteration order of queries is deterministic but not part of the API: it
  can change between versions.

## Platforms

Tested in CI on every change: Linux x86-64, macOS (Apple Silicon) and
Windows x86-64. Built in CI on every change: the web (wasm32, WebGPU with a
WebGL2 fallback); small web builds (2D, 3D and the complete-game reference)
are also run in headless Chromium and WebKit with WebGL2 in the merge
queue (`web/smoke/`; Firefox is tried there too, but the runner's headless
Firefox has no WebGL2, so it is reported, not verified). WebGPU is tried there too and reported, but a runner
without a GPU usually can't show it, so WebGPU is verified on real GPUs by hand,
not gated in CI. Other targets that wgpu and winit support usually work
but aren't tested.
