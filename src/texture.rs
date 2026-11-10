//! [`Texture`] handles and [`DrawParams`].

use glam::Vec2;

use crate::color::Color;

/// A loaded image on the GPU. Cheap to copy: it's just an id plus its size.
///
/// Get one from [`Ctx::load_texture`](crate::Ctx::load_texture) or
/// [`Graphics::create_texture`](crate::Graphics::create_texture).
///
/// ```
/// # use ponko::*;
/// # fn setup(ctx: &mut Ctx) {
/// let crab = ctx.load_texture("crab.png");
/// println!("{} x {}", crab.width(), crab.height());
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)] // shared with C/C++ (`CrabbyTexture`)
pub struct Texture {
    pub(crate) id: u32,
    width: u32,
    height: u32,
}

impl Texture {
    pub(crate) const fn new(id: u32, width: u32, height: u32) -> Self {
        Self { id, width, height }
    }

    /// Width in pixels.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(tex: Texture) {
    /// let w = tex.width();
    /// # }
    /// ```
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(tex: Texture) {
    /// let h = tex.height();
    /// # }
    /// ```
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Size in pixels as a vector.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(tex: Texture) {
    /// let half = tex.size() / 2.0;
    /// # }
    /// ```
    #[must_use]
    pub fn size(&self) -> Vec2 {
        Vec2::new(self.width as f32, self.height as f32)
    }
}

/// How a texture looks when drawn bigger or smaller than its pixels:
/// [`Graphics::set_texture_filter`](crate::Graphics::set_texture_filter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextureFilter {
    /// Smooth blending between pixels (default): photos, painted art.
    #[default]
    Linear,
    /// Hard pixel edges: pixel art and tiles (no seams between tiles).
    Nearest,
}

/// Optional settings for [`Graphics::draw_ex`](crate::Graphics::draw_ex).
/// Start from `DrawParams::default()` and change what you need.
///
/// ```
/// # use ponko::*;
/// # fn update(ctx: &mut Ctx, tex: Texture) {
/// ctx.gfx.draw_ex(&tex, vec2(100.0, 100.0), DrawParams {
///     rotation: 0.5,
///     scale: vec2(2.0, 2.0),
///     color: Color::RED,
///     ..Default::default()
/// });
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawParams {
    /// Size multiplier. Negative values flip the sprite. Default `(1, 1)`.
    pub scale: Vec2,
    /// Rotation in radians, clockwise on screen. Default `0`.
    pub rotation: f32,
    /// Tint multiplied with the texture. Default white (no tint).
    pub color: Color,
    /// Draw order: higher `z` is drawn on top. Sprites with equal `z` are
    /// drawn in the order you call `draw`. Default `0`.
    pub z: f32,
    /// Which point of the sprite sits at the draw position and is rotated
    /// around: `(0, 0)` top-left, `(0.5, 0.5)` center (default), `(1, 1)` bottom-right.
    pub pivot: Vec2,
    /// Draw in window pixels, ignoring the camera, on top of the world:
    /// for UI and HUDs. Default `false` (world units, through the camera).
    pub screen: bool,
    /// The part of the texture to draw, in texture pixels (a frame of a
    /// [`SpriteSheet`](crate::SpriteSheet)). Default `None`: all of it.
    pub region: Option<crate::Rect>,
}

impl Default for DrawParams {
    fn default() -> Self {
        Self {
            scale: Vec2::ONE,
            rotation: 0.0,
            color: Color::WHITE,
            z: 0.0,
            pivot: Vec2::splat(0.5),
            screen: false,
            region: None,
        }
    }
}
