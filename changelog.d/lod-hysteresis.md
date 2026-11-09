### Added
- `LodHysteresis`: an optional component for a `LodGroup` that holds the
  shown level until the group's screen height moves a fraction past the
  level's range, so a camera resting at a threshold doesn't switch levels
  every frame. Each camera keeps its own choice; scene files use
  `"LodHysteresis": 0.1`.
