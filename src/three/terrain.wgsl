// The built-in terrain material (Graphics::create_terrain_material): up to 8
// ground layers, blended by a weight map, their textures lined up with the
// world so neighbouring tiles match.
//
//   custom_array0: each layer's color         custom_array1: their normal maps
//   custom_array2: the weights, layer 0's RGBA for ground layers 0-3, layer
//                  1's for 4-7, stretched over the weight area
//   custom[0]: layers, normal strength, has normal maps (1/0)
//   custom[1], custom[2]: 1 / each layer's size in world units
//   custom[3]: the weight area's corner (x, z), then 1 / its size (x, z)

fn surface(s: Surface) -> Surface {
    var out = s;
    let count = i32(material.custom[0].x);
    // The weights, clamped half a texel in so the edges don't wrap around.
    let half = 0.5 / vec2<f32>(textureDimensions(custom_array2));
    let area = clamp((s.world.xz - material.custom[3].xy) * material.custom[3].zw, half, 1.0 - half);
    let last = i32(textureNumLayers(custom_array2)) - 1;
    let low = textureSample(custom_array2, material_sampler, area, 0);
    let high = textureSample(custom_array2, material_sampler, area, min(1, last));
    var weights = array<f32, 8>(low.x, low.y, low.z, low.w, high.x, high.y, high.z, high.w);
    let c1 = material.custom[1];
    let c2 = material.custom[2];
    let scales = array<f32, 8>(c1.x, c1.y, c1.z, c1.w, c2.x, c2.y, c2.z, c2.w);
    var total = 0.0;
    for (var i = 0; i < 8; i++) {
        if i >= count {
            weights[i] = 0.0;
        }
        total += weights[i];
    }
    if total <= 0.0 {
        weights[0] = 1.0; // nothing painted: the first layer
        total = 1.0;
    }
    let normals = material.custom[0].z > 0.5;
    // Triplanar: steep ground takes its textures from the sides (along X
    // and along Z) as well as from above, so cliffs don't stretch them.
    // Flat ground skips the side samples; the gradients are taken here,
    // outside the branches, so the skipped ones don't upset mip choice.
    let n = s.normal;
    var tri = pow(abs(n), vec3<f32>(4.0));
    tri /= tri.x + tri.y + tri.z;
    let dx = dpdx(s.world);
    let dy = dpdy(s.world);
    let flip = select(vec2<f32>(-1.0), vec2<f32>(1.0), n.xz >= vec2<f32>(0.0));
    var albedo = vec3<f32>(0.0);
    // The normal maps' tilt, in world space, added to the ground's normal.
    var tilt = vec3<f32>(0.0);
    var up = 0.0;
    for (var i = 0; i < count; i++) {
        let w = weights[i] / total;
        if w <= 0.0 {
            continue;
        }
        let k = scales[i];
        // From above: u along +X, the image's up along -Z.
        if tri.y > 0.02 {
            let uv = s.world.xz * k;
            albedo += textureSampleGrad(custom_array0, material_sampler, uv, i, dx.xz * k, dy.xz * k).rgb * (w * tri.y);
            if normals {
                let m = textureSampleGrad(custom_array1, material_sampler, uv, i, dx.xz * k, dy.xz * k).xyz * 2.0 - 1.0;
                tilt += vec3<f32>(m.x, 0.0, -m.y) * (w * tri.y);
                up += m.z * w * tri.y;
            }
        }
        // From the side facing X: u along Z (mirrored on the -X side), up along +Y.
        if tri.x > 0.02 {
            let uv = vec2<f32>(s.world.z * flip.x, -s.world.y) * k;
            let gx = vec2<f32>(dx.z * flip.x, -dx.y) * k;
            let gy = vec2<f32>(dy.z * flip.x, -dy.y) * k;
            albedo += textureSampleGrad(custom_array0, material_sampler, uv, i, gx, gy).rgb * (w * tri.x);
            if normals {
                let m = textureSampleGrad(custom_array1, material_sampler, uv, i, gx, gy).xyz * 2.0 - 1.0;
                tilt += vec3<f32>(0.0, m.y, m.x * flip.x) * (w * tri.x);
                up += m.z * w * tri.x;
            }
        }
        // From the side facing Z: u along X (mirrored on the +Z side), up along +Y.
        if tri.z > 0.02 {
            let uv = vec2<f32>(-s.world.x * flip.y, -s.world.y) * k;
            let gx = vec2<f32>(-dx.x * flip.y, -dx.y) * k;
            let gy = vec2<f32>(-dy.x * flip.y, -dy.y) * k;
            albedo += textureSampleGrad(custom_array0, material_sampler, uv, i, gx, gy).rgb * (w * tri.z);
            if normals {
                let m = textureSampleGrad(custom_array1, material_sampler, uv, i, gx, gy).xyz * 2.0 - 1.0;
                tilt += vec3<f32>(-m.x * flip.y, m.y, 0.0) * (w * tri.z);
                up += m.z * w * tri.z;
            }
        }
    }
    // The skipped samples' share, so the colors still add up.
    let kept = select(0.0, tri.y, tri.y > 0.02) + select(0.0, tri.x, tri.x > 0.02) + select(0.0, tri.z, tri.z > 0.02);
    out.albedo = s.albedo * albedo / max(kept, 1e-4);
    if normals {
        // The tilt along the surface only, scaled by the strength.
        let along = (tilt - n * dot(tilt, n)) / max(kept, 1e-4);
        out.normal = normalize(n * max(up / max(kept, 1e-4), 1e-3) + along * material.custom[0].y);
    }
    return out;
}
