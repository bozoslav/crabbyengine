//! 3D from C: `CrabbyTransform3D` and `CrabbyModel` components (plain
//! values, so query columns too), meshes and materials, the 3D camera and
//! the sun.
use std::ffi::c_char;

use crabbyengine::{
    Blend, Camera3D, Color, Compute, ComputePass, Effect, Lighting, Material, MaterialDesc, Mesh, MeshData, Model,
    Post, Quat, ScreenEffect, Shader, Sky, TERRAIN_LAYERS, TERRAIN_LODS, TargetCamera, TerrainMaterialDesc,
    TerrainTile, TextureArray, TextureKind, Tonemap, Transform3D, Vec2, Vec3, Vec4,
};

use crate::ctx::{ctx_access, out, text};
use crate::types::*;
use crate::{CrabbyCtx, CrabbyStatus, CrabbyWorld, boundary, builtin, invalid, invalid_owned};

/// A point or direction in 3D: right-handed, +Y up.
/// ```
/// let up = crabby_ffi::CrabbyVec3 { x: 0.0, y: 1.0, z: 0.0 };
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
/// let none = crabby_ffi::crabby_quat_identity();
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
/// let t = crabby_ffi::crabby_transform3d_at(Default::default());
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

/// A mesh on the GPU (`crabby_ctx_create_mesh`, `crabby_ctx_load_mesh`).
/// ```
/// let m = crabby_ffi::CrabbyMesh { id: 0 };
/// assert_eq!(m.id, 0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrabbyMesh {
    /// Engine-internal id.
    pub id: u32,
}

/// A material (`crabby_ctx_create_material`).
/// ```
/// let m = crabby_ffi::CrabbyMaterial { id: 0 };
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
/// let m = crabby_ffi::crabby_model(crabby_ffi::CrabbyMesh { id: 3 });
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
/// let t = crabby_ffi::CrabbyTerrainTile { lods: [crabby_ffi::CrabbyMesh { id: 0 }; 4], lod_distance: 256.0 };
/// assert_eq!(t.lods.len(), crabby_ffi::CRABBY_TERRAIN_LODS);
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
/// let c = crabby_ffi::CrabbyCompute { id: 0 };
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
/// let p = crabby_ffi::CrabbyComputePass {
///     compute: crabby_ffi::CrabbyCompute { id: 0 },
///     target: crabby_ffi::CrabbyTexture { id: 0, width: 8, height: 8 },
///     has_material: false,
///     material: Default::default(),
/// };
/// assert!(!p.has_material);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrabbyComputePass {
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
/// let e = crabby_ffi::CrabbyEffect { id: 0 };
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
/// let s = crabby_ffi::CrabbyScreenEffect { effect: crabby_ffi::CrabbyEffect { id: 0 }, has_material: false, material: Default::default() };
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
/// let t = crabby_ffi::CrabbyTargetCamera {
///     camera: crabby_ffi::CrabbyCamera3D { position: Default::default(), rotation: crabby_ffi::crabby_quat_identity(), fov_y: 1.0, near: 0.1, far: 100.0 },
///     target: crabby_ffi::CrabbyTexture { id: 0, width: 64, height: 64 },
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
/// let s = crabby_ffi::CrabbyShader { id: 0 };
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
/// let d = crabby_ffi::crabby_material_desc(crabby_ffi::CrabbyColor { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });
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
/// let a = crabby_ffi::CrabbyTextureArray { id: 0, width: 64, height: 64, layers: 3 };
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
/// let a = crabby_ffi::CrabbyTextureArray::default();
/// let d = crabby_ffi::crabby_terrain_material_desc(a, a, Default::default(), crabby_ffi::CrabbyVec2 { x: 256.0, y: 256.0 });
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
/// let c = crabby_ffi::CrabbyCamera3D { position: Default::default(), rotation: crabby_ffi::crabby_quat_identity(), fov_y: 1.0, near: 0.1, far: 100.0 };
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
/// let mut l = std::mem::MaybeUninit::<crabby_ffi::CrabbyLighting>::uninit();
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_lighting(std::ptr::null_mut(), l.as_mut_ptr()) } as u32, 1);
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
}

/// The sky behind the 3D world and the fog in front of it (Rust
/// `ctx.sky`, `Sky`).
/// ```
/// let mut s = std::mem::MaybeUninit::<crabby_ffi::CrabbySky>::uninit();
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_sky(std::ptr::null_mut(), s.as_mut_ptr()) } as u32, 1);
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
}

/// Which way sunlight travels at `hour` o'clock, for a day and night cycle
/// (Rust `Sky::sun_direction_at`): rising in the east (+X) at 6, highest
/// at noon, setting in the west at 18.
/// ```
/// assert!(crabby_ffi::crabby_sun_direction_at(12.0).y < -0.8);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_sun_direction_at(hour: f32) -> CrabbyVec3 {
    to_c(Sky::sun_direction_at(hour))
}

/// What happens to the lit 3D frame on its way to the screen (Rust
/// `ctx.post`). `tonemap`: 0 ACES (default), 1 AgX, 2 Khronos PBR Neutral,
/// 3 none. `lut` is used when `has_lut`.
/// ```
/// let mut p = std::mem::MaybeUninit::<crabby_ffi::CrabbyPost>::uninit();
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_post(std::ptr::null_mut(), p.as_mut_ptr()) } as u32, 1);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyPost {
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
    /// Whether `lut` is used.
    pub has_lut: bool,
    /// A color lookup table: n squares of n x n pixels in a row.
    pub lut: CrabbyTexture,
}

/// Triangles on the CPU (Rust `MeshData`), before `crabby_ctx_create_mesh`
/// puts them on the GPU. An owned handle: free it with `crabby_mesh_data_free`.
pub struct CrabbyMeshData(MeshData);

/// A read-only look at a `CrabbyMeshData`'s arrays, valid until it's freed.
/// `normals` and `uvs` are null when it has none.
/// ```
/// let v = crabby_ffi::CrabbyMeshView { positions: std::ptr::null(), normals: std::ptr::null(), uvs: std::ptr::null(), vertex_count: 0, indices: std::ptr::null(), index_count: 0 };
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

fn quat_to_c(q: Quat) -> CrabbyQuat {
    let [x, y, z, w] = q.to_array();
    CrabbyQuat { x, y, z, w }
}
fn quat_from_c(q: CrabbyQuat) -> Quat {
    Quat::from_xyzw(q.x, q.y, q.z, q.w)
}
fn transform_to_c(t: Transform3D) -> CrabbyTransform3D {
    CrabbyTransform3D {
        rotation: quat_to_c(t.rotation),
        position: to_c(t.position),
        scale: to_c(t.scale),
        reserved: [0.0; 2],
    }
}
fn transform_from_c(t: CrabbyTransform3D) -> Transform3D {
    Transform3D {
        rotation: quat_from_c(t.rotation),
        position: to_c(t.position),
        scale: to_c(t.scale),
    }
}

// ---- values -----------------------------------------------------------------

/// No rotation.
/// ```
/// assert_eq!(crabby_ffi::crabby_quat_identity().w, 1.0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_quat_identity() -> CrabbyQuat {
    quat_to_c(Quat::IDENTITY)
}

/// A rotation of `angle` radians around `axis` (counter-clockwise looking
/// down the axis). A zero axis gives no rotation.
/// ```
/// use crabby_ffi::*;
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
/// use crabby_ffi::*;
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
/// use crabby_ffi::*;
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
/// let t = crabby_ffi::crabby_transform3d_at(crabby_ffi::CrabbyVec3 { x: 1.0, y: 2.0, z: 3.0 });
/// assert_eq!(t.position.z, 3.0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_transform3d_at(position: CrabbyVec3) -> CrabbyTransform3D {
    transform_to_c(Transform3D::at(to_c(position)))
}

/// `t` turned so its -Z axis points at `target`, +Y up
/// (Rust `Transform3D::looking_at`).
/// ```
/// use crabby_ffi::*;
/// let t = crabby_transform3d_looking_at(crabby_transform3d_at(CrabbyVec3 { x: 0.0, y: 0.0, z: 5.0 }), Default::default());
/// assert!((crabby_transform3d_forward(t).z + 1.0).abs() < 1e-5);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_transform3d_looking_at(t: CrabbyTransform3D, target: CrabbyVec3) -> CrabbyTransform3D {
    transform_to_c(transform_from_c(t).looking_at(to_c(target)))
}

/// The direction `t` faces: its -Z axis.
/// ```
/// let f = crabby_ffi::crabby_transform3d_forward(crabby_ffi::crabby_transform3d_at(Default::default()));
/// assert_eq!(f.z, -1.0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_transform3d_forward(t: CrabbyTransform3D) -> CrabbyVec3 {
    to_c(transform_from_c(t).forward())
}

/// A model of `mesh` in its own colors (Rust `Model::new`).
/// ```
/// assert_eq!(crabby_ffi::crabby_model(crabby_ffi::CrabbyMesh { id: 1 }).color.a, 1.0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_model(mesh: CrabbyMesh) -> CrabbyModel {
    to_c(Model::new(to_c(mesh)))
}

/// A plain material of one color (Rust `MaterialDesc::color`): roughness
/// 0.6, not metallic, no texture, no glow.
/// ```
/// let d = crabby_ffi::crabby_material_desc(crabby_ffi::CrabbyColor { r: 1.0, g: 1.0, b: 1.0, a: 1.0 });
/// assert_eq!(d.emissive.r, 0.0);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_material_desc(color: CrabbyColor) -> CrabbyMaterialDesc {
    let d = MaterialDesc::color(to_c(color));
    let none = CrabbyTexture {
        id: 0,
        width: 0,
        height: 0,
    };
    CrabbyMaterialDesc {
        color: to_c(d.color),
        has_texture: false,
        texture: none,
        roughness: d.roughness,
        metallic: d.metallic,
        emissive: to_c(d.emissive),
        has_normal_map: false,
        normal_map: none,
        normal_strength: d.normal_strength,
        has_orm_map: false,
        orm_map: none,
        has_emissive_map: false,
        emissive_map: none,
        blend: 0,
        double_sided: false,
        has_shader: false,
        shader: CrabbyShader { id: 0 },
        custom: [0.0; 16],
        has_custom_textures: [false; 2],
        custom_textures: [none; 2],
        has_custom_arrays: [false; 3],
        custom_arrays: [CrabbyTextureArray::default(); 3],
    }
}

/// A C material description as Rust's, checked.
fn material_from_c(c: &crabbyengine::Ctx, d: &CrabbyMaterialDesc) -> Result<MaterialDesc, crate::Failure> {
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
    if shader.is_some_and(|s| !c.has_shader(s)) {
        return Err(invalid("that shader wasn't made by this game"));
    }
    let map = |has: bool, t: CrabbyTexture| has.then(|| texture_from_c(t));
    let arrays = std::array::from_fn(|i| d.has_custom_arrays[i].then(|| array_from_c(d.custom_arrays[i])));
    if arrays.iter().flatten().any(|a| !c.has_texture_array(*a)) {
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
/// let a = crabby_ffi::CrabbyTextureArray::default();
/// let d = crabby_ffi::crabby_terrain_material_desc(a, a, Default::default(), crabby_ffi::CrabbyVec2 { x: 1.0, y: 1.0 });
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
/// `Ctx::create_texture_array`): `rgba` holds them one after another, 4
/// bytes a pixel, `len` bytes in all (a whole number of images). `kind` is
/// `CRABBY_TEXTURE_COLOR` or `CRABBY_TEXTURE_DATA`.
/// # Safety
/// `ctx` null or live; `rgba` holds `len` bytes; output writable.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_create_texture_array(std::ptr::null_mut(), 1, 1, std::ptr::null(), 0, 0, std::ptr::null_mut()) } as u32, 1);
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
                .try_create_texture_array(width, height, &layers, kind)
                .map_err(invalid_owned)?;
            out(out_array, array_to_c(array))
        })
    })
}

/// Loads `count` images from assets/, all the same size, into a texture
/// array, one layer each (Rust `Ctx::load_texture_array`). `kind` is
/// `CRABBY_TEXTURE_COLOR` or `CRABBY_TEXTURE_DATA`.
/// # Safety
/// `ctx` null or live; `paths` holds `count` NUL-terminated strings; output writable.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_load_texture_array(std::ptr::null_mut(), std::ptr::null(), 0, 0, std::ptr::null_mut()) } as u32, 1);
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
            let array = c.try_load_texture_array(&names, kind).map_err(invalid_owned)?;
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

/// Makes a terrain material (Rust `Ctx::create_terrain_material`): an
/// ordinary material, for `crabby_ctx_create_terrain_tile` or any mesh.
/// # Safety
/// `ctx` null or live; `desc` readable; output writable.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_create_terrain_material(std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
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
            let material = c.try_create_terrain_material(&rust).map_err(invalid_owned)?;
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
/// use crabby_ffi::*;
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
/// use crabby_ffi::*;
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
/// use crabby_ffi::*;
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
/// use crabby_ffi::*;
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
            colors: Vec::new(),
        };
        data.check().map_err(invalid_owned)?;
        result = boxed(data);
        Ok(())
    });
    result
}

/// A cube `size` wide around the origin (Rust `MeshData::cube`).
/// ```
/// let d = crabby_ffi::crabby_mesh_data_cube(1.0);
/// unsafe { crabby_ffi::crabby_mesh_data_free(d) };
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_mesh_data_cube(size: f32) -> *mut CrabbyMeshData {
    boxed(MeshData::cube(size))
}

/// A flat square `size` wide on the XZ plane, facing up (Rust `MeshData::plane`).
/// ```
/// let d = crabby_ffi::crabby_mesh_data_plane(10.0);
/// unsafe { crabby_ffi::crabby_mesh_data_free(d) };
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_mesh_data_plane(size: f32) -> *mut CrabbyMeshData {
    boxed(MeshData::plane(size))
}

/// A sphere of `radius` from `segments` slices around (at least 3; Rust
/// `MeshData::sphere`).
/// ```
/// let d = crabby_ffi::crabby_mesh_data_sphere(0.5, 24);
/// unsafe { crabby_ffi::crabby_mesh_data_free(d) };
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
/// let d = unsafe { crabby_ffi::crabby_mesh_data_heightfield(h.as_ptr(), 4, 2.0, 1) };
/// assert!(!d.is_null());
/// assert!(unsafe { crabby_ffi::crabby_mesh_data_heightfield(h.as_ptr(), 3, 2.0, 1) }.is_null());
/// unsafe { crabby_ffi::crabby_mesh_data_free(d) };
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
/// use crabby_ffi::{CrabbyStatus::*, *};
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
/// unsafe { crabby_ffi::crabby_mesh_data_free(std::ptr::null_mut()) };
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
/// use crabby_ffi::*;
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
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_create_material(std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
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
            out(out_material, to_c(c.create_material(desc)))
        })
    })
}

/// Changes a material: every model drawn with it changes too.
/// # Safety
/// `ctx` null or live; `desc` valid.
/// ```
/// let m = crabby_ffi::CrabbyMaterial { id: 0 };
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_update_material(std::ptr::null_mut(), m, std::ptr::null()) } as u32, 1);
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
            if !c.has_material(material) {
                return Err(invalid("that material wasn't made by this game"));
            }
            let desc = material_from_c(c, d)?;
            c.update_material(material, desc);
            Ok(())
        })
    })
}

/// Loads a material shader (WGSL) from `assets/`; with hot reload on,
/// saving the file updates the game. A file that can't be read or doesn't
/// compile is an error; `crabby_last_error` says where.
/// # Safety
/// `ctx` null or live; `path` a C string; output writable.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_load_shader(std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
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
            let shader = c.try_load_shader(path).map_err(invalid_owned)?;
            out(out_shader, CrabbyShader { id: shader.id() })
        })
    })
}

/// Makes a material shader from WGSL source; `name` is for error messages.
/// Source that doesn't compile is an error; `crabby_last_error` says where.
/// # Safety
/// `ctx` null or live; `name` and `source` C strings; output writable.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_create_shader(std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
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
            let shader = c.try_create_shader(name, source).map_err(invalid_owned)?;
            out(out_shader, CrabbyShader { id: shader.id() })
        })
    })
}

/// Loads a compute pass shader (WGSL, defining `fn compute(c: Cell) ->
/// vec4<f32>`) from assets/, hot reloaded; the same file again gives the
/// same compute pass (Rust `Ctx::load_compute`).
/// # Safety
/// `ctx` null or live; `path` a NUL-terminated string; output writable.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_load_compute(std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
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
            let compute = c.try_load_compute(path).map_err(invalid_owned)?;
            out(out_compute, CrabbyCompute { id: compute.id() })
        })
    })
}

/// Makes a compute pass shader from WGSL source; `name` is for error
/// messages (Rust `Ctx::create_compute`).
/// # Safety
/// `ctx` null or live; `name` and `source` NUL-terminated strings; output writable.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_create_compute(std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
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
            let compute = c.try_create_compute(name, source).map_err(invalid_owned)?;
            out(out_compute, CrabbyCompute { id: compute.id() })
        })
    })
}

/// Replaces the compute passes run each frame with `count` of them, in
/// order (Rust `ctx.compute_passes`); 0 runs none.
/// # Safety
/// `ctx` null or live; `passes` holds `count` values (may be null when 0).
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_set_compute_passes(std::ptr::null_mut(), std::ptr::null(), 0) } as u32, 1);
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
                if !c.has_compute(compute) {
                    return Err(invalid("that compute pass wasn't made by this game"));
                }
                let target = texture_from_c(p.target);
                if !c.is_render_target(target) {
                    return Err(invalid("that texture isn't a render target of this game"));
                }
                let material = p.has_material.then(|| to_c(p.material));
                if material.is_some_and(|m| !c.has_material(m)) {
                    return Err(invalid("that material wasn't made by this game"));
                }
                list.push(ComputePass {
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
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_compute_passes(std::ptr::null_mut(), std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
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
/// same effect (Rust `Ctx::load_effect`).
/// # Safety
/// `ctx` null or live; `path` a NUL-terminated string; output writable.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_load_effect(std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
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
            let effect = c.try_load_effect(path).map_err(invalid_owned)?;
            out(out_effect, CrabbyEffect { id: effect.id() })
        })
    })
}

/// Makes a screen effect shader from WGSL source; `name` is for error
/// messages (Rust `Ctx::create_effect`).
/// # Safety
/// `ctx` null or live; `name` and `source` NUL-terminated strings; output writable.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_create_effect(std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
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
            let effect = c.try_create_effect(name, source).map_err(invalid_owned)?;
            out(out_effect, CrabbyEffect { id: effect.id() })
        })
    })
}

/// Replaces the screen effects run each frame with `count` of them, in
/// order (Rust `ctx.effects`); 0 runs none.
/// # Safety
/// `ctx` null or live; `effects` holds `count` values (may be null when 0).
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_set_effects(std::ptr::null_mut(), std::ptr::null(), 0) } as u32, 1);
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
                if !c.has_effect(effect) {
                    return Err(invalid("that effect wasn't made by this game"));
                }
                let material = e.has_material.then(|| to_c(e.material));
                if material.is_some_and(|m| !c.has_material(m)) {
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
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_effects(std::ptr::null_mut(), std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
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
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_default_material(std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_default_material(
    ctx: *mut CrabbyCtx,
    out_material: *mut CrabbyMaterial,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| out(out_material, to_c(c.default_material()))) })
}

/// Puts mesh data on the GPU as a mesh of one material. The data can be
/// freed afterwards.
/// # Safety
/// `ctx` null or live; `data` live; output writable.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_create_mesh(std::ptr::null_mut(), std::ptr::null(), Default::default(), std::ptr::null_mut()) } as u32, 1);
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
            if !c.has_material(to_c(material)) {
                return Err(invalid("that material wasn't made by this game"));
            }
            out(out_mesh, to_c(c.create_mesh(&data.0, to_c(material))))
        })
    })
}

/// Makes a terrain tile `size` wide from `count` heights (n x n, row after
/// row along +Z, NaN for a hole) drawn with `material` (Rust
/// `Ctx::create_terrain_tile`). Spawn it with a `CrabbyTransform3D` at the
/// tile's corner.
/// # Safety
/// `ctx` null or live; `heights` holds `count` floats; output writable.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_create_terrain_tile(std::ptr::null_mut(), std::ptr::null(), 0, 1.0, Default::default(), std::ptr::null_mut()) } as u32, 1);
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
            if !c.has_material(to_c(material)) {
                return Err(invalid("that material wasn't made by this game"));
            }
            let heights = std::slice::from_raw_parts(heights, count);
            out(out_tile, to_c(c.create_terrain_tile(heights, size, to_c(material))))
        })
    })
}

/// Loads a glTF model (`.glb`, or `.gltf` with its files) from assets/ as
/// one mesh with its materials. Loading the same file again returns the
/// same mesh.
/// # Safety
/// `ctx` null or live; `path` a NUL-terminated string; output writable.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_load_mesh(std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut()) } as u32, 1);
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
            out(out_mesh, to_c(c.load_mesh(path)))
        })
    })
}

/// How many 3D models were drawn last frame.
/// # Safety
/// `ctx` null or live; output writable.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_models_drawn(std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_models_drawn(ctx: *mut CrabbyCtx, out_count: *mut usize) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe { ctx_access(ctx, |c| out(out_count, c.models_drawn())) })
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
/// the 3D world into (Rust `Ctx::create_render_target`); materials use it
/// like any texture.
/// # Safety
/// `ctx` null or live; output writable.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_create_render_target(std::ptr::null_mut(), 64, 64, std::ptr::null_mut()) } as u32, 1);
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
            let texture = c.try_create_render_target(width, height).map_err(invalid_owned)?;
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
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_reflection_camera(std::ptr::null_mut(), std::ptr::null(), crabby_ffi::CrabbyTexture { id: 0, width: 1, height: 1 }, 0.0, std::ptr::null_mut()) } as u32, 1);
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
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_set_target_cameras(std::ptr::null_mut(), std::ptr::null(), 0) } as u32, 1);
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
                if !c.is_render_target(target) {
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
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_target_cameras(std::ptr::null_mut(), std::ptr::null_mut(), 0, std::ptr::null_mut()) } as u32, 1);
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
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_camera3d(std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
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
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_set_camera3d(std::ptr::null_mut(), std::ptr::null()) } as u32, 1);
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

/// Turns the 3D camera to look at `target`, +Y up.
/// # Safety
/// `ctx` null or live.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_camera3d_look_at(std::ptr::null_mut(), Default::default()) } as u32, 1);
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
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_screen_ray(std::ptr::null_mut(), Default::default(), std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 1);
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
                },
            )
        })
    })
}

/// Changes the sun and sky light. Intensities and the shadow distance
/// can't be negative; `shadow_cascades` is 1 to 4.
/// # Safety
/// `ctx` null or live; `lighting` valid.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_set_lighting(std::ptr::null_mut(), std::ptr::null()) } as u32, 1);
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
                },
            )
        })
    })
}

/// Changes the sky and fog. Haze, stars and fog density can't be negative;
/// the sun's size and the fog falloff must be above 0.
/// # Safety
/// `ctx` null or live; `sky` valid.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_set_sky(std::ptr::null_mut(), std::ptr::null()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_sky(ctx: *mut CrabbyCtx, sky: *const CrabbySky) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let Some(&s) = sky.as_ref() else {
                return Err(invalid("sky is null"));
            };
            let finite = [s.haze, s.sun_size, s.stars, s.fog_density, s.fog_height, s.fog_falloff]
                .iter()
                .all(|v| v.is_finite());
            if !finite || s.haze < 0.0 || s.stars < 0.0 || s.fog_density < 0.0 {
                return Err(invalid("haze, stars and fog density can't be negative"));
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
                    exposure: p.exposure,
                    tonemap: p.tonemap as u32,
                    bloom: p.bloom,
                    bloom_threshold: p.bloom_threshold,
                    contrast: p.contrast,
                    saturation: p.saturation,
                    temperature: p.temperature,
                    tint: p.tint,
                    vignette: p.vignette,
                    has_lut: p.lut.is_some(),
                    lut: p.lut.map(texture_to_c).unwrap_or(CrabbyTexture {
                        id: 0,
                        width: 0,
                        height: 0,
                    }),
                },
            )
        })
    })
}

/// Changes the 3D post-processing settings. `tonemap` must be 0 to 3;
/// bloom, its threshold, contrast and saturation can't be negative.
/// # Safety
/// `ctx` null or live; `post` valid.
/// ```
/// assert_eq!(unsafe { crabby_ffi::crabby_ctx_set_post(std::ptr::null_mut(), std::ptr::null()) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_post(ctx: *mut CrabbyCtx, post: *const CrabbyPost) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let Some(&p) = post.as_ref() else {
                return Err(invalid("post is null"));
            };
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
                exposure: p.exposure,
                tonemap,
                bloom: p.bloom,
                bloom_threshold: p.bloom_threshold,
                contrast: p.contrast,
                saturation: p.saturation,
                temperature: p.temperature,
                tint: p.tint,
                vignette: p.vignette,
                lut: p.has_lut.then(|| texture_from_c(p.lut)),
            };
            Ok(())
        })
    })
}

/// A terrain tile's heights for finding the ground on the CPU (Rust
/// `Heightfield`). Make it with `crabby_heightfield_new`.
pub struct CrabbyHeightfield(crabbyengine::Heightfield);

/// Makes a heightfield `size` wide from `count` heights (n x n, as
/// `crabby_ctx_create_terrain_tile` takes them). Returns null (see
/// `crabby_last_error`) for bad input.
/// ```
/// let h = [0.0f32; 9];
/// let g = unsafe { crabby_ffi::crabby_heightfield_new(h.as_ptr(), 9, 4.0) };
/// assert!(!g.is_null());
/// unsafe { crabby_ffi::crabby_heightfield_free(g) };
/// assert!(unsafe { crabby_ffi::crabby_heightfield_new(h.as_ptr(), 8, 4.0) }.is_null());
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
        made = Box::into_raw(Box::new(CrabbyHeightfield(crabbyengine::Heightfield::new(
            heights, size,
        ))));
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
/// unsafe { crabby_ffi::crabby_heightfield_free(std::ptr::null_mut()) };
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
/// use crabby_ffi::*;
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
