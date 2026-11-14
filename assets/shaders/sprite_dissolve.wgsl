// A sprite shader: the sprite burns away with a glowing edge.
// material.params[0].x: how far it has burned, 0 (whole) to 1 (gone);
// params[1]: the edge's color. Texture 0: noise (where it burns first).

fn fragment(p: SpritePixel) -> vec4<f32> {
    let noise = material_sample0(p.local).r;
    let burned = material.params[0].x;
    if noise < burned {
        discard;
    }
    // Pixels just above the burn line glow.
    let edge = (1.0 - smoothstep(0.0, 0.08, noise - burned)) * step(0.001, burned);
    return vec4<f32>(mix(p.color.rgb, material.params[1].rgb, edge), p.color.a);
}
