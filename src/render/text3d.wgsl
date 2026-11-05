// 3D text: glyph quads sampling a signed distance field (alpha: 0.5 on the
// letters' edge, more inside). Fill, outline and soft edges all come from
// that distance; colors arrive premultiplied.
struct Camera {
    view_proj: mat4x4<f32>,
    right: vec4<f32>,
    up: vec4<f32>,
}
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var atlas: texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;
struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) outline: vec4<f32>,
    // Outline reach and softness, in distance units.
    @location(3) params: vec2<f32>,
}
@vertex fn vs(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) outline: vec4<f32>,
    @location(4) params: vec2<f32>,
) -> Out {
    var out: Out;
    out.position = camera.view_proj * vec4(position, 1.0);
    out.uv = uv;
    out.color = color;
    out.outline = outline;
    out.params = params;
    return out;
}
@fragment fn fs(in: Out) -> @location(0) vec4<f32> {
    let d = textureSample(atlas, atlas_sampler, in.uv).a;
    // Half a screen pixel of distance, so edges stay sharp but smooth at
    // any size; softness widens it.
    let w = max(fwidth(d) * 0.5, 1e-4) + in.params.y;
    let fill = smoothstep(0.5 - w, 0.5 + w, d);
    let edge = 0.5 - in.params.x;
    let shape = smoothstep(edge - w, edge + w, d);
    // The outline under the fill, cut to the outer shape.
    return mix(in.outline * shape, in.color, fill);
}
