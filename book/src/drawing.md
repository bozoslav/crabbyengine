# Drawing

There are two ways to put things on screen. Both end up in the same batched
renderer.

## Sprites as components

Give an entity a `Transform2D` and a `Sprite`, and the built-in
`draw_sprites` system draws it every frame:

<div class="lang-pair">
<div>

```rust
let crab = ctx.load_texture("crab.png");
ctx.world.spawn((
    ponko::d2::Transform::at(vec2(400.0, 300.0)).rotated(0.3).scaled(vec2(2.0, 2.0)),
    ponko::d2::Sprite::new(crab).with_color(Color::RED).with_z(1.0),
));
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:sprites}}
```

</div>
</div>

- `Transform2D`: `position`, `rotation` (radians, clockwise on screen),
  `scale` (negative flips).
- `Sprite`: `texture`, `color` (tint), `z` (higher is on top), `pivot`
  (`(0.5, 0.5)` is the center).

## Immediate drawing

For UI, effects and quick prototypes, draw directly every frame:

<div class="lang-pair">
<div>

```rust
# let crab = ctx.load_texture("crab.png");
ctx.gfx.clear(Color::hex(0x203040));
ctx.draw(crab, vec2(100.0, 100.0));
let params = DrawParams::default()
    .with_rotation(0.5)
    .with_scale(vec2(2.0, 2.0))
    .with_color(Color::RED);
ctx.gfx.draw_ex(crab, vec2(200.0, 100.0), params);
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:immediate}}
```

</div>
</div>

Coordinates are world units, which are window pixels (`(0, 0)` at the
top-left) until you move the camera; see [Camera, text and UI](camera-ui.md).

`DrawParams` (and `TextParams` for text) start from `default()` and change
what they need with `with_*` methods (`with_scale`, `with_rotation`,
`with_color`, `with_z`, `with_pivot`, `with_screen`, `with_region`). They are
`#[non_exhaustive]`, so they can't be written as struct literals.

## Textures

- `ctx.load_texture("path.png")` loads an image file. PNG and JPEG load
  in every default build, with no `3d` feature (see [Image formats](#image-formats)).
- `ctx.gfx.create_texture(w, h, &rgba)` makes one from pixels.

- `ctx.gfx.update_texture(tex, &rgba)` replaces a texture's pixels, for
  images that change while the game runs (a minimap, a heat map, a
  simulated field). The ants example draws its pheromone field this way
  every frame.

`Texture` is a small `Copy` handle; store it in components freely.

```cpp
{{#include ../../examples/cpp/guide.cpp:pixels}}
```

In C++, `Sprite` values are `ponko::sprite` (`sprite_of(texture, tint, z)`).
They can be spawned, read and set, but not used as query columns, because
the engine stores sprites in a Rust layout. `Transform2D` can.

## Hot reload

Save an image in your editor and the running game shows the new version
within a quarter of a second: textures loaded with `load_texture` are
reloaded when their file changes, under the same handle, so every sprite
using them updates by itself. Fonts (`load_font`) and sounds (`load_sound`)
reload the same way, and so do [data files](data-files.md). A file that
can't be read or decoded mid-save keeps the old version. It's on by default
in debug builds and off in release builds; `Game::hot_reload(bool)` or
`PONKO_HOT_RELOAD=1`/`0` changes that.

If you computed something from an image (a level read from its pixels, a
collision mask), redo it when the texture shows up in `reloaded_textures`:

<div class="lang-pair">
<div>

```rust
# struct Level(Texture);
Game::new("Art test", 1280, 720)
    .hot_reload(true) // the default in debug builds
    .system(|ctx: &mut Ctx| {
        let level = ctx.world.resource::<Level>().0;
        if ctx.assets.reloaded_textures().contains(&level) {
            println!("level.png changed: rebuilding the map");
        }
    })
    .setup(|ctx| {
        let level = ctx.load_texture("level.png");
        ctx.world.insert_resource(Level(level));
    })
    .run();
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:hot_reload}}
```

</div>
</div>

An image that changed size isn't reloaded, because sprites keep the size
their texture had; the console says so, and a restart picks it up. A file
caught halfway through saving fails to decode and is tried again on the
next save. Checking costs one file `stat` per loaded texture, four times a
second.

## Screenshots

<div class="lang-pair">
<div>

```rust
if ctx.key_pressed(Key::F12) {
    ctx.gfx.save_screenshot("shot.png");
}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:screenshot}}
```

</div>
</div>

### Image formats

The format of a file is read from its first bytes, so a JPEG saved as
`background.png` still loads. PNG is always built in. The rest are cargo
features, none of them tied to `3d`:

| Feature | Format | On by default |
|---|---|---|
| (always) | PNG | yes |
| `image-jpeg` | JPEG | yes |
| `image-webp` | WebP | |
| `image-bmp` | BMP | |
| `image-tga` | TGA (found from the `.tga` extension) | |
| `image-gif` | GIF (first frame) | |
| `image-tiff` | TIFF | |
| `image-ico` | ICO | |
| `image-qoi` | QOI | |
| `image-pnm` | PNM | |
| `image-dds` | DDS (uncompressed) | |
| `image-hdr`, `image-exr` | Radiance HDR, OpenEXR (read as 8-bit color) | |
| `image-formats` | all of the above | |

```toml
# Cargo.toml: WebP too, and no JPEG decoder in the build
ponko = { version = "0.1", default-features = false, features = ["window", "audio", "image-webp"] }
```

A file in a format the build leaves out fails with an `Unsupported` error
(a panic from `load_texture`) that names the feature to turn on, for
example: `"tile.webp" is a WebP image, but this build of ponko doesn't read
WebP: turn on its `image-webp` cargo feature`. Builds without the default
features, such as a headless server, carry no JPEG decoder. The `3d` feature
turns on `image-jpeg`, `image-hdr` and `image-exr` for glTF textures and
environment maps. The C library builds with the engine's defaults; choosing
image formats from C is not exposed yet.

`save_screenshot` saves the next frame as a PNG once it
has been drawn, even while the window is hidden. Headless runs have no
frames, so it does nothing there.
