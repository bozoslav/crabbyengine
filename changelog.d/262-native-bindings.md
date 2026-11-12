### Added
- C and C++ bindings for features that were Rust-only:
  - Action ids: `ponko_ctx_action_id`, `ponko_ctx_action_ids` and the
    `ponko_ctx_action_*_id` queries; C++ `ctx::action_id`, `action_ids` and
    `ponko::action_id` overloads of the action queries.
  - Input scenarios: `PonkoInputScenario` with input steps, action, resource and
    count checks, JSON, `ponko_game_input_scenario`, `ponko_ctx_start_scenario`,
    `ponko_ctx_cancel_scenario` and `ponko_world_scenario_report(_json)`; C++
    `ponko::input_scenario`, `scenario_check`, `scenario_report` and
    `game::run_headless_scenario`.
  - Asset sources: `ponko_ctx_add_asset_source` with read callbacks,
    `PonkoMemorySource`, and `ponko_ctx_load_asset_with` for your own asset
    types; C++ `memory_source`, `ctx::add_asset_source` and `load_asset_with`.
  - `ponko_resource_scope` and C++ `world_view::resource_scope<T>`: one
    resource and the world together, for queries and nested scopes.
  - Errors: `ponko_last_error_path`, `ponko_error_kind_name` and the
    `ponko_ctx_try_load_*` loaders (texture, font, sound, stream, mesh);
    C++ `ponko::result<T>`, `ctx::try_load_*`, `ponko::error::path()` and
    `ponko::kind_name`.
  - `ponko_ctx_ui_set_grid_named_span` and C++ `tree::set_grid_named_span`
    (CSS `span 2 name`).

- `Assets::has_source(id)`: whether a source with that id was added
  (`add_source` panics on a duplicate).

### Changed
- C++ `ctx::load_texture`, `load_font`, `load_sound`, `load_stream` and
  `load_mesh` still throw `ponko::error`, but with the real kind (`not found`,
  `decode`, ...) and the asset's path: `kind()` used to be `other`, and
  `what()` no longer starts with "Rust panic caught at C ABI boundary".
