// Grass and leaves in the wind: the higher a vertex is above the model's
// base, the more it sways. material.custom[0]: strength, speed.

fn vertex(v: Vertex) -> Vertex {
    var out = v;
    let height = max(v.uv.y, 0.0); // texture v grows towards the top here
    let phase = dot(v.position.xz, vec2<f32>(0.35, 0.21));
    let gust = 0.6 + 0.4 * noise2(v.position.xz * 0.05 + frame.time.x * 0.3);
    let sway = sin(frame.time.x * material.custom[0].y + phase) * material.custom[0].x * gust * height * height;
    out.position.x += sway;
    out.position.z += sway * 0.5;
    return out;
}
