# The inspector: look inside a running game

Press the backquote key (`` ` ``, left of 1) in a debug build and a panel
opens on the right of the window. It shows the game as it runs:

- **Sim controls**: play, pause, step one tick, and the simulation speed
  (drag the `1.00×` box). The tick, frame rate and entity count are below,
  and a graph of the last 240 frame times (the line marks 60 fps; red bars
  are slow frames).
- **The selected entity**: each component it has. Reflected components are
  editable in place: drag numbers, tick booleans, type text (applied when
  you press Enter or click away). **Remove** takes a component off,
  **Duplicate** copies the entity, **Despawn** removes it, **Look at**
  moves the camera to it. A
  yellow ring marks it in the world.
- **Entities**: every entity, with its components. Pick a component under
  **With** to list only entities that have it. Click a row to select it, or
  press **Pick** and then click a unit in the world.
- **Resources**: reflected resources, editable the same way.
- **Systems**: each system's average time per frame, slowest bars longest.
  Sim systems are marked `(sim)`.

Only [reflected](reflection.md) components and resources can be read and
edited; others show up by name, marked *not reflected*. The same reflection
is what agents and save files use, so one `reflect` call covers all three.

## Editing: build levels in the game

To try it: `cargo run --example editor`, press `` ` ``, then **Pause**.

Pause, and the inspector is an editor. Clicks and the editor's keys in the
world go to it, not to the game:

| Do | To |
|---|---|
| Click | Select the sprite under the mouse (the top one), or the nearest entity |
| Drag | Move the selected entity |
| Right or middle drag, wheel | Pan and zoom the camera |
| Delete (or Backspace) | Despawn the selected entity |
| Ctrl+D (⌘D) | Duplicate it, a little down and to the right |
| Ctrl+Z (⌘Z) | Undo: moves, edits, added and removed components, spawns, despawns |
| Esc | Stop placing a prefab |

The **Edit** section has **Undo**, **New entity** (a bare transform in the
middle of the screen), and two rows for files in `assets/`:

- **Scene**: save the world to a [scene](scenes.md) file, or load one
  (loading clears undo).
- **Prefab**: type a name and **Save** the selected entity as a prefab.
  Prefabs in `assets/prefabs/` are listed below: click one, then click in
  the world to place it (hold Shift to place several).

Below a selected entity's components, **Add component** lists the
reflected components it doesn't have. A component is added with its
default fields, so types whose fields all have `#[serde(default)]` add
cleanly; others say which field they need.

**Play** runs the game and remembers the world; **Stop** pauses and puts
the world back as it was at Play, so you can try a level and return to it.
**Keep** forgets the remembered world. Only reflected components are put
back: anything that isn't reflected isn't in the snapshot either.

Undoing a despawn brings the entity back as a new entity: handles never
come back to life, so other components that pointed at it still point at
nothing.

## Turning it on and off

The inspector is on by default in debug builds and off in release builds.
To ship it in a release build for playtesters:

<div class="lang-pair">
<div>

```rust
{{#include ../../tests/reflect.rs:inspector}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:inspector}}
```

</div>
</div>

The environment variable `PONKO_INSPECTOR` overrides the game's choice:
`1` turns it on, `0` turns it off, and `open` turns it on and opens it at
start.

While the inspector is closed it costs nothing: egui doesn't run and no
input goes to it. While it's open, clicks, keys and the mouse wheel over
the panel stay in the panel, and `ctx.ui.wants_pointer()` is `true` there,
so a game that checks it doesn't also act on clicks in the panel. The
panel isn't in screenshots taken with `ctx.save_screenshot` or by agents.

The overlay is built with [egui](https://github.com/emilk/egui), behind
the `inspector` cargo feature (on by default). Games that don't want egui
in their build turn default features off:

```toml
ponko = { version = "0.0.1", default-features = false, features = ["window", "audio"] }
```

C and C++ games get the inspector from `ponko_ffi`'s `inspector` feature,
also on by default.
