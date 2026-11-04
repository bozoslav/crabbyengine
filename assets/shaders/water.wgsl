// A lake: gentle waves move the surface. Looking through it, the bottom is
// bent by the ripples (refraction) and fades into the water's color with
// depth; at grazing angles it reflects the sky (Fresnel).
// material.custom[0]: wave height, wave length, speed, clarity.

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

    // How much water the view goes through to reach the bottom.
    let bent = s.screen + out.normal.xz * 25.0;
    let depth = max(scene_distance(bent) - distance(s.world, frame.camera.xyz), 0.0);
    let seen = exp(-depth * material.custom[0].w);
    let fresnel = pow(1.0 - max(dot(out.normal, s.view), 0.0), 5.0);

    // The bottom shows through as light of its own; the water's color and
    // the sky's reflection are lit as usual.
    out.emissive = scene_color(bent) * seen * (1.0 - fresnel);
    out.albedo = mix(s.albedo * (1.0 - seen), frame.sky.rgb, fresnel);
    out.alpha = 1.0;
    out.roughness = 0.05;
    return out;
}
