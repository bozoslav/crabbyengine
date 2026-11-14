// Makes the player glow: mixes the sprite's color toward white by the
// material's first parameter (0 = unchanged, 1 = white).
fn fragment(p: SpritePixel) -> vec4<f32> {
    let glow = material.params[0].x;
    return mix(p.color.rgb, vec3<f32>(1.0), glow);
}
