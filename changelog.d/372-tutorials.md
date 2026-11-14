### Added
- Two step-by-step tutorials in the guide, each building a small game from
  `ponko new`: "Tutorial: a first 2D game" (Crab Cove: sprites, input
  actions, a Tiled level with solid tiles, sensors and collision events,
  sound, a retained-UI title screen, the best score in `ctx.storage`,
  headless tests, `ponko package` and a web build) and "Tutorial: a first
  3D game" (Lantern Yard: a glTF scene drawn and solid, a character
  controller, a follow camera, the sun and point lights, physics bodies and
  sensors, a retained-UI HUD, headless tests, packaging). Every step's code
  is pulled from the `tutorial_2d` and `tutorial_3d` examples, which
  `tests/tutorial_2d.rs` and `tests/tutorial_3d.rs` play headlessly; both
  build for the web. Rust only so far.
