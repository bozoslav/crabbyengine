# Sprite sheets, animation and tilemaps

## Sprite sheets

A sprite sheet is one texture holding many frames of the same size: an
animation strip, a tileset, a set of icons. Frames are numbered left to
right, top to bottom.

<div class="lang-pair">
<div>

```rust
# let pos = vec2(100.0, 100.0);
let tex = ctx.load_texture("units.png");
let sheet = SpriteSheet::new(tex, 32, 32);   // 32 × 32 frames
ctx.world.spawn((Transform2D::at(pos), sheet.sprite(5))); // frame 5
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:sheet}}
```

</div>
</div>

Under the hood a sprite has a `region`, the part of its texture to show in
texture pixels. `sheet.sprite(i)` is `Sprite::new(tex).with_region(sheet.frame(i))`,
and `ctx.draw_ex` takes the same `region` in `DrawParams`.

For pixel art, turn smoothing off so pixels stay square:

<div class="lang-pair">
<div>

```rust
# let tex = ctx.load_texture("units.png");
ctx.gfx.set_texture_filter(&tex, TextureFilter::Nearest);
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:nearest}}
```

</div>
</div>

## Animation

Add an `Animation` next to the `Sprite`; the built-in `animate_sprites`
system (on by default, all cores) advances it every frame:

<div class="lang-pair">
<div>

```rust
# let pos = vec2(100.0, 100.0);
# let sheet = SpriteSheet::new(ctx.load_texture("units.png"), 32, 32);
ctx.world.spawn((
    Transform2D::at(pos),
    sheet.sprite(0),
    Animation::new(sheet, 0..8, 12.0), // frames 0-7 at 12 fps, looping
));
let boom = Animation::new(sheet, 8..16, 24.0).once(); // stops on the last frame
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:animation}}
```

</div>
</div>

`animation.finished()` tells you when a `once()` animation is done, so you
can despawn the explosion. In C++, `ponko::animation` is a plain struct, so
an `each` can take `ponko::animation&` like `ponko::transform2d&`.

## Tilemaps

A `TileMap` is a grid of sheet frames on an entity with a `Transform2D`
(its position is the map's top-left corner). The built-in `draw_tilemaps`
system draws only the tiles the camera can see, so a map can be huge: the
tiles example has a million tiles, and zoomed out to 380 000 visible tiles it
still runs at over 500 FPS.

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/tiles.rs:setup}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:tilemap}}
```

</div>
</div>

Changing tiles is writing into the map. In C++, `world.tilemap(e)` gives the
engine's tiles directly (`set`, `get`, `tiles()` for bulk writes), with no copy:

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/tiles.rs:paint}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:paint}}
```

</div>
</div>

- `TileMap::EMPTY` (or `set(x, y, None)`) leaves a hole.
- `tile_size` defaults to the sheet's frame size; set it to scale tiles.
- Maps are drawn before sprites, so sprites with the same `z` are on top.

`cargo run --release --example tiles`, or `MAP=4000` for 16 million tiles.
