
// Global illumination is not drawn on this GPU (no compute shaders or no
// texture slot for its probes, as on WebGL2): the sky and ground ambient
// light stays.
fn gi_light(world: vec3<f32>, n: vec3<f32>, view: vec3<f32>) -> vec4<f32> {
    return vec4<f32>(0.0);
}
