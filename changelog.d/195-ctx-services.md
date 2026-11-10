### Added
- `Ctx` is grouped into services: plain structs in public fields, one per
  area, each with its own methods. `ctx.time` (`Time`: clocks, pause, step,
  speed), `ctx.input` (`Input`: keyboard, mouse, gamepads, actions, typed
  text, cursor lock), `ctx.gfx` (`Graphics`: drawing, textures, fonts,
  screenshots and every 3D asset), `ctx.audio` (`Audio`: sounds, playing
  handles, buses, the listener), `ctx.ui` (`Ui`: the retained tree, pointer
  and keyboard capture, immediate widgets), `ctx.assets` (`Assets`: asset
  sources, data files, saving scenes and prefabs, hot reload) and
  `ctx.perf` (`Perf`: timings, statistics, the HUD, traces, reports). The
  services are separate fields, so they borrow apart: play a sound while
  reading `ctx.ui.events()`, read `ctx.time.dt()` or draw with `ctx.gfx`
  inside a `ctx.world` query. The guide's new "Ctx and its services"
  chapter has the table and the old-to-new spellings.
- `Time::frame_dt()` and `Time::sim_dt()` say which clock they mean;
  `dt()` stays the one that follows the running system.
- `UiImmediate` (`ctx.ui.immediate`): the immediate panel, label and button
  with their `style`, and `covers(point)`.
- `Ui::wants_pointer()` is now true over the retained UI, a world panel's
  overlay and the immediate widgets drawn this frame or the last, so one
  call answers "is the mouse over UI?".
- `Assets::hot_reload()` says whether asset hot reload is on.
- `Audio`, `Graphics`, `Assets`, `Perf`, `Time`, `Input` and `UiImmediate`
  are exported from the crate root.

### Changed
- On the services, a call that reads or writes another area takes it as an
  argument: `ctx.assets.load_resource::<R>(&mut ctx.world, path)` (and the
  `load_*_json` calls), `ctx.assets.save_scene(&ctx.world, path)`,
  `ctx.assets.save_prefab(&ctx.world, entity, path)`,
  `ctx.gfx.write_animated_morphs(&mut ctx.world)`,
  `ctx.gfx.bake_reflection_probe(&mut ctx.world, entity)`,
  `ctx.gfx.load_reflection_probe(&mut ctx.world, entity, name)`,
  `ctx.gfx.quality(&ctx.post, &ctx.lighting)`,
  `ctx.gfx.set_quality(&mut ctx.post, &mut ctx.lighting, quality)`,
  `ctx.ui.immediate.panel(&mut ctx.gfx, rect)`,
  `ctx.ui.immediate.label(&mut ctx.gfx, pos, text)` and
  `ctx.ui.immediate.button(&mut ctx.gfx, &ctx.input, rect, label)`. The
  old `Ctx` methods keep their signatures.
- `ctx.audio.listener()` and the UI's pointer follow the camera and the
  mouse as they were when the running system started.
- A GPU buffer is freed with `ctx.free(buffer)`, which also drops the
  compute passes that used it.

### Deprecated
- The flat `Ctx` methods that a service now holds (about 180, for example
  `ctx.mouse_down`, `ctx.create_mesh`, `ctx.set_audio_bus_volume`,
  `ctx.perf_report`). Each forwards to its service, is hidden from the API
  reference and names the new spelling in its warning. The everyday
  shortcuts stay and are not deprecated: `dt`, `key_down`, `key_pressed`,
  `key_released`, `mouse_pos`, `draw`, `draw_text`, `quit`, `load_texture`,
  `load_sound` and `play_sound`. The C API and the C++ wrapper keep their
  flat names.

### Removed
- **Breaking:** the `Ctx` fields `ui_retained` and `ui_style`.
  `ctx.ui_retained` is now `ctx.ui`, and `ctx.ui_style` is
  `ctx.ui.immediate.style`. These two fields are the only old spellings
  that no longer compile. In the C API, `crabby_ctx_ui_wants_pointer` now
  also covers world panels and the immediate widgets (it follows
  `Ui::wants_pointer()`), not only the retained UI.
