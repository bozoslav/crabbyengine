# Sample assets

The files the examples, tests and guide load (`ctx.load_texture("crab.png")`
reads `assets/crab.png`). The examples and these assets ship in the `ponko`
crate; `branding/` does not.

| File | What it is | Where it comes from |
|---|---|---|
| `crab.png` | The sprite of the examples and the guide | Added with the first sprite renderer (#3); no separate provenance note, see git history |
| `models/house.glb` | A small house, walls and a roof with two materials | Added as the sample model of the first 3D support (#51); no separate provenance note, see git history |
| `coin_dash/coin.png` | The coin of the `coin_dash` example | Drawn by a script for that example; see `examples/coin_dash/CONTENT-NOTICES.txt` |
| `levels/beach.json`, `prefabs/*.json` | A scene and prefabs for the scenes chapter | Written for the engine (#50) |
| `levels/meadow.tmx`, `levels/meadow.tsx`, `levels/meadow.png` | A Tiled map, its tileset and the tileset's image, for the `level` example and the tilemaps chapter | Written for the engine (#381); the image drawn by a script |
| `shaders/*.wgsl` | Custom material shaders for the 3D examples | Written for the engine (#54) |

Files written for the engine are under the repository's license (MIT or
Apache-2.0, at your option; see `LICENSE-MIT`, `LICENSE-APACHE` and `NOTICE`
at the root).

The logo in `branding/` has its own note in `branding/README.md`.
