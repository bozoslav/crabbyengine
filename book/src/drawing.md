# Drawing

There are two ways to put things on screen. Both end up in the same batched
renderer.

## Sprites as components

Give an entity a `Transform2D` and a `Sprite`, and the built-in
`draw_sprites` system draws it every frame:

```rust
let crab = ctx.load_texture("crab.png");
ctx.world.spawn((
    Transform2D::at(vec2(400.0, 300.0)).rotated(0.3).scaled(vec2(2.0, 2.0)),
    Sprite::new(crab).with_color(Color::RED).with_z(1.0),
));
```

- `Transform2D`: `position`, `rotation` (radians, clockwise on screen),
  `scale` (negative flips).
- `Sprite`: `texture`, `color` (tint), `z` (higher is on top), `pivot`
  (`(0.5, 0.5)` is the center).

## Immediate drawing

For UI, effects and quick prototypes, draw directly every frame:

```rust
ctx.clear(Color::hex(0x203040));
ctx.draw(&crab, vec2(100.0, 100.0));
ctx.draw_ex(&crab, vec2(200.0, 100.0), DrawParams {
    rotation: 0.5,
    scale: vec2(2.0, 2.0),
    color: Color::RED,
    ..Default::default()
});
```

Coordinates are window pixels, `(0, 0)` at the top-left.

## Textures

- `ctx.load_texture("path.png")` loads an image file.
- `ctx.create_texture(w, h, &rgba)` makes one from pixels.

- `ctx.update_texture(&tex, &rgba)` replaces a texture's pixels, for
  images that change while the game runs (a minimap, a heat map, a
  simulated field). The ants example draws its pheromone field this way
  every frame.

`Texture` is a small `Copy` handle; store it in components freely.

## Screenshots

`ctx.save_screenshot("shot.png")` saves the next frame as a PNG once it
has been drawn, even while the window is hidden. Headless runs have no
frames, so it does nothing there.
