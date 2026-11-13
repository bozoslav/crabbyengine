# Saves and settings

`ctx.storage` keeps the player's own files: settings, progress, save games.
On the desktop they live in the player's data folder; in a web build, in the
browser's storage. The calls are the same on both, and every write is safe
against crashes.

<div class="lang-pair">
<div>

```rust
{{#include ../../tests/storage.rs:storage}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:storage}}
```

</div>
</div>

## Where the files go

Give the game an app id with `Game::app_id` (C++ `game::app_id`, C
`ponko_game_app_id`). Its files go in a folder of that name:

| System | Folder |
|---|---|
| Windows | `%APPDATA%\<app id>\` |
| macOS | `~/Library/Application Support/<app id>/` |
| Linux and other Unix | `$XDG_DATA_HOME/<app id>/`, else `~/.local/share/<app id>/` |
| Web | The page's IndexedDB database `ponko:<app id>` |

Pick the id once and keep it: a new id is a new folder, and the old saves stay
in the old one. Letters, digits, `-`, `_` and `.` (`"space-crab"`,
`"com.example.crabs"`). A game without an app id keeps no files: every call
fails with `ErrorKind::Unsupported`, and `ctx.storage.is_available()` is
`false`.

`ctx.storage.dir()` is the folder (`None` on the web). Two things move it:

- `PONKO_DATA_DIR=<folder>` replaces the system's data folder; the files go
  in `<folder>/<app id>/`. Handy for a smoke test of a packaged game that must
  not touch the real saves.
- `Game::data_dir(folder)` names the exact folder. Tests use it to give each
  test a folder of its own; a portable install can keep its saves beside the
  game.

## Reading and writing

| Call | What it does |
|---|---|
| `write(name, bytes)`, `write_json(name, &value)` | Writes the file, creating its folders. |
| `read(name)`, `read_json::<T>(name)` | The file's bytes, or the value. `ErrorKind::NotFound` if there is none. |
| `read_backup(name)` | The file as it was before its last write. |
| `exists(name)`, `list()` | Whether a file is there; every file's name, sorted. |
| `delete(name)` | Deletes the file and its backup. A missing file is fine. |

Names are relative, with `/` between folders (`"saves/slot1.json"`). Each part
uses letters, digits, spaces, `-`, `_` and `.`, and doesn't start with a `.`;
anything else (`..`, `\`, `:`, a device name like `NUL`) is
`ErrorKind::Invalid`, so a name can't leave the folder or mean something
different on another system. Names ending in `.tmp` or `.bak` are the engine's
own (see below).

Errors are `ponko::prelude::Error`s with the file's name as their path. A
missing file is a first run, not a failure: start from the defaults. A file
that doesn't parse, or holds values your game can't have produced, may be
damaged: start from the defaults, tell the player, and keep the bytes (the
reference game copies them to `<name>.bad`). A write that fails (a read-only
or full disk) shouldn't stop the game either; say that it couldn't save.

## Safe writes

A crash, a killed process or a power cut in the middle of a write never
leaves a half-written file. `write` does it in steps:

1. the bytes go to `<name>.tmp`, and are flushed to the disk;
2. the current file is kept as `<name>.bak`;
3. a rename puts the new file in place. A rename is atomic: the name is the
   old file or the new one, never a mix.

A crash before step 3 leaves the old file in place, and the next write
replaces the leftover `<name>.tmp`. `list` doesn't show either suffix.
`read_backup` gives the version before the last write, for a game that wants
to offer "the save before" when the newest one is damaged by something else.
The engine's tests simulate a crash between each step and check that the old
save is read back whole.

## Save slots

`ctx.storage.save_world(&ctx.world, slot)` saves the world's reflected
components and resources ([reflection](reflection.md)) to `saves/<slot>.json`,
and `ctx.load_world(slot)` replaces the world with it: the same load as
[`load_scene`](scenes.md), from the player's storage instead of the assets, so
its table of what a load replaces and keeps applies. `world_slots()` lists the
slots, `delete_world(slot)` removes one. C and C++: `ponko_ctx_save_world`,
`ponko_ctx_load_world` and `ctx.save_world`, `ctx.load_world`.

A save slot is one player's file, so `load_world` is refused while a
[lockstep session](networking.md) is connecting or running: the other peers
don't have it.

## In the browser

Browsers only offer their storage asynchronously, so a web build reads every
stored file into memory while the page fetches its assets, before setup runs.
After that the calls work as on the desktop:

- reads, `exists` and `list` answer from memory;
- a write or delete changes memory at once, and the browser stores it in one
  IndexedDB transaction: the file and its backup together, so a tab closed
  halfway keeps both old versions or both new ones;
- `ctx.storage.is_saving()` is `true` until the browser confirmed every
  change. Closing the tab sooner may lose the last write (never half of it).
  On the desktop it is always `false`: `write` returns once the file is on the
  disk.

The same game code works on both; nothing needs to wait or poll. Two
differences remain. Some private windows refuse a site its storage:
`ctx.storage.is_persistent()` is then `false` and files last until the page
closes. And browsers may clear a site's storage when the disk runs low, so
keep a player's work small and expect a first run now and then. The [web
smoke test](web.md#testing-in-a-browser) runs the reference game in a headless
browser, leaves its settings screen (which saves), reloads the page and checks
that the settings come back from the browser's storage.

## C and C++

| C | C++ (`ponko::ctx`) |
|---|---|
| `ponko_game_app_id`, `ponko_game_data_dir` | `game::app_id`, `game::data_dir` |
| `ponko_ctx_storage_write(ctx, name, bytes, len)` | `storage_write(name, bytes, size)`, `storage_write(name, text)` |
| `ponko_ctx_storage_read(ctx, name, buffer, capacity, &len)` | `storage_read(name)` (bytes), `storage_read_text(name)`: nothing if missing |
| `ponko_ctx_storage_read_backup` | `storage_read_backup(name)` |
| `ponko_ctx_storage_exists`, `ponko_ctx_storage_delete` | `storage_exists(name)`, `storage_delete(name)` |
| `ponko_ctx_storage_list` (names, one per line) | `storage_list()` |
| `ponko_ctx_storage_dir` (empty on the web) | `storage_dir()` |
| `ponko_ctx_save_world`, `ponko_ctx_load_world` | `save_world(slot)`, `load_world(slot)` |

The C reads take a buffer and always set the length: call with a null buffer
to learn the size, then again with one that big. A missing file is
`PONKO_NOT_FOUND`; storage without an app id fails with the error kind
`PONKO_ERROR_UNSUPPORTED` (`ponko_last_error_kind`).
