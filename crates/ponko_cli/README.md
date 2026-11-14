# ponko_cli

The `ponko` command for [Ponko](https://github.com/ponkoengine/ponko) games.

```sh
cargo install ponko_cli
ponko new my_game            # a 2D game (the default)
ponko new my_game --3d       # a 3D game
ponko new my_sim --headless  # a simulation without a window, GPU or sound
```

Each project depends on the released `ponko`, has a fast dev profile, an
`assets/` folder, headless tests (`cargo test`), an `.mcp.json` for the
`ponko-mcp` server and an `AGENTS.md` for coding agents. `--path <engine>` or
`--git <url>` depends on an engine checkout or repository instead.

The templates are also usable with
[cargo-generate](https://github.com/cargo-generate/cargo-generate):

```sh
cargo generate --git https://github.com/ponkoengine/ponko crates/ponko_cli/templates/2d
```

- Getting started: <https://ponko.dev/guide/getting-started.html>

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
