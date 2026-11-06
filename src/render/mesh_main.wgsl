// The entry points of every material shader (see mesh.wgsl).

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) m0: vec4<f32>,
    @location(4) m1: vec4<f32>,
    @location(5) m2: vec4<f32>,
    @location(6) m3: vec4<f32>,
    @location(7) color: vec4<f32>,  // the model's tint
    @location(8) vertex_color: vec4<f32>,
    @location(9) uv1: vec2<f32>,
};

struct VertexOut {
    // Invariant: the depth prepass and the main pass must land on exactly the same depth.
    @builtin(position) @invariant clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) uv1: vec2<f32>,
    @location(5) @interpolate(flat) origin: vec3<f32>,
    // Which decal layers the model takes (see apply_decals).
    @location(6) @interpolate(flat) decal_layers: u32,
    // The crossfade dither (see `fade_keeps`), else 0.
    @location(7) @interpolate(flat) fade: f32,
};

// The vertex in world space, after the shader's `vertex` function.
fn world_vertex(in: VertexIn) -> Vertex {
    // The matrix's last row is (0, 0, 0, 1); the first slot of it carries the
    // model's decal layers instead (see instance_decal_layers).
    let model = mat4x4<f32>(vec4<f32>(in.m0.xyz, 0.0), vec4<f32>(in.m1.xyz, 0.0), vec4<f32>(in.m2.xyz, 0.0), vec4<f32>(in.m3.xyz, 1.0));
    // For a scale-rotate matrix M = R S, the normal matrix R S^-1 is M S^-2.
    let scale = vec3<f32>(length(in.m0.xyz), length(in.m1.xyz), length(in.m2.xyz));
    let normal = mat3x3<f32>(in.m0.xyz, in.m1.xyz, in.m2.xyz) * (in.normal / max(scale * scale, vec3<f32>(1e-12)));
    var v: Vertex;
    v.position = (model * vec4<f32>(in.position, 1.0)).xyz;
    v.normal = normal;
    v.uv = in.uv;
    v.color = in.color * in.vertex_color;
    v.uv1 = in.uv1;
    v.origin = in.m3.xyz;
    var out = vertex(v);
    out.origin = v.origin;
    return out;
}

// The decal layers the model takes, as the engine put them in the instance.
fn instance_decal_layers(in: VertexIn) -> u32 {
    return u32(in.m0.w + 0.5);
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    let v = world_vertex(in);
    var out: VertexOut;
    out.decal_layers = instance_decal_layers(in);
    out.clip = frame.view_proj * vec4<f32>(v.position, 1.0);
    out.world = v.position;
    out.normal = v.normal;
    out.uv = v.uv;
    out.color = v.color;
    out.uv1 = v.uv1;
    out.origin = v.origin;
    out.fade = in.m1.w;
    return out;
}

// The shadow map: depth seen from the sun.
@vertex
fn vs_shadow(in: VertexIn) -> VertexOut {
    let v = world_vertex(in);
    var out: VertexOut;
    out.decal_layers = 0u;
    out.clip = frame.caster_view_proj * vec4<f32>(v.position, 1.0);
    out.world = v.position;
    out.normal = v.normal;
    out.uv = v.uv;
    out.color = v.color;
    out.uv1 = v.uv1;
    out.origin = v.origin;
    out.fade = in.m1.w;
    return out;
}

// Whether a pixel of a crossfading instance is drawn: a screen-door dither.
// `fade` is the share of pixels kept; negative for the level fading out,
// which keeps the pixels the level fading in doesn't (so together they
// cover every pixel once). The depth prepass, the main pass and the shadow
// pass all use this on the pixel they draw, so they agree.
fn fade_keeps(fade: f32, pixel: vec2<f32>) -> bool {
    if fade == 0.0 {
        return true;
    }
    // Interleaved gradient noise (Jimenez): an even spread of 0..1 per pixel.
    let p = floor(pixel);
    let t = fract(52.9829189 * fract(dot(p, vec2<f32>(0.06711056, 0.00583715))));
    if fade > 0.0 {
        return t < fade;
    }
    return t >= 1.0 + fade;
}

// The surface from the material and its textures, then the shader's `surface`.
fn surface_at(in: VertexOut, front: bool) -> Surface {
    let base = textureSample(base_texture, material_sampler, in.uv) * material.color * in.color;
    var s: Surface;
    s.albedo = base.rgb;
    s.alpha = base.a;
    s.world = in.world;
    s.uv = in.uv;
    s.uv1 = in.uv1;
    s.origin = in.origin;
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

// The depth prepass and the shadow map of cut-out materials (Blend::Cutout):
// depth only, minus the pixels their alpha cuts out. Solid materials use no
// fragment shader there.
@fragment
fn fs_depth(in: VertexOut, @builtin(front_facing) front: bool) {
    if surface_at(in, front).alpha < 0.5 {
        discard;
    }
}

// The same for crossfading instances (see `fade_keeps`).
@fragment
fn fs_depth_fade(in: VertexOut, @builtin(front_facing) front: bool) {
    if !fade_keeps(in.fade, in.clip.xy) {
        discard;
    }
    if surface_at(in, front).alpha < 0.5 {
        discard;
    }
}

// The depth prepass and shadow map of a crossfading instance of a solid
// material: only the dither.
@fragment
fn fs_fade(in: VertexOut) {
    if !fade_keeps(in.fade, in.clip.xy) {
        discard;
    }
}

// Where `world` falls in shadow cascade `i`: the shadow map uv, the depth
// to compare, and how near the edge of the cascade's box it is (0 at the
// center, 1 at the edge, more outside).
fn cascade_coords(i: i32, world: vec3<f32>, n: vec3<f32>) -> vec4<f32> {
    // Looking up a little off the surface keeps it from shadowing itself.
    let offset = n * frame.shadow.z * frame.cascade_texel[i];
    let clip = frame.sun_view_proj[i] * vec4<f32>(world + offset, 1.0);
    let ndc = clip.xyz / clip.w;
    let edge = max(max(abs(ndc.x), abs(ndc.y)), select(0.0, 2.0, ndc.z > 1.0));
    return vec4<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5, ndc.z, edge);
}

// How much light cascade `i` lets through at `c` (from `cascade_coords`),
// softened over 3x3 texels.
fn cascade_light(i: i32, c: vec4<f32>) -> f32 {
    var lit = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let at = c.xy + vec2<f32>(f32(x), f32(y)) * frame.shadow.y;
            lit += textureSampleCompareLevel(shadow_map, shadow_sampler, at, i, c.z);
        }
    }
    return lit / 9.0;
}

// How much sun reaches `world` (0 in shadow, 1 lit). The first cascade
// whose box holds the point gives the sharpest shadow; near that box's edge
// it fades into the next cascade, so the step between them doesn't show.
// Only the cascades used are sampled.
fn sunlight(world: vec3<f32>, n: vec3<f32>) -> f32 {
    let count = i32(frame.shadow.x);
    // Towards the shadow distance, shadows fade out.
    let fade = smoothstep(0.85, 1.0, distance(world, frame.camera.xyz) / frame.shadow.w);
    if count == 0 || fade >= 1.0 {
        return 1.0;
    }
    // The 3x3 filter reaches one texel further out than the point.
    let inside = 1.0 - 2.0 * frame.shadow.y;
    var lit = 1.0;
    for (var i = 0; i < count; i++) {
        let c = cascade_coords(i, world, n);
        if c.w < inside {
            lit = cascade_light(i, c);
            let blend = smoothstep(0.85, 1.0, c.w / inside);
            if blend > 0.0 && i + 1 < count {
                lit = mix(lit, cascade_light(i + 1, cascade_coords(i + 1, world, n)), blend);
            }
            break;
        }
    }
    return mix(lit, 1.0, fade);
}

fn shade(s: Surface) -> vec3<f32> {
    let n = s.normal;
    let f0 = mix(vec3<f32>(0.04), s.albedo, s.metallic);
    var sun = 0.0;
    if dot(n, frame.sun_dir.xyz) > 0.0 { sun = sunlight(s.world, n); }
    let direct = direct_light(s, frame.sun_dir.xyz, frame.sun_color.rgb * sun) + punctual_light(s);

    let ambient = environment_light(s, f0);

    // Linear HDR light; the post pass maps it to the screen.
    return direct + ambient + s.emissive;
}

// Opaque: the prepass already cut the cut-outs.
@fragment
fn fs_main(in: VertexOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    // Screen derivatives need uniform control flow: taken before any branch.
    return opaque_color(in, front, dpdx(in.world), dpdy(in.world));
}

// Opaque, for crossfading instances (see `fade_keeps`).
@fragment
fn fs_main_fade(in: VertexOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let dp_x = dpdx(in.world);
    let dp_y = dpdy(in.world);
    if !fade_keeps(in.fade, in.clip.xy) {
        discard;
    }
    return opaque_color(in, front, dp_x, dp_y);
}

fn opaque_color(in: VertexOut, front: bool, dp_x: vec3<f32>, dp_y: vec3<f32>) -> vec4<f32> {
    var s = surface_at(in, front);
    var geometric = normalize(in.normal);
    if !front {
        geometric = -geometric;
    }
    s = apply_decals(s, geometric, in.decal_layers, dp_x, dp_y);
    s.roughness = clamp(s.roughness, 0.03, 1.0);
    s.metallic = clamp(s.metallic, 0.0, 1.0);
    // The opaque pass uses the otherwise unused scene-color slot for GTAO
    // occlusion (zero in the no-scene stand-in). Transparent passes retain
    // the actual scene color and never sample it as occlusion.
    let ao_at = clamp(vec2<i32>(s.screen), vec2<i32>(0), vec2<i32>(textureDimensions(scene_color_texture)) - 1);
    s.occlusion *= 1.0 - textureLoad(scene_color_texture, ao_at, 0).r;
    return vec4<f32>(fog(shade(s), s.world), 1.0);
}

// Transparent: blended over what's behind it (premultiplied alpha).
@fragment
fn fs_transparent(in: VertexOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    return transparent_color(in, front);
}

@fragment
fn fs_transparent_fade(in: VertexOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    if !fade_keeps(in.fade, in.clip.xy) {
        discard;
    }
    return transparent_color(in, front);
}

fn transparent_color(in: VertexOut, front: bool) -> vec4<f32> {
    let s = surface_at(in, front);
    return vec4<f32>(fog(shade(s), s.world) * s.alpha, s.alpha);
}
