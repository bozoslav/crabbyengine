
@vertex
fn vs_effect(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
}

@fragment
fn fs_effect(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    var p: Pixel;
    p.screen = at.xy;
    p.uv = at.xy * frame.screen.zw;
    p.color = scene_color(at.xy);
    p.world = scene_position(at.xy);
    p.distance = distance(p.world, frame.camera.xyz);
    let size = vec2<i32>(textureDimensions(scene_depth_texture));
    p.sky = textureLoad(scene_depth_texture, clamp(vec2<i32>(at.xy), vec2<i32>(0), size - 1), 0) >= 1.0;
    return vec4<f32>(effect(p), 1.0);
}
