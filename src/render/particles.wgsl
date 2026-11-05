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
@vertex fn vs(
    @builtin(vertex_index) vertex: u32,
    @location(0) position_size: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) rotation_sheet: vec4<f32>,
) -> Out {
    let corners = array<vec2<f32>, 6>(vec2(-0.5,-0.5),vec2(0.5,-0.5),vec2(-0.5,0.5),vec2(-0.5,0.5),vec2(0.5,-0.5),vec2(0.5,0.5));
    let p = corners[vertex];
    let c = cos(rotation_sheet.x);
    let s = sin(rotation_sheet.x);
    let rotated = vec2(c*p.x-s*p.y,s*p.x+c*p.y)*position_size.w;
    let world = position_size.xyz + camera.right.xyz*rotated.x + camera.up.xyz*rotated.y;
    let columns = rotation_sheet.z;
    let rows = rotation_sheet.w;
    let frame = rotation_sheet.y;
    let cell = vec2(frame % columns, floor(frame/columns));
    var out: Out;
    out.position = camera.view_proj*vec4(world,1.0);
    out.uv = (cell+vec2(p.x+0.5,0.5-p.y))/vec2(columns,rows);
    out.color = color;
    return out;
}
@fragment fn fs(in: Out) -> @location(0) vec4<f32> {
    let texel = textureSample(image,image_sampler,in.uv)*in.color;
    return vec4(texel.rgb*texel.a,texel.a);
}
