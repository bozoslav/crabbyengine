// Ribbons and lines: strips already facing the camera, built on the CPU.
struct Camera {
    view_proj: mat4x4<f32>,
    right: vec4<f32>,
    up: vec4<f32>,
}
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;
struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}
@vertex fn vs(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>) -> Out {
    var out: Out;
    out.position = camera.view_proj * vec4(position, 1.0);
    out.uv = uv;
    out.color = color;
    return out;
}
@fragment fn fs(in: Out) -> @location(0) vec4<f32> {
    let texel = textureSample(image, image_sampler, in.uv) * in.color;
    return vec4(texel.rgb * texel.a, texel.a);
}
