### Breaking changes

- **One way to register a system.** `Game::add(phase, system)` is the
  general form and `Game::add_system(phase, system)` the same returning a
  `SystemId`. What they take is the new `IntoSystem` trait (in the
  prelude): a function or closure of `&mut Ctx`, a view system (a `Send`
  function or closure of `&mut SystemView`), or either with options set by
  the trait's methods `named(name)`, `every(ticks)` and `access(declaration)`,
  which return a `SystemConfig`. `sim_system(f)` and `system(f)` stay as
  the two shortcuts (`add(Phase::AfterPhysics, f)` and
  `add(Phase::Frame, f)`). Removed, with the replacement:
  - `Game::late_system(f)` → `Game::add(Phase::Late, f)`.
  - `Game::sim_system_every(n, f)` → `Game::add(Phase::AfterPhysics, f.every(n))`.
  - `Game::add_with_access(phase, access, f)` → `Game::add(phase, f.access(access))`.
  - `Game::add_view(phase, access, f)` → `Game::add(phase, f.access(access))`;
    the system is a view system because it takes `&mut SystemView`.
  - `Game::add_view_system(phase, access, f)` → `Game::add_system(phase, f.access(access))`.
  - The hidden `Game::named_add(phase, name, ticks, f)`, `named_add_system`,
    `named_add_with_access`, `named_sim_system`, `named_late_system` and
    `named_system` → `Game::add(phase, f.named(name).every(ticks).access(access))`
    (or `add_system`), with only the options needed.
  - A closure passed to `add` or `add_system` now names its parameter type:
    `game.add(phase, |ctx| ...)` → `game.add(phase, |ctx: &mut Ctx| ...)`.
    A closure with options is wrapped in parentheses:
    `(|ctx: &mut Ctx| ...).every(4)`. `sim_system` and `system` take
    closures as before.
  - A view system added without `.access(..)` can reach nothing; an
    interval in a per-frame phase and an interval of 0 still panic, now
    from `Game::add` and `IntoSystem::every`.
- **Removal is by id only.** `Game::remove_system(function)` and
  `Game::remove_named_system(name)` are gone → `Game::remove(id)` with a
  `Systems` constant for a built-in (`Systems::DRAW_SPRITES`) or the
  `SystemId` from `Game::add_system`. Removing an absent id does nothing
  (`remove_system` panicked). The C `ponko_game_remove_system(name)` is
  unchanged.
- `Game::add_setup(f)` (hidden) is gone → `Game::setup(f)` for the game's
  own startup, or a `Plugin` whose `setup` does the work when more systems
  are added afterwards.
- `Game::order_problem() -> Option<String>` → `Game::check_order() ->
  Result<(), ponko::schedule::OrderError>`; the error has the two systems'
  names (`first`, `second`) and prints the old message.
- `ReportedSystem` has a new public field `id: SystemId` (the
  registration's id, for `Game::remove` and `Game::order`); code that builds
  one names it.
- **One access builder.** `ponko_ecs::ViewAccess` (`ponko::ecs_core::ViewAccess`)
  is gone: a `ViewState` declares its own components and resources.
  `ViewState::new(ViewAccess::new().read::<A>().write::<B>())` →
  `ViewState::new().read::<A>().write::<B>()`; `ViewAccess::read_resource` /
  `write_resource` / `components` / `resources` / `overlaps` → the same
  methods on `ViewState` (`a.access().overlaps(b.access())` →
  `a.overlaps(&b)`); `ViewState::access()` is gone. Declaring on a prepared
  state panics. In a game, `SystemAccess` is the one declaration type, for
  `Ctx` systems and view systems alike.
- **`SystemView` uses the names `Ctx` has.** Its services are fields that
  check the declaration when they are used (`ViewService`, read with the
  service declared, changed with it declared written), so a view system
  reads like a `Ctx` system. Old → new, for `sys: &mut SystemView`:
  - `sys.action_down(a)`, `action_pressed`, `action_released`, `action_axis`,
    `action_axis2` → `sys.input.action_down(a)` and so on.
  - `sys.mouse_down(b)`, `mouse_pressed`, `mouse_released` →
    `sys.input.mouse_down(b)` and so on.
  - `sys.sim_tick()` → `sys.time.sim_tick()`; `sys.fixed_dt()` →
    `sys.time.fixed_dt()`; `sys.frame()` → `sys.time.frame()`;
    `sys.time()` → `sys.time.elapsed()`.
  - `sys.play_sound_ex(sound, params)` → `sys.audio.play_ex(sound, params)`.
  - `sys.camera()` / `sys.camera_mut()` → `sys.camera`; `sys.camera3d()` /
    `sys.camera3d_mut()` → `sys.camera3d`.
  - `sys.spatial()` → `sys.spatial`.
  - `sys.physics()` / `sys.physics_mut()` → `sys.physics`; `sys.physics3d()` /
    `sys.physics3d_mut()` → `sys.physics3d`.
  - `sys.net()` / `sys.net_mut()` → `sys.net`.
  - `sys.commands()` → `sys.commands`; `sys.events()` → `sys.events`.
  - Kept, the shortcuts `Ctx` keeps for these services: `sys.dt()`,
    `sys.key_down(k)`, `sys.key_pressed(k)`, `sys.key_released(k)`,
    `sys.mouse_pos()`, `sys.play_sound(sound)` and `sys.random()`.
  - `sys.input`, `sys.time` and `sys.audio` are the whole services, so a
    view system also gets what the flat methods left out (gamepads, the
    mouse wheel, `time.turns(n)`, audio buses).
