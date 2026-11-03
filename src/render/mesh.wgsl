// Instanced 3D meshes: one instance = one Model, lit by a sun (Cook-Torrance
// GGX, metallic-roughness, as glTF) and sky/ground ambient light.
//
// The engine builds each material shader from three parts: this prelude,
// the game's shader (its `vertex` and `surface` functions, when it has
// them) and `mesh_main.wgsl`, which calls them.

const PI: f32 = 3.14159265;

struct Frame {
    view_proj: mat4x4<f32>,
    sun_view_proj: mat4x4<f32>, // world to the shadow map's clip space
    camera: vec4<f32>,     // xyz: camera position
    sun_dir: vec4<f32>,    // xyz: unit vector towards the sun
    sun_color: vec4<f32>,  // rgb: linear color times intensity
    sky: vec4<f32>,        // rgb: linear, times ambient intensity
    ground: vec4<f32>,
    shadow: vec4<f32>,     // on, texel size in uv, normal offset, fade distance
    time: vec4<f32>,       // x: seconds since the game started, y: seconds since the last frame
    inv_view_proj: mat4x4<f32>, // clip space back to the world
    screen: vec4<f32>,     // width and height in pixels, then their inverses
};

struct MaterialData {
    color: vec4<f32>,      // linear RGBA
    emissive: vec4<f32>,   // linear RGB
    params: vec4<f32>,     // roughness, metallic, normal map strength
    maps: vec4<u32>,       // has: normal map, ORM map, emissive map
    custom: array<vec4<f32>, 4>,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var shadow_map: texture_depth_2d;
@group(0) @binding(2) var shadow_sampler: sampler_comparison;
@group(1) @binding(0) var<uniform> material: MaterialData;
@group(1) @binding(1) var material_sampler: sampler;
@group(1) @binding(2) var base_texture: texture_2d<f32>;
@group(1) @binding(3) var normal_texture: texture_2d<f32>;
@group(1) @binding(4) var orm_texture: texture_2d<f32>;
@group(1) @binding(5) var emissive_texture: texture_2d<f32>;
@group(1) @binding(6) var custom_texture0: texture_2d<f32>;
@group(1) @binding(7) var custom_texture1: texture_2d<f32>;
// The opaque scene behind a transparent surface (see scene_color).
@group(2) @binding(0) var scene_color_texture: texture_2d<f32>;
@group(2) @binding(1) var scene_depth_texture: texture_depth_multisampled_2d;

// A vertex in world space, as a shader's `vertex` function gets and returns it.
struct Vertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
    color: vec4<f32>, // the model's tint
};

// A pixel's surface, as a shader's `surface` function gets and returns it:
// filled in from the material and its textures first.
struct Surface {
    albedo: vec3<f32>,
    alpha: f32,
    normal: vec3<f32>,   // world space, unit length
    roughness: f32,
    metallic: f32,
    occlusion: f32,      // 1: fully lit by ambient light, 0: none
    emissive: vec3<f32>,
    // Read-only inputs.
    world: vec3<f32>,
    uv: vec2<f32>,
    color: vec4<f32>,    // the model's tint
    view: vec3<f32>,     // unit vector from the surface to the camera
    screen: vec2<f32>,   // the pixel, in pixels from the top left
};

// The opaque scene's light at a pixel, before tone mapping. Only
// transparent materials see the scene; elsewhere this is black.
fn scene_color(screen: vec2<f32>) -> vec3<f32> {
    let size = vec2<i32>(textureDimensions(scene_color_texture));
    let at = clamp(vec2<i32>(screen), vec2<i32>(0), size - 1);
    return textureLoad(scene_color_texture, at, 0).rgb;
}

// Where the opaque scene is at a pixel, in the world. Only transparent
// materials see the scene; elsewhere this is as far as the camera sees.
fn scene_position(screen: vec2<f32>) -> vec3<f32> {
    let size = vec2<i32>(textureDimensions(scene_depth_texture));
    let at = clamp(vec2<i32>(screen), vec2<i32>(0), size - 1);
    let depth = textureLoad(scene_depth_texture, at, 0);
    let ndc = vec2<f32>(screen.x * frame.screen.z * 2.0 - 1.0, 1.0 - screen.y * frame.screen.w * 2.0);
    let world = frame.inv_view_proj * vec4<f32>(ndc, depth, 1.0);
    return world.xyz / world.w;
}

// How far the opaque scene is from the camera at a pixel.
fn scene_distance(screen: vec2<f32>) -> f32 {
    return distance(scene_position(screen), frame.camera.xyz);
}

// A pseudo-random number in 0..1 for a 2D point.
fn hash2(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 456.21));
    let r = q + dot(q, q + 45.32);
    return fract(r.x * r.y);
}

// Smooth value noise in 0..1.
fn noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Five octaves of noise2, in 0..1.
fn fbm2(p: vec2<f32>) -> f32 {
    var sum = 0.0;
    var amplitude = 0.5;
    var q = p;
    for (var i = 0; i < 5; i++) {
        sum += noise2(q) * amplitude;
        q = q * 2.03 + vec2<f32>(17.0, 31.0);
        amplitude *= 0.5;
    }
    return sum / 0.96875;
}
