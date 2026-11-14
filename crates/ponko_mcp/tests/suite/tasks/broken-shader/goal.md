A small 2D game is running headless and paused, with its control endpoint
on and hot reload on; reach it with the ponko-mcp tools. It loads the sprite
shader `assets/shaders/glow.wgsl`, which makes the player glow by the
material's first parameter, but the shader doesn't compile, so the player is
drawn with a stand-in.

Goal: find out what's wrong and fix `assets/shaders/glow.wgsl` so it
compiles in the running game and still glows by that parameter. The game
reloads the file when it changes.
