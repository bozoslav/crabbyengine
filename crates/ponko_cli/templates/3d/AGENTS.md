# Agent instructions

{{project-name}} is a 3D game made with the Ponko engine
(<https://ponko.dev/guide/>).

## Commands

- `cargo test`: the headless tests in `tests/game.rs`. Run them after every change.
- `cargo run` opens the window; `HEADLESS=1 cargo run` runs 600 ticks without one (`TICKS` sets how many).
- `ponko package` builds the download players get (release build, assets, licenses, the host OS's archive) and starts it once headless; `[package.metadata.ponko]` in Cargo.toml says what goes in.
- Never open a window in tests: use `run_headless(ticks)`, which needs no
  window or GPU.

## Layout

- `src/lib.rs`: the game's rules (systems, components, setup). Sim systems
  hold the game logic and read no input directly, so tests and agents can
  drive them headless.
- `src/main.rs`: starts it.
- `assets/`: files the game loads by name (`settings.json` holds the numbers
  to tune).

## Driving the running game (MCP)

`.mcp.json` registers `ponko-mcp`, an MCP server that launches the game,
reads and changes its reflected components and resources, steps ticks,
injects input and takes screenshots. Install it once with
`cargo install ponko_mcp`. Read the guide's chapter on working with agents
before using it: <https://ponko.dev/guide/agents.html>. In short:

- `launch` (in this folder) builds and starts the game headless and paused;
  `run_ticks`, `get_entity`, `get_resource` and `set_component` inspect and
  change it; `quit_instance` stops it.
- Only reflected types are visible: `ctx.world.reflect::<T>()` for
  components, `ctx.world.reflect_resource::<T>()` for resources (see
  `setup` in `src/lib.rs`). Reflect new gameplay types too.
- The control endpoint the server talks to opens only in debug builds run
  with `PONKO_CONTROL=1` (`launch` sets it): <https://ponko.dev/guide/control.html>.
