### Added
- Optional parallel system scheduler: `Game::parallel_systems(true)` runs
  view systems (`Game::add_view`) whose declarations don't conflict at the
  same time. Off by default; `&mut Ctx` systems (and all C and C++ systems)
  always run alone, in registration order. Each phase's registration order
  is cut into stages: a system alone, or a batch of neighbouring view
  systems with no conflict, so conflicting systems keep their order.
- Barriers: commands recorded in a batch apply, and events sent in it become
  visible, when the batch ends, in registration order. `Game::barrier(phase)`
  and `Game::order(first, second)` add explicit ones;
  `Game::add_view_system` returns the `SystemId` for `order`.
- `Game::schedule_plan()` and the `schedule` module (`SchedulePlan`, `Stage`,
  `Boundary`, `Alone`): the stages, which systems share one, and why each
  stage starts where it does.
- A simulation gives the same result with the scheduler on or off and on any
  number of threads (lockstep peers may differ in the setting). A batch only
  goes to other threads when the usual run time of its systems, without the
  slowest, reaches `Game::parallel_threshold` (default 50 µs, the measured
  crossover); otherwise, and while profiling and on the web, it runs in
  order on the game's thread through
  the same code. `PONKO_PARALLEL_SYSTEMS=0|1|always` overrides the builder.
- Measured on an M4 Max (`cargo run --release -p ponko_bench --bin
  ponko_bench -- schedule`, see `crates/ponko_bench/SCHEDULE.md`): handing
  a batch to the pool costs 20 to 45 µs; 64 tiny systems take 8 µs with the
  scheduler off or on (49 µs if forced to threads); eight light systems over
  64,000 entities each 81 → 59 µs; eight heavy systems over 16,000 entities
  each 7.15 → 1.04 ms; a mixed simulation with a `Ctx` system 8.45 → 5.71 ms;
  a chain, or one big system next to tiny ones, gains nothing. With the
  scheduler off, games take what they took before it existed (64 tiny
  systems 8 µs, eight systems over 250,000 entities 321 → 323 µs).
- C API: `ponko_game_parallel_systems` and `ponko_game_schedule_plan`; C++
  `game::parallel_systems()` and `game::schedule_plan()`.
- `ponko_ecs`: `ViewGroup`, a checked set of view states that don't overlap,
  whose `run` gives each its view on several threads or in order (if one
  call panics the others still run, then the first panic in order is passed
  on); `ViewState::set_changes_since` gives a view its own change window
  and `ViewState::set_change_tick` its own tick to write in.
  `Commands::clear` forgets recorded commands without applying them.
- With the scheduler on, change ticks are the same numbers as with it off.
  With it off, the scheduler never starts rayon's global thread pool, and a
  `Game::order` pair against the run order panics at the start either way.
  If a system of a batch panics, the batch's other systems finish, the
  batch's commands and events are dropped, and the panic is passed on.
- Guide chapter "Parallel systems": declaring access, turning the scheduler
  on, stages and barriers, the determinism rules, and when it doesn't pay
  off.

### Changed
- `Game::add_view` systems must be `Send`, and event types named by
  `SystemAccess::read_events` / `send_events` (and used through
  `SystemView::events`) must be `Send + Sync`, so they can run in a batch.
