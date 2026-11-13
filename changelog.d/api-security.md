### Breaking changes
- `PONKO_CONTROL=1` no longer opens the control endpoint in a release
  build, unless that build has the engine's new `control-env` feature
  (`ponko/control-env`, or `control-env` on `ponko_ffi`). Debug builds and
  `Game::control_endpoint(true)` (`ponko_game_control_endpoint`) work as
  before. Migration: `PONKO_CONTROL=1 cargo run --release` →
  `PONKO_CONTROL=1 cargo run --release --features control-env` (a game
  that renames the dependency: `--features <its name>/control-env`), or
  have the game call `control_endpoint(true)`. `ponko-mcp`'s `launch` adds
  the feature to the release builds it makes, under the game's own name
  for its direct, normal `ponko` dependency (from `cargo metadata`, given
  at most 30 seconds); a release game without it now fails fast with an
  explanation instead of timing out.
- Control endpoint file paths: `world.save` and `world.load` take a
  relative `.json` path below the game's working folder (was: any path);
  `scene.save`, `scene.load`, `prefab.spawn` and `prefab.save` a relative
  `.json` path below `assets/`; `screenshot` and `perf.trace` a relative
  path to a new `.png`/`.json` file below the working folder. Absolute
  paths, `..`, names starting with `.`, Windows device names (`NUL`,
  `COM1`, `CONIN$`, ...) and paths that a link leads out of the folder are
  refused (a linked `assets/` folder itself is trusted).
  Migration: `"path": "/tmp/before.json"` → `"path": "saves/before.json"`
  (resolved against the game's `cwd`), or leave out `path` for a temp
  file where it's optional. The same applies to the MCP tools
  `save_world`, `load_world`, `save_scene`, `load_scene`, `spawn_prefab`,
  `save_prefab`, `screenshot` and `record_trace`.
- `world.save` (MCP `save_world`) refuses to replace an existing file:
  pass the new `overwrite: true` to replace it: only a plain file (not a
  link, a pipe or a device; on unix also not one with hard links), and not
  one a screenshot or trace will still write (recognized on unix by its
  device and inode, on Windows by its full path only). `scene.save` and
  `prefab.save` (MCP `save_scene`, `save_prefab`) refuse such a claimed
  file too. A read-only `ponko-mcp` server refuses `overwrite`.
- `ponko-mcp` no longer reads `CRABBY_INSTANCES_DIR` or the old
  `~/.crabby/instances` (`%LOCALAPPDATA%\crabby\instances`) registry, and
  `launch` no longer sets `CRABBY_*` variables or mirrors `PONKO_*`
  overrides to `CRABBY_*`: `CRABBY_INSTANCES_DIR` → `PONKO_INSTANCES_DIR`;
  games register in `~/.ponko/instances`.

### Added
- `ponko_mcp::control_env_feature(cwd, package)`: the `--features` entry
  that lets a release build of that package honor `PONKO_CONTROL`, as
  `launch` uses it.

### Security
- The control endpoint's token is 128 bits from the operating system's
  secure random source (was: hashes of the clock with `RandomState` keys),
  and is compared in constant time.
- The control endpoint reads at most 4 MiB per request line, before the
  token is checked, and serves at most 32 connections at once. A new
  connection whose first request, within 10 seconds, lacks the right token
  gets one error reply and is closed, and replies to it time out, so
  clients that stay silent or never read can't hold the slots. Past any of
  these limits the client gets an error and is disconnected.
- A shipped release build no longer opens a port when a player sets
  `PONKO_CONTROL=1` (see Breaking changes), and tools can no longer read
  or write files outside the game's folder through the endpoint, also not
  through links. Screenshots and traces are written through a file made
  new when the request arrives (temp files get a random name), which is
  removed if it isn't written, also when the game shuts down first;
  `perf.trace` is refused by a headless game, which has no frames to
  trace, before any file or folder is made.
- `ponko-mcp` marks `launch`, `load_world`, `load_scene`, `save_scene` and
  `save_prefab` as destructive (besides `despawn_entity`,
  `remove_component` and `quit_instance`), so MCP clients can ask before
  calling them.
- `ponko_unity`: crafted or corrupt files are errors instead of panics
  (capacity or arithmetic overflow, stack overflow), endless loops,
  quadratic work or memory far beyond their size:
  - Binary assets decode to at most 64 bytes of memory per byte of the
    file (plus 1 MiB), counting every value, copied name, text, data and
    number. Refused besides: negative or oversized counts, sizes and
    offsets; object data outside the data section or overlapping another
    object's; type tree names longer than 1 KiB or without their NUL; an
    array whose element type isn't inside it; values nested more than 128
    deep.
  - Text (YAML) files nest maps and lists at most 128 deep.
  - Animation clips: at most 2^24 samples (distinct sample times, frames
    and keys between them, times curves); packed vectors with no bits per
    item and dense clips with no frames at most 2^20 items or curves; a
    streamed clip's curve count at most its data's word count; key lookups
    no longer grow quadratically with the number of curves.
  - Meshes: vertex counts, streams, offsets and submesh ranges are
    checked (submeshes read at most four times the index buffer); blend
    shape channels must not share frames, and shapes expand to the
    vertices the data holds, at most 2^25 entries (at least one per
    frame, plus the offsets read).
  - Textures and terrains: sizes that overflow are errors.
  - Prefabs: nest at most 64 deep, a flattened scene's objects take at most
    about 1 GiB (prefabs placed exponentially often stop there), and
    property path modifications are at most 64 parts, with array sizes,
    indexes and map searches bounded by the number of modifications.
