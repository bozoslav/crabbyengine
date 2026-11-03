# The inspector: look inside a running game

Press the backquote key (`` ` ``, left of 1) in a debug build and a panel
opens on the right of the window. It shows the game as it runs:

- **Sim controls**: pause, resume, step one tick, and the simulation speed
  (drag the `1.00×` box). The tick, frame rate and entity count are below,
  and a graph of the last 240 frame times (the line marks 60 fps; red bars
  are slow frames).
- **The selected entity**: each component it has. Reflected components are
  editable in place: drag numbers, tick booleans, type text (applied when
  you press Enter or click away). **Remove** takes a component off,
  **Despawn** removes the entity, **Look at** moves the camera to it. A
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

The environment variable `CRABBY_INSPECTOR` overrides the game's choice:
`1` turns it on, `0` turns it off, and `open` turns it on and opens it at
start.

While the inspector is closed it costs nothing: egui doesn't run and no
input goes to it. While it's open, clicks, keys and the mouse wheel over
the panel stay in the panel, and `ctx.mouse_over_ui()` is `true` there,
so a game that checks it doesn't also act on clicks in the panel. The
panel isn't in screenshots taken with `ctx.save_screenshot` or by agents.

The overlay is built with [egui](https://github.com/emilk/egui), behind
the `inspector` cargo feature (on by default). Games that don't want egui
in their build turn default features off:

```toml
crabbyengine = { version = "1", default-features = false, features = ["window", "audio"] }
```

C and C++ games get the inspector from `crabby_ffi`'s `inspector` feature,
also on by default.
