// The entry points of every material shader (see mesh.wgsl).

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) m0: vec4<f32>,
    @location(4) m1: vec4<f32>,
    @location(5) m2: vec4<f32>,
    @location(6) m3: vec4<f32>,
    @location(7) color: vec4<f32>,
};

struct VertexOut {
    // Invariant: the depth prepass and the main pass must land on exactly the same depth.
    @builtin(position) @invariant clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

// The vertex in world space, after the shader's `vertex` function.
fn world_vertex(in: VertexIn) -> Vertex {
    let model = mat4x4<f32>(in.m0, in.m1, in.m2, in.m3);
    // For a scale-rotate matrix M = R S, the normal matrix R S^-1 is M S^-2.
    let scale = vec3<f32>(length(in.m0.xyz), length(in.m1.xyz), length(in.m2.xyz));
    let normal = mat3x3<f32>(in.m0.xyz, in.m1.xyz, in.m2.xyz) * (in.normal / max(scale * scale, vec3<f32>(1e-12)));
    var v: Vertex;
    v.position = (model * vec4<f32>(in.position, 1.0)).xyz;
    v.normal = normal;
    v.uv = in.uv;
    v.color = in.color;
    return vertex(v);
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    let v = world_vertex(in);
    var out: VertexOut;
    out.clip = frame.view_proj * vec4<f32>(v.position, 1.0);
    out.world = v.position;
    out.normal = v.normal;
    out.uv = v.uv;
    out.color = v.color;
    return out;
}

// The shadow map: depth seen from the sun.
@vertex
fn vs_shadow(in: VertexIn) -> VertexOut {
    let v = world_vertex(in);
    var out: VertexOut;
    out.clip = frame.sun_view_proj * vec4<f32>(v.position, 1.0);
    out.world = v.position;
    out.normal = v.normal;
    out.uv = v.uv;
    out.color = v.color;
    return out;
}

// The surface from the material and its textures, then the shader's `surface`.
fn surface_at(in: VertexOut, front: bool) -> Surface {
    let base = textureSample(base_texture, material_sampler, in.uv) * material.color * in.color;
    var s: Surface;
    s.albedo = base.rgb;
    s.alpha = base.a;
    s.world = in.world;
    s.uv = in.uv;
    s.color = in.color;
    s.view = normalize(frame.camera.xyz - in.world);
    s.screen = in.clip.xy;
    var n = normalize(in.normal);
    if !front {
        n = -n; // the back of a double-sided surface
    }
    if material.maps.x != 0u {
        // Tangent space from screen derivatives (Schüler's cotangent frame):
        // no tangents needed in the mesh.
        let dp1 = dpdx(in.world);
        let dp2 = dpdy(in.world);
        let duv1 = dpdx(in.uv);
        let duv2 = dpdy(in.uv);
        let dp2perp = cross(dp2, n);
        let dp1perp = cross(n, dp1);
        let t = dp2perp * duv1.x + dp1perp * duv2.x;
        let b = dp2perp * duv1.y + dp1perp * duv2.y;
        let inv = inverseSqrt(max(max(dot(t, t), dot(b, b)), 1e-20));
        var m = textureSample(normal_texture, material_sampler, in.uv).xyz * 2.0 - 1.0;
        m = vec3<f32>(m.xy * material.params.z, m.z);
        // glTF's texture coordinates run down the image: flip the bitangent.
        n = normalize(t * inv * m.x - b * inv * m.y + n * m.z);
    }
    s.normal = n;
    s.roughness = material.params.x;
    s.metallic = material.params.y;
    s.occlusion = 1.0;
    if material.maps.y != 0u {
        let orm = textureSample(orm_texture, material_sampler, in.uv);
        s.occlusion = orm.r;
        s.roughness *= orm.g;
        s.metallic *= orm.b;
    }
    s.emissive = material.emissive.rgb;
    if material.maps.z != 0u {
        s.emissive *= textureSample(emissive_texture, material_sampler, in.uv).rgb;
    }
    s = surface(s);
    s.roughness = clamp(s.roughness, 0.03, 1.0);
    s.metallic = clamp(s.metallic, 0.0, 1.0);
    return s;
}

// The depth prepass and the shadow map: depth only, but cut-outs still cut.
@fragment
fn fs_depth(in: VertexOut, @builtin(front_facing) front: bool) {
    if surface_at(in, front).alpha < 0.5 {
        discard;
    }
}

// How much sun reaches `world` (0 in shadow, 1 lit), softened over 3x3 texels.
fn sunlight(world: vec3<f32>, n: vec3<f32>) -> f32 {
    if frame.shadow.x == 0.0 {
        return 1.0;
    }
    // Looking up a little off the surface keeps it from shadowing itself.
    let clip = frame.sun_view_proj * vec4<f32>(world + n * frame.shadow.z, 1.0);
    let ndc = clip.xyz / clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    var lit = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let at = uv + vec2<f32>(f32(x), f32(y)) * frame.shadow.y;
            lit += textureSampleCompareLevel(shadow_map, shadow_sampler, at, ndc.z);
        }
    }
    lit /= 9.0;
    // Outside the map, and towards its far edge, shadows fade out.
    let inside = all(uv > vec2<f32>(0.0)) && all(uv < vec2<f32>(1.0)) && ndc.z < 1.0;
    let far = distance(world, frame.camera.xyz) / frame.shadow.w;
    let fade = select(1.0, smoothstep(0.8, 1.0, far), inside);
    return mix(lit, 1.0, fade);
}

fn shade(s: Surface) -> vec3<f32> {
    let n = s.normal;
    let v = s.view;
    let l = frame.sun_dir.xyz;
    let h = normalize(l + v);
    let n_l = max(dot(n, l), 0.0);
    let n_v = max(dot(n, v), 1e-4);
    let n_h = max(dot(n, h), 0.0);
    let v_h = max(dot(v, h), 0.0);

    let a = s.roughness * s.roughness;
    let a2 = a * a;
    let d_denom = n_h * n_h * (a2 - 1.0) + 1.0;
    let d = a2 / (PI * d_denom * d_denom);
    let k = (s.roughness + 1.0) * (s.roughness + 1.0) / 8.0;
    let g = n_v / (n_v * (1.0 - k) + k) * n_l / (n_l * (1.0 - k) + k);
    let f0 = mix(vec3<f32>(0.04), s.albedo, s.metallic);
    let f = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - v_h, 5.0);
    let specular = d * g * f / max(4.0 * n_v * n_l, 1e-4);
    let diffuse = (vec3<f32>(1.0) - f) * (1.0 - s.metallic) * s.albedo / PI;
    let direct = (diffuse + specular) * frame.sun_color.rgb * n_l * sunlight(s.world, n);

    // Sky from above, ground from below; metals reflect it in their color.
    let ambient_light = mix(frame.ground.rgb, frame.sky.rgb, n.y * 0.5 + 0.5);
    let reflectance = s.albedo * (1.0 - s.metallic) + f0 * mix(0.3, 1.0, s.metallic) * (1.0 - s.roughness * 0.7);
    let ambient = ambient_light * reflectance * s.occlusion;

    // Linear HDR light; the post pass maps it to the screen.
    return direct + ambient + s.emissive;
}

// Opaque: the prepass already cut the cut-outs.
@fragment
fn fs_main(in: VertexOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    return vec4<f32>(shade(surface_at(in, front)), 1.0);
}

// Transparent: blended over what's behind it (premultiplied alpha).
@fragment
fn fs_transparent(in: VertexOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let s = surface_at(in, front);
    return vec4<f32>(shade(s) * s.alpha, s.alpha);
}
