A small 2D game is running headless and paused, with its control endpoint
on; reach it with the ponko-mcp tools. A hazard deals 30 damage per second
to every enemy, less their armor; an enemy at zero health dies and sends a
`Died` event. Waves of enemies are spawned from `assets/enemies.json`, which
the game reloads while it runs; setting the `Waves` resource's `wave` to
true spawns a fresh wave.

Bug report: "every enemy should be dead within 5 seconds of a wave starting,
but one never dies."

Goal: reproduce it, find the cause, and fix the bad value in
`assets/enemies.json` and in the running game. Keep every enemy's name and
health as they are, and don't kill or remove enemies by hand.
