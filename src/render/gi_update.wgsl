// Global illumination's probe update (see gi.rs, gi.wgsl): turns each
// updated probe's six views into its tiles, written to a scratch image the
// engine then copies into the atlas. Appended to the mesh prelude, for the
// frame (group 0) and the sky's light in the environment texture.
//
// One workgroup of 8x8 per 8x8 block of a probe's tiles: block 0 is its
// light tile, blocks 1 to 4 the four quarters of its distance tile.

struct GiProbeUpdate {
    probe: vec4<f32>, // xyz: where the probe is; w: its code (see GiGrid::code)
    tile: vec4<f32>,  // xy: its tile column and row; z: how much old light stays
};

struct GiUpdates {
    list: array<GiProbeUpdate, 16>,
    params: vec4<f32>, // x: the views' near plane, y: far plane, z: rows of tiles, w: view size
};

@group(3) @binding(0) var<uniform> gi_updates: GiUpdates;
// The updated probes' views, side by side (+X, -X, +Y, -Y, +Z, -Z), one
// row per probe, and their depth.
@group(3) @binding(1) var gi_views: texture_2d<f32>;
@group(3) @binding(2) var gi_view_depth: texture_depth_multisampled_2d;
@group(3) @binding(3) var gi_old: texture_2d<f32>;
@group(3) @binding(4) var gi_out: texture_storage_2d<rgba16float, write>;

// Each view's direction and up (the same table as `GI_FACES`).
const GI_FORWARD = array<vec3<f32>, 6>(
    vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(-1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0),
    vec3<f32>(0.0, -1.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(0.0, 0.0, -1.0),
);
const GI_UP = array<vec3<f32>, 6>(
    vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0, 0.0, -1.0),
    vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0, 1.0, 0.0),
);

// The unit direction at octahedral coordinates `e` (-1 to 1 each).
fn gi_unoctahedral(e: vec2<f32>) -> vec3<f32> {
    var d = vec3<f32>(e, 1.0 - abs(e.x) - abs(e.y));
    if d.z < 0.0 {
        let s = select(vec2<f32>(-1.0), vec2<f32>(1.0), d.xy >= vec2<f32>(0.0));
        d = vec3<f32>((1.0 - abs(d.yx)) * s, d.z);
    }
    return normalize(d);
}

// The inside texel a tile texel stands for: the border copies the texel
// across the octahedral map's edge, so filtering wraps around it.
fn gi_inside(p: vec2<i32>, size: i32) -> vec2<i32> {
    var q = p;
    if p.y < 0 || p.y >= size {
        q.x = size - 1 - p.x;
    }
    if p.x < 0 || p.x >= size {
        q.y = size - 1 - p.y;
    }
    return clamp(q, vec2<i32>(0), vec2<i32>(size - 1));
}

@compute @workgroup_size(8, 8, 1)
fn cs_update(@builtin(workgroup_id) block: vec3<u32>, @builtin(local_invocation_id) local: vec3<u32>) {
    let update = gi_updates.list[block.z];
    let views = i32(gi_updates.params.w);
    let near = gi_updates.params.x;
    let far = gi_updates.params.y;
    let rows = gi_updates.params.z;
    let light = block.x == 0u;
    var texel = vec2<i32>(local.xy);
    var size = 8;
    if !light {
        let quarter = i32(block.x) - 1;
        texel += vec2<i32>(quarter % 2, quarter / 2) * 8;
        size = 16;
    }
    let inside = size - 2;
    let e = (vec2<f32>(gi_inside(texel - 1, inside)) + 0.5) / f32(inside) * 2.0 - 1.0;
    let dir = gi_unoctahedral(e);
    var sum = vec4<f32>(0.0);
    for (var face = 0; face < 6; face++) {
        let forward = GI_FORWARD[face];
        let up = GI_UP[face];
        let right = cross(forward, up);
        for (var y = 0; y < views; y++) {
            for (var x = 0; x < views; x++) {
                let uv = (vec2<f32>(f32(x), f32(y)) + 0.5) / f32(views) * 2.0 - 1.0;
                let ray = forward + right * uv.x - up * uv.y;
                let length2 = dot(ray, ray);
                let length1 = sqrt(length2);
                let d = ray / length1;
                // Each texel's share of the sphere (times 4 / views²).
                let solid = 1.0 / (length2 * length1);
                let c = dot(dir, d);
                let at = vec2<i32>(face * views + x, i32(block.z) * views + y);
                if light {
                    if c > 0.0 {
                        let depth = textureLoad(gi_view_depth, at, 0);
                        var radiance = textureLoad(gi_views, at, 0).rgb;
                        if depth >= 1.0 {
                            // Nothing there: the sky.
                            radiance = environment_sample(0.0, d, 0.3, false);
                        }
                        sum += vec4<f32>(radiance * (c * solid), 0.0);
                    }
                } else if c > 0.0 {
                    let w = pow(c, 50.0) * solid;
                    if w > 1e-7 {
                        let depth = textureLoad(gi_view_depth, at, 0);
                        // Distance along the view, then along the ray.
                        let along = near * far / max(far - depth * (far - near), 1e-6);
                        let distance = min(along * length1, far);
                        sum += vec4<f32>(distance * w, distance * distance * w, 0.0, w);
                    }
                }
            }
        }
    }
    let column = update.tile.x;
    let row = update.tile.y;
    var result: vec4<f32>;
    var old_at: vec2<i32>;
    var out_at = vec2<i32>(0, i32(block.z) * 16) + texel;
    if light {
        // Irradiance over pi: the sum times each texel's 4 / views² of the sphere.
        let irradiance = sum.rgb * 4.0 / (f32(views * views) * PI);
        result = vec4<f32>(irradiance, update.probe.w);
        old_at = vec2<i32>(i32(column) * 8, i32(row) * 8) + texel;
    } else {
        let moments = sum.xy / max(sum.w, 1e-12);
        result = vec4<f32>(moments, 0.0, 1.0);
        old_at = vec2<i32>(i32(column) * 16, i32(rows) * 8 + i32(row) * 16) + texel;
        out_at.x += 8;
    }
    let keep = update.tile.z;
    if keep > 0.0 {
        let old = textureLoad(gi_old, old_at, 0);
        result = vec4<f32>(mix(result.rgb, old.rgb, keep), result.a);
    }
    textureStore(gi_out, out_at, result);
}
