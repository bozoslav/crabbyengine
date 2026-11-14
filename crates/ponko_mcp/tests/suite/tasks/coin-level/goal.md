A small 2D arena game is running headless and paused, with its control
endpoint on; reach it with the ponko-mcp tools. The player (`Player`,
`Transform2D`) starts at (100, 300) and the arrow keys move it at 300 pixels
per second. Touching a coin (`Coin { value }`) collects it and adds its value
to the `Score` resource.

Goal: make a level and save it as the scene `levels/coins.json` (in the
game's `assets/` folder). It has the player at (100, 300) and exactly five
coins worth 1 each, placed so that holding Right from the start collects all
five within 2 seconds (120 ticks). Build it in the running game with the
tools; don't write the scene file by hand.
