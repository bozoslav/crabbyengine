### Added
- An agent acceptance suite for the MCP tools: six game-development tasks
  (tuning, building a level, a seeded bug, a tick-time regression, a broken
  shader, a hidden HUD label), each with a starting project, a goal, a
  checker and negative controls. `cargo test -p ponko_mcp --test
  agent_suite` replays scripted solutions with no model and compares the
  scores with a published baseline; `cargo run -p ponko_mcp --example
  agent_suite -- mcp TASK` serves one task to a real agent over MCP, with a
  `submit_task` tool and a result file. See "The agent acceptance suite" in
  the guide's agents chapter.
