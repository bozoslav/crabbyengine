// A lake: gentle waves move the surface, and the color goes from deep blue
// to the sky's reflection at grazing angles (Fresnel).
// material.custom[0]: wave height, wave length, speed.

fn vertex(v: Vertex) -> Vertex {
    var out = v;
    let p = v.position.xz / material.custom[0].y;
    let t = frame.time.x * material.custom[0].z;
    out.position.y += (sin(p.x + t) + sin(p.y * 1.3 + t * 0.8)) * material.custom[0].x;
    return out;
}

fn surface(s: Surface) -> Surface {
    var out = s;
    // Ripples: the normal follows the waves plus fine noise.
    let p = s.world.xz / material.custom[0].y;
    let t = frame.time.x * material.custom[0].z;
    let slope = vec2<f32>(cos(p.x + t), cos(p.y * 1.3 + t * 0.8) * 1.3) * material.custom[0].x / material.custom[0].y;
    let fine = vec2<f32>(noise2(s.world.xz * 3.0 + t), noise2(s.world.zx * 3.0 - t)) - 0.5;
    out.normal = normalize(vec3<f32>(-slope.x - fine.x * 0.15, 1.0, -slope.y - fine.y * 0.15));
    let fresnel = pow(1.0 - max(dot(out.normal, s.view), 0.0), 5.0);
    out.albedo = mix(s.albedo, frame.sky.rgb, fresnel);
    out.alpha = mix(0.75, 1.0, fresnel);
    out.roughness = 0.05;
    return out;
}
