### Added
- `ctx.physics.set_paused(bool)` and `ctx.physics.is_paused()` hold only the
  2D physics step: bodies stay where they are, no collision events arrive,
  and every other simulation system and the tick counter keep running, so a
  pause screen's own sim systems can read the press that resumes. C:
  `ponko_ctx_set_physics_paused` and `ponko_ctx_is_physics_paused`; C++:
  `ctx.set_physics_paused(bool)` and `ctx.is_physics_paused()`. The guide's
  "Pausing only physics" section shows the pattern.

### Changed
- Headless runs without a control endpoint honour `ctx.time.pause()`: a tick
  made while paused runs no simulation system and doesn't advance
  `sim_tick()`, frame systems still run with `headless_frames`, and
  `ctx.time.step()` runs one tick. Before, they ignored the pause. With the
  control endpoint on, a paused headless game waits for tools, as before.
- Ember Hop holds the built-in physics step with `ctx.physics.set_paused`
  instead of replacing it with its own system.
