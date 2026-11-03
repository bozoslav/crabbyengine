//! 3D from C: `CrabbyTransform3D` and `CrabbyModel` components (plain
//! values, so query columns too), meshes and materials, the 3D camera and
//! the sun.
use std::ffi::c_char;

use crabbyengine::{
    Blend, Camera3D, Color, Lighting, Material, MaterialDesc, Mesh, MeshData, Model, Post, Quat, Shader, Tonemap,
    Transform3D, Vec2, Vec3, Vec4,
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
    /// 0 opaque (alpha below 0.5 cut out), 1 transparent.
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
}

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
        _ => return Err(invalid("blend must be 0 (opaque) or 1 (transparent)")),
    };
    let shader = d.has_shader.then_some(Shader::from_id(d.shader.id));
    if shader.is_some_and(|s| !c.has_shader(s)) {
        return Err(invalid("that shader wasn't made by this game"));
    }
    let map = |has: bool, t: CrabbyTexture| has.then(|| texture_from_c(t));
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
            let Some(&cam) = camera.as_ref() else {
                return Err(invalid("camera is null"));
            };
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
            let rotation = rotation.normalize();
            c.camera3d.position = to_c(cam.position);
            c.camera3d.rotation = rotation;
            c.camera3d.fov_y = cam.fov_y;
            c.camera3d.near = cam.near;
            c.camera3d.far = cam.far;
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
                },
            )
        })
    })
}

/// Changes the sun and sky light. Intensities and the shadow distance
/// can't be negative.
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
