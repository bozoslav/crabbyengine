### Added
- The automatic animator, particle and positional-audio updates are ordinary
  systems (feature `3d`): `Systems::UPDATE_ANIMATORS` and
  `Systems::UPDATE_PARTICLES` at the end of `Phase::Animate`,
  `Systems::UPDATE_AUDIO3D` at the end of `Phase::Draw`. They are listed in
  `system_timings`, and `Game::remove` takes them out. They run where they
  ran before: the animators and particles after your frame (`Animate`)
  systems and before your `Late` ones, positional audio after your `Late`
  and drawing systems.
- `ponko::systems`: the engine's built-in system functions in one module
  (`systems::draw_sprites`, `systems::step_physics3d`,
  `systems::follow_attachments`, `systems::update_animators`, ...), with a
  table of ids and phases.
- `Game::system_count(id)`: how many installed systems have this id.
- `SystemId::to_bits` / `from_bits`, for bindings and tools.
- C: `ponko_game_add_system_id` (a system with an id), `ponko_game_remove`
  and the `PONKO_SYSTEM_*` built-in ids, `ponko_game_setup`,
  `ponko_game_plugin`. C++: `game::add_system` returning `ponko::system_id`,
  `game::remove`, `ponko::systems::*`, `game::setup` and `game::plugin`
  (any object with `build(ponko::game&)` and an optional
  `setup(ponko::ctx&)`).
- The guide's "Systems and the frame" chapter is written around `Phase` and
  `Plugin`, with the table of built-in systems; `examples/coin_dash` is
  built from plugins.

### Deprecated
- `Game::remove_system(f)` and `Game::remove_named_system(name)`: use
  `Game::remove` with a `Systems` constant or the id `Game::add_system`
  returned. Both still work.
- The built-in system functions at the crate root (`ponko::draw_sprites`,
  `ponko::step_physics3d`, `ponko::follow_attachments`, ...): use
  `ponko::systems::...`. The old names forward to the same functions, and a
  system added under an old name has the same id and timing name.
