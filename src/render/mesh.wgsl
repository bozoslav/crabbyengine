// Instanced 3D meshes: one instance = one Model, lit by a sun (Cook-Torrance
// GGX, metallic-roughness, as glTF) and sky/ground ambient light.
//
// The engine builds each material shader from three parts: this prelude,
// the game's shader (its `vertex` and `surface` functions, when it has
// them) and `mesh_main.wgsl`, which calls them.

const PI: f32 = 3.14159265;

struct Frame {
    view_proj: mat4x4<f32>,
    // World to each shadow cascade's clip space (see `sunlight`).
    sun_view_proj: array<mat4x4<f32>, 4>,
    // In the shadow pass: the cascade being drawn.
    caster_view_proj: mat4x4<f32>,
    camera: vec4<f32>,     // xyz: camera position; w: 0 in shadow maps, else 1
    sun_dir: vec4<f32>,    // xyz: unit vector towards the sun
    sun_color: vec4<f32>,  // rgb: linear color times intensity
    sky: vec4<f32>,        // rgb: linear, times ambient intensity
    ground: vec4<f32>,
    shadow: vec4<f32>,     // cascades (0: no shadows), texel size in uv, normal offset in texels, fade distance
    cascade_texel: vec4<f32>, // each cascade's texel size, in world units
    time: vec4<f32>,       // x: seconds since the game started, y: seconds since the last frame
    inv_view_proj: mat4x4<f32>, // clip space back to the world
    screen: vec4<f32>,     // width and height in pixels, then their inverses
    atmosphere: vec4<f32>, // x: a physical sky (1) or not (0), y: haze, z: cos of the sun's radius, w: stars
    sky_light: vec4<f32>,  // rgb: the sunlight the sky is lit by, w: camera height in km
    fog: vec4<f32>,        // x: density per unit, y: fog height, z: 1 / falloff
    fog_color: vec4<f32>,  // rgb: linear, without an atmosphere
    night: vec4<f32>,      // rgb: light at night
    fog_far: vec4<f32>,    // x: distance fog start, y: its end (0: none), z: sun glow, w: sun focus
    cluster_depth: vec4<f32>,
    cluster_view_proj: mat4x4<f32>,
};

struct MaterialData {
    color: vec4<f32>,      // linear RGBA
    emissive: vec4<f32>,   // linear RGB
    params: vec4<f32>,     // roughness, metallic, normal map strength
    maps: vec4<u32>,       // has: normal map, ORM map, emissive map
    custom: array<vec4<f32>, 4>,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var shadow_map: texture_depth_2d_array;
@group(0) @binding(2) var shadow_sampler: sampler_comparison;
// The sky around the camera (see sky_radiance), made each frame.
@group(0) @binding(3) var sky_texture: texture_2d<f32>;
@group(0) @binding(4) var sky_sampler: sampler;
@group(0) @binding(5) var punctual_records: texture_2d<f32>;
@group(0) @binding(6) var punctual_clusters: texture_2d<u32>;
@group(1) @binding(0) var<uniform> material: MaterialData;
@group(1) @binding(1) var material_sampler: sampler;
@group(1) @binding(2) var base_texture: texture_2d<f32>;
@group(1) @binding(3) var normal_texture: texture_2d<f32>;
@group(1) @binding(4) var orm_texture: texture_2d<f32>;
@group(1) @binding(5) var emissive_texture: texture_2d<f32>;
@group(1) @binding(6) var custom_texture0: texture_2d<f32>;
@group(1) @binding(7) var custom_texture1: texture_2d<f32>;
@group(1) @binding(8) var custom_array0: texture_2d_array<f32>;
@group(1) @binding(9) var custom_array1: texture_2d_array<f32>;
@group(1) @binding(10) var custom_array2: texture_2d_array<f32>;
// The opaque scene behind a transparent surface (see scene_color).
@group(2) @binding(0) var scene_color_texture: texture_2d<f32>;
@group(2) @binding(1) var scene_depth_texture: texture_depth_multisampled_2d;

// A vertex in world space, as a shader's `vertex` function gets and returns it.
struct Vertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
    color: vec4<f32>, // the model's tint times the vertex color
    uv1: vec2<f32>,   // the mesh's second texture coordinates (zero without)
    origin: vec3<f32>, // read-only: where the model is (its transform's position)
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
    uv1: vec2<f32>,      // the mesh's second texture coordinates (zero without)
    origin: vec3<f32>,   // where the model is (its transform's position)
};

// Whether this draw is into a shadow map (seen from the sun), not for a camera.
fn drawing_shadows() -> bool {
    return frame.camera.w == 0.0;
}

// The unit direction from `p` towards whoever is looking: the camera, or
// the sun when drawing shadow maps. Billboards face it in their `vertex`
// function, so their shadows are cast by the side the sun sees.
fn toward_viewer(p: vec3<f32>) -> vec3<f32> {
    if drawing_shadows() {
        return frame.sun_dir.xyz;
    }
    return normalize(frame.camera.xyz - p);
}

// The opaque scene's light at a pixel, before tone mapping (in screen
// effects: the frame so far). Only transparent materials and screen effects
// see the scene; elsewhere this is black.
fn scene_color(screen: vec2<f32>) -> vec3<f32> {
    let size = vec2<i32>(textureDimensions(scene_color_texture));
    let at = clamp(vec2<i32>(screen), vec2<i32>(0), size - 1);
    return textureLoad(scene_color_texture, at, 0).rgb;
}

// Where the opaque scene is at a pixel, in the world. Only transparent
// materials and screen effects see the scene; elsewhere this is as far as
// the camera sees.
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

// Where direction `dir` is in the sky texture: around the horizon in u,
// from straight up (v 0) to straight down (v 1), squeezed towards the
// horizon where the sky changes fastest.
fn sky_uv(dir: vec3<f32>) -> vec2<f32> {
    let around = atan2(dir.z, dir.x) / (2.0 * PI) + 0.5;
    let up = asin(clamp(dir.y, -1.0, 1.0)) / (0.5 * PI);
    return vec2<f32>(around, 0.5 - 0.5 * sign(up) * sqrt(abs(up)));
}

// The direction at `uv` of the sky texture (the inverse of sky_uv).
fn sky_dir(uv: vec2<f32>) -> vec3<f32> {
    let around = (uv.x - 0.5) * 2.0 * PI;
    let s = (0.5 - uv.y) * 2.0;
    let up = sign(s) * s * s * 0.5 * PI;
    return vec3<f32>(cos(up) * cos(around), sin(up), cos(up) * sin(around));
}

// The sky's light from direction `dir` (towards the sky), without the sun
// itself. Black without an atmosphere.
fn sky_radiance(dir: vec3<f32>) -> vec3<f32> {
    return textureSampleLevel(sky_texture, sky_sampler, sky_uv(dir), 0.0).rgb;
}

// `color` seen through the fog between the camera and `world`: height fog,
// thickest at the fog height and thinning out above it, and distance fog,
// none before its start and all fog past its end. Both take the sky's color
// behind them (or the fog color, without an atmosphere), and glow toward the
// sun. The sky itself takes only the height fog (`distance` false).
fn fog_along(color: vec3<f32>, world: vec3<f32>, distance: bool) -> vec3<f32> {
    let density = frame.fog.x;
    let far = select(0.0, frame.fog_far.y, distance);
    if density <= 0.0 && far <= 0.0 {
        return color;
    }
    let ray = world - frame.camera.xyz;
    let span = length(ray);
    let k = frame.fog.z;
    // The fog along the ray: density * exp(-(height - fog height) * k),
    // integrated from the camera to `world`.
    let start = density * exp(-(frame.camera.y - frame.fog.y) * k);
    let rise = ray.y * k;
    let along = select((1.0 - exp(-rise)) / rise, 1.0, abs(rise) < 1e-4);
    var seen = exp(-start * span * along);
    if far > 0.0 {
        let near = min(frame.fog_far.x, far - 1e-3);
        seen *= clamp((far - span) / (far - near), 0.0, 1.0);
    }
    let dir = ray / max(span, 1e-6);
    var fog_color = frame.fog_color.rgb;
    if frame.atmosphere.x > 0.0 {
        fog_color = sky_radiance(normalize(vec3<f32>(dir.x, max(dir.y, 0.03), dir.z)));
    }
    // Light scattered forward out of the sun (Henyey-Greenstein, 1 at focus 0).
    let g = frame.fog_far.w;
    let mu = dot(dir, frame.sun_dir.xyz);
    let phase = (1.0 - g * g) / pow(max(1.0 + g * g - 2.0 * g * mu, 1e-4), 1.5);
    fog_color += frame.sun_color.rgb * phase * frame.fog_far.z;
    return mix(fog_color, color, seen);
}

// `color` seen through all the fog between the camera and `world`.
fn fog(color: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    return fog_along(color, world, true);
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

fn direct_light(s: Surface, l: vec3<f32>, radiance: vec3<f32>) -> vec3<f32> {
    let n = s.normal;
    let v = s.view;
    let h = (l + v) / max(length(l + v), 1e-6);
    let n_l = max(dot(n, l), 0.0);
    let n_v = max(dot(n, v), 1e-4);
    let n_h = max(dot(n, h), 0.0);
    let v_h = max(dot(v, h), 0.0);

    let a = s.roughness * s.roughness;
    let a2 = a * a;
    let d_denom = n_h * n_h * (a2 - 1.0) + 1.0;
    let d = a2 / (PI * d_denom * d_denom);
    let k = (s.roughness + 1.0) * (s.roughness + 1.0) / 8.0;
    let g = n_v / (n_v * (1.0 - k) + k) * n_l / (n_l * (1.0 - k) + k);
    let f0 = mix(vec3<f32>(0.04), s.albedo, s.metallic);
    let f = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - v_h, 5.0);
    let specular = d * g * f / max(4.0 * n_v * n_l, 1e-4);
    let diffuse = (vec3<f32>(1.0) - f) * (1.0 - s.metallic) * s.albedo / PI;
    return (diffuse + specular) * radiance * n_l;
}

// Evaluate only the lights in the world position's froxel, using the same
// GGX BRDF as sunlight. Available to custom material shaders too.
fn punctual_light(s: Surface) -> vec3<f32> {
    let clip = frame.cluster_view_proj * vec4<f32>(s.world, 1.0);
    if clip.w <= 0.0 || frame.cluster_depth.x <= 0.0 { return vec3<f32>(0.0); }
    let xy = vec2<u32>(clamp(floor((clip.xy / clip.w * 0.5 + 0.5) * vec2<f32>(16.0, 9.0)), vec2<f32>(0.0), vec2<f32>(15.0, 8.0)));
    let z = u32(clamp(floor(log(max(clip.w, frame.cluster_depth.x) / frame.cluster_depth.x) * frame.cluster_depth.y), 0.0, 23.0));
    let cell = (z * 9u + xy.y) * 16u + xy.x;
    let offset = cluster_word(cell * 2u);
    let count = cluster_word(cell * 2u + 1u);
    var sum = vec3<f32>(0.0);
    for (var i = 0u; i < count; i++) {
        let id = i32(cluster_word(offset + i));
        let pr = textureLoad(punctual_records, vec2<i32>(0, id), 0);
        let delta = pr.xyz - s.world;
        let d2 = dot(delta, delta);
        if d2 >= pr.w * pr.w { continue; }
        let l = delta / max(sqrt(d2), 1e-6);
        let cone = textureLoad(punctual_records, vec2<i32>(2, id), 0);
        var attenuation = 1.0;
        if cone.w > -1.0 {
            let inner = textureLoad(punctual_records, vec2<i32>(3, id), 0).x;
            let angle = dot(-l, cone.xyz);
            // A zero-width transition is an explicit hard cone.
            attenuation = select(smoothstep(cone.w, max(inner, cone.w + 1e-6), angle), select(0.0, 1.0, angle >= cone.w), inner == cone.w);
        }
        let window = max(1.0 - (d2 / (pr.w * pr.w)) * (d2 / (pr.w * pr.w)), 0.0);
        attenuation *= window * window / max(d2, 1.0);
        let radiance = textureLoad(punctual_records, vec2<i32>(1, id), 0).rgb * attenuation;
        sum += direct_light(s, l, radiance);
    }
    return sum;
}
fn cluster_word(i: u32) -> u32 {
    return textureLoad(punctual_clusters, vec2<i32>(i32(i % 1024u), i32(i / 1024u)), 0).x;
}
