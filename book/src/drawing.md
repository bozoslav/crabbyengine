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

## Blend modes and sprite shaders

A sprite blends with what's behind it by its alpha. Add a `BlendMode` next to
its `Sprite` (or pass one with `DrawParams::with_blend`) to change that:

| `BlendMode` | What it does | For |
|---|---|---|
| `Alpha` (default) | covers what's behind by the pixel's alpha | most sprites |
| `Additive` | adds the color, weighted by alpha; never darkens | glows, fire, sparks, light |
| `Multiply` | multiplies what's behind by the color; white changes nothing | shadows, tinted glass |
| `Premultiplied` | like `Alpha`, for colors already multiplied by their alpha | premultiplied art and shaders |

For a look of your own, write a **sprite shader**: WGSL with one function,
`fn fragment(p: SpritePixel) -> vec4<f32>`, that returns each pixel's color.
A `SpriteMaterial` puts it together with four `vec4` parameters and two extra
textures; add it next to a `Sprite`, or pass it with
`DrawParams::with_material`. This one burns a sprite away with a glowing edge
(the `sprite_effects` example animates it, next to an additive glow and an
outline):

```wgsl
{{#include ../../assets/shaders/sprite_dissolve.wgsl}}
```

<div class="lang-pair">
<div>

```rust
# use ponko::d2::{BlendMode, SpriteMaterialDesc};
let crab = ctx.load_texture("crab.png");
let noise = ctx.load_texture("noise.png"); // grey: where it burns first
// A glow that brightens what's behind it:
ctx.world.spawn((
    ponko::d2::Transform::at(vec2(400.0, 300.0)),
    ponko::d2::Sprite::new(crab),
    BlendMode::Additive,
));
// A sprite with a shader of its own:
let dissolve = ctx.gfx.load_sprite_shader("shaders/sprite_dissolve.wgsl");
let burn = ctx.gfx.create_sprite_material(
    SpriteMaterialDesc::new(dissolve)
        .with_param(1, vec4(1.0, 0.5, 0.1, 1.0)) // the edge's color
        .with_texture(0, noise),
);
ctx.world.spawn((ponko::d2::Transform::at(vec2(200.0, 300.0)), ponko::d2::Sprite::new(crab), burn));
// Immediate draws take both too:
let glow = DrawParams::default().with_blend(BlendMode::Additive).with_material(burn);
ctx.gfx.draw_ex(crab, vec2(100.0, 100.0), glow);
// Each frame: burn a little further.
if let Some(desc) = ctx.gfx.sprite_material_desc(burn) {
    let t = (ctx.time.elapsed() as f32 * 0.25).fract();
    ctx.gfx.update_sprite_material(burn, desc.with_param(0, vec4(t, 0.0, 0.0, 0.0)));
}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:sprite_materials}}
```

</div>
</div>

What a sprite shader can use:

- `p.color`: what the built-in look would draw (texture × tint), and its
  parts `p.texture` and `p.tint`; `p.uv` (in the texture), `p.local` (0 to 1
  across the sprite) and `p.world` (world units).
- `material.params[0]` to `[3]`: the material's four `vec4<f32>`.
- `sprite_sample(uv)` and `sprite_texel()` (the size of one texel in uv, for
  outlines and blurs), and `material_sample0(uv)` and `material_sample1(uv)`
  for the extra textures (white when the material has none).

It returns a color with straight alpha (premultiplied for
`BlendMode::Premultiplied`) and may `discard`. Every binding is the
engine's, so a sprite shader declares no `@group`/`@binding` and no entry
points; it works with every blend mode.

Loaded sprite shaders hot reload like textures. One that doesn't compile is
an error with the line in your file (`dissolve.wgsl:6:12: error: ...`), also
listed by `ctx.gfx.shader_status()` with kind `ShaderKind::Sprite` (see
[Shader errors and reloads](3d.md#shader-errors-and-reloads)). An edit that
doesn't compile keeps the last good version drawing. Shaders are checked against what every supported
GPU runs, WebGL2 included, so an error shows up on your machine rather than
in a player's browser.

Sprites are batched by texture, blend mode and material together, so many
sprites with one material cost one draw call, and sprites without a blend
mode or material draw as before. Each different material or blend mode in a
row is its own draw call.

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
