# {{project-name}}

A headless simulation made with [Ponko](https://ponko.dev/).

```sh
cargo run              # 600 ticks, then a summary
TICKS=6000 cargo run
cargo test             # the headless tests
ponko package          # the download for players (zip, app, tar.gz)
```

- The guide: <https://ponko.dev/guide/>
- The API: <https://docs.rs/ponko/>
- AI agents: `AGENTS.md`, and `.mcp.json` for the `ponko-mcp` server
  (`cargo install ponko_mcp`).
