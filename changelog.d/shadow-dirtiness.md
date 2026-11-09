### Changed
- Shadow maps that would be drawn exactly as they were are kept instead:
  sun cascades and point and spot light views whose casters, light, box and
  shadow settings didn't change since they were drawn. A still scene with a
  still camera draws no shadow casters; a moving model redraws only the maps
  that see it, and a light atlas tile is cleared and drawn alone. Shadows look
  the same as drawing every map every frame. Views drawn from skinned or
  morphing models, buffer-driven meshes or shaders that move vertices
  themselves are still drawn every frame.
