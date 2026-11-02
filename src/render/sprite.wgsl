// Instanced sprite shader. One instance = one sprite; each instance draws a
// 6-vertex quad generated from the vertex index (no vertex buffer).

struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

struct Instance {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) pivot: vec2<f32>,
    @location(3) rot: vec2<f32>,   // cos, sin
    @location(4) uv: vec4<f32>,    // u0, v0, du, dv
    @location(5) color: vec4<f32>, // sRGB tint
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, inst: Instance) -> VertexOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
    );
    let corner = corners[vi];

    let local = (corner - inst.pivot) * inst.size;
    let rotated = vec2<f32>(
        local.x * inst.rot.x - local.y * inst.rot.y,
        local.x * inst.rot.y + local.y * inst.rot.x,
    );

    var out: VertexOut;
    out.clip = camera.view_proj * vec4<f32>(inst.pos + rotated, 0.0, 1.0);
    out.uv = inst.uv.xy + corner * inst.uv.zw;
    out.color = vec4<f32>(srgb_to_linear(inst.color.rgb), inst.color.a);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(sprite_texture, sprite_sampler, in.uv) * in.color;
}
