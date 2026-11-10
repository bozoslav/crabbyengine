//! Render targets: textures a [`TargetCamera`] draws the 3D world into,
//! read by materials (mirrors, water reflections, security cameras,
//! portals).

use glam::{Mat4, Vec3, Vec4};

use super::Camera3D;
use crate::Texture;
use crate::gfx::Graphics;

/// A camera drawing the 3D world into a render target each frame, before
/// the main view: one entry of [`Ctx::target_cameras`](crate::Ctx::target_cameras).
///
/// The target holds the lit scene with its sky and fog, as linear HDR
/// light, before screen effects, bloom and tone mapping. It uses the main
/// view's shadows. Transparent models aren't drawn into it.
///
/// ```no_run
/// # use ponko::*;
/// # fn f(ctx: &mut Ctx) {
/// let screen = ctx.gfx.create_render_target(512, 512);
/// let mut camera = ctx.camera3d;
/// camera.position = vec3(0.0, 10.0, 0.0);
/// camera.look_at(Vec3::ZERO);
/// ctx.target_cameras.push(TargetCamera::new(camera, screen));
/// // A monitor showing it: a material with the target as its texture.
/// let monitor = ctx.gfx.create_material(MaterialDesc {
///     emissive_map: Some(screen),
///     emissive: Color::WHITE,
///     ..MaterialDesc::color(Color::BLACK)
/// });
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetCamera {
    /// Where it looks from. Its aspect ratio follows the target's size.
    pub camera: Camera3D,
    /// Made by [`Graphics::create_render_target`](crate::Graphics::create_render_target); other textures are skipped.
    pub target: Texture,
    /// Only what lies on the front of this plane is drawn: `(normal, d)`,
    /// keeping points where `normal.dot(p) + d >= 0`. For reflections,
    /// which mustn't show what's below the mirror. It costs nothing: the
    /// camera's near plane is tilted onto it.
    pub clip_plane: Option<Vec4>,
}

impl TargetCamera {
    /// `camera` drawing into `target`, with no clip plane.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let target = ctx.gfx.create_render_target(256, 256);
    /// ctx.target_cameras.push(TargetCamera::new(ctx.camera3d, target));
    /// # }
    /// ```
    #[must_use]
    pub fn new(camera: Camera3D, target: Texture) -> Self {
        Self {
            camera,
            target,
            clip_plane: None,
        }
    }

    /// The camera seeing `camera`'s view mirrored in the level plane at
    /// height `level` (a lake, a polished floor), clipped to what's above
    /// it. Read its target at the screen position flipped upside down:
    /// `vec2(uv.x, 1.0 - uv.y)`.
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("lake", 640, 360).run_headless(0, |ctx| {
    ///     let reflection = ctx.gfx.create_render_target(640, 360);
    ///     ctx.camera3d.position = vec3(0.0, 5.0, 0.0);
    ///     let mirrored = TargetCamera::reflection(ctx.camera3d, reflection, 1.0);
    ///     assert_eq!(mirrored.camera.position, vec3(0.0, -3.0, 0.0));
    ///     ctx.target_cameras.push(mirrored);
    /// });
    /// ```
    #[must_use]
    pub fn reflection(camera: Camera3D, target: Texture, level: f32) -> Self {
        let mut mirrored = camera;
        mirrored.position.y = 2.0 * level - camera.position.y;
        // Forward and up mirror in y; the image comes out upside down,
        // which the flipped lookup undoes.
        let forward = camera.forward() * Vec3::new(1.0, -1.0, 1.0);
        mirrored.rotation = super::look_rotation(forward);
        Self {
            camera: mirrored,
            target,
            clip_plane: Some(Vec4::new(0.0, 1.0, 0.0, -level)),
        }
    }
}

/// `view_proj` with its near plane tilted onto world plane `plane` (Eric
/// Lengyel's oblique frustum), for a camera at `camera`. Unchanged when
/// the camera is on the plane's kept side (the plane is then in front).
pub(crate) fn oblique(view_proj: Mat4, view: Mat4, plane: Vec4) -> Mat4 {
    // The plane in view space: transforming planes takes the inverse
    // transpose, and `view.inverse()` is the camera's own matrix.
    let c = view.inverse().transpose() * plane;
    let c = c.normalize_or_zero();
    if c.w >= 0.0 || c == Vec4::ZERO {
        return view_proj;
    }
    let projection = view_proj * view.inverse();
    // The far corner of the frustum on the plane's side, in view space.
    let clip_plane = projection.inverse().transpose() * c;
    let q = projection.inverse() * Vec4::new(clip_plane.x.signum(), clip_plane.y.signum(), 1.0, 1.0);
    // Depth 0..1: the near plane is the z row; scaled so the far plane
    // still passes through that corner.
    let z = c / c.dot(q);
    let mut rows = projection.transpose().to_cols_array_2d();
    rows[2] = z.to_array();
    Mat4::from_cols_array_2d(&rows).transpose() * view
}

/// Whether `p` is on the kept side of `plane`.
#[cfg(test)]
fn kept(plane: Vec4, p: Vec3) -> bool {
    use glam::Vec4Swizzles;
    plane.xyz().dot(p) + plane.w >= 0.0
}

impl Graphics {
    /// A texture of `width` x `height` pixels that a [`TargetCamera`] in
    /// [`Ctx::target_cameras`](crate::Ctx::target_cameras) draws the 3D world into (feature `3d`).
    /// Use it in a material like any texture; it holds linear HDR light.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let mirror = ctx.gfx.create_render_target(512, 512);
    /// assert_eq!(mirror.width(), 512);
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// If the size is zero or larger than the GPU takes.
    pub fn create_render_target(&mut self, width: u32, height: u32) -> Texture {
        self.try_create_render_target(width, height)
            .unwrap_or_else(|e| panic!("ponko: {e}"))
    }

    /// [`Graphics::create_render_target`](crate::Graphics::create_render_target) that returns errors (for the C API).
    #[doc(hidden)]
    pub fn try_create_render_target(&mut self, width: u32, height: u32) -> Result<Texture, String> {
        let texture = self.make_render_target(width, height)?;
        self.textures_live.insert(texture.id);
        Ok(texture)
    }

    /// [`Graphics::try_create_render_target`](crate::Graphics::try_create_render_target) for the engine's own targets
    /// (reflection probe faces): not the game's to free or count.
    pub(crate) fn make_render_target(&mut self, width: u32, height: u32) -> Result<Texture, String> {
        if width == 0 || height == 0 {
            return Err(format!("render target size must not be zero (got {width} x {height})"));
        }
        let texture = if self.headless {
            self.headless_textures += 1;
            Texture::new(self.headless_textures, width, height)
        } else {
            let renderer = self.renderer();
            let max = renderer.max_texture_size();
            if width > max || height > max {
                return Err(format!(
                    "{width} x {height} is larger than this GPU's maximum texture size ({max})"
                ));
            }
            Texture::new(renderer.create_render_target(width, height), width, height)
        };
        self.assets3d.render_targets.push(texture);
        Ok(texture)
    }

    /// Whether `texture` is a render target made by this game and not freed.
    /// Only its id is compared.
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     let target = ctx.gfx.create_render_target(64, 64);
    ///     assert!(ctx.gfx.is_render_target(target));
    ///     let image = ctx.gfx.create_texture(1, 1, &[255; 4]);
    ///     assert!(!ctx.gfx.is_render_target(image));
    /// });
    /// ```
    #[must_use]
    pub fn is_render_target(&self, texture: Texture) -> bool {
        self.assets3d.render_targets.iter().any(|t| t.id == texture.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Quat, Vec2};

    fn camera() -> Camera3D {
        let mut c = Camera3D::new(Vec2::new(800.0, 600.0));
        c.position = Vec3::new(0.0, -4.0, 10.0);
        c.rotation = super::super::look_rotation(Vec3::new(0.0, 0.5, -1.0));
        c
    }

    fn view(c: &Camera3D) -> Mat4 {
        Mat4::from_rotation_translation(c.rotation, c.position).inverse()
    }

    #[test]
    fn the_oblique_near_plane_is_the_clip_plane() {
        let c = camera();
        let plane = Vec4::new(0.0, 1.0, 0.0, -1.0); // keep y >= 1
        let m = oblique(c.view_proj(), view(&c), plane);
        let depth = |p: Vec3| {
            let clip = m * p.extend(1.0);
            clip.z / clip.w
        };
        // In front of the camera: above the plane drawn, below cut.
        for p in [
            Vec3::new(0.0, 1.5, 0.0),
            Vec3::new(3.0, 4.0, -20.0),
            Vec3::new(-2.0, 2.0, 5.0),
        ] {
            assert!(kept(plane, p));
            let d = depth(p);
            assert!((0.0..=1.0).contains(&d), "{p} kept, depth {d}");
        }
        for p in [Vec3::new(0.0, 0.5, 0.0), Vec3::new(2.0, -1.0, -10.0)] {
            assert!(depth(p) < 0.0, "{p} is below the plane");
        }
        // On the plane: depth 0.
        assert!(depth(Vec3::new(0.0, 1.0, -3.0)).abs() < 1e-4);
    }

    #[test]
    fn a_plane_behind_the_camera_changes_nothing() {
        let c = camera();
        let plane = Vec4::new(0.0, 1.0, 0.0, 10.0); // the camera is on the kept side
        assert_eq!(oblique(c.view_proj(), view(&c), plane), c.view_proj());
    }

    #[test]
    fn reflections_mirror_the_camera_in_the_level() {
        let mut c = Camera3D::new(Vec2::new(800.0, 600.0));
        c.position = Vec3::new(1.0, 6.0, 2.0);
        c.rotation = Quat::from_rotation_y(0.7) * Quat::from_rotation_x(-0.4);
        let target = Texture::new(1, 8, 8);
        let r = TargetCamera::reflection(c, target, 2.0);
        assert_eq!(r.camera.position, Vec3::new(1.0, -2.0, 2.0));
        let (f, g) = (c.forward(), r.camera.forward());
        assert!((g - Vec3::new(f.x, -f.y, f.z)).length() < 1e-5);
        assert_eq!(r.clip_plane, Some(Vec4::new(0.0, 1.0, 0.0, -2.0)));
    }
}
