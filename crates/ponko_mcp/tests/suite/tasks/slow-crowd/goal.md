A crowd simulation is running headless and paused, with its control
endpoint on; reach it with the ponko-mcp tools. Each tick it counts the
pairs of villagers that stand closer together than a radius (the `Crowding`
resource has the result of the last tick).

Report: "after the last change to the game's data files, every tick got much
slower."

Goal: find which system got slow and why, and cut the work it does per tick
at least tenfold without changing what it computes (the same pairs, with the
same radius). Change only the game's files under `assets/`; the game
reloads them while it runs.
