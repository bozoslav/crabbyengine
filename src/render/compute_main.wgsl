// The compute entry point: one invocation per texel, in 8 x 8 groups.
@compute @workgroup_size(8, 8)
fn cs_compute(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = vec2<i32>(textureDimensions(state_in));
    let pixel = vec2<i32>(id.xy);
    if pixel.x >= size.x || pixel.y >= size.y {
        return;
    }
    var c: Cell;
    c.value = textureLoad(state_in, pixel, 0);
    c.pixel = pixel;
    c.size = size;
    c.uv = (vec2<f32>(pixel) + 0.5) / vec2<f32>(size);
    textureStore(state_out, pixel, compute(c));
}
