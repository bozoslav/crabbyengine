# Ctx and its services

Every system gets one argument, `ctx: &mut Ctx`. `Ctx` is a set of public
fields, one per area of the engine. Each area is a plain struct with its own
methods, so you find a call by asking "which area is this?" and typing
`ctx.<area>.`.

| Field | Type | What lives there |
|---|---|---|
| `ctx.world`, `ctx.commands`, `ctx.events` | `World`, `Commands`, `Events` | Entities, components and resources; changes to make after the system; messages between systems. |
| `ctx.time` | `Time` | The frame clock and the simulation clock, pause, step, speed. |
| `ctx.input` | `Input` | Keyboard, mouse, gamepads, named actions, typed text, the cursor lock. |
| `ctx.gfx` | `Graphics` | Drawing sprites, text and shapes; textures, fonts, screenshots; with the `3d` feature meshes, materials, shaders, effects, compute, instance batches, terrain, render targets, texture arrays, reflection probes and the quality presets. |
| `ctx.audio` | `Audio` | Sounds and music, what is playing, mixer buses, the 3D listener. |
| `ctx.ui` | `Ui` | The retained widget tree and its events, who has the pointer and the keyboard, and the immediate debug widgets (`ctx.ui.immediate`). |
| `ctx.assets` | `Assets` | Asset sources, data files, saving scenes and prefabs, what hot reload changed. |
| `ctx.perf` | `Perf` | System timings, frame and render statistics, the HUD, traces, profiling reports. |
| `ctx.camera`, `ctx.camera3d`, `ctx.lighting`, `ctx.sky`, `ctx.post`, `ctx.lod` | plain settings | What the renderer reads when it draws the frame. |
| `ctx.spatial`, `ctx.physics`, `ctx.physics3d`, `ctx.net` | | Neighbor queries, the physics worlds, the lockstep session. |

The API reference of each service type starts with what lives there, when to
call it (setup, simulation systems, frame systems) and what it needs (a
window, the `3d` feature, nothing).

## Shortcuts

The calls that nearly every system makes are also methods of `Ctx`. They
forward to their service:

| Shortcut | Same as |
|---|---|
| `ctx.dt()` | `ctx.time.dt()` |
| `ctx.key_down(k)`, `ctx.key_pressed(k)`, `ctx.key_released(k)` | `ctx.input.key_down(k)`, ... |
| `ctx.mouse_pos()` | `ctx.input.mouse_pos()` |
| `ctx.draw(&texture, pos)`, `ctx.draw_text(text, pos, size, color)` | `ctx.gfx.draw(...)`, `ctx.gfx.draw_text(...)` |
| `ctx.load_texture(name)` | `ctx.gfx.load_texture(name)` |
| `ctx.load_sound(name)`, `ctx.play_sound(&sound)` | `ctx.audio.load(name)`, `ctx.audio.play(&sound)` |
| `ctx.quit()` | |

A few more methods stay on `Ctx` because they use several areas at once:
`ctx.free(handle)`, `ctx.is_live(handle)` and `ctx.live_counts()` (see
[Handles](handles.md)), `ctx.load_scene(name)` and `ctx.spawn_prefab(name, at)`
(the world, textures and sounds together, see [Scenes and prefabs](scenes.md)),
`ctx.random()`, `ctx.mouse_world()`, `ctx.window_size()`, the ragdoll calls,
and the residency and streaming calls that load or evict (`ctx.resident_mesh`,
`ctx.stream_texture`, `ctx.set_residency_budget`, `ctx.finish_uploads`, ...:
they look at what the world shows). What only reads their state is on
`ctx.gfx` (`ctx.gfx.asset_state`, `ctx.gfx.residency`, `ctx.gfx.upload_stats`).

## Two areas at once

The services are separate fields, so Rust lets you use one while another is
borrowed. A shortcut takes all of `ctx`, so inside a query or a loop over UI
events, spell the service:

```rust
{{#include ../../tests/services.rs:time_borrow}}
```

```rust
{{#include ../../tests/services.rs:ui_borrow}}
```

`ctx.dt()` in the first loop would not compile: the query holds `ctx.world`,
and a method of `Ctx` asks for the whole of it. Either read the value before
the loop, or use the field.

## Which `dt`?

`ctx.time.dt()` (and the `ctx.dt()` shortcut) follows the system that is
running: the fixed step in a simulation system, the time since the last
frame anywhere else. When the code should say which one it means:

| Call | What it is |
|---|---|
| `ctx.time.sim_dt()` | The simulation step, in every kind of system (times `every` inside a system added with `sim_system_every`). |
| `ctx.time.frame_dt()` | Real time since the last frame, in every kind of system. |

## Calls that need another area

A service owns its own state and nothing else. The few calls that read or
write something outside it take that thing as an argument:

```rust
# use ponko::*;
# #[derive(serde::Deserialize)]
# struct Rules { speed: f32 }
# fn setup(ctx: &mut Ctx) {
// Data files fill the world.
ctx.assets.load_resource::<Rules>(&mut ctx.world, "rules.json");
// Immediate widgets draw with `gfx` and read the mouse from `input`.
let rect = Rect::new(10.0, 10.0, 120.0, 32.0);
if ctx.ui.immediate.button(&mut ctx.gfx, &ctx.input, rect, "Pause") {
    ctx.time.pause();
}
# }
```

With the `3d` feature the same holds for
`ctx.gfx.set_quality(&mut ctx.post, &mut ctx.lighting, quality)`,
`ctx.gfx.write_animated_morphs(&mut ctx.world)` and the reflection probe
calls.

## Coming from the flat spelling

Before the services, every call was a method of `Ctx`. Those methods still
compile and forward to the service; each one is deprecated, and the compiler
warning names the new spelling. Most keep their name and only gain the
field: `ctx.set_cursor_locked(true)` is `ctx.input.set_cursor_locked(true)`,
`ctx.create_mesh(...)` is `ctx.gfx.create_mesh(...)`. The ones whose name
changed, because the field now says the area:

| Before | Now |
|---|---|
| `ctx.time()` | `ctx.time.elapsed()` |
| `ctx.set_sim_speed(x)`, `ctx.sim_speed()` | `ctx.time.set_speed(x)`, `ctx.time.speed()` |
| `ctx.feed_input(event)` | `ctx.input.feed(event)` |
| `ctx.ui_insert_text(text)` | `ctx.input.insert_text(text)` |
| `ctx.create_sound`, `ctx.try_load_sound` | `ctx.audio.create`, `ctx.audio.try_load` |
| `ctx.play_sound_ex`, `ctx.play_sound_on`, `ctx.play_sound_at` | `ctx.audio.play_ex`, `ctx.audio.play_on`, `ctx.audio.play_at` |
| `ctx.stop_sound`, `ctx.set_sound_effects` | `ctx.audio.stop`, `ctx.audio.set_effects` |
| `ctx.audio_bus`, `ctx.set_audio_bus_volume`, `ctx.mute_audio_bus` | `ctx.audio.bus`, `ctx.audio.set_bus_volume`, `ctx.audio.mute_bus` |
| `ctx.set_audio_listener`, `ctx.audio_listener` | `ctx.audio.set_listener`, `ctx.audio.listener` |
| `ctx.graphics_quality()`, `ctx.set_graphics_quality(q)` | `ctx.gfx.quality(&ctx.post, &ctx.lighting)`, `ctx.gfx.set_quality(&mut ctx.post, &mut ctx.lighting, q)` |
| `ctx.free_gpu_buffer(buffer)` | `ctx.free(buffer)` |
| `ctx.ui_retained` (field) | `ctx.ui` |
| `ctx.ui_style` (field) | `ctx.ui.immediate.style` |
| `ctx.ui_panel`, `ctx.ui_label`, `ctx.ui_button` | `ctx.ui.immediate.panel(&mut ctx.gfx, ...)`, `.label(&mut ctx.gfx, ...)`, `.button(&mut ctx.gfx, &ctx.input, ...)` |
| `ctx.mouse_over_ui()` | `ctx.ui.wants_pointer()` |
| `ctx.ui_wants_keyboard()`, `ctx.set_ui_pointer_ray(ray)` | `ctx.ui.wants_keyboard()`, `ctx.ui.set_pointer_ray(ray)` |
| `ctx.add_asset_source`, `ctx.load_asset_with` | `ctx.assets.add_source`, `ctx.assets.load_with` |
| `ctx.load_resource::<R>(path)` and the `load_*_json` calls | `ctx.assets.load_resource::<R>(&mut ctx.world, path)`, ... |
| `ctx.save_scene(path)`, `ctx.save_prefab(entity, path)` | `ctx.assets.save_scene(&ctx.world, path)`, `ctx.assets.save_prefab(&ctx.world, entity, path)` |
| `ctx.set_perf_hud`, `ctx.perf_hud`, `ctx.perf_report` | `ctx.perf.set_hud`, `ctx.perf.hud`, `ctx.perf.report` |
| `ctx.profile(name, \|ctx\| ...)` | `let _timing = ctx.perf.scope(name);` |

The two fields in the table (`ui_retained`, `ui_style`) are the only
spellings that no longer compile.

## C and C++

The C API and the C++ wrapper keep their flat names (`ponko_ctx_*`,
`ctx.key_down(...)`, `ctx.play_sound(...)`): they don't have Rust's borrow
rules to work around, and the ABI does not change with this grouping. The
C++ examples in this guide therefore look the same as before.
