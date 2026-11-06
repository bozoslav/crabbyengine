// Screen-space reflections, at half size. Each pixel follows its reflected
// view ray across the depth buffer; where the ray first goes behind
// something on screen, the color there is what the pixel reflects. The color comes from the last
// frame's opaque image, reprojected (this frame's isn't lit yet: the opaque
// pass mixes the result into its reflection of the environment, by the
// surface's roughness and with its Fresnel, see environment_light in
// mesh.wgsl). Normals come from the depth, as there is no normal buffer: flat
// faces, so normal maps don't bend these reflections.
struct Settings {
    inverse: mat4x4<f32>,  // this frame's clip space to the world
    projection: mat4x4<f32>, // the world to this frame's clip space
    previous: mat4x4<f32>, // the world to the clip space of `image` (the last frame)
    camera: vec4<f32>,
    params: vec4<f32>,     // strength (0: none), longest ray, frame number
};
@group(0) @binding(0) var depth: texture_depth_multisampled_2d;
// fs_trace: the last frame's opaque image. fs_copy: this frame's.
@group(0) @binding(1) var image: texture_2d<f32>;
@group(0) @binding(2) var image_sampler: sampler;
@group(0) @binding(3) var<uniform> settings: Settings;
struct Full { @builtin(position) at: vec4<f32> };
@vertex fn vs_full(@builtin(vertex_index) i: u32) -> Full {
    var out: Full;
    out.at = vec4<f32>(vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u)) * 2.0 - 1.0, 0.0, 1.0);
    return out;
}

// Linear steps along the ray, then halvings of the step that crossed.
const STEPS: i32 = 32;
const REFINE: i32 = 6;

fn coordinate(p: vec2<i32>) -> vec2<i32> {
    return clamp(p, vec2<i32>(0), vec2<i32>(textureDimensions(depth)) - 1);
}
fn z_at(p: vec2<i32>) -> f32 { return textureLoad(depth, coordinate(p), 0); }
fn ndc(p: vec2<i32>) -> vec2<f32> {
    let uv = (vec2<f32>(coordinate(p)) + 0.5) / vec2<f32>(textureDimensions(depth));
    return uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
}
fn position(p: vec2<i32>) -> vec3<f32> {
    let h = settings.inverse * vec4<f32>(ndc(p), z_at(p), 1.0);
    return h.xyz / h.w;
}
// How far in front of the camera (clip w) the depth `z` at pixel `p` is.
fn scene_w(p: vec2<i32>, z: f32) -> f32 {
    let h = settings.inverse * vec4<f32>(ndc(p), z, 1.0);
    return 1.0 / h.w;
}
fn safe_normalize(v: vec3<f32>, fallback: vec3<f32>) -> vec3<f32> {
    return select(fallback, v / max(length(v), 0.00001), dot(v, v) > 0.00000001);
}
fn to_uv(clip: vec4<f32>) -> vec2<f32> {
    return clip.xy / clip.w * vec2<f32>(0.5, -0.5) + 0.5;
}
fn on_screen(uv: vec2<f32>) -> bool {
    return all(uv >= vec2<f32>(0.0)) && all(uv < vec2<f32>(1.0));
}
// 0 at the screen's edges, 1 a tenth of the way in: a ray that leaves the
// screen there fades instead of cutting off.
fn edge_fade(uv: vec2<f32>) -> f32 {
    let d = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    return clamp(d * 10.0, 0.0, 1.0);
}
// Whether the ray at clip position `c` is behind the depth buffer at its pixel.
fn behind(c: vec4<f32>) -> bool {
    let q = vec2<i32>(to_uv(c) * vec2<f32>(textureDimensions(depth)));
    let z = z_at(q);
    return z < 0.999999 && c.z / c.w > z;
}

// What the pixel reflects: rgb the color, alpha how much it counts (0: the
// ray found nothing on screen; the environment stands in).
@fragment fn fs_trace(in: Full) -> @location(0) vec4<f32> {
    // The top left of the 2x2 full-size pixels this one stands for.
    let p = vec2<i32>(in.at.xy) * 2;
    let strength = settings.params.x;
    if strength <= 0.0 || z_at(p) >= 0.999999 { return vec4<f32>(0.0); }
    let center = position(p);
    let view = normalize(settings.camera.xyz - center);
    // The less discontinuous neighbor on either side of each axis.
    let left = center - position(p - vec2<i32>(1, 0));
    let right = position(p + vec2<i32>(1, 0)) - center;
    let up = center - position(p - vec2<i32>(0, 1));
    let down = position(p + vec2<i32>(0, 1)) - center;
    let dx = select(left, right, dot(right, right) < dot(left, left));
    let dy = select(up, down, dot(down, down) < dot(up, up));
    var normal = safe_normalize(cross(dx, dy), view);
    normal *= select(-1.0, 1.0, dot(normal, view) >= 0.0);
    let ray = reflect(-view, normal);
    // Off the surface a little, so the ray doesn't find where it starts; the
    // depth buffer's precision falls with distance, and so does the offset.
    let eye = distance(center, settings.camera.xyz);
    let origin = center + normal * (0.01 + eye * 0.002);
    let longest = settings.params.y;
    let start = settings.projection * vec4<f32>(origin, 1.0);
    var end = settings.projection * vec4<f32>(origin + ray * longest, 1.0);
    // A ray toward the camera stops before it: clip space is linear along
    // the ray, so stopping is a mix.
    var reach = 1.0;
    let nearest = start.w * 0.1;
    if end.w < nearest {
        reach = (start.w - nearest) / (start.w - end.w);
        end = mix(start, end, reach);
    }
    // Interleaved gradient noise, moving each frame: TAA averages the steps
    // of several frames into a finer march.
    let noise = vec2<f32>(p) + 5.588238 * f32(u32(settings.params.z) % 64u);
    let jitter = fract(52.9829189 * fract(dot(noise, vec2<f32>(0.06711056, 0.00583715))));
    let size = vec2<f32>(textureDimensions(depth));
    var before = 0.0;
    var before_w = start.w;
    var hit = -1.0;
    for (var i = 0; i < STEPS; i++) {
        // Squared: short steps near the surface, where contact matters most.
        let k = (f32(i) + jitter) / f32(STEPS);
        let f = k * k;
        let c = mix(start, end, f);
        let uv = to_uv(c);
        if !on_screen(uv) { break; }
        let q = vec2<i32>(uv * size);
        let z = z_at(q);
        if z < 0.999999 && c.z / c.w > z {
            // Behind it, by less than its assumed thickness (or the step):
            // a hit. Further behind, the ray passed behind it; go on.
            let gap = c.w - scene_w(q, z);
            if gap < 0.1 + 0.02 * c.w + 2.0 * abs(c.w - before_w) {
                var lo = before;
                var hi = f;
                for (var j = 0; j < REFINE; j++) {
                    let mid = (lo + hi) * 0.5;
                    if behind(mix(start, end, mid)) { hi = mid; } else { lo = mid; }
                }
                hit = hi;
                break;
            }
        }
        before = f;
        before_w = c.w;
    }
    if hit < 0.0 { return vec4<f32>(0.0); }
    let uv = to_uv(mix(start, end, hit));
    // Where the hit was in the last frame's image.
    let last = settings.previous * vec4<f32>(position(vec2<i32>(uv * size)), 1.0);
    if last.w <= 0.0 { return vec4<f32>(0.0); }
    let last_uv = to_uv(last);
    if !on_screen(last_uv) { return vec4<f32>(0.0); }
    let color = textureSampleLevel(image, image_sampler, last_uv, 0.0).rgb;
    let far = 1.0 - smoothstep(0.6, 1.0, hit * reach);
    let confidence = strength * far * edge_fade(uv) * edge_fade(last_uv);
    return vec4<f32>(color * confidence, confidence);
}

// This frame's opaque image, for the next frame's reflections.
@fragment fn fs_copy(in: Full) -> @location(0) vec4<f32> {
    return textureLoad(image, vec2<i32>(in.at.xy), 0);
}

// Four-tap downsample of premultiplied radiance/confidence. Reject samples
// across the receiver's plane to keep reflections off unrelated geometry.
// Only reflection mips are bound here; mip zero (AO) is never sampled.
@fragment fn fs_blur(in: Full) -> @location(0) vec4<f32> {
    let source_size = vec2<i32>(textureDimensions(image));
    let dest_size = max(source_size / 2, vec2<i32>(1));
    let uv = in.at.xy / vec2<f32>(dest_size);
    let depth_size = vec2<f32>(textureDimensions(depth));
    let p = vec2<i32>(uv * depth_size);
    if z_at(p) >= 0.999999 { return vec4<f32>(0.0); }
    let center = position(p);
    let normal = safe_normalize(cross(position(p + vec2<i32>(1, 0)) - center,
                                    position(p + vec2<i32>(0, 1)) - center), vec3<f32>(0.0, 1.0, 0.0));
    var sum = vec4<f32>(0.0);
    var weight = 0.0;
    for (var y = 0; y < 2; y++) {
        for (var x = 0; x < 2; x++) {
            let q = min(vec2<i32>(in.at.xy) * 2 + vec2<i32>(x, y), source_size - 1);
            let dp = vec2<i32>((vec2<f32>(q) + 0.5) / vec2<f32>(source_size) * depth_size);
            let gap = abs(dot(position(dp) - center, normal));
            let tolerance = 0.02 + distance(center, settings.camera.xyz) * 0.005;
            let w = select(0.0, exp(-gap / tolerance), z_at(dp) < 0.999999);
            sum += textureLoad(image, q, 0) * w;
            weight += w;
        }
    }
    return sum / max(weight, 0.00001);
}
