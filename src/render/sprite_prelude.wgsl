// The engine's code before a sprite shader's `fragment` hook: bindings,
// the hook's input and helpers. Group 0 is the camera, group 1 the sprite's
// own texture, group 2 the material.

struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

// `SpriteMaterialDesc::params`.
struct SpriteMaterial {
    params: array<vec4<f32>, 4>,
};

@group(2) @binding(0) var<uniform> material: SpriteMaterial;
@group(2) @binding(1) var material_texture0: texture_2d<f32>;
@group(2) @binding(2) var material_texture1: texture_2d<f32>;
@group(2) @binding(3) var material_sampler: sampler;

// One pixel of a sprite, as the `fragment` hook sees it. Colors are linear,
// with straight (not premultiplied) alpha.
struct SpritePixel {
    // What the built-in shader draws: `texture * tint`.
    color: vec4<f32>,
    // The sprite's texture at `uv`.
    texture: vec4<f32>,
    // The sprite's tint (`Sprite::color`, `DrawParams::color`).
    tint: vec4<f32>,
    // Where in the sprite's texture this pixel is (0..1 over the texture).
    uv: vec2<f32>,
    // Where in the sprite this pixel is: (0, 0) top-left, (1, 1) bottom-right.
    local: vec2<f32>,
    // Where this pixel is in world units (window pixels for screen draws).
    world: vec2<f32>,
};

// The sprite's texture at `uv`; works anywhere, also in branches and loops.
fn sprite_sample(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(sprite_texture, sprite_sampler, uv, 0.0);
}

// The size of one texel of the sprite's texture, in `uv` units.
fn sprite_texel() -> vec2<f32> {
    return 1.0 / vec2<f32>(textureDimensions(sprite_texture));
}

// The material's first extra texture at `uv` (white if it has none).
fn material_sample0(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(material_texture0, material_sampler, uv, 0.0);
}

// The material's second extra texture at `uv` (white if it has none).
fn material_sample1(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(material_texture1, material_sampler, uv, 0.0);
}
