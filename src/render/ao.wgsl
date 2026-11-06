// GTAO: horizon search in view-aligned slices and analytic cosine-weighted
// integration (Jimenez et al., Practical Realtime Strategies for Accurate
// Indirect Occlusion, 2016). World positions/normals reconstructed from depth.
struct Settings {
    inverse: mat4x4<f32>,
    projection: mat4x4<f32>,
    camera: vec4<f32>,
    params: vec4<f32>, // radius, intensity (0: no AO), slices, steps
    sun: vec4<f32>,    // xyz: toward the sun; w: contact shadow length (0: none)
};
@group(0) @binding(0) var depth: texture_depth_multisampled_2d;
@group(0) @binding(1) var image: texture_2d<f32>;
@group(0) @binding(2) var<uniform> settings: Settings;
struct Full { @builtin(position) at: vec4<f32> };
@vertex fn vs_full(@builtin(vertex_index) i: u32) -> Full {
    var out: Full;
    out.at = vec4<f32>(vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u)) * 2.0 - 1.0, 0.0, 1.0);
    return out;
}
fn coordinate(p: vec2<i32>) -> vec2<i32> {
    return clamp(p, vec2<i32>(0), vec2<i32>(textureDimensions(depth)) - 1);
}
fn safe_normalize(v: vec3<f32>, fallback: vec3<f32>) -> vec3<f32> {
    return select(fallback, v / max(length(v), 0.00001), dot(v, v) > 0.00000001);
}
fn z_at(p: vec2<i32>) -> f32 { return textureLoad(depth, coordinate(p), 0); }
fn position(p: vec2<i32>) -> vec3<f32> {
    let uv = (vec2<f32>(coordinate(p)) + 0.5) / vec2<f32>(textureDimensions(depth));
    let h = settings.inverse * vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), z_at(p), 1.0);
    return h.xyz / h.w;
}
// Steps of the march toward the sun for contact shadows.
const CONTACT_STEPS: i32 = 12;

// 1 where something on screen blocks the sun within `settings.sun.w` of
// `center` (pixel `p`), else 0: a march toward the sun, each step tested
// against the depth there. Only what's in front of a step by no more than
// the march length counts, so a wall far behind doesn't shade a point.
fn contact_shadow(p: vec2<i32>, center: vec3<f32>) -> f32 {
    let length = settings.sun.w;
    let size = vec2<f32>(textureDimensions(depth));
    let eye = distance(center, settings.camera.xyz);
    // The depth buffer's precision falls with distance: so does trust.
    let bias = 0.02 + eye * 0.002;
    let jitter = fract(52.9829189 * fract(dot(vec2<f32>(p), vec2<f32>(0.06711056, 0.00583715))));
    for (var i = 0; i < CONTACT_STEPS; i++) {
        let at = center + settings.sun.xyz * length * (f32(i) + jitter + 0.5) / f32(CONTACT_STEPS);
        let clip = settings.projection * vec4<f32>(at, 1.0);
        if clip.w <= 0.0 { break; }
        let uv = clip.xy / clip.w * vec2<f32>(0.5, -0.5) + 0.5;
        if any(uv < vec2<f32>(0.0)) || any(uv >= vec2<f32>(1.0)) { break; }
        let q = vec2<i32>(uv * size);
        if z_at(q) >= 0.999999 { continue; }
        let gap = distance(at, settings.camera.xyz) - distance(position(q), settings.camera.xyz);
        if gap > bias && gap < length {
            return 1.0;
        }
    }
    return 0.0;
}

@fragment fn fs_ao(in: Full) -> @location(0) vec4<f32> {
    let p = vec2<i32>(in.at.xy);
    if z_at(p) >= 0.999999 { return vec4<f32>(0.0); }
    let center = position(p);
    var contact = 0.0;
    if settings.sun.w > 0.0 {
        contact = contact_shadow(p, center);
    }
    if settings.params.y <= 0.0 {
        return vec4<f32>(0.0, contact, 0.0, 1.0);
    }
    let view = normalize(settings.camera.xyz - center);
    // Use the less discontinuous neighbor on either side of each axis.
    let left = center - position(p - vec2<i32>(1, 0));
    let right = position(p + vec2<i32>(1, 0)) - center;
    let up = center - position(p - vec2<i32>(0, 1));
    let down = position(p + vec2<i32>(0, 1)) - center;
    let dx = select(left, right, dot(right, right) < dot(left, left));
    let dy = select(up, down, dot(down, down) < dot(up, up));
    var normal = safe_normalize(cross(dx, dy), view);
    normal *= select(-1.0, 1.0, dot(normal, view) >= 0.0);
    let radius = settings.params.x;
    // Projection of a world-space radius at this pixel (perspective).
    let clip = settings.projection * vec4<f32>(center, 1.0);
    let pixel_radius = clamp(radius * length(vec3<f32>(settings.projection[0].y, settings.projection[1].y, settings.projection[2].y)) * f32(textureDimensions(depth).y) / max(abs(clip.w), 0.01) * 0.5, 1.0, 256.0);
    var visibility = 0.0;
    for (var slice = 0u; slice < u32(settings.params.z); slice++) {
        let phi = (f32(slice) + 0.5) * 3.14159265 / settings.params.z;
        let direction = vec2<f32>(cos(phi), sin(phi));
        let neighbor = position(p + vec2<i32>(round(direction * 2.0)));
        let fallback = safe_normalize(cross(view, vec3<f32>(0.0, 1.0, 0.0)), vec3<f32>(1.0, 0.0, 0.0));
        let tangent = safe_normalize(neighbor - center - view * dot(neighbor - center, view), fallback);
        let axis = normalize(cross(tangent, view));
        let projected = normal - axis * dot(normal, axis);
        let projected_length = max(length(projected), 0.0001);
        let cos_n = clamp(dot(projected / projected_length, view), -1.0, 1.0);
        let angle_n = acos(cos_n) * sign(dot(projected, tangent));
        var horizons = vec2<f32>(-1.0);
        for (var side = 0u; side < 2u; side++) {
            let sign_side = select(-1.0, 1.0, side == 1u);
            // A local max, not horizons[side]: FXC (D3D) can't write a vector
            // through a dynamic index without unrolling the dynamic loop below.
            var best = -1.0;
            for (var step = 1u; step <= u32(settings.params.w); step++) {
                let offset = direction * sign_side * max(1.0, pixel_radius * pow(f32(step) / settings.params.w, 2.0));
                let at = p + vec2<i32>(round(offset));
                if any(at < vec2<i32>(0)) || any(at >= vec2<i32>(textureDimensions(depth))) || z_at(at) >= 0.999999 { continue; }
                let delta = position(at) - center;
                let dist = length(delta);
                if dist < 0.001 || dist > radius { continue; }
                // Fade distant occluders back to the open hemisphere.
                let horizon = dot(delta / dist, view) - 0.02 * radius / max(dist, 0.01);
                let falloff = 1.0 - smoothstep(radius * 0.5, radius, dist);
                best = max(best, mix(-1.0, horizon, falloff));
            }
            if side == 0u { horizons.x = best; } else { horizons.y = best; }
        }
        let h1 = angle_n + clamp(-acos(clamp(horizons.x, -1.0, 1.0)) - angle_n, -1.5707963, 1.5707963);
        let h2 = angle_n + clamp(acos(clamp(horizons.y, -1.0, 1.0)) - angle_n, -1.5707963, 1.5707963);
        visibility += projected_length * 0.25 * (2.0 * cos_n + 2.0 * (h1 + h2) * sin(angle_n) - cos(2.0 * h1 - angle_n) - cos(2.0 * h2 - angle_n));
    }
    let ao = pow(clamp(visibility / settings.params.z, 0.0, 1.0), settings.params.y);
    return vec4<f32>(1.0 - ao, contact, 0.0, 1.0);
}
// Blue 1 marks screen-space reflections in the target's second level (an RG8
// target drops it; see ssr.rs).
@fragment fn fs_denoise(in: Full) -> @location(0) vec4<f32> {
    let p = vec2<i32>(in.at.xy);
    if z_at(p) >= 0.999999 { return vec4<f32>(0.0, 0.0, 1.0, 0.0); }
    let center = position(p);
    var sum = vec2<f32>(0.0);
    var total = 0.0;
    for (var y = -2; y <= 2; y++) {
        for (var x = -2; x <= 2; x++) {
            let at = coordinate(p + vec2<i32>(x, y));
            let delta = position(at) - center;
            let w = exp(-f32(x*x+y*y) * 0.25) * exp(-dot(delta, delta) / max(settings.params.x * settings.params.x * 0.01, 0.000001));
            sum += textureLoad(image, at, 0).rg * w;
            total += w;
        }
    }
    return vec4<f32>(sum / max(total, 0.00001), 1.0, 1.0);
}
