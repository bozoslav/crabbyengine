A small 2D arena game is running headless and paused, with its control
endpoint on; reach it with the ponko-mcp tools. The player starts at
x = 100 and the finish flag stands at x = 700 (the `Flag` resource notes the
tick the player first reaches it). The arrow keys move the player at the
speed in `assets/settings.json`.

Goal: holding Right from the start, the player must reach the flag within 2
seconds (120 ticks), but not within 1.5 seconds (90 ticks), so the run stays
readable. Change only `assets/settings.json`; the game reloads it while it
runs. Check your result in the game before you finish.
