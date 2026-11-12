// The 256 x 128 sky occupies the top-left of either its own texture or
// the shared IBL atlas. Filtering must never reach a neighbouring region.
// Longitude is periodic: blend the last and first columns across -X;
// clamping longitude alone would leave a one-texel discontinuity there.
fn sample_sky(image: texture_2d<f32>, linear_sampler: sampler, uv: vec2<f32>) -> vec3<f32> {
    let size = vec2<f32>(256.0, 128.0);
    let dimensions = vec2<f32>(textureDimensions(image));
    let pixel = vec2<f32>(fract(uv.x) * size.x, clamp(uv.y * size.y, 0.5, size.y - 0.5));
    if pixel.x < 0.5 || pixel.x > size.x - 0.5 {
        let first = textureSampleLevel(image, linear_sampler, vec2<f32>(0.5, pixel.y) / dimensions, 0.0).rgb;
        let last = textureSampleLevel(image, linear_sampler, vec2<f32>(size.x - 0.5, pixel.y) / dimensions, 0.0).rgb;
        // Pixel coordinates measure from the region's edge, not its centres.
        return mix(last, first, fract(pixel.x + 0.5));
    }
    return textureSampleLevel(image, linear_sampler, pixel / dimensions, 0.0).rgb;
}
