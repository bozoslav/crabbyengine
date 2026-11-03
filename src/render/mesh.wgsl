// Instanced 3D meshes: one instance = one Model, lit by a sun (Cook-Torrance
// GGX, metallic-roughness, as glTF) and sky/ground ambient light.

const PI: f32 = 3.14159265;

struct Frame {
    view_proj: mat4x4<f32>,
    camera: vec4<f32>,     // xyz: camera position
    sun_dir: vec4<f32>,    // xyz: unit vector towards the sun
    sun_color: vec4<f32>,  // rgb: linear color times intensity
    sky: vec4<f32>,        // rgb: linear, times ambient intensity
    ground: vec4<f32>,
};

struct Material {
    color: vec4<f32>,      // linear RGBA
    emissive: vec4<f32>,   // linear RGB
    params: vec4<f32>,     // roughness, metallic
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var base_texture: texture_2d<f32>;
@group(1) @binding(1) var base_sampler: sampler;
@group(2) @binding(0) var<uniform> material: Material;

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
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    let model = mat4x4<f32>(in.m0, in.m1, in.m2, in.m3);
    let world = model * vec4<f32>(in.position, 1.0);
    // For a scale-rotate matrix M = R S, the normal matrix R S^-1 is M S^-2.
    let scale = vec3<f32>(length(in.m0.xyz), length(in.m1.xyz), length(in.m2.xyz));
    let normal = mat3x3<f32>(in.m0.xyz, in.m1.xyz, in.m2.xyz) * (in.normal / max(scale * scale, vec3<f32>(1e-12)));
    var out: VertexOut;
    out.clip = frame.view_proj * world;
    out.world = world.xyz;
    out.normal = normal;
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

// Narkowicz's fit of the ACES filmic curve: highlights roll off instead of clipping.
fn aces(x: vec3<f32>) -> vec3<f32> {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let base = textureSample(base_texture, base_sampler, in.uv) * material.color * in.color;
    if base.a < 0.5 {
        discard; // cut-out (leaves, fences)
    }
    let albedo = base.rgb;
    let roughness = material.params.x;
    let metallic = material.params.y;

    let n = normalize(in.normal);
    let v = normalize(frame.camera.xyz - in.world);
    let l = frame.sun_dir.xyz;
    let h = normalize(l + v);
    let n_l = max(dot(n, l), 0.0);
    let n_v = max(dot(n, v), 1e-4);
    let n_h = max(dot(n, h), 0.0);
    let v_h = max(dot(v, h), 0.0);

    let a = roughness * roughness;
    let a2 = a * a;
    let d_denom = n_h * n_h * (a2 - 1.0) + 1.0;
    let d = a2 / (PI * d_denom * d_denom);
    let k = (roughness + 1.0) * (roughness + 1.0) / 8.0;
    let g = n_v / (n_v * (1.0 - k) + k) * n_l / (n_l * (1.0 - k) + k);
    let f0 = mix(vec3<f32>(0.04), albedo, metallic);
    let f = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - v_h, 5.0);
    let specular = d * g * f / max(4.0 * n_v * n_l, 1e-4);
    let diffuse = (vec3<f32>(1.0) - f) * (1.0 - metallic) * albedo / PI;
    let direct = (diffuse + specular) * frame.sun_color.rgb * n_l;

    // Sky from above, ground from below; metals reflect it in their color.
    let ambient_light = mix(frame.ground.rgb, frame.sky.rgb, n.y * 0.5 + 0.5);
    let ambient = ambient_light * (albedo * (1.0 - metallic) + f0 * mix(0.3, 1.0, metallic) * (1.0 - roughness * 0.7));

    let color = direct + ambient + material.emissive.rgb;
    return vec4<f32>(aces(color), 1.0);
}
