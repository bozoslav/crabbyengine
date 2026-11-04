// Screen effects (Ctx::create_effect): a game's `fn effect(p: Pixel) ->
// vec3<f32>` run over the lit 3D frame, before bloom and tone mapping.
// The engine builds each effect from the mesh prelude (frame, material and
// scene bindings and helpers), this file's first part, the game's
// shader and this file's second part (`effect_main.wgsl`).

// A pixel of the lit frame, as an effect gets it.
struct Pixel {
    color: vec3<f32>,   // the light here so far: HDR, before bloom and tone mapping
    world: vec3<f32>,   // the nearest solid surface here, in the world
    distance: f32,      // from the camera to `world`
    sky: bool,          // nothing solid here, only sky or background
    screen: vec2<f32>,  // the pixel's center, in pixels from the top left
    uv: vec2<f32>,      // 0 to 1 across the screen, from the top left
};
