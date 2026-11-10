//! 3D from C: `CrabbyTransform3D` and `CrabbyModel` components (plain
//! values, so query columns too), meshes and materials, the 3D camera and
//! the sun.
use std::ffi::c_char;

use ponko::{
    AmbientOcclusion, AntiAliasing, Blend, Camera3D, Color, Compute, ComputePass, Decal3D, Effect, GlobalIllumination,
    GpuBuffer, GraphicsQuality, Lighting, LodGroup, LodLevel, LodPick, LodRenderer, LodSettings, LodShadow,
    MAX_GI_PROBES_PER_FRAME, MAX_LIGHT_SHADOW_VIEWS, MAX_RENDER_SCALE, MIN_RENDER_SCALE, Material, MaterialDesc, Mesh,
    MeshData, Model, PointLight3D, Post, QualityLevel, Quat, ReceivesDecals, ReflectionProbe3D, ScreenEffect, Shader,
    Sky, SpotLight3D, TERRAIN_LAYERS, TERRAIN_LODS, TargetCamera, TerrainMaterialDesc, TerrainTile, TextureArray,
    TextureKind, Tonemap, Transform3D, Vec2, Vec3, Vec4,
};

use crate::ctx::{ctx_access, out, text};
use crate::types::*;
use crate::{CrabbyCtx, CrabbyStatus, CrabbyWorld, boundary, builtin, invalid, invalid_owned};

/// A point or direction in 3D: right-handed, +Y up.
/// ```
/// let up = ponko_ffi::CrabbyVec3 { x: 0.0, y: 1.0, z: 0.0 };
/// assert_eq!(up.y, 1.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CrabbyVec3 {
    /// Right.
    pub x: f32,
    /// Up.
    pub y: f32,
    /// Towards the viewer (the default camera looks along -Z).
    pub z: f32,
}

/// A rotation (unit quaternion). `{0, 0, 0, 1}` is none; make others with
/// `crabby_quat_from_axis_angle`.
/// ```
/// let none = ponko_ffi::crabby_quat_identity();
/// assert_eq!(none.w, 1.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyQuat {
    /// Vector part, x.
    pub x: f32,
    /// Vector part, y.
    pub y: f32,
    /// Vector part, z.
    pub z: f32,
    /// Scalar part.
    pub w: f32,
}

/// The built-in `Transform3D` component, same bytes as in Rust (48, the
/// last 8 unused): a plain value and a query column. Start from
/// `crabby_transform3d_at`.
/// ```
/// let t = ponko_ffi::crabby_transform3d_at(Default::default());
/// assert_eq!((t.scale.x, t.rotation.w), (1.0, 1.0));
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyTransform3D {
    /// Which way it faces.
    pub rotation: CrabbyQuat,
    /// Where it is, in world units.
    pub position: CrabbyVec3,
    /// Size along its own axes.
    pub scale: CrabbyVec3,
    /// Unused; keeps the Rust size.
    pub reserved: [f32; 2],
}

/// Entity PointLight3D: position from its transform; spots point along local -Z.
/// Angles are half angles in radians. Range ignores transform scale.
/// ```
/// use ponko_ffi::*;
/// assert!(std::mem::size_of::<CrabbyPointLight3D>() >= 24);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyPointLight3D {
    /// sRGB color; alpha ignored.
    pub color: CrabbyColor,
    /// Same as Rust `PointLight3D::intensity`.
    pub intensity: f32,
    /// Same as Rust `PointLight3D::range`.
    pub range: f32,
    /// Cast shadows (Rust `PointLight3D::shadows`): six of the frame's
    /// `MAX_LIGHT_SHADOW_VIEWS` shadow views, nearest lights first.
    pub shadows: bool,
    /// Each shadow view's size in the shadow atlas, in texels (Rust
    /// `PointLight3D::shadow_resolution`): 0 sizes it by how big the light
    /// looks; otherwise 64 to 2048, rounded down to a power of two.
    pub shadow_resolution: u32,
}

/// Entity SpotLight3D: position from its transform; spots point along local -Z.
/// Angles are half angles in radians. Range ignores transform scale.
/// ```
/// use ponko_ffi::*;
/// assert!(std::mem::size_of::<CrabbySpotLight3D>() >= 24);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbySpotLight3D {
    /// sRGB color; alpha ignored.
    pub color: CrabbyColor,
    /// Same as Rust `SpotLight3D::intensity`.
    pub intensity: f32,
    /// Same as Rust `SpotLight3D::range`.
    pub range: f32,
    /// Same as Rust `SpotLight3D::inner_angle`.
    pub inner_angle: f32,
    /// Same as Rust `SpotLight3D::outer_angle`.
    pub outer_angle: f32,
    /// Cast shadows (Rust `SpotLight3D::shadows`): one shadow view.
    pub shadows: bool,
    /// Its shadow view's size in the atlas (Rust
    /// `SpotLight3D::shadow_resolution`); 0: automatic.
    pub shadow_resolution: u32,
}

/// A mesh on the GPU (`crabby_ctx_create_mesh`, `crabby_ctx_load_mesh`).
/// ```
/// let m = ponko_ffi::CrabbyMesh { id: 0 };
/// assert_eq!(m.id, 0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrabbyMesh {
    /// Engine-internal id.
    pub id: u32,
}

/// `CrabbyMesh` id of no mesh (Rust `Mesh::NONE`): a model showing it draws
/// nothing, and no mesh is ever made with it. The same as `CRABBY_HANDLE_NONE`.
/// ```
/// assert_eq!(ponko_ffi::CRABBY_MESH_NONE, u32::MAX);
/// ```
pub const CRABBY_MESH_NONE: u32 = u32::MAX;

/// A material (`crabby_ctx_create_material`).
/// ```
/// let m = ponko_ffi::CrabbyMaterial { id: 0 };
/// assert_eq!(m.id, 0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrabbyMaterial {
    /// Engine-internal id.
    pub id: u32,
}

/// The built-in `Model` component: draws `mesh` where the entity's
/// `CrabbyTransform3D` is, its materials' colors times `color`. A plain
/// value and a query column. Start from `crabby_model`.
/// ```
/// let m = ponko_ffi::crabby_model(ponko_ffi::CrabbyMesh { id: 3 });
/// assert_eq!((m.mesh.id, m.color.r), (3, 1.0));
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyModel {
    /// What to draw.
    pub mesh: CrabbyMesh,
    /// Multiplied with the materials' colors (white: no change).
    pub color: CrabbyColor,
}

/// The built-in `TerrainTile` component: a square of heightfield terrain
/// drawn more coarsely further from the camera, spawned with a
/// `CrabbyTransform3D` at its corner instead of a `CrabbyModel`. Made with
/// `crabby_ctx_create_terrain_tile`.
/// ```
/// let t = ponko_ffi::CrabbyTerrainTile { lods: [ponko_ffi::CrabbyMesh { id: 0 }; 4], lod_distance: 256.0 };
/// assert_eq!(t.lods.len(), ponko_ffi::CRABBY_TERRAIN_LODS);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyTerrainTile {
    /// One mesh per detail level, finest first: every height, then every
    /// 2nd, 4th and 8th.
    pub lods: [CrabbyMesh; CRABBY_TERRAIN_LODS],
    /// Up to how far from the camera the finest mesh is drawn; each
    /// doubling of that drops one level.
    pub lod_distance: f32,
}

/// How many detail levels a `CrabbyTerrainTile` has.
pub const CRABBY_TERRAIN_LODS: usize = 4;
const _: () = assert!(CRABBY_TERRAIN_LODS == TERRAIN_LODS);

/// A compute pass shader (`crabby_ctx_load_compute`, `crabby_ctx_create_compute`).
/// ```
/// let c = ponko_ffi::CrabbyCompute { id: 0 };
/// assert_eq!(c.id, 0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrabbyCompute {
    /// Engine-internal id.
    pub id: u32,
}

/// One compute pass run each frame (Rust `ComputePass`): its shader, the
/// render target it updates, and the material its custom values and
/// textures come from (the default material without one).
/// ```
/// let p = ponko_ffi::CrabbyComputePass {
///     has_buffer: false, buffer: Default::default(),
///     compute: ponko_ffi::CrabbyCompute { id: 0 },
///     target: ponko_ffi::CrabbyTexture { id: 0, width: 8, height: 8 },
///     has_material: false,
///     material: Default::default(),
/// };
/// assert!(!p.has_material);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrabbyComputePass {
    /// Whether this pass updates `buffer` instead of `target`.
    pub has_buffer: bool,
    /// Storage buffer target when `has_buffer` is true.
    pub buffer: CrabbyGpuBuffer,
    /// The shader.
    pub compute: CrabbyCompute,
    /// What it updates: from `crabby_ctx_create_render_target`.
    pub target: CrabbyTexture,
    /// Whether `material` is used.
    pub has_material: bool,
    /// Where its values and textures come from.
    pub material: CrabbyMaterial,
}

/// A screen effect shader (`crabby_ctx_load_effect`, `crabby_ctx_create_effect`).
/// ```
/// let e = ponko_ffi::CrabbyEffect { id: 0 };
/// assert_eq!(e.id, 0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrabbyEffect {
    /// Engine-internal id.
    pub id: u32,
}

/// One screen effect run over the lit 3D frame (Rust `ScreenEffect`):
/// its shader, and the material its custom values and textures come from
/// (the default material without one).
/// ```
/// let s = ponko_ffi::CrabbyScreenEffect { effect: ponko_ffi::CrabbyEffect { id: 0 }, has_material: false, material: Default::default() };
/// assert!(!s.has_material);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrabbyScreenEffect {
    /// The shader.
    pub effect: CrabbyEffect,
    /// Whether `material` is used.
    pub has_material: bool,
    /// Where its values and textures come from.
    pub material: CrabbyMaterial,
}

/// A camera drawing the 3D world into a render target each frame (Rust
/// `TargetCamera`). Only what's on the front of the clip plane (`x, y, z`
/// the normal, `w` the offset: kept where `normal . p + w >= 0`) is drawn,
/// when `has_clip_plane`.
/// ```
/// let t = ponko_ffi::CrabbyTargetCamera {
///     camera: ponko_ffi::CrabbyCamera3D { position: Default::default(), rotation: ponko_ffi::crabby_quat_identity(), fov_y: 1.0, near: 0.1, far: 100.0 },
///     target: ponko_ffi::CrabbyTexture { id: 0, width: 64, height: 64 },
///     has_clip_plane: false,
///     clip_plane: [0.0; 4],
/// };
/// assert!(!t.has_clip_plane);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyTargetCamera {
    /// Where it looks from; its aspect ratio follows the target's.
    pub camera: CrabbyCamera3D,
    /// From `crabby_ctx_create_render_target`.
    pub target: CrabbyTexture,
    /// Whether `clip_plane` is used.
    pub has_clip_plane: bool,
    /// The plane: normal (x, y, z) and offset (w).
    pub clip_plane: [f32; 4],
}

/// A material shader (`crabby_ctx_load_shader`, `crabby_ctx_create_shader`).
/// ```
/// let s = ponko_ffi::CrabbyShader { id: 0 };
/// assert_eq!(s.id, 0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrabbyShader {
    /// Engine-internal id.
    pub id: u32,
}

/// What a material is made from (Rust `MaterialDesc`). Start from
/// `crabby_material_desc`. Each texture is used when its `has_` flag is set.
/// `blend`: 0 opaque, 1 transparent.
/// ```
/// let d = ponko_ffi::crabby_material_desc(ponko_ffi::CrabbyColor { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });
/// assert_eq!((d.roughness, d.metallic, d.has_texture, d.normal_strength), (0.6, 0.0, false, 1.0));
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyMaterialDesc {
    /// Base color, multiplied with the texture.
    pub color: CrabbyColor,
    /// Whether `texture` is used.
    pub has_texture: bool,
    /// An image on the surface, by the mesh's texture coordinates.
    pub texture: CrabbyTexture,
    /// 0 mirror-smooth to 1 fully rough.
    pub roughness: f32,
    /// 0 for most things, 1 for bare metal.
    pub metallic: f32,
    /// Light it gives off itself.
    pub emissive: CrabbyColor,
    /// Whether `normal_map` is used.
    pub has_normal_map: bool,
    /// Surface detail as tangent-space normals (OpenGL style, green up).
    pub normal_map: CrabbyTexture,
    /// How strong the normal map is (1: as painted).
    pub normal_strength: f32,
    /// Whether `orm_map` is used.
    pub has_orm_map: bool,
    /// Occlusion (red), roughness (green) and metallic (blue), as glTF.
    pub orm_map: CrabbyTexture,
    /// Whether `emissive_map` is used.
    pub has_emissive_map: bool,
    /// Multiplies `emissive`.
    pub emissive_map: CrabbyTexture,
    /// 0 opaque, 1 transparent, 2 cut out (pixels with alpha below 0.5).
    pub blend: u32,
    /// Draws the back of each triangle too.
    pub double_sided: bool,
    /// Whether `shader` is used (else the built-in one).
    pub has_shader: bool,
    /// A custom shader.
    pub shader: CrabbyShader,
    /// Values for a custom shader: `material.custom[0..4]` there, four floats each.
    pub custom: [f32; 16],
    /// Whether each of `custom_textures` is used.
    pub has_custom_textures: [bool; 2],
    /// `custom_texture0` and `custom_texture1` in a custom shader.
    pub custom_textures: [CrabbyTexture; 2],
    /// Whether each of `custom_arrays` is used.
    pub has_custom_arrays: [bool; 3],
    /// `custom_array0` to `custom_array2` in a custom shader.
    pub custom_arrays: [CrabbyTextureArray; 3],
}

/// Several images of the same size in one texture (Rust `TextureArray`),
/// from `crabby_ctx_create_texture_array` or `crabby_ctx_load_texture_array`.
/// ```
/// let a = ponko_ffi::CrabbyTextureArray { id: 0, width: 64, height: 64, layers: 3 };
/// assert_eq!(a.layers, 3);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CrabbyTextureArray {
    /// Engine-internal id.
    pub id: u32,
    /// Width of each image, in pixels.
    pub width: u32,
    /// Height of each image, in pixels.
    pub height: u32,
    /// How many images it holds.
    pub layers: u32,
}

/// What a terrain material is made from (Rust `TerrainMaterialDesc`):
/// ground layers and a weight map. Start from `crabby_terrain_material_desc`.
/// ```
/// let a = ponko_ffi::CrabbyTextureArray::default();
/// let d = ponko_ffi::crabby_terrain_material_desc(a, a, Default::default(), ponko_ffi::CrabbyVec2 { x: 256.0, y: 256.0 });
/// assert_eq!(d.roughness, 0.9);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyTerrainMaterialDesc {
    /// Each layer's color texture (`CRABBY_TEXTURE_COLOR`), up to `CRABBY_TERRAIN_LAYERS`.
    pub layers: CrabbyTextureArray,
    /// Whether `normals` is used.
    pub has_normals: bool,
    /// The layers' normal maps, one each (`CRABBY_TEXTURE_DATA`).
    pub normals: CrabbyTextureArray,
    /// How strong the normal maps are.
    pub normal_strength: f32,
    /// How much of each layer shows where (`CRABBY_TEXTURE_DATA`): the first
    /// image's RGBA are layers 0-3, the second's 4-7.
    pub weights: CrabbyTextureArray,
    /// The world corner (x, z) under the weight map's top-left pixel.
    pub weights_corner: CrabbyVec2,
    /// How much ground (x, z) the weight map covers.
    pub weights_size: CrabbyVec2,
    /// How many world units each layer's texture covers before it repeats.
    pub layer_size: [f32; CRABBY_TERRAIN_LAYERS],
    /// 0 is mirror-smooth, 1 fully rough.
    pub roughness: f32,
}

/// The most ground layers a terrain material blends.
pub const CRABBY_TERRAIN_LAYERS: usize = 8;
const _: () = assert!(CRABBY_TERRAIN_LAYERS == TERRAIN_LAYERS);
/// `crabby_ctx_create_texture_array` kinds (Rust `TextureKind`): colors, stored as sRGB.
pub const CRABBY_TEXTURE_COLOR: u32 = 0;
/// Numbers read as stored: normal maps, masks, weights.
pub const CRABBY_TEXTURE_DATA: u32 = 1;
const _: () =
    assert!(CRABBY_TEXTURE_COLOR == TextureKind::Color as u32 && CRABBY_TEXTURE_DATA == TextureKind::Data as u32);

/// The 3D camera (Rust `ctx.camera3d`).
/// ```
/// let c = ponko_ffi::CrabbyCamera3D { position: Default::default(), rotation: ponko_ffi::crabby_quat_identity(), fov_y: 1.0, near: 0.1, far: 100.0 };
/// assert!(c.near < c.far);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyCamera3D {
    /// Where it is.
    pub position: CrabbyVec3,
    /// Which way it looks: its -Z axis.
    pub rotation: CrabbyQuat,
    /// Vertical field of view, radians (0 to pi).
    pub fov_y: f32,
    /// Nearest distance drawn (positive).
    pub near: f32,
    /// Farthest distance drawn (more than `near`).
    pub far: f32,
}

/// The sun and the sky's light (Rust `ctx.lighting`).
/// ```
/// let mut l = std::mem::MaybeUninit::<ponko_ffi::CrabbyLighting>::uninit();
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_lighting(std::ptr::null_mut(), l.as_mut_ptr()) } as u32, 1);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyLighting {
    /// Which way sunlight travels (from the sun, towards the ground).
    pub sun_direction: CrabbyVec3,
    /// Color of sunlight.
    pub sun_color: CrabbyColor,
    /// Brightness of sunlight.
    pub sun_intensity: f32,
    /// Light from above, on surfaces facing up.
    pub sky_color: CrabbyColor,
    /// Light from below, on surfaces facing down.
    pub ground_color: CrabbyColor,
    /// Brightness of sky and ground light.
    pub ambient_intensity: f32,
    /// Whether the sun casts shadows.
    pub shadows: bool,
    /// How far from the camera shadows reach.
    pub shadow_distance: f32,
    /// How many shadow maps share that distance, 1 to 4: more keep near
    /// shadows sharp, fewer are cheaper.
    pub shadow_cascades: u32,
    /// Width and height of each of the sun's shadow maps, 256 to 4096.
    pub shadow_resolution: u32,
    /// How many shadow views point and spot lights share, 0 (no shadows
    /// from them) to 24. Default 12.
    pub light_shadow_views: u32,
    /// Width and height of the largest point or spot light shadow view
    /// (what a light near the camera gets), 128 to 2048.
    pub light_shadow_resolution: u32,
    /// Width and height of the atlas the light shadow views are packed
    /// into: a power of two from 1024 to 8192. Default 4096.
    pub light_shadow_atlas: u32,
    /// How often the far shadow cascades are drawn again, in frames, 1 to 8
    /// (1: every frame). Default 2.
    pub shadow_refresh: u32,
    /// How soft the sun's shadow edges are, 1 (one lookup per pixel, the
    /// cheapest) to 3 (nine, the softest). Default 3.
    pub shadow_filter: u32,
    /// Whether `equator_color` is used: the ambient light then goes from
    /// `ground_color` below through `equator_color` on the horizon to
    /// `sky_color` above. Off by default. Only without an atmosphere.
    pub has_equator: bool,
    /// Light from the horizon, on surfaces facing sideways (with `has_equator`).
    pub equator_color: CrabbyColor,
    /// Light bounced off surfaces (global illumination) from a grid of
    /// probes around the camera. Off by default. Needs compute shaders and
    /// 18 texture slots; elsewhere it's skipped.
    pub gi_enabled: bool,
    /// The probe grid's size, in world units. Default 32 x 16 x 32.
    pub gi_extent: CrabbyVec3,
    /// How far apart the probes are. Default 2.
    pub gi_spacing: f32,
    /// How many probes are updated each frame, 1 to 16. Default 4.
    pub gi_probes_per_frame: u32,
    /// How bright the bounced light is (1: physically based). Default 1.
    pub gi_intensity: f32,
}

/// The sky behind the 3D world and the fog in front of it (Rust
/// `ctx.sky`, `Sky`).
/// ```
/// let mut s = std::mem::MaybeUninit::<ponko_ffi::CrabbySky>::uninit();
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_sky(std::ptr::null_mut(), s.as_mut_ptr()) } as u32, 1);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbySky {
    /// A physical sky lit by the sun, which also lights the scene.
    pub atmosphere: bool,
    /// Dust and water in the air, 0 up.
    pub haze: f32,
    /// The sun's size in the sky, in degrees across.
    pub sun_size: f32,
    /// How bright the stars are at night.
    pub stars: f32,
    /// Light at night.
    pub night_light: CrabbyColor,
    /// Fog thickness near the ground, per world unit; 0 is none.
    pub fog_density: f32,
    /// The height the fog lies at.
    pub fog_height: f32,
    /// How quickly the fog thins out above it, in world units.
    pub fog_falloff: f32,
    /// The fog's color without an atmosphere.
    pub fog_color: CrabbyColor,
    /// Distance fog: none closer than this, in world units.
    pub fog_start: f32,
    /// Distance fog: all fog past this; 0 (or not past the start) is none.
    pub fog_end: f32,
    /// How much sunlight the fog scatters toward the camera; 0 is none.
    pub fog_sun_glow: f32,
    /// How tight that glow is around the sun, 0 to under 1.
    pub fog_sun_focus: f32,
    /// Light shafts: how much shadows cut the fog's sun glow, 0 (off) to 1.
    pub light_shafts: f32,
}

/// Which way sunlight travels at `hour` o'clock, for a day and night cycle
/// (Rust `Sky::sun_direction_at`): rising in the east (+X) at 6, highest
/// at noon, setting in the west at 18.
/// ```
/// assert!(ponko_ffi::crabby_sun_direction_at(12.0).y < -0.8);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_sun_direction_at(hour: f32) -> CrabbyVec3 {
    to_c(Sky::sun_direction_at(hour))
}

/// What happens to the lit 3D frame on its way to the screen (Rust
/// `ctx.post`). `tonemap`: 0 ACES (default), 1 AgX, 2 Khronos PBR Neutral,
/// 3 none. `lut` is used when `has_lut`.
/// ```
/// let mut p = std::mem::MaybeUninit::<ponko_ffi::CrabbyPost>::uninit();
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_post(std::ptr::null_mut(), p.as_mut_ptr()) } as u32, 1);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyPost {
    /// The 3D scene's resolution relative to the window, 0.25 to 2: it is
    /// drawn at that scale, then scaled to the window under the sprites and UI.
    pub render_scale: f32,
    /// Sharpening of the scaled image, 0 to 1.
    pub sharpen: f32,
    /// Post AA: 0 none, 1 FXAA, 2 SMAA 1x, 3 temporal (TAA; WebGL2 falls
    /// back to FXAA).
    pub anti_aliasing: u32,
    /// Enable GTAO on ambient light only (unsupported backends ignore it).
    pub ao_enabled: bool,
    /// GTAO world-space radius, above zero.
    pub ao_radius: f32,
    /// GTAO strength, nonnegative.
    pub ao_intensity: f32,
    /// GTAO angular slices, 1 to 8.
    pub ao_slices: u32,
    /// GTAO horizon steps per side, 1 to 16.
    pub ao_steps: u32,
    /// Half-resolution GTAO with bilateral upsampling; contacts stay full resolution.
    pub ao_half_resolution: bool,
    /// Camera-reprojected GTAO accumulation (independent of TAA).
    pub ao_temporal: bool,
    /// Brightness in stops: +1 doubles the light.
    pub exposure: f32,
    /// How light maps to the screen (see above).
    pub tonemap: u32,
    /// How strongly bright light glows; 0 is off.
    pub bloom: f32,
    /// How bright light must be to glow (1 is white).
    pub bloom_threshold: f32,
    /// 1 keeps the contrast.
    pub contrast: f32,
    /// 1 keeps the colors, 0 is grey.
    pub saturation: f32,
    /// White balance, -1 (cool) to 1 (warm).
    pub temperature: f32,
    /// White balance, -1 (green) to 1 (magenta).
    pub tint: f32,
    /// How much the corners darken, 0 to 1.
    pub vignette: f32,
    /// Color fringes toward the edges, 0 to 1.
    pub chromatic_aberration: f32,
    /// Lens bend, -1 (pinch) to 1 (bulge); 0 is off.
    pub lens_distortion: f32,
    /// Zoom after the lens bend, above zero (default 1).
    pub lens_scale: f32,
    /// Whether `lut` is used.
    pub has_lut: bool,
    /// A color lookup table: n squares of n x n pixels in a row.
    pub lut: CrabbyTexture,
    /// How much TAA sharpens its result, 0 to 1 (default 0.25).
    pub taa_sharpness: f32,
    /// Contact shadows: how far each pixel looks toward the sun for
    /// something close blocking it, in world units; 0 (default) is off.
    pub contact_shadows: f32,
    /// Screen-space reflections on smooth surfaces, 0 (default, off) to 1.
    pub ssr: f32,
    /// How far a screen-space reflection looks, in world units, above zero
    /// (default 30).
    pub ssr_max_distance: f32,
}

/// Triangles on the CPU (Rust `MeshData`), before `crabby_ctx_create_mesh`
/// puts them on the GPU. An owned handle: free it with `crabby_mesh_data_free`.
pub struct CrabbyMeshData(pub(crate) MeshData);

/// A read-only look at a `CrabbyMeshData`'s arrays, valid until it's freed.
/// `normals` and `uvs` are null when it has none.
/// ```
/// let v = ponko_ffi::CrabbyMeshView { positions: std::ptr::null(), normals: std::ptr::null(), uvs: std::ptr::null(), vertex_count: 0, indices: std::ptr::null(), index_count: 0 };
/// assert_eq!(v.index_count, 0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CrabbyMeshView {
    /// `vertex_count` corner positions.
    pub positions: *const CrabbyVec3,
    /// `vertex_count` unit normals, or null.
    pub normals: *const CrabbyVec3,
    /// `vertex_count` texture coordinates, or null.
    pub uvs: *const CrabbyVec2,
    /// Corners.
    pub vertex_count: usize,
    /// Three corner indices per triangle.
    pub indices: *const u32,
    /// Indices (three per triangle).
    pub index_count: usize,
}

// ---- layouts ----------------------------------------------------------------

same_layout!(CrabbyVec3, Vec3, x = x, y = y, z = z);
same_layout!(CrabbyModel, Model, mesh = mesh, color = color);
same_layout!(CrabbyTerrainTile, TerrainTile, lods = lods, lod_distance = lod_distance);
const _: () = {
    assert!(size_of::<CrabbyTransform3D>() == size_of::<Transform3D>());
    assert!(std::mem::offset_of!(CrabbyTransform3D, rotation) == std::mem::offset_of!(Transform3D, rotation));
    assert!(std::mem::offset_of!(CrabbyTransform3D, position) == std::mem::offset_of!(Transform3D, position));
    assert!(std::mem::offset_of!(CrabbyTransform3D, scale) == std::mem::offset_of!(Transform3D, scale));
    assert!(size_of::<CrabbyQuat>() == size_of::<Quat>());
    assert!(size_of::<CrabbyMesh>() == size_of::<Mesh>() && size_of::<CrabbyMaterial>() == size_of::<Material>());
};
// SAFETY: both repr(C) { u32 id } (sizes asserted above).
unsafe impl SameLayout<Mesh> for CrabbyMesh {}
// SAFETY: as above.
unsafe impl SameLayout<CrabbyMesh> for Mesh {}
// SAFETY: as above.
unsafe impl SameLayout<Material> for CrabbyMaterial {}
// SAFETY: as above.
unsafe impl SameLayout<CrabbyMaterial> for Material {}

pub(crate) fn quat_to_c(q: Quat) -> CrabbyQuat {
    let [x, y, z, w] = q.to_array();
    CrabbyQuat { x, y, z, w }
}
pub(crate) fn quat_from_c(q: CrabbyQuat) -> Quat {
    Quat::from_xyzw(q.x, q.y, q.z, q.w)
}
pub(crate) fn transform_to_c(t: Transform3D) -> CrabbyTransform3D {
    CrabbyTransform3D {
        rotation: quat_to_c(t.rotation),
        position: to_c(t.position),
        scale: to_c(t.scale),
        reserved: [0.0; 2],
    }
}
pub(crate) fn transform_from_c(t: CrabbyTransform3D) -> Transform3D {
    Transform3D {
        rotation: quat_from_c(t.rotation),
        position: to_c(t.position),
        scale: to_c(t.scale),
    }
}

// ---- values -----------------------------------------------------------------

/// No rotation.
/// ```
/// assert_eq!(ponko_ffi::crabby_quat_identity().w, 1.0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_quat_identity() -> CrabbyQuat {
    quat_to_c(Quat::IDENTITY)
}

/// A rotation of `angle` radians around `axis` (counter-clockwise looking
/// down the axis). A zero axis gives no rotation.
/// ```
/// use ponko_ffi::*;
/// let q = crabby_quat_from_axis_angle(CrabbyVec3 { x: 0.0, y: 1.0, z: 0.0 }, 0.5);
/// assert!((q.y - 0.25f32.sin()).abs() < 1e-6);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_quat_from_axis_angle(axis: CrabbyVec3, angle: f32) -> CrabbyQuat {
    match to_c::<_, Vec3>(axis).try_normalize() {
        Some(axis) => quat_to_c(Quat::from_axis_angle(axis, angle)),
        None => crabby_quat_identity(),
    }
}

/// `a` after `b`: rotating by the result is rotating by `b`, then by `a`.
/// ```
/// use ponko_ffi::*;
/// let y = CrabbyVec3 { x: 0.0, y: 1.0, z: 0.0 };
/// let q = crabby_quat_mul(crabby_quat_from_axis_angle(y, 0.2), crabby_quat_from_axis_angle(y, 0.3));
/// assert!((q.y - crabby_quat_from_axis_angle(y, 0.5).y).abs() < 1e-6);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_quat_mul(a: CrabbyQuat, b: CrabbyQuat) -> CrabbyQuat {
    quat_to_c((quat_from_c(a) * quat_from_c(b)).normalize())
}

/// `v` rotated by `q`.
/// ```
/// use ponko_ffi::*;
/// let q = crabby_quat_from_axis_angle(CrabbyVec3 { x: 0.0, y: 1.0, z: 0.0 }, std::f32::consts::FRAC_PI_2);
/// let v = crabby_quat_rotate(q, CrabbyVec3 { x: 1.0, y: 0.0, z: 0.0 });
/// assert!((v.z + 1.0).abs() < 1e-6);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_quat_rotate(q: CrabbyQuat, v: CrabbyVec3) -> CrabbyVec3 {
    to_c(quat_from_c(q) * to_c::<_, Vec3>(v))
}

/// A transform at `position`, not rotated, scale 1 (Rust `Transform3D::at`).
/// ```
/// let t = ponko_ffi::crabby_transform3d_at(ponko_ffi::CrabbyVec3 { x: 1.0, y: 2.0, z: 3.0 });
/// assert_eq!(t.position.z, 3.0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_transform3d_at(position: CrabbyVec3) -> CrabbyTransform3D {
    transform_to_c(Transform3D::at(to_c(position)))
}

/// `t` turned so its -Z axis points at `target`, +Y up
/// (Rust `Transform3D::looking_at`).
/// ```
/// use ponko_ffi::*;
/// let t = crabby_transform3d_looking_at(crabby_transform3d_at(CrabbyVec3 { x: 0.0, y: 0.0, z: 5.0 }), Default::default());
/// assert!((crabby_transform3d_forward(t).z + 1.0).abs() < 1e-5);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_transform3d_looking_at(t: CrabbyTransform3D, target: CrabbyVec3) -> CrabbyTransform3D {
    transform_to_c(transform_from_c(t).looking_at(to_c(target)))
}

/// The direction `t` faces: its -Z axis.
/// ```
/// let f = ponko_ffi::crabby_transform3d_forward(ponko_ffi::crabby_transform3d_at(Default::default()));
/// assert_eq!(f.z, -1.0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_transform3d_forward(t: CrabbyTransform3D) -> CrabbyVec3 {
    to_c(transform_from_c(t).forward())
}

/// A model of `mesh` in its own colors (Rust `Model::new`).
/// ```
/// assert_eq!(ponko_ffi::crabby_model(ponko_ffi::CrabbyMesh { id: 1 }).color.a, 1.0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_model(mesh: CrabbyMesh) -> CrabbyModel {
    to_c(Model::new(to_c(mesh)))
}

/// A plain material of one color (Rust `MaterialDesc::color`): roughness
/// 0.6, not metallic, no texture, no glow.
/// ```
/// let d = ponko_ffi::crabby_material_desc(ponko_ffi::CrabbyColor { r: 1.0, g: 1.0, b: 1.0, a: 1.0 });
/// assert_eq!(d.emissive.r, 0.0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_material_desc(color: CrabbyColor) -> CrabbyMaterialDesc {
    material_to_c(&MaterialDesc::color(to_c(color)))
}

/// A Rust material description as C's.
fn material_to_c(d: &MaterialDesc) -> CrabbyMaterialDesc {
    let none = CrabbyTexture {
        id: 0,
        width: 0,
        height: 0,
    };
    let map = |t: Option<ponko::Texture>| (t.is_some(), t.map_or(none, texture_to_c));
    let (has_texture, texture) = map(d.texture);
    let (has_normal_map, normal_map) = map(d.normal_map);
    let (has_orm_map, orm_map) = map(d.orm_map);
    let (has_emissive_map, emissive_map) = map(d.emissive_map);
    let customs = d.custom_textures.map(map);
    let mut custom = [0.0; 16];
    for (i, v) in d.custom.iter().enumerate() {
        custom[i * 4..i * 4 + 4].copy_from_slice(&v.to_array());
    }
    CrabbyMaterialDesc {
        color: to_c(d.color),
        has_texture,
        texture,
        roughness: d.roughness,
        metallic: d.metallic,
        emissive: to_c(d.emissive),
        has_normal_map,
        normal_map,
        normal_strength: d.normal_strength,
        has_orm_map,
        orm_map,
        has_emissive_map,
        emissive_map,
        blend: match d.blend {
            Blend::Opaque => 0,
            Blend::Transparent => 1,
            Blend::Cutout => 2,
        },
        double_sided: d.double_sided,
        has_shader: d.shader.is_some(),
        shader: CrabbyShader {
            id: d.shader.map_or(0, |s| s.id()),
        },
        custom,
        has_custom_textures: customs.map(|c| c.0),
        custom_textures: customs.map(|c| c.1),
        has_custom_arrays: d.custom_arrays.map(|a| a.is_some()),
        custom_arrays: d
            .custom_arrays
            .map(|a| a.map_or_else(CrabbyTextureArray::default, array_to_c)),
    }
}

/// A C material description as Rust's, checked.
fn material_from_c(c: &ponko::Ctx, d: &CrabbyMaterialDesc) -> Result<MaterialDesc, crate::Failure> {
    if !((0.0..=1.0).contains(&d.roughness) && (0.0..=1.0).contains(&d.metallic)) {
        return Err(invalid("roughness and metallic go from 0 to 1"));
    }
    let blend = match d.blend {
        0 => Blend::Opaque,
        1 => Blend::Transparent,
        2 => Blend::Cutout,
        _ => return Err(invalid("blend must be 0 (opaque), 1 (transparent) or 2 (cut out)")),
    };
    let shader = d.has_shader.then_some(Shader::from_id(d.shader.id));
    if shader.is_some_and(|s| !c.gfx.has_shader(s)) {
        return Err(invalid("that shader wasn't made by this game"));
    }
    let map = |has: bool, t: CrabbyTexture| has.then(|| texture_from_c(t));
    let arrays = std::array::from_fn(|i| d.has_custom_arrays[i].then(|| array_from_c(d.custom_arrays[i])));
    if arrays.iter().flatten().any(|a| !c.gfx.has_texture_array(*a)) {
        return Err(invalid("that texture array wasn't made by this game"));
    }
    let v = |i: usize| {
        Vec4::new(
            d.custom[i * 4],
            d.custom[i * 4 + 1],
            d.custom[i * 4 + 2],
            d.custom[i * 4 + 3],
        )
    };
    Ok(MaterialDesc {
        color: to_c(d.color),
        texture: map(d.has_texture, d.texture),
        roughness: d.roughness,
        metallic: d.metallic,
        emissive: to_c(d.emissive),
        normal_map: map(d.has_normal_map, d.normal_map),
        normal_strength: d.normal_strength,
        orm_map: map(d.has_orm_map, d.orm_map),
        emissive_map: map(d.has_emissive_map, d.emissive_map),
        blend,
        double_sided: d.double_sided,
        shader,
        custom: [v(0), v(1), v(2), v(3)],
        custom_textures: [
            map(d.has_custom_textures[0], d.custom_textures[0]),
            map(d.has_custom_textures[1], d.custom_textures[1]),
        ],
        custom_arrays: arrays,
    })
}

const _: () = assert!(
    size_of::<CrabbyTextureArray>() == size_of::<TextureArray>()
        && align_of::<CrabbyTextureArray>() == align_of::<TextureArray>()
);
fn array_from_c(a: CrabbyTextureArray) -> TextureArray {
    // SAFETY: both repr(C), four u32 in the same order (id, width, height,
    // layers); unknown ids are refused by `has_texture_array` or read as white.
    unsafe { std::mem::transmute(a) }
}
fn array_to_c(a: TextureArray) -> CrabbyTextureArray {
    // SAFETY: as above.
    unsafe { std::mem::transmute(a) }
}

/// A terrain material description with the defaults (Rust
/// `TerrainMaterialDesc::new`): ground `layers` blended by a `weights` map
/// lying over `size` world units from `corner`, no normal maps, each layer
/// repeating every 4 units, roughness 0.9.
/// ```
/// let a = ponko_ffi::CrabbyTextureArray::default();
/// let d = ponko_ffi::crabby_terrain_material_desc(a, a, Default::default(), ponko_ffi::CrabbyVec2 { x: 1.0, y: 1.0 });
/// assert_eq!(d.layer_size[7], 4.0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_terrain_material_desc(
    layers: CrabbyTextureArray,
    weights: CrabbyTextureArray,
    corner: CrabbyVec2,
    size: CrabbyVec2,
) -> CrabbyTerrainMaterialDesc {
    let d = TerrainMaterialDesc::new(array_from_c(layers), array_from_c(weights), to_c(corner), to_c(size));
    CrabbyTerrainMaterialDesc {
        layers,
        has_normals: false,
        normals: CrabbyTextureArray::default(),
        normal_strength: d.normal_strength,
        weights,
        weights_corner: corner,
        weights_size: size,
        layer_size: d.layer_size,
        roughness: d.roughness,
    }
}

/// Makes a texture array of `width` x `height` images (Rust
/// `Graphics::create_texture_array`): `rgba` holds them one after another, 4
/// bytes a pixel, `len` bytes in all (a whole number of images). `kind` is
/// `CRABBY_TEXTURE_COLOR` or `CRABBY_TEXTURE_DATA`.
/// # Safety
/// `ctx` null or live; `rgba` holds `len` bytes; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_create_texture_array(std::ptr::null_mut(), 1, 1, std::ptr::null(), 0, 0, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_texture_array(
    ctx: *mut CrabbyCtx,
    width: u32,
    height: u32,
    rgba: *const u8,
    len: usize,
    kind: u32,
    out_array: *mut CrabbyTextureArray,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            if rgba.is_null() || out_array.is_null() {
                return Err(invalid("pixels or output is null"));
            }
            let kind = kind_from_c(kind)?;
            let image = width as usize * height as usize * 4;
            if image == 0 || len == 0 || !len.is_multiple_of(image) {
                return Err(invalid_owned(format!(
                    "{len} bytes isn't a whole number of {width} x {height} RGBA images"
                )));
            }
            let bytes = std::slice::from_raw_parts(rgba, len);
            let layers: Vec<&[u8]> = bytes.chunks_exact(image).collect();
            let array = c
                .gfx
                .try_create_texture_array(width, height, &layers, kind)
                .map_err(invalid_owned)?;
            out(out_array, array_to_c(array))
        })
    })
}

/// Loads `count` images from assets/, all the same size, into a texture
/// array, one layer each (Rust `Graphics::load_texture_array`). `kind` is
/// `CRABBY_TEXTURE_COLOR` or `CRABBY_TEXTURE_DATA`.
/// # Safety
/// `ctx` null or live; `paths` holds `count` NUL-terminated strings; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_load_texture_array(std::ptr::null_mut(), std::ptr::null(), 0, 0, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_texture_array(
    ctx: *mut CrabbyCtx,
    paths: *const *const c_char,
    count: usize,
    kind: u32,
    out_array: *mut CrabbyTextureArray,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            if paths.is_null() || out_array.is_null() {
                return Err(invalid("paths or output is null"));
            }
            let kind = kind_from_c(kind)?;
            let names = std::slice::from_raw_parts(paths, count)
                .iter()
                .map(|&p| text(p))
                .collect::<Result<Vec<&str>, _>>()?;
            let array = c.gfx.try_load_texture_array(&names, kind).map_err(invalid_owned)?;
            out(out_array, array_to_c(array))
        })
    })
}

fn kind_from_c(kind: u32) -> Result<TextureKind, crate::Failure> {
    match kind {
        CRABBY_TEXTURE_COLOR => Ok(TextureKind::Color),
        CRABBY_TEXTURE_DATA => Ok(TextureKind::Data),
        _ => Err(invalid(
            "kind must be CRABBY_TEXTURE_COLOR (0) or CRABBY_TEXTURE_DATA (1)",
        )),
    }
}

/// Makes a terrain material (Rust `Graphics::create_terrain_material`): an
/// ordinary material, for `crabby_ctx_create_terrain_tile` or any mesh.
/// # Safety
/// `ctx` null or live; `desc` readable; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_create_terrain_material(std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_terrain_material(
    ctx: *mut CrabbyCtx,
    desc: *const CrabbyTerrainMaterialDesc,
    out_material: *mut CrabbyMaterial,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let Some(d) = desc.as_ref() else {
                return Err(invalid("description is null"));
            };
            if out_material.is_null() {
                return Err(invalid("output is null"));
            }
            let rust = TerrainMaterialDesc {
                layers: array_from_c(d.layers),
                normals: d.has_normals.then(|| array_from_c(d.normals)),
                normal_strength: d.normal_strength,
                weights: array_from_c(d.weights),
                weights_corner: to_c(d.weights_corner),
                weights_size: to_c(d.weights_size),
                layer_size: d.layer_size,
                roughness: d.roughness,
            };
            let material = c.gfx.try_create_terrain_material(&rust).map_err(invalid_owned)?;
            out(out_material, to_c(material))
        })
    })
}

// ---- components -------------------------------------------------------------

/// The id of the built-in `Transform3D` component (`CrabbyTransform3D`),
/// registering it if needed.
/// # Safety
/// Live handle and writable output.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe { let mut id = 99; assert_eq!(crabby_transform3d_component(w, &mut id), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_transform3d_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { builtin::<Transform3D>(world, out_id) }
}

/// The id of the built-in `Model` component (`CrabbyModel`), registering
/// it if needed.
/// # Safety
/// Live handle and writable output.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe { let mut id = 99; assert_eq!(crabby_model_component(w, &mut id), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_model_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { builtin::<Model>(world, out_id) }
}

/// The id of the built-in `TerrainTile` component (`CrabbyTerrainTile`),
/// registering it if needed.
/// # Safety
/// Live handle and writable output.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe { let mut id = 99; assert_eq!(crabby_terrain_tile_component(w, &mut id), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_terrain_tile_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { builtin::<TerrainTile>(world, out_id) }
}

// ---- mesh data --------------------------------------------------------------

fn boxed(data: MeshData) -> *mut CrabbyMeshData {
    Box::into_raw(Box::new(CrabbyMeshData(data)))
}

/// Copies triangles into new mesh data: `vertex_count` positions, normals
/// and texture coordinates (each may be null: normals are then computed,
/// coordinates zero) and `index_count` indices, three per triangle,
/// counter-clockwise seen from the front. Null (see `crabby_last_error`)
/// if they don't fit together.
/// # Safety
/// Non-null arrays hold the stated counts.
/// ```
/// use ponko_ffi::*;
/// let p = [CrabbyVec3 { x: 0.0, y: 0.0, z: 0.0 }, CrabbyVec3 { x: 1.0, y: 0.0, z: 0.0 }, CrabbyVec3 { x: 0.0, y: 1.0, z: 0.0 }];
/// let d = unsafe { crabby_mesh_data_new(p.as_ptr(), std::ptr::null(), std::ptr::null(), 3, [0, 1, 2].as_ptr(), 3) };
/// assert!(!d.is_null());
/// unsafe { crabby_mesh_data_free(d) };
/// assert!(unsafe { crabby_mesh_data_new(p.as_ptr(), std::ptr::null(), std::ptr::null(), 3, [0, 1, 5].as_ptr(), 3) }.is_null());
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_mesh_data_new(
    positions: *const CrabbyVec3,
    normals: *const CrabbyVec3,
    uvs: *const CrabbyVec2,
    vertex_count: usize,
    indices: *const u32,
    index_count: usize,
) -> *mut CrabbyMeshData {
    let mut result = std::ptr::null_mut();
    boundary(|| {
        if (positions.is_null() && vertex_count > 0) || (indices.is_null() && index_count > 0) {
            return Err(invalid("positions or indices are null"));
        }
        // SAFETY: caller passes arrays of the stated lengths (null only when optional).
        let slice = |ptr: *const CrabbyVec3| unsafe {
            if ptr.is_null() || vertex_count == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(ptr.cast::<Vec3>(), vertex_count).to_vec()
            }
        };
        let data = MeshData {
            positions: slice(positions),
            normals: slice(normals),
            // SAFETY: as above.
            uvs: unsafe {
                if uvs.is_null() || vertex_count == 0 {
                    Vec::new()
                } else {
                    std::slice::from_raw_parts(uvs.cast::<Vec2>(), vertex_count).to_vec()
                }
            },
            // SAFETY: as above.
            indices: unsafe {
                if index_count == 0 {
                    Vec::new()
                } else {
                    std::slice::from_raw_parts(indices, index_count).to_vec()
                }
            },
            ..Default::default()
        };
        data.check().map_err(invalid_owned)?;
        result = boxed(data);
        Ok(())
    });
    result
}

/// A cube `size` wide around the origin (Rust `MeshData::cube`).
/// ```
/// let d = ponko_ffi::crabby_mesh_data_cube(1.0);
/// unsafe { ponko_ffi::crabby_mesh_data_free(d) };
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_mesh_data_cube(size: f32) -> *mut CrabbyMeshData {
    boxed(MeshData::cube(size))
}

/// A flat square `size` wide on the XZ plane, facing up (Rust `MeshData::plane`).
/// ```
/// let d = ponko_ffi::crabby_mesh_data_plane(10.0);
/// unsafe { ponko_ffi::crabby_mesh_data_free(d) };
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_mesh_data_plane(size: f32) -> *mut CrabbyMeshData {
    boxed(MeshData::plane(size))
}

/// A sphere of `radius` from `segments` slices around (at least 3; Rust
/// `MeshData::sphere`).
/// ```
/// let d = ponko_ffi::crabby_mesh_data_sphere(0.5, 24);
/// unsafe { ponko_ffi::crabby_mesh_data_free(d) };
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_mesh_data_sphere(radius: f32, segments: u32) -> *mut CrabbyMeshData {
    boxed(MeshData::sphere(radius, segments))
}

/// Whether `count` heights make a square grid of at least 2 x 2.
pub(crate) fn square_grid(count: usize) -> bool {
    let n = (count as f64).sqrt() as usize;
    n >= 2 && n * n == count
}

/// A square of terrain `size` wide from `count` heights: n x n of them, row
/// after row along +Z, NaN for a hole; every `step`-th height is used, with
/// skirts at the edges (Rust `MeshData::heightfield`). Null if the heights
/// aren't a square grid of at least 2 x 2 or `step` is 0.
/// # Safety
/// `heights` holds `count` floats.
/// ```
/// let h = [0.0f32, 0.0, 0.0, 1.0];
/// let d = unsafe { ponko_ffi::crabby_mesh_data_heightfield(h.as_ptr(), 4, 2.0, 1) };
/// assert!(!d.is_null());
/// assert!(unsafe { ponko_ffi::crabby_mesh_data_heightfield(h.as_ptr(), 3, 2.0, 1) }.is_null());
/// unsafe { ponko_ffi::crabby_mesh_data_free(d) };
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_mesh_data_heightfield(
    heights: *const f32,
    count: usize,
    size: f32,
    step: u32,
) -> *mut CrabbyMeshData {
    if heights.is_null() || !square_grid(count) || step == 0 {
        return std::ptr::null_mut();
    }
    // SAFETY: the caller's contract: `count` floats at `heights`.
    let heights = unsafe { std::slice::from_raw_parts(heights, count) };
    boxed(MeshData::heightfield(heights, size, step as usize))
}

/// Gives mesh data one color per corner (Rust `MeshData::colors`),
/// multiplying its material's color; `count` must be its number of corners,
/// or 0 to remove them.
/// # Safety
/// `data` null or live; `colors` holds `count` colors (may be null when 0).
/// ```
/// use ponko_ffi::{CrabbyStatus::*, *};
/// let d = crabby_mesh_data_plane(1.0);
/// let red = [CrabbyColor { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }; 4];
/// assert_eq!(unsafe { crabby_mesh_data_set_colors(d, red.as_ptr(), 4) }, CRABBY_OK);
/// assert_eq!(unsafe { crabby_mesh_data_set_colors(d, red.as_ptr(), 3) }, CRABBY_INVALID_ARGUMENT);
/// unsafe { crabby_mesh_data_free(d) };
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_mesh_data_set_colors(
    data: *mut CrabbyMeshData,
    colors: *const CrabbyColor,
    count: usize,
) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: caller contract: null or live.
        let Some(data) = (unsafe { data.as_mut() }) else {
            return Err(invalid("mesh data is null"));
        };
        if colors.is_null() && count > 0 {
            return Err(invalid("colors is null"));
        }
        if count != 0 && count != data.0.positions.len() {
            return Err(invalid_owned(format!(
                "{count} colors for {} corners",
                data.0.positions.len()
            )));
        }
        data.0.colors = if count == 0 {
            Vec::new()
        } else {
            // SAFETY: caller passes `count` colors.
            unsafe { std::slice::from_raw_parts(colors, count) }
                .iter()
                .map(|&c| to_c(c))
                .collect()
        };
        Ok(())
    })
}

/// Frees mesh data. Null is ignored.
/// # Safety
/// `data` must be null or live, and not used afterwards.
/// ```
/// unsafe { ponko_ffi::crabby_mesh_data_free(std::ptr::null_mut()) };
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_mesh_data_free(data: *mut CrabbyMeshData) {
    if !data.is_null() {
        // SAFETY: caller passes mesh data from a crabby_mesh_data_* call, once.
        drop(unsafe { Box::from_raw(data) });
    }
}

/// Looks at mesh data's arrays.
/// # Safety
/// `data` live; `out_view` writable.
/// ```
/// use ponko_ffi::*;
/// let d = crabby_mesh_data_cube(1.0);
/// let mut v = std::mem::MaybeUninit::<CrabbyMeshView>::uninit();
/// assert_eq!(unsafe { crabby_mesh_data_view(d, v.as_mut_ptr()) }, CrabbyStatus::CRABBY_OK);
/// assert_eq!(unsafe { v.assume_init() }.index_count, 36);
/// unsafe { crabby_mesh_data_free(d) };
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_mesh_data_view(
    data: *const CrabbyMeshData,
    out_view: *mut CrabbyMeshView,
) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: caller passes null or live mesh data.
        let Some(data) = (unsafe { data.as_ref() }) else {
            return Err(invalid("mesh data is null"));
        };
        let d = &data.0;
        let ptr = |v: &[Vec3]| {
            if v.is_empty() {
                std::ptr::null()
            } else {
                v.as_ptr().cast::<CrabbyVec3>()
            }
        };
        // SAFETY: caller passes a writable output.
        unsafe {
            out(
                out_view,
                CrabbyMeshView {
                    positions: ptr(&d.positions),
                    normals: ptr(&d.normals),
                    uvs: if d.uvs.is_empty() {
                        std::ptr::null()
                    } else {
                        d.uvs.as_ptr().cast()
                    },
                    vertex_count: d.positions.len(),
                    indices: d.indices.as_ptr(),
                    index_count: d.indices.len(),
                },
            )
        }
    })
}

// ---- ctx ----------------------------------------------------------------------

/// Puts a material on the GPU. Roughness and metallic go from 0 to 1.
/// # Safety
/// `ctx` null or live; pointers valid.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_create_material(std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_material(
    ctx: *mut CrabbyCtx,
    desc: *const CrabbyMaterialDesc,
    out_material: *mut CrabbyMaterial,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let Some(d) = desc.as_ref() else {
                return Err(invalid("material desc is null"));
            };
            if out_material.is_null() {
                return Err(invalid("output is null"));
            }
            let desc = material_from_c(c, d)?;
            out(out_material, to_c(c.gfx.create_material(desc)))
        })
    })
}

/// Changes a material: every model drawn with it changes too.
/// # Safety
/// `ctx` null or live; `desc` valid.
/// ```
/// let m = ponko_ffi::CrabbyMaterial { id: 0 };
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_update_material(std::ptr::null_mut(), m, std::ptr::null()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_update_material(
    ctx: *mut CrabbyCtx,
    material: CrabbyMaterial,
    desc: *const CrabbyMaterialDesc,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let Some(d) = desc.as_ref() else {
                return Err(invalid("material desc is null"));
            };
            let material: Material = to_c(material);
            if !c.gfx.has_material(material) {
                return Err(invalid("that material wasn't made by this game"));
            }
            let desc = material_from_c(c, d)?;
            c.gfx.update_material(material, desc);
            Ok(())
        })
    })
}

/// The materials loaded from glTF files so far that the files call `name`,
/// with what each was made from (Rust `Graphics::loaded_materials`): to give
/// them a shader of your own with `crabby_ctx_update_material`. Writes up
/// to `capacity` of them to `out_materials` and `out_descs` (either may be
/// null to only count), and how many there are to `out_count`.
/// # Safety
/// `ctx` null or live; `name` a C string; the outputs writable for `capacity` entries.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_loaded_materials(std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_loaded_materials(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    out_materials: *mut CrabbyMaterial,
    out_descs: *mut CrabbyMaterialDesc,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let name = text(name)?;
            if out_count.is_null() {
                return Err(invalid("output is null"));
            }
            let found = c.gfx.loaded_materials(name);
            for (i, (material, desc)) in found.iter().take(capacity).enumerate() {
                if !out_materials.is_null() {
                    out_materials.add(i).write(to_c(*material));
                }
                if !out_descs.is_null() {
                    out_descs.add(i).write(material_to_c(desc));
                }
            }
            out(out_count, found.len())
        })
    })
}

/// Loads a material shader (WGSL) from `assets/`; with hot reload on,
/// saving the file updates the game. A file that can't be read or doesn't
/// compile is an error; `crabby_last_error` says where. With hot reload on,
/// a file that doesn't compile still gives a shader, drawing with the
/// built-in look until a saved fix compiles (`crabby_ctx_shader_status`
/// says what's wrong).
/// # Safety
/// `ctx` null or live; `path` a C string; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_load_shader(std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_shader(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_shader: *mut CrabbyShader,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let path = text(path)?;
            if out_shader.is_null() {
                return Err(invalid("output is null"));
            }
            let shader = c.gfx.try_load_shader(path).map_err(invalid_owned)?;
            out(out_shader, CrabbyShader { id: shader.id() })
        })
    })
}

/// Makes a material shader from WGSL source; `name` is for error messages.
/// Source that doesn't compile is an error; `crabby_last_error` says where.
/// # Safety
/// `ctx` null or live; `name` and `source` C strings; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_create_shader(std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_shader(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    source: *const c_char,
    out_shader: *mut CrabbyShader,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let name = text(name)?;
            let source = text(source)?;
            if out_shader.is_null() {
                return Err(invalid("output is null"));
            }
            let shader = c.gfx.try_create_shader(name, source).map_err(invalid_owned)?;
            out(out_shader, CrabbyShader { id: shader.id() })
        })
    })
}

/// Loads a compute pass shader (WGSL, defining `fn compute(c: Cell) ->
/// vec4<f32>`) from assets/, hot reloaded; the same file again gives the
/// same compute pass (Rust `Graphics::load_compute`). With hot reload on, a
/// file that doesn't compile still gives a compute pass, skipped until a
/// saved fix compiles (`crabby_ctx_shader_status` says what's wrong).
/// # Safety
/// `ctx` null or live; `path` a NUL-terminated string; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_load_compute(std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_compute(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_compute: *mut CrabbyCompute,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let path = text(path)?;
            if out_compute.is_null() {
                return Err(invalid("output is null"));
            }
            let compute = c.gfx.try_load_compute(path).map_err(invalid_owned)?;
            out(out_compute, CrabbyCompute { id: compute.id() })
        })
    })
}

/// Makes a compute pass shader from WGSL source; `name` is for error
/// messages (Rust `Graphics::create_compute`).
/// # Safety
/// `ctx` null or live; `name` and `source` NUL-terminated strings; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_create_compute(std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_compute(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    source: *const c_char,
    out_compute: *mut CrabbyCompute,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let name = text(name)?;
            let source = text(source)?;
            if out_compute.is_null() {
                return Err(invalid("output is null"));
            }
            let compute = c.gfx.try_create_compute(name, source).map_err(invalid_owned)?;
            out(out_compute, CrabbyCompute { id: compute.id() })
        })
    })
}

/// Replaces the compute passes run each frame with `count` of them, in
/// order (Rust `ctx.compute_passes`); 0 runs none.
/// # Safety
/// `ctx` null or live; `passes` holds `count` values (may be null when 0).
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_compute_passes(std::ptr::null_mut(), std::ptr::null(), 0) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_compute_passes(
    ctx: *mut CrabbyCtx,
    passes: *const CrabbyComputePass,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            if passes.is_null() && count > 0 {
                return Err(invalid("passes is null"));
            }
            let given = if count == 0 {
                &[][..]
            } else {
                std::slice::from_raw_parts(passes, count)
            };
            let mut list = Vec::with_capacity(count);
            for p in given {
                let compute = Compute::from_id(p.compute.id);
                if !c.gfx.has_compute(compute) {
                    return Err(invalid("that compute pass wasn't made by this game"));
                }
                let target = texture_from_c(p.target);
                let buffer = p.has_buffer.then(|| GpuBuffer::from_id(p.buffer.id));
                if buffer.is_some_and(|b| c.gfx.gpu_buffer_len(b).is_none()) {
                    return Err(invalid("invalid GPU buffer"));
                }
                if buffer.is_none() && !c.gfx.is_render_target(target) {
                    return Err(invalid("that texture isn't a render target of this game"));
                }
                let material = p.has_material.then(|| to_c(p.material));
                if material.is_some_and(|m| !c.gfx.has_material(m)) {
                    return Err(invalid("that material wasn't made by this game"));
                }
                list.push(ComputePass {
                    buffer,
                    compute,
                    target,
                    material,
                });
            }
            c.compute_passes = list;
            Ok(())
        })
    })
}

/// The compute passes run each frame: writes up to `capacity` of them to
/// `out_passes` and how many there are to `out_count` (Rust
/// `ctx.compute_passes`).
/// # Safety
/// `ctx` null or live; `out_passes` holds `capacity` values (may be null
/// when 0); `out_count` writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_compute_passes(std::ptr::null_mut(), std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_compute_passes(
    ctx: *mut CrabbyCtx,
    out_passes: *mut CrabbyComputePass,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            if out_count.is_null() || (out_passes.is_null() && capacity > 0) {
                return Err(invalid("output is null"));
            }
            for (i, p) in c.compute_passes.iter().take(capacity).enumerate() {
                out_passes.add(i).write(CrabbyComputePass {
                    has_buffer: p.buffer.is_some(),
                    buffer: CrabbyGpuBuffer {
                        id: p.buffer.map_or(0, |b| b.id()),
                    },
                    compute: CrabbyCompute { id: p.compute.id() },
                    target: texture_to_c(p.target),
                    has_material: p.material.is_some(),
                    material: p.material.map(to_c).unwrap_or_default(),
                });
            }
            out(out_count, c.compute_passes.len())
        })
    })
}

/// Loads a screen effect shader (WGSL, defining `fn effect(p: Pixel) ->
/// vec3<f32>`) from assets/, hot reloaded; the same file again gives the
/// same effect (Rust `Graphics::load_effect`). With hot reload on, a file that
/// doesn't compile still gives an effect, showing the frame as it is until
/// a saved fix compiles (`crabby_ctx_shader_status` says what's wrong).
/// # Safety
/// `ctx` null or live; `path` a NUL-terminated string; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_load_effect(std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_effect(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_effect: *mut CrabbyEffect,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let path = text(path)?;
            if out_effect.is_null() {
                return Err(invalid("output is null"));
            }
            let effect = c.gfx.try_load_effect(path).map_err(invalid_owned)?;
            out(out_effect, CrabbyEffect { id: effect.id() })
        })
    })
}

/// Makes a screen effect shader from WGSL source; `name` is for error
/// messages (Rust `Graphics::create_effect`).
/// # Safety
/// `ctx` null or live; `name` and `source` NUL-terminated strings; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_create_effect(std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_effect(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    source: *const c_char,
    out_effect: *mut CrabbyEffect,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let name = text(name)?;
            let source = text(source)?;
            if out_effect.is_null() {
                return Err(invalid("output is null"));
            }
            let effect = c.gfx.try_create_effect(name, source).map_err(invalid_owned)?;
            out(out_effect, CrabbyEffect { id: effect.id() })
        })
    })
}

/// Replaces the screen effects run each frame with `count` of them, in
/// order (Rust `ctx.effects`); 0 runs none.
/// # Safety
/// `ctx` null or live; `effects` holds `count` values (may be null when 0).
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_effects(std::ptr::null_mut(), std::ptr::null(), 0) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_effects(
    ctx: *mut CrabbyCtx,
    effects: *const CrabbyScreenEffect,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            if effects.is_null() && count > 0 {
                return Err(invalid("effects is null"));
            }
            let given = if count == 0 {
                &[][..]
            } else {
                std::slice::from_raw_parts(effects, count)
            };
            let mut list = Vec::with_capacity(count);
            for e in given {
                let effect = Effect::from_id(e.effect.id);
                if !c.gfx.has_effect(effect) {
                    return Err(invalid("that effect wasn't made by this game"));
                }
                let material = e.has_material.then(|| to_c(e.material));
                if material.is_some_and(|m| !c.gfx.has_material(m)) {
                    return Err(invalid("that material wasn't made by this game"));
                }
                list.push(ScreenEffect { effect, material });
            }
            c.effects = list;
            Ok(())
        })
    })
}

/// The screen effects run each frame: writes up to `capacity` of them to
/// `out_effects` and how many there are to `out_count` (Rust `ctx.effects`).
/// # Safety
/// `ctx` null or live; `out_effects` holds `capacity` values (may be null
/// when 0); `out_count` writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_effects(std::ptr::null_mut(), std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_effects(
    ctx: *mut CrabbyCtx,
    out_effects: *mut CrabbyScreenEffect,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            if out_count.is_null() || (out_effects.is_null() && capacity > 0) {
                return Err(invalid("output is null"));
            }
            for (i, e) in c.effects.iter().take(capacity).enumerate() {
                out_effects.add(i).write(CrabbyScreenEffect {
                    effect: CrabbyEffect { id: e.effect.id() },
                    has_material: e.material.is_some(),
                    material: e.material.map(to_c).unwrap_or_default(),
                });
            }
            out(out_count, c.effects.len())
        })
    })
}

/// A plain white material, made once and shared.
/// # Safety
/// `ctx` null or live; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_default_material(std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_default_material(
    ctx: *mut CrabbyCtx,
    out_material: *mut CrabbyMaterial,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| out(out_material, to_c(c.gfx.default_material()))) })
}

/// Puts mesh data on the GPU as a mesh of one material. The data can be
/// freed afterwards.
/// # Safety
/// `ctx` null or live; `data` live; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_create_mesh(std::ptr::null_mut(), std::ptr::null(), Default::default(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_mesh(
    ctx: *mut CrabbyCtx,
    data: *const CrabbyMeshData,
    material: CrabbyMaterial,
    out_mesh: *mut CrabbyMesh,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let Some(data) = data.as_ref() else {
                return Err(invalid("mesh data is null"));
            };
            if out_mesh.is_null() {
                return Err(invalid("output is null"));
            }
            // Checked here so bad input is an error, not a panic.
            data.0.check().map_err(invalid_owned)?;
            if !c.gfx.has_material(to_c(material)) {
                return Err(invalid("that material wasn't made by this game"));
            }
            out(out_mesh, to_c(c.gfx.create_mesh(&data.0, to_c(material))))
        })
    })
}

/// Puts mesh data on the GPU as a mesh with no material of its own (Rust
/// `Ctx::create_plain_mesh`): `crabby_ctx_create_mesh` with the default
/// material, for meshes whose look comes from each model
/// (`crabby_ctx_set_model_material`). The data can be freed afterwards.
/// # Safety
/// `ctx` null or live; `data` live; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_create_plain_mesh(std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_plain_mesh(
    ctx: *mut CrabbyCtx,
    data: *const CrabbyMeshData,
    out_mesh: *mut CrabbyMesh,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let Some(data) = data.as_ref() else {
                return Err(invalid("mesh data is null"));
            };
            if out_mesh.is_null() {
                return Err(invalid("output is null"));
            }
            let mesh = c
                .gfx
                .try_create_plain_mesh(&data.0)
                .map_err(|e| invalid_owned(e.to_string()))?;
            out(out_mesh, to_c(mesh))
        })
    })
}

/// Makes a terrain tile `size` wide from `count` heights (n x n, row after
/// row along +Z, NaN for a hole) drawn with `material` (Rust
/// `Graphics::create_terrain_tile`). Spawn it with a `CrabbyTransform3D` at the
/// tile's corner.
/// # Safety
/// `ctx` null or live; `heights` holds `count` floats; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_create_terrain_tile(std::ptr::null_mut(), std::ptr::null(), 0, 1.0, Default::default(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_terrain_tile(
    ctx: *mut CrabbyCtx,
    heights: *const f32,
    count: usize,
    size: f32,
    material: CrabbyMaterial,
    out_tile: *mut CrabbyTerrainTile,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            if heights.is_null() || out_tile.is_null() {
                return Err(invalid("heights or output is null"));
            }
            // Checked here so bad input is an error, not a panic.
            if !square_grid(count) {
                return Err(invalid("the heights aren't a square grid of at least 2 x 2"));
            }
            if !(size > 0.0 && size.is_finite()) {
                return Err(invalid("size must be above 0"));
            }
            if !c.gfx.has_material(to_c(material)) {
                return Err(invalid("that material wasn't made by this game"));
            }
            let heights = std::slice::from_raw_parts(heights, count);
            out(out_tile, to_c(c.gfx.create_terrain_tile(heights, size, to_c(material))))
        })
    })
}

/// Loads a glTF model (`.glb`, or `.gltf` with its files) from assets/ as
/// one mesh with its materials. Loading the same file again returns the
/// same mesh.
/// # Safety
/// `ctx` null or live; `path` a NUL-terminated string; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_load_mesh(std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_mesh(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_mesh: *mut CrabbyMesh,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let path = text(path)?;
            if out_mesh.is_null() {
                return Err(invalid("output is null"));
            }
            out(out_mesh, to_c(c.gfx.load_mesh(path)))
        })
    })
}

/// How many 3D models were drawn last frame.
/// # Safety
/// `ctx` null or live; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_models_drawn(std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_models_drawn(ctx: *mut CrabbyCtx, out_count: *mut usize) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| out(out_count, c.gfx.models_drawn())) })
}

fn camera_to_c(c: &Camera3D) -> CrabbyCamera3D {
    CrabbyCamera3D {
        position: to_c(c.position),
        rotation: quat_to_c(c.rotation),
        fov_y: c.fov_y,
        near: c.near,
        far: c.far,
    }
}

/// A Rust camera from a C one, keeping `base`'s window size; checked.
fn camera_from_c(base: Camera3D, cam: &CrabbyCamera3D) -> Result<Camera3D, crate::Failure> {
    if !(cam.fov_y > 0.0 && cam.fov_y < std::f32::consts::PI) {
        return Err(invalid("fov_y must be between 0 and pi"));
    }
    if !(cam.near > 0.0 && cam.far > cam.near && cam.far.is_finite()) {
        return Err(invalid("needs 0 < near < far"));
    }
    let rotation = quat_from_c(cam.rotation);
    if !(rotation.length_squared() > 1e-12 && rotation.is_finite()) {
        return Err(invalid("rotation must not be zero"));
    }
    let mut camera = base;
    camera.position = to_c(cam.position);
    camera.rotation = rotation.normalize();
    camera.fov_y = cam.fov_y;
    camera.near = cam.near;
    camera.far = cam.far;
    Ok(camera)
}

fn target_camera_to_c(t: &TargetCamera) -> CrabbyTargetCamera {
    CrabbyTargetCamera {
        camera: camera_to_c(&t.camera),
        target: texture_to_c(t.target),
        has_clip_plane: t.clip_plane.is_some(),
        clip_plane: t.clip_plane.unwrap_or(Vec4::ZERO).to_array(),
    }
}

/// Makes a texture of `width` x `height` pixels that target cameras draw
/// the 3D world into (Rust `Graphics::create_render_target`); materials use it
/// like any texture.
/// # Safety
/// `ctx` null or live; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_create_render_target(std::ptr::null_mut(), 64, 64, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_render_target(
    ctx: *mut CrabbyCtx,
    width: u32,
    height: u32,
    out_texture: *mut CrabbyTexture,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            if out_texture.is_null() {
                return Err(invalid("output is null"));
            }
            let texture = c.gfx.try_create_render_target(width, height).map_err(invalid_owned)?;
            out(out_texture, texture_to_c(texture))
        })
    })
}

/// The target camera seeing `camera`'s view mirrored in the level plane at
/// height `level`, clipped to what's above it (Rust
/// `TargetCamera::reflection`). Read its target at the screen position
/// flipped upside down.
/// # Safety
/// `ctx` null or live; `camera` valid; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_reflection_camera(std::ptr::null_mut(), std::ptr::null(), ponko_ffi::CrabbyTexture { id: 0, width: 1, height: 1 }, 0.0, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_reflection_camera(
    ctx: *mut CrabbyCtx,
    camera: *const CrabbyCamera3D,
    target: CrabbyTexture,
    level: f32,
    out_camera: *mut CrabbyTargetCamera,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let Some(cam) = camera.as_ref() else {
                return Err(invalid("camera is null"));
            };
            if out_camera.is_null() {
                return Err(invalid("output is null"));
            }
            if !level.is_finite() {
                return Err(invalid("level must be finite"));
            }
            let eye = camera_from_c(c.camera3d, cam)?;
            let reflection = TargetCamera::reflection(eye, texture_from_c(target), level);
            out(out_camera, target_camera_to_c(&reflection))
        })
    })
}

/// Replaces the target cameras drawn each frame with `count` of them
/// (Rust `ctx.target_cameras`); 0 draws none. Each target must come from
/// `crabby_ctx_create_render_target`.
/// # Safety
/// `ctx` null or live; `cameras` holds `count` values (may be null when 0).
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_target_cameras(std::ptr::null_mut(), std::ptr::null(), 0) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_target_cameras(
    ctx: *mut CrabbyCtx,
    cameras: *const CrabbyTargetCamera,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            if cameras.is_null() && count > 0 {
                return Err(invalid("cameras is null"));
            }
            let given = if count == 0 {
                &[][..]
            } else {
                std::slice::from_raw_parts(cameras, count)
            };
            let mut list = Vec::with_capacity(count);
            for t in given {
                let target = texture_from_c(t.target);
                if !c.gfx.is_render_target(target) {
                    return Err(invalid("that texture isn't a render target of this game"));
                }
                let plane = Vec4::from_array(t.clip_plane);
                if t.has_clip_plane && !(plane.is_finite() && plane.truncate().length_squared() > 0.0) {
                    return Err(invalid("the clip plane needs a finite, nonzero normal"));
                }
                list.push(TargetCamera {
                    camera: camera_from_c(c.camera3d, &t.camera)?,
                    target,
                    clip_plane: t.has_clip_plane.then_some(plane),
                });
            }
            c.target_cameras = list;
            Ok(())
        })
    })
}

/// The target cameras drawn each frame: writes up to `capacity` of them
/// to `out_cameras` and how many there are to `out_count` (Rust
/// `ctx.target_cameras`).
/// # Safety
/// `ctx` null or live; `out_cameras` holds `capacity` values (may be null
/// when 0); `out_count` writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_target_cameras(std::ptr::null_mut(), std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_target_cameras(
    ctx: *mut CrabbyCtx,
    out_cameras: *mut CrabbyTargetCamera,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            if out_count.is_null() || (out_cameras.is_null() && capacity > 0) {
                return Err(invalid("output is null"));
            }
            for (i, t) in c.target_cameras.iter().take(capacity).enumerate() {
                out_cameras.add(i).write(target_camera_to_c(t));
            }
            out(out_count, c.target_cameras.len())
        })
    })
}

/// Reads the 3D camera.
/// # Safety
/// `ctx` null or live; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_camera3d(std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_camera3d(ctx: *mut CrabbyCtx, out_camera: *mut CrabbyCamera3D) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| out(out_camera, camera_to_c(&c.camera3d))) })
}

/// Moves or turns the 3D camera. `fov_y` must be between 0 and pi, and
/// `0 < near < far`.
/// # Safety
/// `ctx` null or live; `camera` valid.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_camera3d(std::ptr::null_mut(), std::ptr::null()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_camera3d(ctx: *mut CrabbyCtx, camera: *const CrabbyCamera3D) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let Some(cam) = camera.as_ref() else {
                return Err(invalid("camera is null"));
            };
            c.camera3d = camera_from_c(c.camera3d, cam)?;
            Ok(())
        })
    })
}

/// Turns occlusion culling on or off: models hidden behind what was drawn a
/// frame or two before aren't drawn (they still cast shadows). Off by
/// default. A model coming out from behind something can show up a frame or
/// two late when the camera moves fast. Without compute shaders (WebGL2)
/// nothing is culled.
/// # Safety
/// `ctx` null or live.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_occlusion_culling(std::ptr::null_mut(), true) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_occlusion_culling(ctx: *mut CrabbyCtx, on: bool) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.camera3d.occlusion_culling = on;
            Ok(())
        })
    })
}

/// Whether occlusion culling is on (see `crabby_ctx_set_occlusion_culling`).
/// # Safety
/// `ctx` null or live; output writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_occlusion_culling(std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_occlusion_culling(ctx: *mut CrabbyCtx, out_on: *mut bool) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| out(out_on, c.camera3d.occlusion_culling)) })
}

/// Turns the 3D camera to look at `target`, +Y up.
/// # Safety
/// `ctx` null or live.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_camera3d_look_at(std::ptr::null_mut(), Default::default()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_camera3d_look_at(ctx: *mut CrabbyCtx, target: CrabbyVec3) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.camera3d.look_at(to_c(target));
            Ok(())
        })
    })
}

/// The ray from the 3D camera through a window point (logical pixels, as
/// the mouse position): where it starts and its unit direction. For
/// picking things with the mouse.
/// # Safety
/// `ctx` null or live; outputs writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_screen_ray(std::ptr::null_mut(), Default::default(), std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_screen_ray(
    ctx: *mut CrabbyCtx,
    screen: CrabbyVec2,
    out_origin: *mut CrabbyVec3,
    out_direction: *mut CrabbyVec3,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            if out_origin.is_null() || out_direction.is_null() {
                return Err(invalid("output is null"));
            }
            let (origin, direction) = c.camera3d.screen_ray(to_c(screen));
            out(out_origin, to_c(origin))?;
            out(out_direction, to_c(direction))
        })
    })
}

/// Reads the sun and sky light.
/// # Safety
/// `ctx` null or live; output writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_lighting(ctx: *mut CrabbyCtx, out_lighting: *mut CrabbyLighting) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let l = &c.lighting;
            out(
                out_lighting,
                CrabbyLighting {
                    sun_direction: to_c(l.sun_direction),
                    sun_color: to_c(l.sun_color),
                    sun_intensity: l.sun_intensity,
                    sky_color: to_c(l.sky_color),
                    ground_color: to_c(l.ground_color),
                    ambient_intensity: l.ambient_intensity,
                    shadows: l.shadows,
                    shadow_distance: l.shadow_distance,
                    shadow_cascades: l.shadow_cascades,
                    shadow_resolution: l.shadow_resolution,
                    light_shadow_views: l.light_shadow_views,
                    light_shadow_resolution: l.light_shadow_resolution,
                    light_shadow_atlas: l.light_shadow_atlas,
                    shadow_refresh: l.shadow_refresh,
                    shadow_filter: l.shadow_filter,
                    has_equator: l.equator_color.is_some(),
                    equator_color: to_c(l.equator_color.unwrap_or(l.sky_color)),
                    gi_enabled: l.gi.enabled,
                    gi_extent: to_c(l.gi.extent),
                    gi_spacing: l.gi.spacing,
                    gi_probes_per_frame: l.gi.probes_per_frame,
                    gi_intensity: l.gi.intensity,
                },
            )
        })
    })
}

/// Changes the sun and sky light. Intensities and the shadow distance
/// can't be negative; `shadow_cascades` is 1 to 4, `shadow_refresh` 1 to 8, `shadow_filter` 1 to 3; the shadow resolutions
/// and `light_shadow_views` must be in their ranges (see `CrabbyLighting`).
/// With `gi_enabled`, `gi_extent` and `gi_spacing` must be above zero,
/// `gi_probes_per_frame` 1 to 16 and `gi_intensity` not negative.
/// # Safety
/// `ctx` null or live; `lighting` valid.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_lighting(std::ptr::null_mut(), std::ptr::null()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_lighting(ctx: *mut CrabbyCtx, lighting: *const CrabbyLighting) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let Some(&l) = lighting.as_ref() else {
                return Err(invalid("lighting is null"));
            };
            if !(l.sun_intensity >= 0.0 && l.ambient_intensity >= 0.0 && l.shadow_distance >= 0.0) {
                return Err(invalid("intensities and shadow distance can't be negative"));
            }
            if !(1..=4).contains(&l.shadow_cascades) {
                return Err(invalid("shadow_cascades must be 1 to 4"));
            }
            if !(1..=8).contains(&l.shadow_refresh) {
                return Err(invalid("shadow_refresh must be 1 to 8"));
            }
            if !(1..=3).contains(&l.shadow_filter) {
                return Err(invalid("shadow_filter must be 1 to 3"));
            }
            if !(256..=4096).contains(&l.shadow_resolution)
                || !(128..=2048).contains(&l.light_shadow_resolution)
                || l.light_shadow_views > MAX_LIGHT_SHADOW_VIEWS as u32
            {
                return Err(invalid(
                    "shadow_resolution must be 256 to 4096, light_shadow_resolution 128 to 2048 and light_shadow_views 0 to 24",
                ));
            }
            if !light_atlas_ok(l.light_shadow_atlas) {
                return Err(invalid("light_shadow_atlas must be a power of two from 1024 to 8192"));
            }
            let extent: Vec3 = to_c(l.gi_extent);
            if l.gi_enabled
                && !(extent.is_finite()
                    && extent.min_element() > 0.0
                    && l.gi_spacing.is_finite()
                    && l.gi_spacing > 0.0
                    && (1..=MAX_GI_PROBES_PER_FRAME).contains(&l.gi_probes_per_frame)
                    && l.gi_intensity.is_finite()
                    && l.gi_intensity >= 0.0)
            {
                return Err(invalid(
                    "gi_extent and gi_spacing must be above zero, gi_probes_per_frame 1 to 16 and gi_intensity not negative",
                ));
            }
            let color = |c: CrabbyColor| -> Color { to_c(c) };
            c.lighting = Lighting {
                sun_direction: to_c(l.sun_direction),
                sun_color: color(l.sun_color),
                sun_intensity: l.sun_intensity,
                sky_color: color(l.sky_color),
                ground_color: color(l.ground_color),
                ambient_intensity: l.ambient_intensity,
                shadows: l.shadows,
                shadow_distance: l.shadow_distance,
                shadow_cascades: l.shadow_cascades,
                shadow_resolution: l.shadow_resolution,
                light_shadow_views: l.light_shadow_views,
                light_shadow_resolution: l.light_shadow_resolution,
                light_shadow_atlas: l.light_shadow_atlas,
                shadow_refresh: l.shadow_refresh,
                shadow_filter: l.shadow_filter,
                equator_color: l.has_equator.then(|| color(l.equator_color)),
                gi: GlobalIllumination {
                    enabled: l.gi_enabled,
                    extent,
                    spacing: l.gi_spacing,
                    probes_per_frame: l.gi_probes_per_frame,
                    intensity: l.gi_intensity,
                },
            };
            Ok(())
        })
    })
}

/// Reads the sky and fog settings.
/// # Safety
/// `ctx` null or live; output writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_sky(ctx: *mut CrabbyCtx, out_sky: *mut CrabbySky) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let s = &c.sky;
            out(
                out_sky,
                CrabbySky {
                    atmosphere: s.atmosphere,
                    haze: s.haze,
                    sun_size: s.sun_size,
                    stars: s.stars,
                    night_light: to_c(s.night_light),
                    fog_density: s.fog_density,
                    fog_height: s.fog_height,
                    fog_falloff: s.fog_falloff,
                    fog_color: to_c(s.fog_color),
                    fog_start: s.fog_start,
                    fog_end: s.fog_end,
                    fog_sun_glow: s.fog_sun_glow,
                    fog_sun_focus: s.fog_sun_focus,
                    light_shafts: s.light_shafts,
                },
            )
        })
    })
}

/// Changes the sky and fog. Haze, stars, fog density, the distance fog's
/// distances and the sun glow can't be negative; the sun's size and the fog
/// falloff must be above 0; the glow's focus is from 0 to under 1; light
/// shafts are from 0 to 1.
/// # Safety
/// `ctx` null or live; `sky` valid.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_sky(std::ptr::null_mut(), std::ptr::null()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_sky(ctx: *mut CrabbyCtx, sky: *const CrabbySky) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let Some(&s) = sky.as_ref() else {
                return Err(invalid("sky is null"));
            };
            let finite = [
                s.haze,
                s.sun_size,
                s.stars,
                s.fog_density,
                s.fog_height,
                s.fog_falloff,
                s.fog_start,
                s.fog_end,
                s.fog_sun_glow,
                s.fog_sun_focus,
                s.light_shafts,
            ]
            .iter()
            .all(|v| v.is_finite());
            let negative = [s.haze, s.stars, s.fog_density, s.fog_start, s.fog_end, s.fog_sun_glow]
                .iter()
                .any(|&v| v < 0.0);
            if !finite || negative {
                return Err(invalid(
                    "haze, stars, fog density, distances and glow can't be negative",
                ));
            }
            if !(0.0..1.0).contains(&s.fog_sun_focus) {
                return Err(invalid("fog_sun_focus must be from 0 to under 1"));
            }
            if !(0.0..=1.0).contains(&s.light_shafts) {
                return Err(invalid("light_shafts must be from 0 to 1"));
            }
            if s.sun_size <= 0.0 || s.fog_falloff <= 0.0 {
                return Err(invalid("sun_size and fog_falloff must be above 0"));
            }
            c.sky = Sky {
                atmosphere: s.atmosphere,
                haze: s.haze,
                sun_size: s.sun_size,
                stars: s.stars,
                night_light: to_c(s.night_light),
                fog_density: s.fog_density,
                fog_height: s.fog_height,
                fog_falloff: s.fog_falloff,
                fog_color: to_c(s.fog_color),
                fog_start: s.fog_start,
                fog_end: s.fog_end,
                fog_sun_glow: s.fog_sun_glow,
                fog_sun_focus: s.fog_sun_focus,
                light_shafts: s.light_shafts,
            };
            Ok(())
        })
    })
}

/// Reads the 3D post-processing settings.
/// # Safety
/// `ctx` null or live; output writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_post(ctx: *mut CrabbyCtx, out_post: *mut CrabbyPost) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let p = &c.post;
            out(
                out_post,
                CrabbyPost {
                    render_scale: p.render_scale,
                    sharpen: p.sharpen,
                    anti_aliasing: p.anti_aliasing as u32,
                    ao_enabled: p.ambient_occlusion.enabled,
                    ao_radius: p.ambient_occlusion.radius,
                    ao_intensity: p.ambient_occlusion.intensity,
                    ao_slices: p.ambient_occlusion.slices,
                    ao_steps: p.ambient_occlusion.steps,
                    ao_half_resolution: p.ao_half_resolution,
                    ao_temporal: p.ao_temporal,
                    exposure: p.exposure,
                    tonemap: p.tonemap as u32,
                    bloom: p.bloom,
                    bloom_threshold: p.bloom_threshold,
                    contrast: p.contrast,
                    saturation: p.saturation,
                    temperature: p.temperature,
                    tint: p.tint,
                    vignette: p.vignette,
                    chromatic_aberration: p.chromatic_aberration,
                    lens_distortion: p.lens_distortion,
                    lens_scale: p.lens_scale,
                    has_lut: p.lut.is_some(),
                    lut: p.lut.map(texture_to_c).unwrap_or(CrabbyTexture {
                        id: 0,
                        width: 0,
                        height: 0,
                    }),
                    taa_sharpness: p.taa_sharpness,
                    contact_shadows: p.contact_shadows,
                    ssr: p.ssr,
                    ssr_max_distance: p.ssr_max_distance,
                },
            )
        })
    })
}

/// Changes the 3D post-processing settings. `tonemap` must be 0 to 3;
/// bloom, its threshold, contrast and saturation can't be negative;
/// `render_scale` is 0.25 to 2 and `sharpen` 0 to 1; `contact_shadows`
/// can't be negative; `ssr` is 0 to 1 and `ssr_max_distance` above zero;
/// `chromatic_aberration` is 0 to 1, `lens_distortion` -1 to 1 and
/// `lens_scale` above zero.
/// # Safety
/// `ctx` null or live; `post` valid.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_post(std::ptr::null_mut(), std::ptr::null()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_post(ctx: *mut CrabbyCtx, post: *const CrabbyPost) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let Some(&p) = post.as_ref() else {
                return Err(invalid("post is null"));
            };
            let anti_aliasing = match p.anti_aliasing {
                0 => AntiAliasing::None,
                1 => AntiAliasing::Fxaa,
                2 => AntiAliasing::Smaa,
                3 => AntiAliasing::Taa,
                _ => return Err(invalid("anti_aliasing must be 0 to 3")),
            };
            if !(0.0..=1.0).contains(&p.taa_sharpness) {
                return Err(invalid("taa_sharpness must be 0 to 1"));
            }
            if !(p.contact_shadows.is_finite() && p.contact_shadows >= 0.0) {
                return Err(invalid("contact_shadows can't be negative"));
            }
            if !(0.0..=1.0).contains(&p.ssr) || !(p.ssr_max_distance.is_finite() && p.ssr_max_distance > 0.0) {
                return Err(invalid("ssr must be 0 to 1 and ssr_max_distance above zero"));
            }
            if !p.ao_radius.is_finite()
                || p.ao_radius <= 0.0
                || !p.ao_intensity.is_finite()
                || p.ao_intensity < 0.0
                || !(1..=8).contains(&p.ao_slices)
                || !(1..=16).contains(&p.ao_steps)
            {
                return Err(invalid(
                    "AO requires positive finite radius, nonnegative finite intensity, 1..8 slices and 1..16 steps",
                ));
            }
            if !(0.0..=1.0).contains(&p.chromatic_aberration)
                || !(-1.0..=1.0).contains(&p.lens_distortion)
                || !(p.lens_scale.is_finite() && p.lens_scale > 0.0)
            {
                return Err(invalid(
                    "chromatic_aberration must be 0 to 1, lens_distortion -1 to 1 and lens_scale above zero",
                ));
            }
            if !(MIN_RENDER_SCALE..=MAX_RENDER_SCALE).contains(&p.render_scale) || !(0.0..=1.0).contains(&p.sharpen) {
                return Err(invalid("render_scale must be 0.25 to 2 and sharpen 0 to 1"));
            }
            let tonemap = match p.tonemap {
                0 => Tonemap::Aces,
                1 => Tonemap::AgX,
                2 => Tonemap::Neutral,
                3 => Tonemap::None,
                _ => return Err(invalid("tonemap must be 0 to 3")),
            };
            let finite = [
                p.exposure,
                p.bloom,
                p.bloom_threshold,
                p.contrast,
                p.saturation,
                p.temperature,
                p.tint,
                p.vignette,
            ]
            .iter()
            .all(|v| v.is_finite());
            if !finite || p.bloom < 0.0 || p.bloom_threshold < 0.0 || p.contrast < 0.0 || p.saturation < 0.0 {
                return Err(invalid("bloom, threshold, contrast and saturation can't be negative"));
            }
            c.post = Post {
                render_scale: p.render_scale,
                sharpen: p.sharpen,
                anti_aliasing,
                taa_sharpness: p.taa_sharpness,
                ambient_occlusion: AmbientOcclusion {
                    enabled: p.ao_enabled,
                    radius: p.ao_radius,
                    intensity: p.ao_intensity,
                    slices: p.ao_slices,
                    steps: p.ao_steps,
                },
                ao_half_resolution: p.ao_half_resolution,
                ao_temporal: p.ao_temporal,
                exposure: p.exposure,
                tonemap,
                bloom: p.bloom,
                bloom_threshold: p.bloom_threshold,
                contrast: p.contrast,
                saturation: p.saturation,
                temperature: p.temperature,
                tint: p.tint,
                vignette: p.vignette,
                chromatic_aberration: p.chromatic_aberration,
                lens_distortion: p.lens_distortion,
                lens_scale: p.lens_scale,
                lut: p.has_lut.then(|| texture_from_c(p.lut)),
                contact_shadows: p.contact_shadows,
                ssr: p.ssr,
                ssr_max_distance: p.ssr_max_distance,
            };
            Ok(())
        })
    })
}

/// The settings that decide how heavy the 3D picture is, in one struct
/// (Rust `GraphicsQuality`). Presets: `crabby_graphics_quality_preset`.
/// Every field is also a field of `CrabbyPost` or `CrabbyLighting`; applying
/// this writes those and leaves their other fields alone.
/// ```
/// let mut q = std::mem::MaybeUninit::<ponko_ffi::CrabbyGraphicsQuality>::uninit();
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_graphics_quality(std::ptr::null_mut(), q.as_mut_ptr()) } as u32, 1);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyGraphicsQuality {
    /// The 3D scene's resolution relative to the window, 0.25 to 2.
    pub render_scale: f32,
    /// Sharpening of the scaled image, 0 to 1.
    pub sharpen: f32,
    /// Whether anything casts shadows.
    pub shadows: bool,
    /// How far from the camera shadows reach.
    pub shadow_distance: f32,
    /// Sun shadow cascades, 1 to 4.
    pub shadow_cascades: u32,
    /// Width of each sun shadow map, 256 to 4096.
    pub shadow_resolution: u32,
    /// Shadow views point and spot lights share, 0 to 24.
    pub light_shadow_views: u32,
    /// Width of the largest point or spot light shadow view, 128 to 2048.
    pub light_shadow_resolution: u32,
    /// Width of the light shadow atlas: a power of two, 1024 to 8192.
    pub light_shadow_atlas: u32,
    /// Ambient occlusion on or off.
    pub ambient_occlusion: bool,
    /// AO angular slices, 1 to 8.
    pub ao_slices: u32,
    /// AO horizon steps per side, 1 to 16.
    pub ao_steps: u32,
    /// Half-resolution GTAO with bilateral upsampling; contacts stay full resolution.
    pub ao_half_resolution: bool,
    /// Camera-reprojected GTAO accumulation (independent of TAA).
    pub ao_temporal: bool,
    /// Post AA: 0 none, 1 FXAA, 2 SMAA 1x, 3 temporal.
    pub anti_aliasing: u32,
    /// How often the far shadow cascades are drawn again, in frames, 1 to 8.
    pub shadow_refresh: u32,
    /// How soft the sun's shadow edges are, 1 (one lookup) to 3 (nine).
    pub shadow_filter: u32,
    /// Screen-space reflections' strength, 0 (off) to 1.
    pub ssr: f32,
    /// Global illumination (bounced light) on or off.
    pub gi: bool,
}

pub(crate) fn quality_to_c(q: GraphicsQuality) -> CrabbyGraphicsQuality {
    CrabbyGraphicsQuality {
        render_scale: q.render_scale,
        sharpen: q.sharpen,
        shadows: q.shadows,
        shadow_distance: q.shadow_distance,
        shadow_cascades: q.shadow_cascades,
        shadow_resolution: q.shadow_resolution,
        light_shadow_views: q.light_shadow_views,
        light_shadow_resolution: q.light_shadow_resolution,
        light_shadow_atlas: q.light_shadow_atlas,
        ambient_occlusion: q.ambient_occlusion,
        ao_half_resolution: q.ao_half_resolution,
        ao_temporal: q.ao_temporal,
        ao_slices: q.ao_slices,
        ao_steps: q.ao_steps,
        anti_aliasing: q.anti_aliasing as u32,
        shadow_refresh: q.shadow_refresh,
        shadow_filter: q.shadow_filter,
        ssr: q.ssr,
        gi: q.gi,
    }
}

/// Whether a light shadow atlas size is allowed: a power of two, 1024 to 8192.
fn light_atlas_ok(size: u32) -> bool {
    (1024..=8192).contains(&size) && size.is_power_of_two()
}

/// Checks every field of a C quality struct against its range, and makes
/// the Rust one.
fn quality_from_c(q: &CrabbyGraphicsQuality) -> Result<GraphicsQuality, crate::Failure> {
    let anti_aliasing = match q.anti_aliasing {
        0 => AntiAliasing::None,
        1 => AntiAliasing::Fxaa,
        2 => AntiAliasing::Smaa,
        3 => AntiAliasing::Taa,
        _ => return Err(invalid("anti_aliasing must be 0 to 3")),
    };
    if !(MIN_RENDER_SCALE..=MAX_RENDER_SCALE).contains(&q.render_scale) || !(0.0..=1.0).contains(&q.sharpen) {
        return Err(invalid("render_scale must be 0.25 to 2 and sharpen 0 to 1"));
    }
    if !q.shadow_distance.is_finite() || q.shadow_distance < 0.0 || !(1..=4).contains(&q.shadow_cascades) {
        return Err(invalid(
            "shadow_distance can't be negative and shadow_cascades is 1 to 4",
        ));
    }
    if !(256..=4096).contains(&q.shadow_resolution)
        || !(128..=2048).contains(&q.light_shadow_resolution)
        || q.light_shadow_views > MAX_LIGHT_SHADOW_VIEWS as u32
    {
        return Err(invalid(
            "shadow_resolution must be 256 to 4096, light_shadow_resolution 128 to 2048 and light_shadow_views 0 to 24",
        ));
    }
    if !light_atlas_ok(q.light_shadow_atlas) {
        return Err(invalid("light_shadow_atlas must be a power of two from 1024 to 8192"));
    }
    if !(1..=8).contains(&q.ao_slices) || !(1..=16).contains(&q.ao_steps) {
        return Err(invalid("ao_slices must be 1 to 8 and ao_steps 1 to 16"));
    }
    if !(1..=8).contains(&q.shadow_refresh) {
        return Err(invalid("shadow_refresh must be 1 to 8"));
    }
    if !(1..=3).contains(&q.shadow_filter) {
        return Err(invalid("shadow_filter must be 1 to 3"));
    }
    if !(0.0..=1.0).contains(&q.ssr) {
        return Err(invalid("ssr must be 0 to 1"));
    }
    Ok(GraphicsQuality {
        render_scale: q.render_scale,
        sharpen: q.sharpen,
        shadows: q.shadows,
        shadow_distance: q.shadow_distance,
        shadow_cascades: q.shadow_cascades,
        shadow_resolution: q.shadow_resolution,
        light_shadow_views: q.light_shadow_views,
        light_shadow_resolution: q.light_shadow_resolution,
        light_shadow_atlas: q.light_shadow_atlas,
        ambient_occlusion: q.ambient_occlusion,
        ao_half_resolution: q.ao_half_resolution,
        ao_temporal: q.ao_temporal,
        ao_slices: q.ao_slices,
        ao_steps: q.ao_steps,
        ssr: q.ssr,
        gi: q.gi,
        anti_aliasing,
        shadow_refresh: q.shadow_refresh,
        shadow_filter: q.shadow_filter,
    })
}

/// Writes preset `level` into `out_quality`: 0 low, 1 medium, 2 high, 3
/// ultra (Rust `GraphicsQuality::low` and so on). Any other level is
/// `CRABBY_INVALID_ARGUMENT`.
/// # Safety
/// `out_quality` writable.
/// ```
/// let mut q = std::mem::MaybeUninit::<ponko_ffi::CrabbyGraphicsQuality>::uninit();
/// assert_eq!(unsafe { ponko_ffi::crabby_graphics_quality_preset(0, q.as_mut_ptr()) } as u32, 0);
/// assert_eq!(unsafe { q.assume_init() }.shadow_cascades, 2);
/// assert_eq!(unsafe { ponko_ffi::crabby_graphics_quality_preset(4, q.as_mut_ptr()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_graphics_quality_preset(
    level: u32,
    out_quality: *mut CrabbyGraphicsQuality,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let quality = match level {
            0 => QualityLevel::Low,
            1 => QualityLevel::Medium,
            2 => QualityLevel::High,
            3 => QualityLevel::Ultra,
            _ => return Err(invalid("level must be 0 (low), 1 (medium), 2 (high) or 3 (ultra)")),
        }
        .quality()
        .expect("a preset");
        out(out_quality, quality_to_c(quality))
    })
}

/// Which preset `quality` is: 0 low, 1 medium, 2 high, 3 ultra, or 4 when
/// it is none of them (Rust `GraphicsQuality::level`). `quality` must be
/// valid (see `crabby_ctx_set_graphics_quality`).
/// # Safety
/// `quality` valid; `out_level` writable.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_graphics_quality_level(std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_graphics_quality_level(
    quality: *const CrabbyGraphicsQuality,
    out_level: *mut u32,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        let Some(q) = quality.as_ref() else {
            return Err(invalid("quality is null"));
        };
        out(out_level, quality_from_c(q)?.level() as u32)
    })
}

/// Reads the current graphics quality (Rust `ctx.gfx.quality(&ctx.post, &ctx.lighting)`),
/// gathered from the post and lighting settings.
/// # Safety
/// `ctx` null or live; output writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_graphics_quality(
    ctx: *mut CrabbyCtx,
    out_quality: *mut CrabbyGraphicsQuality,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            out(out_quality, quality_to_c(c.gfx.quality(&c.post, &c.lighting)))
        })
    })
}

/// Switches the 3D picture to `quality` without a restart (Rust
/// `ctx.gfx.set_quality`): writes the matching post and lighting
/// settings; the next frame draws at the new resolution and shadow map
/// sizes. Fields outside their ranges (see `CrabbyGraphicsQuality`) are
/// `CRABBY_INVALID_ARGUMENT` and change nothing.
/// # Safety
/// `ctx` null or live; `quality` valid.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_graphics_quality(std::ptr::null_mut(), std::ptr::null()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_graphics_quality(
    ctx: *mut CrabbyCtx,
    quality: *const CrabbyGraphicsQuality,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let Some(q) = quality.as_ref() else {
                return Err(invalid("quality is null"));
            };
            c.gfx.set_quality(&mut c.post, &mut c.lighting, quality_from_c(q)?);
            Ok(())
        })
    })
}

/// Makes TAA (post `anti_aliasing` 3) and temporal AO forget the frames
/// before this one: call it after camera cuts or object teleports. The engine
/// also resets on resize and when the camera jumps or turns sharply.
/// # Safety
/// `ctx` null or live.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_reset_temporal_history(std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_reset_temporal_history(ctx: *mut CrabbyCtx) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.gfx.reset_temporal_history();
            Ok(())
        })
    })
}

/// A terrain tile's heights for finding the ground on the CPU (Rust
/// `Heightfield`). Make it with `crabby_heightfield_new`.
pub struct CrabbyHeightfield(ponko::Heightfield);

/// Makes a heightfield `size` wide from `count` heights (n x n, as
/// `crabby_ctx_create_terrain_tile` takes them). Returns null (see
/// `crabby_last_error`) for bad input.
/// ```
/// let h = [0.0f32; 9];
/// let g = unsafe { ponko_ffi::crabby_heightfield_new(h.as_ptr(), 9, 4.0) };
/// assert!(!g.is_null());
/// unsafe { ponko_ffi::crabby_heightfield_free(g) };
/// assert!(unsafe { ponko_ffi::crabby_heightfield_new(h.as_ptr(), 8, 4.0) }.is_null());
/// ```
/// # Safety
/// `heights` holds `count` floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_heightfield_new(
    heights: *const f32,
    count: usize,
    size: f32,
) -> *mut CrabbyHeightfield {
    let mut made = std::ptr::null_mut();
    let status = boundary(|| {
        if heights.is_null() {
            return Err(invalid("heights is null"));
        }
        if !square_grid(count) {
            return Err(invalid("the heights aren't a square grid of at least 2 x 2"));
        }
        if !(size > 0.0 && size.is_finite()) {
            return Err(invalid("size must be above 0"));
        }
        // SAFETY: caller supplies `count` floats.
        let heights = unsafe { std::slice::from_raw_parts(heights, count) };
        made = Box::into_raw(Box::new(CrabbyHeightfield(ponko::Heightfield::new(heights, size))));
        Ok(())
    });
    if status == CrabbyStatus::CRABBY_OK {
        made
    } else {
        std::ptr::null_mut()
    }
}

/// Frees a heightfield. Null is ignored.
/// ```
/// unsafe { ponko_ffi::crabby_heightfield_free(std::ptr::null_mut()) };
/// ```
/// # Safety
/// `ground` must be null or live, and not used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_heightfield_free(ground: *mut CrabbyHeightfield) {
    if !ground.is_null() {
        // SAFETY: caller passes a heightfield from crabby_heightfield_new, once.
        drop(unsafe { Box::from_raw(ground) });
    }
}

/// The ground's height and facing at `(x, z)` from the tile's corner,
/// following the drawn triangles. `out_found` is false off the tile or
/// over a hole (the others are then left alone). `out_normal` may be null.
/// Safe from any thread at once.
/// ```
/// use ponko_ffi::*;
/// let h = [0.0f32, 2.0, 0.0, 2.0];
/// let (mut found, mut y, mut n) = (false, 0.0, CrabbyVec3::default());
/// unsafe {
///     let g = crabby_heightfield_new(h.as_ptr(), 4, 1.0);
///     assert_eq!(crabby_heightfield_at(g, 0.5, 0.5, &mut found, &mut y, &mut n), CrabbyStatus::CRABBY_OK);
///     assert!(found && (y - 1.0).abs() < 1e-6 && n.x < 0.0);
///     crabby_heightfield_at(g, 5.0, 0.5, &mut found, &mut y, std::ptr::null_mut());
///     assert!(!found);
///     crabby_heightfield_free(g);
/// }
/// ```
/// # Safety
/// `ground` null or live; `out_found` and `out_height` writable; `out_normal` null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_heightfield_at(
    ground: *const CrabbyHeightfield,
    x: f32,
    z: f32,
    out_found: *mut bool,
    out_height: *mut f32,
    out_normal: *mut CrabbyVec3,
) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: caller supplies a live heightfield.
        let Some(ground) = (unsafe { ground.as_ref() }) else {
            return Err(invalid("heightfield is null"));
        };
        if out_found.is_null() || out_height.is_null() {
            return Err(invalid("output is null"));
        }
        let height = ground.0.height_at(x, z);
        // SAFETY: outputs checked non-null (normal below).
        unsafe {
            out_found.write(height.is_some());
            if let Some(h) = height {
                out_height.write(h);
                if !out_normal.is_null()
                    && let Some(n) = ground.0.normal_at(x, z)
                {
                    out_normal.write(CrabbyVec3 { x: n.x, y: n.y, z: n.z });
                }
            }
        }
        Ok(())
    })
}

/// Gives `entity` a different shadow than its model's (Rust `ShadowMesh`):
/// `mesh` is drawn into the shadow maps instead, or with `casts` false, the
/// entity casts none. An entity with a shadow mesh and no `CrabbyModel`
/// casts only that shadow; on a terrain tile, `casts` false turns the
/// ground's shadow off. Stale entities are an error.
/// # Safety
/// `ctx` null or live.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_shadow_mesh(std::ptr::null_mut(), 0, Default::default(), false) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_shadow_mesh(
    ctx: *mut CrabbyCtx,
    entity: u64,
    mesh: CrabbyMesh,
    casts: bool,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let entity = ponko::Entity::from_bits(entity);
            if !c.world.is_alive(entity) {
                return Err(invalid("no such entity"));
            }
            c.world.insert(entity, ponko::ShadowMesh(casts.then(|| to_c(mesh))));
            Ok(())
        })
    })
}

/// Draws `entity`'s model with `material` instead of the ones its mesh was
/// made with, every part (Rust `ModelMaterial`); with `enabled` false, with
/// its mesh's own again. One uploaded mesh can so be drawn in several
/// materials. It applies to plain, skinned and morphing models, LOD groups
/// and terrain tiles. Stale entities and materials are an error.
/// # Safety
/// `ctx` null or live.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_model_material(std::ptr::null_mut(), 0, Default::default(), false) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_model_material(
    ctx: *mut CrabbyCtx,
    entity: u64,
    material: CrabbyMaterial,
    enabled: bool,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let entity = ponko::Entity::from_bits(entity);
            if !c.world.is_alive(entity) {
                return Err(invalid("no such entity"));
            }
            if !enabled {
                c.world.remove::<ponko::ModelMaterial>(entity);
                return Ok(());
            }
            if !c.gfx.has_material(to_c(material)) {
                return Err(invalid("that material wasn't made by this game"));
            }
            c.world.insert(entity, ponko::ModelMaterial(to_c(material)));
            Ok(())
        })
    })
}

/// The material set with `crabby_ctx_set_model_material`: `out_has` says
/// whether `entity` has one, `out_material` (optional) is it.
/// # Safety
/// `ctx` null or live; outputs writable or null as documented.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_model_material(std::ptr::null_mut(), 0, std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_model_material(
    ctx: *mut CrabbyCtx,
    entity: u64,
    out_material: *mut CrabbyMaterial,
    out_has: *mut bool,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let entity = ponko::Entity::from_bits(entity);
            if !c.world.is_alive(entity) {
                return Err(invalid("no such entity"));
            }
            if out_has.is_null() {
                return Err(invalid("output is null"));
            }
            let found = c.world.get::<ponko::ModelMaterial>(entity).copied();
            if let (Some(m), false) = (found, out_material.is_null()) {
                *out_material = to_c(m.0);
            }
            out(out_has, found.is_some())
        })
    })
}

/// Draws some parts of `entity`'s model with other materials (Rust
/// `ModelMaterials`): part `parts[i]` with `materials[i]`, for `count`
/// pairs. Parts are numbered from 0 in the order the mesh was made with;
/// parts not listed keep the material of `crabby_ctx_set_model_material`,
/// or the mesh's own. A part listed twice takes the last; a part the mesh
/// doesn't have is ignored. `count` 0 takes the list off. Stale entities
/// and materials are an error.
/// # Safety
/// `ctx` null or live; `parts` and `materials` hold `count` values each.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_model_part_materials(std::ptr::null_mut(), 0, std::ptr::null(), std::ptr::null(), 0) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_model_part_materials(
    ctx: *mut CrabbyCtx,
    entity: u64,
    parts: *const u32,
    materials: *const CrabbyMaterial,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let entity = ponko::Entity::from_bits(entity);
            if !c.world.is_alive(entity) {
                return Err(invalid("no such entity"));
            }
            if count == 0 {
                c.world.remove::<ponko::ModelMaterials>(entity);
                return Ok(());
            }
            if parts.is_null() || materials.is_null() {
                return Err(invalid("parts or materials is null"));
            }
            let parts = std::slice::from_raw_parts(parts, count);
            let materials = std::slice::from_raw_parts(materials, count);
            if materials.iter().any(|&m| !c.gfx.has_material(to_c(m))) {
                return Err(invalid("a material wasn't made by this game"));
            }
            let list: Vec<(usize, ponko::Material)> = parts
                .iter()
                .zip(materials)
                .map(|(&part, &m)| (part as usize, to_c(m)))
                .collect();
            c.world.insert(entity, ponko::ModelMaterials::new(&list));
            Ok(())
        })
    })
}

/// How many parts `crabby_ctx_set_model_part_materials` listed for
/// `entity` (0: none), and with `out_parts` and `out_materials` non-null,
/// up to `capacity` of them, by part number.
/// # Safety
/// `ctx` null or live; the outputs hold `capacity` values each, or are null.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_model_part_materials(std::ptr::null_mut(), 0, std::ptr::null_mut(), std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_model_part_materials(
    ctx: *mut CrabbyCtx,
    entity: u64,
    out_parts: *mut u32,
    out_materials: *mut CrabbyMaterial,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let entity = ponko::Entity::from_bits(entity);
            if !c.world.is_alive(entity) {
                return Err(invalid("no such entity"));
            }
            if out_count.is_null() {
                return Err(invalid("output is null"));
            }
            let Some(list) = c.world.get::<ponko::ModelMaterials>(entity) else {
                return out(out_count, 0);
            };
            if !out_parts.is_null() && !out_materials.is_null() {
                for (i, (part, material)) in list.parts().take(capacity).enumerate() {
                    *out_parts.add(i) = part as u32;
                    *out_materials.add(i) = to_c(material);
                }
            }
            out(out_count, list.parts().len())
        })
    })
}

/// Whether models showing `mesh` cast shadows (Rust
/// `Graphics::set_mesh_shadows`): on for every mesh at first. Off suits small
/// things crowding the ground, such as grass.
/// # Safety
/// `ctx` null or live.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_mesh_shadows(std::ptr::null_mut(), Default::default(), false) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_mesh_shadows(
    ctx: *mut CrabbyCtx,
    mesh: CrabbyMesh,
    casts: bool,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.gfx.set_mesh_shadows(to_c(mesh), casts);
            Ok(())
        })
    })
}

/// Whether models showing `mesh` cast shadows.
/// # Safety
/// `ctx` null or live; output writable.
/// ```
/// let mut casts = false;
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_mesh_shadows(std::ptr::null_mut(), Default::default(), &mut casts) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_mesh_shadows(
    ctx: *mut CrabbyCtx,
    mesh: CrabbyMesh,
    out_casts: *mut bool,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| out(out_casts, c.gfx.mesh_shadows(to_c(mesh)))) })
}

same_layout!(
    CrabbyPointLight3D,
    PointLight3D,
    color = color,
    intensity = intensity,
    range = range,
    shadows = shadows,
    shadow_resolution = shadow_resolution
);

/// Registers the built-in `PointLight3D` for entity insertion and query columns.
/// # Safety
/// Live world and writable output.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe { let mut id = 0; assert_eq!(crabby_point_light3d_component(w, &mut id), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_point_light3d_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { builtin::<PointLight3D>(world, out_id) }
}

same_layout!(
    CrabbySpotLight3D,
    SpotLight3D,
    color = color,
    intensity = intensity,
    range = range,
    inner_angle = inner_angle,
    outer_angle = outer_angle,
    shadows = shadows,
    shadow_resolution = shadow_resolution
);

/// Registers the built-in `SpotLight3D` for entity insertion and query columns.
/// # Safety
/// Live world and writable output.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe { let mut id = 0; assert_eq!(crabby_spot_light3d_component(w, &mut id), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_spot_light3d_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { builtin::<SpotLight3D>(world, out_id) }
}

/// Axis-aligned local reflection volume. Zero content falls back to sky IBL.
/// ```
/// let p = ponko_ffi::crabby_reflection_probe3d_default();
/// assert!(p.box_projection);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CrabbyReflectionProbe3D {
    /// Full size in world units (transform scale and rotation ignored).
    pub size: CrabbyVec3,
    /// Fade distance inside the boundary.
    pub blend_distance: f32,
    /// Linear radiance multiplier.
    pub intensity: f32,
    /// Context-owned baked content, initially zero.
    pub content: u32,
    /// Enable parallax correction against the volume's walls.
    pub box_projection: bool,
}
same_layout!(
    CrabbyReflectionProbe3D,
    ReflectionProbe3D,
    size = size,
    blend_distance = blend_distance,
    intensity = intensity,
    content = content,
    box_projection = box_projection
);

/// Default 10-unit box, 1-unit blend, intensity 1, box projection enabled.
/// ```
/// assert_eq!(ponko_ffi::crabby_reflection_probe3d_default().content, 0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_reflection_probe3d_default() -> CrabbyReflectionProbe3D {
    CrabbyReflectionProbe3D {
        size: CrabbyVec3 {
            x: 10.0,
            y: 10.0,
            z: 10.0,
        },
        blend_distance: 1.0,
        intensity: 1.0,
        content: 0,
        box_projection: true,
    }
}

/// Register the reflection probe component for insertion and queries.
/// # Safety
/// Live world and writable output.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe { let mut id = 0; assert_eq!(crabby_reflection_probe3d_component(w, &mut id), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_reflection_probe3d_component(
    world: *mut CrabbyWorld,
    out_id: *mut u32,
) -> CrabbyStatus {
    // SAFETY: caller supplies the live world and writable output.
    unsafe { builtin::<ReflectionProbe3D>(world, out_id) }
}

/// A box that paints an image onto the opaque surfaces inside it (Rust
/// `Decal3D`). The entity's transform places it; the image is projected
/// along its -Z. Start from `crabby_decal3d_default`.
/// ```
/// let d = ponko_ffi::crabby_decal3d_default();
/// assert_eq!((d.image, d.normal_image), (0, ponko_ffi::CRABBY_DECAL_NO_IMAGE));
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CrabbyDecal3D {
    /// Width, height and depth of the box (times the transform's scale).
    pub size: CrabbyVec3,
    /// Layer of the decal textures holding the color image, or `CRABBY_DECAL_NO_IMAGE`.
    pub image: u32,
    /// Layer holding a tangent-space normal map, or `CRABBY_DECAL_NO_IMAGE`.
    pub normal_image: u32,
    /// Top left of the image in its layer (0 to 1).
    pub uv_min: CrabbyVec2,
    /// Bottom right of the image in its layer.
    pub uv_max: CrabbyVec2,
    /// sRGB tint; its alpha multiplies `opacity`.
    pub color: CrabbyColor,
    /// 0 to 1, times the image's alpha.
    pub opacity: f32,
    /// How strongly the normal map tilts the surface.
    pub normal_strength: f32,
    /// Surface roughness under the decal; negative leaves it.
    pub roughness: f32,
    /// Surface metallic value under the decal; negative leaves it.
    pub metallic: f32,
    /// Radians from the projection axis up to which the decal is fully visible.
    pub angle_fade_start: f32,
    /// Radians where it has faded out completely.
    pub angle_fade_end: f32,
    /// Fraction of the box's depth, from each end, it fades over.
    pub depth_fade: f32,
    /// Distance past which it is not drawn (0: unlimited).
    pub max_distance: f32,
    /// Higher orders draw over lower ones.
    pub order: i32,
    /// Models with any of these bits (low 16) in their `CrabbyReceivesDecals` get it.
    pub layers: u32,
}
same_layout!(
    CrabbyDecal3D,
    Decal3D,
    size = size,
    image = image,
    normal_image = normal_image,
    uv_min = uv_min,
    uv_max = uv_max,
    color = color,
    opacity = opacity,
    normal_strength = normal_strength,
    roughness = roughness,
    metallic = metallic,
    angle_fade_start = angle_fade_start,
    angle_fade_end = angle_fade_end,
    depth_fade = depth_fade,
    max_distance = max_distance,
    order = order,
    layers = layers
);

/// `image` and `normal_image` value that selects no image.
pub const CRABBY_DECAL_NO_IMAGE: u32 = u32::MAX;

/// A 1 x 1 x 1 white decal from image 0, no normal map, all layers (Rust `Decal3D::default`).
/// ```
/// let d = ponko_ffi::crabby_decal3d_default();
/// assert_eq!((d.opacity, d.layers), (1.0, 0xFFFF));
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_decal3d_default() -> CrabbyDecal3D {
    // SAFETY: the layouts are the same (checked above); every field is plain data.
    unsafe { std::mem::transmute(Decal3D::default()) }
}

/// Register the decal component for insertion and queries.
/// # Safety
/// Live world and writable output.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe { let mut id = 0; assert_eq!(crabby_decal3d_component(w, &mut id), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_decal3d_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { builtin::<Decal3D>(world, out_id) }
}

/// Which decal layers a model takes (Rust `ReceivesDecals`); the low 16
/// bits count. Models without the component take every decal.
/// ```
/// assert_eq!(std::mem::size_of::<ponko_ffi::CrabbyReceivesDecals>(), 4);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CrabbyReceivesDecals {
    /// The layer bits.
    pub layers: u32,
}
same_layout!(CrabbyReceivesDecals, ReceivesDecals);

/// Register the decal receiver component for insertion and queries.
/// # Safety
/// Live world and writable output.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe { let mut id = 0; assert_eq!(crabby_receives_decals_component(w, &mut id), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_receives_decals_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    unsafe { builtin::<ReceivesDecals>(world, out_id) }
}

/// Sets the texture array decals take their images from (Rust
/// `Graphics::set_decal_textures`). It must have been made with
/// `CRABBY_TEXTURE_DATA`: decals decode color layers from sRGB themselves.
/// # Safety
/// `ctx` null or live.
/// ```
/// let a = ponko_ffi::CrabbyTextureArray::default();
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_decal_textures(std::ptr::null_mut(), a) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_decal_textures(ctx: *mut CrabbyCtx, array: CrabbyTextureArray) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.gfx
                .try_set_decal_textures(Some(array_from_c(array)))
                .map_err(invalid_owned)
        })
    })
}

/// Goes back to flat-colored decals.
/// # Safety
/// `ctx` null or live.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_clear_decal_textures(std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_clear_decal_textures(ctx: *mut CrabbyCtx) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.gfx.set_decal_textures(None);
            Ok(())
        })
    })
}

/// Whether this GPU draws projected decals (Rust `Graphics::supports_decals`);
/// false in headless runs and on WebGL2.
/// # Safety
/// `ctx` null or live; output writable.
/// ```
/// let mut yes = true;
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_supports_decals(std::ptr::null_mut(), &mut yes) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_supports_decals(ctx: *mut CrabbyCtx, out_supported: *mut bool) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| out(out_supported, c.gfx.supports_decals())) })
}

/// Build, now, the 3D pipeline variants frames would build the first time
/// they draw them (Rust `Graphics::warm_render_pipelines`); call it at the end of
/// loading. Does nothing in headless runs.
/// # Safety
/// `ctx` null or live.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_warm_render_pipelines(std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_warm_render_pipelines(ctx: *mut CrabbyCtx) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.gfx.warm_render_pipelines();
            Ok(())
        })
    })
}

/// Queue six HDR views of the scene at the probe and prefilter them.
/// # Safety
/// Context is null or a live callback context.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_bake_reflection_probe(std::ptr::null_mut(), 0) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_bake_reflection_probe(ctx: *mut CrabbyCtx, entity: u64) -> CrabbyStatus {
    // SAFETY: forwarded context contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.gfx
                .bake_reflection_probe(&mut c.world, ponko::Entity::from_bits(entity))
                .map_err(invalid_owned)
        })
    })
}

/// Load a linear HDR/EXR 2:1 equirectangular environment for a probe.
/// # Safety
/// Context is null or live; name is a null-terminated UTF-8 string.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_load_reflection_probe(std::ptr::null_mut(), 0, std::ptr::null()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_reflection_probe(
    ctx: *mut CrabbyCtx,
    entity: u64,
    name: *const c_char,
) -> CrabbyStatus {
    // SAFETY: forwarded context and string contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.gfx
                .load_reflection_probe(&mut c.world, ponko::Entity::from_bits(entity), text(name)?)
                .map_err(invalid_owned)
        })
    })
}

// ---- levels of detail ---------------------------------------------------------

/// One mesh of a LOD level (Rust `LodRenderer`), placed by `offset` in the
/// group's space and tinted by `color` (white: no change).
/// ```
/// let r = ponko_ffi::crabby_lod_renderer(Default::default());
/// assert_eq!((r.color.r, r.offset.scale.x), (1.0, 1.0));
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyLodRenderer {
    /// What to draw.
    pub mesh: CrabbyMesh,
    /// Where it sits in the group's space.
    pub offset: CrabbyTransform3D,
    /// Multiplied with the materials' colors.
    pub color: CrabbyColor,
}

/// `mesh` at the group's origin, untinted.
/// ```
/// let r = ponko_ffi::crabby_lod_renderer(ponko_ffi::CrabbyMesh { id: 4 });
/// assert_eq!(r.mesh.id, 4);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_lod_renderer(mesh: CrabbyMesh) -> CrabbyLodRenderer {
    CrabbyLodRenderer {
        mesh,
        offset: transform_to_c(Transform3D::default()),
        color: to_c(Color::WHITE),
    }
}

/// One level of a LOD group (Rust `LodLevel`): its renderers are drawn while
/// the group covers at least `screen_height` of the screen's height. With
/// `custom_shadow`, `shadow_casters` cast the shadow instead of the
/// renderers (none: no shadow).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CrabbyLodLevel {
    /// 0 to 1, going down from level to level.
    pub screen_height: f32,
    /// How much of the way up to the previous threshold it crossfades into
    /// the next level, 0 to 1 (0: switches at once).
    pub fade_width: f32,
    /// `renderer_count` renderers (may be null when 0).
    pub renderers: *const CrabbyLodRenderer,
    /// How many.
    pub renderer_count: usize,
    /// Whether `shadow_casters` cast the shadow instead of the renderers.
    pub custom_shadow: bool,
    /// `shadow_caster_count` renderers (may be null when 0).
    pub shadow_casters: *const CrabbyLodRenderer,
    /// How many.
    pub shadow_caster_count: usize,
}

/// Global LOD settings (Rust `ctx.lod`).
/// ```
/// let s = ponko_ffi::CrabbyLodSettings { bias: 1.0, crossfade: true };
/// assert!(s.crossfade);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyLodSettings {
    /// Multiplies every group's screen height (2: detail twice as far).
    pub bias: f32,
    /// Whether levels with a fade width crossfade.
    pub crossfade: bool,
}

/// Which level of a group is drawn (Rust `LodPick`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CrabbyLodPick {
    /// 0 culled, 1 one level (`level`), 2 crossfading from `level` to `to`.
    pub kind: u32,
    /// The level drawn, or fading out.
    pub level: u32,
    /// The level fading in; -1 for none (fading out past the last level).
    pub to: i32,
    /// How far along the fade is, 0 to 1.
    pub blend: f32,
}

/// # Safety
/// `count` values at `list` (may be null when 0).
unsafe fn lod_renderers(list: *const CrabbyLodRenderer, count: usize) -> Result<Vec<LodRenderer>, crate::Failure> {
    if list.is_null() && count > 0 {
        return Err(invalid("renderers is null"));
    }
    if count == 0 {
        return Ok(Vec::new());
    }
    // SAFETY: caller contract.
    let given = unsafe { std::slice::from_raw_parts(list, count) };
    Ok(given
        .iter()
        .map(|r| LodRenderer {
            mesh: to_c(r.mesh),
            offset: transform_from_c(r.offset),
            color: to_c(r.color),
        })
        .collect())
}

/// Gives `entity` levels of detail (Rust `LodGroup`): `count` levels, most
/// detailed first, for an object `size` tall in its own space, its
/// distance measured to `center`. Replaces any it had. Stale entities are an
/// error.
/// # Safety
/// `ctx` null or live; `levels` holds `count` values (may be null when 0),
/// each pointing at as many renderers as it says.
/// ```
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_lod_group(std::ptr::null_mut(), 0, 1.0, Default::default(), std::ptr::null(), 0) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_lod_group(
    ctx: *mut CrabbyCtx,
    entity: u64,
    size: f32,
    center: CrabbyVec3,
    levels: *const CrabbyLodLevel,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let entity = ponko::Entity::from_bits(entity);
            if !c.world.is_alive(entity) {
                return Err(invalid("no such entity"));
            }
            if levels.is_null() && count > 0 {
                return Err(invalid("levels is null"));
            }
            let given = if count == 0 {
                &[][..]
            } else {
                std::slice::from_raw_parts(levels, count)
            };
            let mut list = Vec::with_capacity(count);
            for l in given {
                let shadow = if l.custom_shadow {
                    LodShadow::Casters(lod_renderers(l.shadow_casters, l.shadow_caster_count)?)
                } else {
                    LodShadow::Same
                };
                list.push(
                    LodLevel::new(l.screen_height, lod_renderers(l.renderers, l.renderer_count)?)
                        .with_fade(l.fade_width)
                        .with_shadow(shadow),
                );
            }
            c.world
                .insert(entity, LodGroup::new(size, list).with_center(to_c(center)));
            Ok(())
        })
    })
}

/// The level of `entity`'s LOD group the 3D camera picks, under the current
/// settings (Rust `LodGroup::pick_for`). Entities without a group, or
/// without a `CrabbyTransform3D`, are an error.
/// # Safety
/// `ctx` null or live; output writable.
/// ```
/// let mut pick = Default::default();
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_lod_pick(std::ptr::null_mut(), 0, &mut pick) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_lod_pick(
    ctx: *mut CrabbyCtx,
    entity: u64,
    out_pick: *mut CrabbyLodPick,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let entity = ponko::Entity::from_bits(entity);
            let (Some(group), Some(transform)) = (c.world.get::<LodGroup>(entity), c.world.get::<Transform3D>(entity))
            else {
                return Err(invalid("that entity has no LOD group and transform"));
            };
            let pick = match group.pick_for(transform, &c.camera3d, &c.lod) {
                LodPick::Culled => CrabbyLodPick::default(),
                LodPick::Level(i) => CrabbyLodPick {
                    kind: 1,
                    level: i as u32,
                    to: -1,
                    blend: 0.0,
                },
                LodPick::Fade { from, to, blend } => CrabbyLodPick {
                    kind: 2,
                    level: from as u32,
                    to: to.map_or(-1, |t| t as i32),
                    blend,
                },
            };
            out(out_pick, pick)
        })
    })
}

/// The global LOD settings.
/// # Safety
/// `ctx` null or live; output writable.
/// ```
/// let mut s = ponko_ffi::CrabbyLodSettings { bias: 0.0, crossfade: false };
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_lod_settings(std::ptr::null_mut(), &mut s) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_lod_settings(
    ctx: *mut CrabbyCtx,
    out_settings: *mut CrabbyLodSettings,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            out(
                out_settings,
                CrabbyLodSettings {
                    bias: c.lod.bias,
                    crossfade: c.lod.crossfade,
                },
            )
        })
    })
}

/// Replaces the global LOD settings: a quality setting changes `bias`.
/// Negative or non-finite biases are an error.
/// # Safety
/// `ctx` null or live.
/// ```
/// let s = ponko_ffi::CrabbyLodSettings { bias: 1.0, crossfade: true };
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_set_lod_settings(std::ptr::null_mut(), s) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_lod_settings(ctx: *mut CrabbyCtx, settings: CrabbyLodSettings) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            if !settings.bias.is_finite() || settings.bias < 0.0 {
                return Err(invalid("bias must be 0 or more"));
            }
            c.lod = LodSettings {
                bias: settings.bias,
                crossfade: settings.crossfade,
            };
            Ok(())
        })
    })
}

/// The fraction of the screen's height something `size` tall covers at
/// `distance` from a camera with vertical field of view `fov_y` (radians):
/// to turn distances into thresholds.
/// ```
/// let h = ponko_ffi::crabby_lod_screen_height_at(2.0, 10.0, 60f32.to_radians());
/// assert!((h - 0.1732).abs() < 1e-3);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_lod_screen_height_at(size: f32, distance: f32, fov_y: f32) -> f32 {
    LodGroup::screen_height_at(size, distance, fov_y)
}

/// The distance at which something `size` tall covers `height` of the
/// screen: the inverse of `crabby_lod_screen_height_at`.
/// ```
/// let d = ponko_ffi::crabby_lod_distance_at(2.0, 0.1732, 60f32.to_radians());
/// assert!((d - 10.0).abs() < 0.01);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_lod_distance_at(size: f32, height: f32, fov_y: f32) -> f32 {
    LodGroup::distance_at(size, height, fov_y)
}

/// A GPU array of four-float elements.
/// ```
/// let b = ponko_ffi::CrabbyGpuBuffer { id: 0 }; assert_eq!(b.id, 0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrabbyGpuBuffer {
    /// Engine-internal handle ID.
    pub id: u32,
}

/// Creates a GPU buffer. `count` is zero for zeroed data or equals `len`.
/// # Safety
/// `ctx` is live or null; `data` holds `count` arrays, output is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_gpu_buffer(
    ctx: *mut CrabbyCtx,
    len: u32,
    data: *const [f32; 4],
    count: usize,
    output: *mut CrabbyGpuBuffer,
) -> CrabbyStatus {
    // SAFETY: caller supplies the described arrays and output.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            if output.is_null() || len == 0 || (count != 0 && (count != len as usize || data.is_null())) {
                return Err(invalid("invalid GPU buffer arguments"));
            }
            let data = if count == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(data, count)
                    .iter()
                    .map(|v| Vec4::from_array(*v))
                    .collect()
            };
            let b = c.gfx.create_gpu_buffer(len, &data);
            out(output, CrabbyGpuBuffer { id: b.id() })
        })
    })
}
/// Writes a GPU buffer range. Out-of-range writes return InvalidArgument.
/// # Safety
/// `ctx` is live or null; `data` holds `count` arrays, null allowed for zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_write_gpu_buffer(
    ctx: *mut CrabbyCtx,
    buffer: CrabbyGpuBuffer,
    offset: u32,
    data: *const [f32; 4],
    count: usize,
) -> CrabbyStatus {
    // SAFETY: caller supplies the described array.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let b = GpuBuffer::from_id(buffer.id);
            let len = c.gfx.gpu_buffer_len(b).ok_or_else(|| invalid("invalid GPU buffer"))?;
            if offset > len || count > (len - offset) as usize || (count > 0 && data.is_null()) {
                return Err(invalid("invalid GPU buffer write"));
            }
            let data = if count == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(data, count)
                    .iter()
                    .map(|v| Vec4::from_array(*v))
                    .collect()
            };
            c.gfx.write_gpu_buffer(b, offset, &data);
            Ok(())
        })
    })
}
/// Frees a GPU buffer's memory; its compute passes are removed from the
/// context's list and meshes using it draw nothing. Its id is never
/// reused. An invalid or already freed buffer is an error (unlike
/// `crabby_ctx_handle_free`, which ignores it).
/// # Safety
/// `ctx` is live or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_free_gpu_buffer(ctx: *mut CrabbyCtx, buffer: CrabbyGpuBuffer) -> CrabbyStatus {
    // SAFETY: forwarded context contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let b = GpuBuffer::from_id(buffer.id);
            if c.gfx.gpu_buffer_len(b).is_none() {
                return Err(invalid("invalid GPU buffer"));
            }
            c.free(b);
            Ok(())
        })
    })
}
/// Grows a mesh's culling bounds to hold the sphere `center`, `radius` (in
/// the mesh's space): where a buffer-driven mesh's shader puts its instances.
/// # Safety
/// `ctx` is live or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_mesh_gpu_bounds(
    ctx: *mut CrabbyCtx,
    mesh: CrabbyMesh,
    center: CrabbyVec3,
    radius: f32,
) -> CrabbyStatus {
    // SAFETY: forwarded context contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let mesh = to_c(mesh);
            let center = to_c(center);
            if !c.gfx.is_mesh_live(mesh) || !(radius >= 0.0 && radius.is_finite() && center.is_finite()) {
                return Err(invalid("invalid mesh bounds arguments"));
            }
            c.gfx.set_mesh_gpu_bounds(mesh, center, radius);
            Ok(())
        })
    })
}
/// Draws `count` GPU instances of a mesh, reading `items[v.instance_index]`,
/// each at its entity's transform and tint.
/// # Safety
/// `ctx` is live or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_mesh_gpu_buffer(
    ctx: *mut CrabbyCtx,
    mesh: CrabbyMesh,
    buffer: CrabbyGpuBuffer,
    count: u32,
) -> CrabbyStatus {
    // SAFETY: forwarded context contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let mesh = to_c(mesh);
            let b = GpuBuffer::from_id(buffer.id);
            if !c.gfx.is_mesh_live(mesh) || c.gfx.gpu_buffer_len(b).is_none_or(|len| count > len) {
                return Err(invalid("invalid GPU instance arguments"));
            }
            c.gfx.set_mesh_gpu_buffer(mesh, b, count);
            Ok(())
        })
    })
}

/// Persistent static instance batch handle.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CrabbyInstanceBatch {
    /// Opaque batch identifier.
    pub id: u32,
}
/// Static batch settings. Materials come from the mesh.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CrabbyInstanceBatchOptions {
    /// World-space cell width; finite and positive.
    pub cell_size: f32,
    /// Nonnegative distance from this view's camera or light; infinity disables it.
    pub max_distance: f32,
    /// Whether to cast sun, point-light and spot-light shadows.
    pub cast_shadows: bool,
    /// Uniform sRGB tint.
    pub tint: CrabbyColor,
}
/// Creates a persistent batch from column-major affine matrices.
/// # Safety
/// Context is live or null, matrices hold `count` entries, output is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_instance_batch(
    ctx: *mut CrabbyCtx,
    mesh: CrabbyMesh,
    matrices: *const [f32; 16],
    count: usize,
    options: CrabbyInstanceBatchOptions,
    output: *mut CrabbyInstanceBatch,
) -> CrabbyStatus {
    // SAFETY: caller supplies the described matrices and output.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            if output.is_null()
                || !c.gfx.is_mesh_live(to_c(mesh))
                || !options.cell_size.is_finite()
                || options.cell_size <= 0.0
                || (options.max_distance.is_nan() || options.max_distance < 0.0)
                || count > 16_777_216
                || (count > 0 && matrices.is_null())
            {
                return Err(invalid("invalid instance batch"));
            }
            let data = if count == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(matrices, count)
                    .iter()
                    .map(ponko::Mat4::from_cols_array)
                    .collect::<Vec<_>>()
            };
            if data.iter().any(|m| !m.is_finite() || m.row(3) != ponko::Vec4::W) {
                return Err(invalid("invalid instance transform"));
            }
            let b = c.gfx.create_instance_batch(
                to_c(mesh),
                &data,
                ponko::InstanceBatchOptions {
                    cell_size: options.cell_size,
                    max_distance: options.max_distance,
                    cast_shadows: options.cast_shadows,
                    tint: Color::rgba(options.tint.r, options.tint.g, options.tint.b, options.tint.a),
                },
            );
            out(output, CrabbyInstanceBatch { id: b.id() })
        })
    })
}
/// Replaces transforms and rebuilds spatial cells.
/// # Safety
/// Context is live or null; matrices hold `count` entries.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_update_instance_batch(
    ctx: *mut CrabbyCtx,
    batch: CrabbyInstanceBatch,
    matrices: *const [f32; 16],
    count: usize,
) -> CrabbyStatus {
    // SAFETY: caller supplies the described matrix array.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let b = ponko::InstanceBatch::from_id(batch.id);
            if c.gfx.instance_batch_len(b).is_none() || count > 16_777_216 || (count > 0 && matrices.is_null()) {
                return Err(invalid("invalid instance batch update"));
            }
            let data = if count == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(matrices, count)
                    .iter()
                    .map(ponko::Mat4::from_cols_array)
                    .collect::<Vec<_>>()
            };
            if data.iter().any(|m| !m.is_finite() || m.row(3) != ponko::Vec4::W) {
                return Err(invalid("invalid instance transform"));
            }
            if !c.gfx.update_instance_batch(b, &data) {
                return Err(invalid("invalid instance mesh"));
            }
            Ok(())
        })
    })
}
/// Frees a batch; repeated frees are harmless.
/// # Safety
/// Context is live or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_free_instance_batch(
    ctx: *mut CrabbyCtx,
    batch: CrabbyInstanceBatch,
) -> CrabbyStatus {
    // SAFETY: forwarded context contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.gfx.free_instance_batch(ponko::InstanceBatch::from_id(batch.id));
            Ok(())
        })
    })
}

/// Mesh, half-open origin-distance band, and view flags for a batch level.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CrabbyInstanceLevel {
    /// Live mesh handle.
    pub mesh: CrabbyMesh,
    /// Inclusive finite nonnegative lower distance.
    pub min_distance: f32,
    /// Exclusive upper distance; infinity is allowed.
    pub max_distance: f32,
    /// Draw in camera and target views.
    pub camera: bool,
    /// Cast shadows when the batch and mesh also allow it.
    pub shadows: bool,
}

/// Creates a multi-level batch. Empty colours use the options' tint.
/// # Safety
/// Context is live or null. Nonempty arrays have their stated lengths;
/// output is writable. Matrices are column-major affine transforms.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_instance_batch_levels(
    ctx: *mut CrabbyCtx,
    levels: *const CrabbyInstanceLevel,
    level_count: usize,
    matrices: *const [f32; 16],
    count: usize,
    colors: *const CrabbyColor,
    color_count: usize,
    options: CrabbyInstanceBatchOptions,
    output: *mut CrabbyInstanceBatch,
) -> CrabbyStatus {
    // SAFETY: caller supplies live arrays and writable output as documented.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            if output.is_null()
                || levels.is_null()
                || level_count == 0
                || !options.cell_size.is_finite()
                || options.cell_size <= 0.0
                || options.max_distance.is_nan()
                || options.max_distance < 0.0
            {
                return Err(invalid("invalid instance batch levels"));
            }
            let levels = std::slice::from_raw_parts(levels, level_count);
            if levels.iter().any(|l| {
                !c.gfx.is_mesh_live(to_c(l.mesh))
                    || !l.min_distance.is_finite()
                    || l.min_distance < 0.0
                    || l.max_distance <= l.min_distance
                    || l.max_distance.is_nan()
            }) {
                return Err(invalid("invalid instance level mesh or distance band"));
            }
            let levels: Vec<_> = levels
                .iter()
                .map(|l| ponko::InstanceLevel {
                    mesh: to_c(l.mesh),
                    min_distance: l.min_distance,
                    max_distance: l.max_distance,
                    camera: l.camera,
                    shadows: l.shadows,
                })
                .collect();
            let (transforms, colors) = instance_arrays(matrices, count, colors, color_count)?;
            let b = c.gfx.create_instance_batch_levels(
                &levels,
                &transforms,
                &colors,
                ponko::InstanceBatchOptions {
                    cell_size: options.cell_size,
                    max_distance: options.max_distance,
                    cast_shadows: options.cast_shadows,
                    tint: Color::rgba(options.tint.r, options.tint.g, options.tint.b, options.tint.a),
                },
            );
            out(output, CrabbyInstanceBatch { id: b.id() })
        })
    })
}

/// Replaces transforms and colours; zero colours restore the batch tint.
/// # Safety
/// Context is live or null; nonempty arrays have the stated lengths.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_update_instance_batch_colors(
    ctx: *mut CrabbyCtx,
    batch: CrabbyInstanceBatch,
    matrices: *const [f32; 16],
    count: usize,
    colors: *const CrabbyColor,
    color_count: usize,
) -> CrabbyStatus {
    // SAFETY: forwarded array/context contracts; validation precedes slicing.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let (transforms, colors) = instance_arrays(matrices, count, colors, color_count)?;
            if !c
                .gfx
                .update_instance_batch_colors(ponko::InstanceBatch::from_id(batch.id), &transforms, &colors)
            {
                return Err(invalid("invalid instance batch or level mesh"));
            }
            Ok(())
        })
    })
}

/// Caller guarantees non-null arrays have their stated lengths.
unsafe fn instance_arrays(
    matrices: *const [f32; 16],
    count: usize,
    colors: *const CrabbyColor,
    color_count: usize,
) -> Result<(Vec<ponko::Mat4>, Vec<Color>), crate::Failure> {
    if count > 16_777_216
        || (count > 0 && matrices.is_null())
        || (color_count != 0 && color_count != count)
        || (color_count > 0 && colors.is_null())
    {
        return Err(invalid("invalid instance transform or color count"));
    }
    // SAFETY: pointer/length contracts are forwarded; zero counts avoid null slices.
    let transforms = if count == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(matrices, count) }
            .iter()
            .map(ponko::Mat4::from_cols_array)
            .collect::<Vec<_>>()
    };
    if transforms.iter().any(|m| !m.is_finite() || m.row(3) != ponko::Vec4::W) {
        return Err(invalid("invalid instance transform"));
    }
    // SAFETY: as above, and colour length was validated before slicing.
    let colors = if color_count == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(colors, color_count) }
            .iter()
            .map(|c| Color::rgba(c.r, c.g, c.b, c.a))
            .collect()
    };
    Ok((transforms, colors))
}
