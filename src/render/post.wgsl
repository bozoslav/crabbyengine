// The last 3D pass: turns the HDR frame into display colors.

@group(0) @binding(0) var hdr: texture_2d<f32>;

@vertex
fn vs_full(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    // One triangle covering the screen.
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

// Narkowicz's fit of the ACES filmic curve: highlights roll off instead of clipping.
fn aces(x: vec3<f32>) -> vec3<f32> {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_tonemap(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    // Alpha is 1 where models were drawn and 0 on the background, which
    // keeps the game's clear color as it is (averaged at the edges).
    let color = textureLoad(hdr, vec2<i32>(at.xy), 0);
    return vec4<f32>(mix(color.rgb, aces(color.rgb), color.a), 1.0);
}
