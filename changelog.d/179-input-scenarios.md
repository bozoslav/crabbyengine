### Added
- Input scenarios grow checks and reports: `InputScenario::expect` with
  `ScenarioCheck::{action, resource, count}` evaluates after a tick's systems,
  and a `ScenarioReport` resource says how the run went. A failed check ends the
  run and names its step and tick, with the expected and actual values.
  `Game::run_headless_scenario` is the bounded headless runner, and
  `ScenarioReport::assert_passed` panics with the failing step for tests.
- A small versioned JSON scenario format (`InputScenario::from_json`,
  `from_value`, `to_json`) that refuses unknown fields, newer versions and
  oversized files.
- `Input::start_scenario`, `Input::cancel_scenario` and
  `Input::scenario_report` (on `ctx.input`; they take the world and the clock,
  `ctx.input.start_scenario(&mut ctx.world, &mut ctx.time, scenario)`). Cancelling, replacing or failing a scenario
  releases the keys, buttons and axes it still held.
- Control endpoint methods `input.scenario.load`, `input.scenario.status` and
  `input.scenario.cancel`, and the MCP tools `run_input_scenario` and
  `cancel_input_scenario`. Tools can only load scenarios when the game allows
  it (`Game::input_injection(true)` or `PONKO_INPUT_INJECTION=1`; the MCP
  `launch` tool sets it). A scenario loaded with `bind_connection` is cancelled
  when its client disconnects. A tool's scenario releases the keys it holds
  when it ends unless `keep_input_when_done` is set (the kept input stays owned
  and a cancel or the owner's disconnect releases it); `max_ticks` is checked
  before a running scenario is touched, and `queue_ticks` makes a failed check or
  a cancel take back only the steps the scenario queued itself.

### Changed
- `InputScenario` now lives in its own module; the public path
  (`ponko::InputScenario`) and its `new` and `at` methods are unchanged.
