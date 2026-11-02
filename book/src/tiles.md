# Sprite sheets, animation and tilemaps

> **C++:** Sprite sheets, animation and tilemaps are Rust-only for now; the C API grows chapter by chapter.

## Sprite sheets

A sprite sheet is one texture holding many frames of the same size: an
animation strip, a tileset, a set of icons. Frames are numbered left to
right, top to bottom.

```rust
let tex = ctx.load_texture("units.png");
let sheet = SpriteSheet::new(tex, 32, 32);   // 32 × 32 frames
ctx.world.spawn((Transform2D::at(pos), sheet.sprite(5))); // frame 5
```

Under the hood a sprite has a `region`, the part of its texture to show in
texture pixels. `sheet.sprite(i)` is `Sprite::new(tex).with_region(sheet.frame(i))`,
and `ctx.draw_ex` takes the same `region` in `DrawParams`.

For pixel art, turn smoothing off so pixels stay square:

```rust
ctx.set_texture_filter(&tex, TextureFilter::Nearest);
```

## Animation

Add an `Animation` next to the `Sprite`; the built-in `animate_sprites`
system (on by default, all cores) advances it every frame:

```rust
ctx.world.spawn((
    Transform2D::at(pos),
    sheet.sprite(0),
    Animation::new(sheet, 0..8, 12.0), // frames 0-7 at 12 fps, looping
));
let boom = Animation::new(sheet, 8..16, 24.0).once(); // stops on the last frame
```

`animation.finished()` tells you when a `once()` animation is done, so you
can despawn the explosion.

## Tilemaps

A `TileMap` is a grid of sheet frames on an entity with a `Transform2D`
(its position is the map's top-left corner). The built-in `draw_tilemaps`
system draws only the tiles the camera can see, so a map can be huge: the
tiles example has a million tiles, and zoomed out to 380 000 visible tiles it
still runs at over 500 FPS.

```rust
{{#include ../../examples/tiles.rs:setup}}
```

Changing tiles is writing into the map:

```rust
{{#include ../../examples/tiles.rs:paint}}
```

- `TileMap::EMPTY` (or `set(x, y, None)`) leaves a hole.
- `tile_size` defaults to the sheet's frame size; set it to scale tiles.
- Maps are drawn before sprites, so sprites with the same `z` are on top.

`cargo run --release --example tiles`, or `MAP=4000` for 16 million tiles.
