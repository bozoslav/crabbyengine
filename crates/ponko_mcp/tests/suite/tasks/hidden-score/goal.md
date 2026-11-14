A small 2D arena game is running headless and paused, with its control
endpoint on; reach it with the ponko-mcp tools. It runs whole frames
headless, retained UI included, and its HUD has a score label (the UI node
`hud.score`) styled by `assets/hud.json`, which the game reloads while it
runs.

Bug report: "I can't see my score."

Goal: the HUD must always show the score as `Score: N`, with N the current
value of the `Score` resource's `points`. Fix `assets/hud.json`, and check
what the player sees in the running game (input scenarios can check UI
nodes with `expect_ui`).
