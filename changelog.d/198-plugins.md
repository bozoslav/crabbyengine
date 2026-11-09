### Plugin composition and system removal
- `Plugin`, with `Game::plugin` and `ConfiguredGame::plugin`: `build` registers
  systems in named phases right away; `setup` runs once, in registration order
  with `Game::setup` callbacks and before a legacy `run(|ctx| ...)` closure,
  applying deferred commands after each callback.
- `SystemId`: `Game::add_system(phase, f)` returns a unique, never reused ID and
  `Game::remove(id)` removes it (an absent ID is a no-op). `Systems` holds
  stable IDs for built-in systems, such as `Systems::DRAW_SPRITES` and
  `Systems::PHYSICS3D`, and removes every registration of that built-in.
  `remove_system` and `remove_named_system` keep working and forward to it.
