
// Projected decals are not drawn on this GPU: it has no texture slot to
// spare for their images (WebGL2). Surfaces are left as they are.
fn apply_decals(s: Surface, geometric: vec3<f32>, layers: u32, dp_x: vec3<f32>, dp_y: vec3<f32>) -> Surface {
    return s;
}
