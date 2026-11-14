// A sprite shader: an outline around the opaque part of the sprite.
// material.params[0]: the outline's color; params[1].x: its width in texels.
// The outline is drawn inside the sprite's own rectangle, so the image
// needs a transparent margin at least that wide.

fn fragment(p: SpritePixel) -> vec4<f32> {
    if p.texture.a > 0.5 {
        return p.color;
    }
    // How opaque the sprite is anywhere within the width, in 8 directions.
    let reach = sprite_texel() * material.params[1].x;
    var near = 0.0;
    for (var i = 0; i < 8; i++) {
        let angle = f32(i) * 0.7853982;
        near = max(near, sprite_sample(p.uv + vec2<f32>(cos(angle), sin(angle)) * reach).a);
    }
    let outline = material.params[0];
    return vec4<f32>(outline.rgb, outline.a * near * p.tint.a);
}
