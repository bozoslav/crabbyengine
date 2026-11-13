### Added
- `ctx.storage` (`ponko::storage::Storage`): the player's own files in the
  per-user data folder (Windows `%APPDATA%`, macOS `~/Library/Application
  Support`, Linux `$XDG_DATA_HOME` or `~/.local/share`), named by
  `Game::app_id`; `Game::data_dir` and `PONKO_DATA_DIR` move it. `write`
  is crash-safe (temporary file, flushed, previous file kept as
  `<name>.bak`, then a rename); `read`, `read_backup`, `exists`, `list`,
  `delete`, `write_json`, `read_json`.
- Web builds keep the same files in an IndexedDB database, read into memory
  before setup and written through in one transaction per write, so the same
  game code runs in the browser; `is_persistent` and `is_saving` say what
  the browser does with them.
- Save slots: `ctx.storage.save_world(&ctx.world, slot)`, `world_slots`,
  `delete_world` and `ctx.load_world(slot)`.
- C and C++: `ponko_game_app_id`, `ponko_game_data_dir`,
  `ponko_ctx_storage_*`, `ponko_ctx_save_world`, `ponko_ctx_load_world`, and
  the `ponko::game` / `ponko::ctx` methods of the same names.
- The guide's "Saves and settings" chapter.
- The web smoke runner can reload the page and check what the game kept
  (`--after-reload`, fixture field `afterReload`).

### Changed
- The `coin_dash` reference game keeps its progress and settings through
  `ctx.storage` (in the browser too) and has no path code of its own.
  `PONKO_DATA_DIR` replaces its `COIN_DASH_SAVE` variable; the files stay
  in the same `coin-dash` folder.
