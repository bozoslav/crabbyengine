// The built-in terrain material (Ctx::create_terrain_material): up to 8
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
    var albedo = vec3<f32>(0.0);
    var bump = vec3<f32>(0.0);
    for (var i = 0; i < count; i++) {
        let uv = s.world.xz * scales[i];
        let w = weights[i] / total;
        albedo += textureSample(custom_array0, material_sampler, uv, i).rgb * w;
        bump += (textureSample(custom_array1, material_sampler, uv, i).xyz * 2.0 - 1.0) * w;
    }
    out.albedo = s.albedo * albedo;
    if normals {
        // Texture space on the ground: u along +X, the image's up along -Z
        // (v runs down the image, along +Z), tilted onto the slope.
        let n = s.normal;
        let t = normalize(vec3<f32>(1.0, 0.0, 0.0) - n * n.x);
        let b = cross(n, t);
        let m = vec3<f32>(bump.xy * material.custom[0].y, max(bump.z, 1e-3));
        out.normal = normalize(t * m.x + b * m.y + n * m.z);
    }
    return out;
}
