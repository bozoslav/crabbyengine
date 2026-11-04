// Compute passes (Ctx::create_compute): a game's `fn compute(c: Cell) ->
// vec4<f32>` run once per texel of a render target, each frame, reading
// the target as it was. The engine builds each one from the mesh prelude
// (frame and material bindings and helpers), this file's first part, the
// game's shader and this file's second part (`compute_main.wgsl`).

// One texel of the target, as `compute` gets it.
struct Cell {
    value: vec4<f32>,    // what it holds now (all 0 at first)
    pixel: vec2<i32>,    // which texel, from the top left
    size: vec2<i32>,     // the target's size in texels
    uv: vec2<f32>,       // the texel's center, 0 to 1 across the target
};

@group(3) @binding(0) var state_in: texture_2d<f32>;
@group(3) @binding(1) var state_out: texture_storage_2d<rgba16float, write>;

// What texel `pixel` holds now, clamped to the edges (for neighbours).
fn cell_at(pixel: vec2<i32>) -> vec4<f32> {
    let size = vec2<i32>(textureDimensions(state_in));
    return textureLoad(state_in, clamp(pixel, vec2<i32>(0), size - 1), 0);
}
