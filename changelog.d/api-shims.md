### Breaking changes

- Removed `src/ctx_compat.rs`: all 182 deprecated flat `Ctx` forwarders are gone.
  Call the service instead (`ctx.action_down(..)` → `ctx.input.action_down(..)`).
  Methods that kept their name moved to the field named here:
  - `ctx.input`: `action_axis`, `action_axis2`, `action_down`, `action_id`, `action_ids`, `action_pressed`, `action_released`, `actions`, `cursor_locked`, `gamepad_axis`, `gamepad_down`, `gamepad_pressed`, `gamepad_released`, `gamepad_stick`, `gamepads`, `mouse_delta`, `mouse_down`, `mouse_motion`, `mouse_pressed`, `mouse_released`, `mouse_wheel`, `set_actions`, `set_cursor_locked`, `try_action_id`, `typed_text`.
  - `ctx.gfx`: `clear`, `create_compute`, `create_effect`, `create_gpu_buffer`, `create_instance_batch`, `create_material`, `create_mesh`, `create_render_target`, `create_shader`, `create_skinned_mesh`, `create_terrain_material`, `create_terrain_tile`, `create_texture`, `create_texture_array`, `create_ui_panel`, `decal_textures`, `default_material`, `draw_circle`, `draw_ex`, `draw_line`, `draw_rect`, `draw_text_ex`, `free_terrain_tile`, `gpu_buffer_len`, `gpu_capabilities`, `has_texture_array`, `instance_batch_len`, `is_render_target`, `is_resident`, `load_clip`, `load_clips`, `load_compute`, `load_effect`, `load_font`, `load_mesh`, `load_shader`, `load_skeleton`, `load_texture_array`, `loaded_materials`, `measure_text`, `measure_text_ex`, `mesh_costs`, `mesh_shadows`, `models_drawn`, `morph_target`, `reset_temporal_history`, `residency`, `save_screenshot`, `set_decal_textures`, `set_mesh_gpu_bounds`, `set_mesh_gpu_buffer`, `set_mesh_shadows`, `set_texture_filter`, `shader_status`, `sprite_count`, `supports_decals`, `try_create_compute`, `try_create_effect`, `try_create_material`, `try_create_mesh`, `try_create_render_target`, `try_create_shader`, `try_create_terrain_material`, `try_create_texture_array`, `try_create_ui_panel`, `try_load_clip`, `try_load_clips`, `try_load_compute`, `try_load_effect`, `try_load_font`, `try_load_mesh`, `try_load_shader`, `try_load_skeleton`, `try_load_texture`, `try_load_texture_array`, `try_set_decal_textures`, `update_instance_batch`, `update_material`, `update_texture`, `upload_terrain_tile`, `warm_render_pipelines`, `write_gpu_buffer`.
  - `ctx.time`: `fixed_alpha`, `fixed_dt`, `fixed_steps`, `fps`, `frame`, `is_paused`, `pause`, `resume`, `sim_tick`, `sim_time`, `step`, `turns`.
  - `ctx.perf`: `display_wait_ms`, `frame_stats`, `gpu_passes`, `profiling`, `record_trace`, `render_stats`, `scope_timings`, `set_profiling`, `system_timings`.
  - `ctx.audio`: `load_stream`.
  - `ctx.assets`: `reloaded_files`, `reloaded_textures`.
  The methods whose name or arguments changed:
  `ctx.time()` → `ctx.time.elapsed()`; `ctx.sim_speed()` / `ctx.set_sim_speed(x)` →
  `ctx.time.speed()` / `ctx.time.set_speed(x)`;
  `ctx.feed_input(e)` → `ctx.input.feed(e)`; `ctx.ui_insert_text(t)` → `ctx.input.insert_text(t)`;
  `ctx.create_sound`, `try_load_sound`, `stop_sound`, `set_sound_effects`, `play_sound_ex`,
  `play_sound_on`, `play_sound_at` → `ctx.audio.create`, `try_load`, `stop`, `set_effects`,
  `play_ex`, `play_on`, `play_at`; `ctx.audio_bus`, `set_audio_bus_volume`, `mute_audio_bus`,
  `audio_listener`, `set_audio_listener` → `ctx.audio.bus`, `set_bus_volume`, `mute_bus`,
  `listener`, `set_listener` (the old `audio_listener()` and `play_sound_at(..)` read
  `ctx.camera3d` as it is now; `ctx.audio.listener()` and `ctx.audio.play_at(..)` use the
  camera as it was when the running system started. If a system moves the camera and then
  plays a positional sound, call `ctx.audio.set_listener(Some(..))` with the new pose first);
  `ctx.mouse_over_ui()` → `ctx.ui.wants_pointer()` (the old method also counted the debug
  panels under the mouse as it is now; for exactly that, use
  `ctx.ui.wants_pointer() || ctx.ui.immediate.covers(ctx.input.mouse_pos())`);
  `ctx.ui_wants_keyboard()` / `ctx.set_ui_pointer_ray(r)` → `ctx.ui.wants_keyboard()` /
  `ctx.ui.set_pointer_ray(r)`; `ctx.ui_panel` / `ui_label` / `ui_button` →
  `ctx.ui.immediate.panel(&mut ctx.gfx, ..)` / `.label(&mut ctx.gfx, ..)` /
  `.button(&mut ctx.gfx, &ctx.input, ..)`; `ctx.add_asset_source`, `load_asset_with` →
  `ctx.assets.add_source`, `load_with`; `ctx.load_resource::<R>(p)`, `load_resource_json`,
  `load_resource_json_named`, `load_component_json`, `load_component_json_named`,
  `save_scene`, `save_prefab` → the same names on `ctx.assets`, taking `&mut ctx.world` (or
  `&ctx.world`) first; `ctx.set_perf_hud`, `perf_hud`, `perf_report` → `ctx.perf.set_hud`,
  `hud`, `report` (the old `perf_report` counted the entities in the world as it is now;
  `ctx.perf.report()` has the count from the start of the system, so for the old number set
  `report.entities = ctx.world.len()` on the result); `ctx.profile(name, ..)` → `let _timing = ctx.perf.scope(name);`;
  `ctx.graphics_quality()` / `set_graphics_quality(q)` →
  `ctx.gfx.quality(&ctx.post, &ctx.lighting)` / `ctx.gfx.set_quality(&mut ctx.post, &mut ctx.lighting, q)`;
  `ctx.free_gpu_buffer(b)` → `ctx.free(b)`; `ctx.has_font(f)`, `has_material`, `has_shader`,
  `has_compute`, `has_effect`, `is_mesh_live` → `ctx.is_live(h)`;
  `ctx.free_mesh(m)`, `free_instance_batch(b)` → `ctx.free(h)`;
  `ctx.load_mesh_data(..)` → `ctx.gfx.try_load_mesh_data(..)` (or `ctx.gfx.load_mesh_data(..)`, which panics);
  `ctx.bake_reflection_probe`, `load_reflection_probe`, `write_animated_morphs` → the same names on `ctx.gfx`,
  taking `&mut ctx.world`.
  `Ctx` keeps these shortcuts: `dt`, `key_down`, `key_pressed`, `key_released`,
  `mouse_pos`, `mouse_world`, `window_size`, `quit`, `draw`, `draw_text`, `load_texture`,
  `load_sound`, `play_sound`, `random`. Methods that need several services at once stay on
  `Ctx` too: `free`, `is_live`, `load_scene`, `load_scene_lockstep`, `spawn_prefab`,
  `stream_mesh`, `stream_texture`, `resident_mesh`, `resident_texture` (and their `try_`,
  `pin_` and `unpin_` forms), `set_residency_budget`, `finish_uploads`, `start_ragdoll`, `stop_ragdoll`, `draw_skeleton`,
  `live_counts`, `asset_inventory`, `ecs_diagnostics`, `capabilities_json`, `control_addr`.
- Removed the deprecated `Graphics` aliases `has_font`, `has_material`, `has_shader`,
  `has_compute`, `has_effect`, `is_mesh_live` (→ `is_live(h)`) and `free_mesh`,
  `free_instance_batch` (→ `free(h)`).
- Removed `Color::CRABBY_DARK` → `Color::PONKO_DARK`.
- The `CRABBY_*` environment fallbacks are no longer read (`CRABBY_CONTROL`, `CRABBY_BENCH*`,
  `CRABBY_TRACE`, `CRABBY_HUD`, `CRABBY_INSTANCES_DIR`, ...) → the same names with `PONKO_`.
- World saves and prefabs tagged `crabby-world` / `crabby-prefab` no longer load → change the
  `format` field to `ponko-world` / `ponko-prefab`.
- glTF extras `crabbyMorphFrames` / `crabbyAnimatedTargets` are no longer read →
  `ponkoMorphFrames` / `ponkoAnimatedTargets`.
- The web canvas fallback `<canvas id="crabby">` is gone → `<canvas id="ponko">`.
- Steam message lobbies no longer publish or accept the `crabby_protocol` metadata → `ponko_protocol`.
