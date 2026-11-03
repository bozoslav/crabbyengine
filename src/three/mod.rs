//! 3D: meshes with materials, drawn from entities with a [`Transform3D`] and
//! a [`Model`], through a perspective [`Camera3D`], lit by a sun and ambient
//! light ([`Lighting`]). Behind the `3d` feature.
//!
//! The 3D pass is drawn first, with a depth buffer; 2D sprites, text and UI
//! go on top of it. Games without 3D models never run it, and builds without
//! the feature don't have it at all.
//!
//! Coordinates are right-handed with Y up (as in glTF): a camera with no
//! rotation looks down -Z.

mod api;
mod collect;
mod gltf;
mod mesh;

pub(crate) use api::MaterialUniform;
pub(crate) use collect::{Frame3D, Instance3D, SHADOW_MAP_SIZE};
pub use mesh::MeshData;

use glam::{Mat3, Mat4, Quat, Vec2, Vec3, Vec4};

use crate::{Color, Texture};

/// Position, rotation and scale of a 3D entity.
///
/// ```
/// # use crabbyengine::*;
/// let crate_box = Transform3D::at(vec3(0.0, 0.5, -3.0)).rotated(Quat::from_rotation_y(0.3));
/// # let _ = crate_box;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Transform3D {
    /// Where it is, in world units.
    pub position: Vec3,
    /// Which way it faces.
    pub rotation: Quat,
    /// Size along its own axes. Default `(1, 1, 1)`.
    pub scale: Vec3,
}

impl Default for Transform3D {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform3D {
    /// At `position`, not rotated, scale 1.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let t = Transform3D::at(vec3(1.0, 2.0, 3.0));
    /// assert_eq!(t.scale, Vec3::ONE);
    /// ```
    #[must_use]
    pub fn at(position: Vec3) -> Self {
        Self {
            position,
            ..Self::default()
        }
    }

    /// The same, rotated by `rotation`.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let t = Transform3D::default().rotated(Quat::from_rotation_y(1.0));
    /// # let _ = t;
    /// ```
    #[must_use]
    pub fn rotated(self, rotation: Quat) -> Self {
        Self { rotation, ..self }
    }

    /// The same, scaled by `scale` on every axis.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// assert_eq!(Transform3D::default().scaled(2.0).scale, Vec3::splat(2.0));
    /// ```
    #[must_use]
    pub fn scaled(self, scale: f32) -> Self {
        Self {
            scale: Vec3::splat(scale),
            ..self
        }
    }

    /// The same, turned so its -Z axis points at `target` (with +Y as up).
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let t = Transform3D::at(vec3(0.0, 0.0, 5.0)).looking_at(Vec3::ZERO);
    /// assert!((t.forward() - vec3(0.0, 0.0, -1.0)).length() < 1e-5);
    /// ```
    #[must_use]
    pub fn looking_at(self, target: Vec3) -> Self {
        Self {
            rotation: look_rotation(target - self.position),
            ..self
        }
    }

    /// The direction it faces: its -Z axis.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// assert_eq!(Transform3D::default().forward(), vec3(0.0, 0.0, -1.0));
    /// ```
    #[must_use]
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::NEG_Z
    }

    /// The matrix from its local space to the world.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let m = Transform3D::at(vec3(1.0, 0.0, 0.0)).matrix();
    /// assert_eq!(m.transform_point3(Vec3::ZERO), vec3(1.0, 0.0, 0.0));
    /// ```
    #[must_use]
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

/// The rotation that turns -Z towards `direction`, keeping +Y up (or +Z up
/// when looking straight up or down).
fn look_rotation(direction: Vec3) -> Quat {
    let forward = direction.try_normalize().unwrap_or(Vec3::NEG_Z);
    let up = if forward.y.abs() > 0.999 { Vec3::Z } else { Vec3::Y };
    let right = forward.cross(up).normalize();
    let up = right.cross(forward);
    Quat::from_mat3(&Mat3::from_cols(right, up, -forward))
}

/// A mesh on the GPU: triangles, split into parts that each have a
/// [`Material`]. Made with [`Ctx::create_mesh`](crate::Ctx::create_mesh) or
/// loaded with [`Ctx::load_mesh`](crate::Ctx::load_mesh); a cheap handle.
///
/// ```no_run
/// # use crabbyengine::*;
/// # fn f(ctx: &mut Ctx) {
/// let tree: Mesh = ctx.load_mesh("tree.glb");
/// ctx.world.spawn((Transform3D::default(), Model::new(tree)));
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Mesh {
    pub(crate) id: u32,
}

/// How a surface looks: its color (and texture), how rough and how metallic
/// it is, and light it gives off. Made with
/// [`Ctx::create_material`](crate::Ctx::create_material); a cheap handle.
///
/// ```no_run
/// # use crabbyengine::*;
/// # fn f(ctx: &mut Ctx) {
/// let gold = ctx.create_material(MaterialDesc { metallic: 1.0, roughness: 0.3, ..MaterialDesc::color(Color::hex(0xFFC83D)) });
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Material {
    pub(crate) id: u32,
}

/// What a [`Material`] is made from.
///
/// ```
/// # use crabbyengine::*;
/// let grass = MaterialDesc { roughness: 0.9, ..MaterialDesc::color(Color::hex(0x4C8A3A)) };
/// assert_eq!(grass.metallic, 0.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialDesc {
    /// Base color, multiplied with the texture. Default white.
    pub color: Color,
    /// An image on the surface (by the mesh's texture coordinates).
    pub texture: Option<Texture>,
    /// 0 is mirror-smooth, 1 is fully rough (matte). Default 0.6.
    pub roughness: f32,
    /// 0 for most things, 1 for bare metal. Default 0.
    pub metallic: f32,
    /// Light it gives off itself (not lighting anything else). Default black.
    pub emissive: Color,
}

impl Default for MaterialDesc {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            texture: None,
            roughness: 0.6,
            metallic: 0.0,
            emissive: Color::rgb(0.0, 0.0, 0.0),
        }
    }
}

impl MaterialDesc {
    /// A plain material of one color.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let red = MaterialDesc::color(Color::rgb(0.8, 0.1, 0.1));
    /// # let _ = red;
    /// ```
    #[must_use]
    pub fn color(color: Color) -> Self {
        Self {
            color,
            ..Self::default()
        }
    }
}

/// Draws a [`Mesh`] where the entity's [`Transform3D`] is.
///
/// ```no_run
/// # use crabbyengine::*;
/// # fn f(ctx: &mut Ctx) {
/// let cube = ctx.create_mesh(&MeshData::cube(1.0), ctx.default_material());
/// for i in 0..10 {
///     let tint = Color::rgb(i as f32 / 10.0, 0.5, 1.0);
///     ctx.world.spawn((Transform3D::at(vec3(i as f32 * 2.0, 0.0, 0.0)), Model::new(cube).with_color(tint)));
/// }
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Model {
    /// What to draw.
    pub mesh: Mesh,
    /// Multiplied with the materials' colors, so one mesh can come in many
    /// colors without more materials. Default white (no change).
    pub color: Color,
}

impl Model {
    /// Draws `mesh` as it is.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let rock = ctx.load_mesh("rock.glb");
    /// ctx.world.spawn((Transform3D::default(), Model::new(rock)));
    /// # }
    /// ```
    #[must_use]
    pub fn new(mesh: Mesh) -> Self {
        Self {
            mesh,
            color: Color::WHITE,
        }
    }

    /// The same, tinted with `color`.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx, cube: Mesh) {
    /// let red_cube = Model::new(cube).with_color(Color::rgb(1.0, 0.2, 0.2));
    /// # }
    /// ```
    #[must_use]
    pub fn with_color(self, color: Color) -> Self {
        Self { color, ..self }
    }
}

/// The 3D camera: where it is, which way it looks, and how wide it sees.
/// One per game, at [`Ctx::camera3d`](crate::Ctx::camera3d).
///
/// ```no_run
/// # use crabbyengine::*;
/// # fn f(ctx: &mut Ctx) {
/// ctx.camera3d.position = vec3(0.0, 4.0, 10.0);
/// ctx.camera3d.look_at(Vec3::ZERO);
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera3D {
    /// Where the camera is.
    pub position: Vec3,
    /// Which way it looks: it sees along its -Z axis.
    pub rotation: Quat,
    /// How tall the view is, as an angle in radians. Default 60°.
    pub fov_y: f32,
    /// Nothing closer than this is drawn. Default 0.1.
    pub near: f32,
    /// Nothing farther than this is drawn. Default 1000.
    pub far: f32,
    /// Window size in pixels, kept up to date by the engine.
    viewport: Vec2,
}

impl Camera3D {
    pub(crate) fn new(viewport: Vec2) -> Self {
        Self {
            position: Vec3::new(0.0, 2.0, 8.0),
            rotation: look_rotation(Vec3::new(0.0, -2.0, -8.0)),
            fov_y: 60f32.to_radians(),
            near: 0.1,
            far: 1000.0,
            viewport,
        }
    }

    pub(crate) fn set_viewport(&mut self, viewport: Vec2) {
        self.viewport = viewport;
    }

    /// Turns the camera to look at `target`, keeping +Y up.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx, player: Vec3) {
    /// ctx.camera3d.position = player + vec3(0.0, 6.0, 8.0);
    /// ctx.camera3d.look_at(player);
    /// # }
    /// ```
    pub fn look_at(&mut self, target: Vec3) {
        self.rotation = look_rotation(target - self.position);
    }

    /// The direction the camera looks.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let ahead = ctx.camera3d.position + ctx.camera3d.forward() * 5.0;
    /// # }
    /// ```
    #[must_use]
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::NEG_Z
    }

    /// World to clip space: projection times view.
    #[must_use]
    pub(crate) fn view_proj(&self) -> Mat4 {
        let aspect = (self.viewport.x / self.viewport.y.max(1.0)).max(1e-3);
        let projection = glam::camera::rh::proj::directx::perspective(self.fov_y, aspect, self.near, self.far);
        let view = Mat4::from_rotation_translation(self.rotation, self.position).inverse();
        projection * view
    }

    /// The ray from the camera through window point `screen` (logical
    /// pixels, as [`Ctx::mouse_pos`](crate::Ctx::mouse_pos)): its start and
    /// unit direction. For picking things in the world with the mouse.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let (from, dir) = ctx.camera3d.screen_ray(ctx.mouse_pos());
    /// // Where the mouse points on the ground (y = 0):
    /// if dir.y < 0.0 {
    ///     let ground = from + dir * (-from.y / dir.y);
    /// }
    /// # }
    /// ```
    #[must_use]
    pub fn screen_ray(&self, screen: Vec2) -> (Vec3, Vec3) {
        let ndc = Vec2::new(
            screen.x / self.viewport.x.max(1.0) * 2.0 - 1.0,
            1.0 - screen.y / self.viewport.y.max(1.0) * 2.0,
        );
        let inverse = self.view_proj().inverse();
        let far = inverse * Vec4::new(ndc.x, ndc.y, 1.0, 1.0);
        let far = far.truncate() / far.w;
        (self.position, (far - self.position).normalize_or(self.forward()))
    }
}

/// The light in the 3D world: a sun far away (one direction everywhere)
/// and ambient light from all around. At [`Ctx::lighting`](crate::Ctx::lighting).
///
/// ```no_run
/// # use crabbyengine::*;
/// # fn f(ctx: &mut Ctx) {
/// // Evening: low, orange sun.
/// ctx.lighting.sun_direction = vec3(-1.0, -0.3, -0.2);
/// ctx.lighting.sun_color = Color::hex(0xFFB066);
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lighting {
    /// The way the sunlight travels (from the sun towards the ground).
    pub sun_direction: Vec3,
    /// The sun's color.
    pub sun_color: Color,
    /// How bright the sun is. Default 3.
    pub sun_intensity: f32,
    /// Light from the sky, on surfaces facing up.
    pub sky_color: Color,
    /// Light bounced off the ground, on surfaces facing down.
    pub ground_color: Color,
    /// How bright the sky and ground light is. Default 0.5.
    pub ambient_intensity: f32,
    /// Whether the sun casts shadows. Default on.
    pub shadows: bool,
    /// How far from the camera shadows reach, in world units: shorter is
    /// sharper. Default 60.
    pub shadow_distance: f32,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            sun_direction: Vec3::new(-0.4, -1.0, -0.3),
            sun_color: Color::hex(0xFFF4E0),
            sun_intensity: 3.0,
            sky_color: Color::hex(0xB8D4F0),
            ground_color: Color::hex(0x6B5B4B),
            ambient_intensity: 0.5,
            shadows: true,
            shadow_distance: 60.0,
        }
    }
}

/// What the engine keeps about each mesh and material on the CPU side: for
/// culling, and for headless runs, which have no GPU.
#[derive(Default)]
pub(crate) struct Assets3D {
    /// Bounding sphere of each mesh, in its own space: center and radius.
    pub bounds: Vec<(Vec3, f32)>,
    pub materials: u32,
    /// The material handed out for meshes without one.
    pub default_material: Option<Material>,
    /// Meshes loaded from files, by resolved path.
    pub loaded: std::collections::HashMap<std::path::PathBuf, Mesh>,
}

/// sRGB color to linear RGBA, for the GPU.
pub(crate) fn linear(c: Color) -> [f32; 4] {
    let lin = |v: f32| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    [lin(c.r), lin(c.g), lin(c.b), c.a]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looking_at_points_minus_z_at_the_target() {
        for target in [Vec3::X, Vec3::NEG_X, Vec3::new(1.0, 2.0, -3.0), Vec3::Y, Vec3::NEG_Y] {
            let t = Transform3D::default().looking_at(target);
            assert!((t.forward() - target.normalize()).length() < 1e-5, "{target}");
        }
    }

    #[test]
    fn the_screen_center_ray_goes_where_the_camera_looks() {
        let mut camera = Camera3D::new(Vec2::new(800.0, 600.0));
        camera.position = Vec3::new(3.0, 4.0, 5.0);
        camera.look_at(Vec3::ZERO);
        let (from, dir) = camera.screen_ray(Vec2::new(400.0, 300.0));
        assert_eq!(from, camera.position);
        assert!((dir - camera.forward()).length() < 1e-4);
        // Left of center points left of forward.
        let (_, left) = camera.screen_ray(Vec2::new(0.0, 300.0));
        let right_axis = camera.rotation * Vec3::X;
        assert!(left.dot(right_axis) < 0.0);
    }

    #[test]
    fn the_view_projection_puts_the_target_in_the_middle() {
        let mut camera = Camera3D::new(Vec2::new(800.0, 600.0));
        camera.position = Vec3::new(0.0, 10.0, 10.0);
        camera.look_at(Vec3::new(1.0, 0.0, 0.0));
        let clip = camera.view_proj() * Vec4::new(1.0, 0.0, 0.0, 1.0);
        let ndc = clip.truncate() / clip.w;
        assert!(
            ndc.x.abs() < 1e-4 && ndc.y.abs() < 1e-4 && (0.0..1.0).contains(&ndc.z),
            "{ndc}"
        );
    }
}
