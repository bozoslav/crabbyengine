//! Plain C structs shared with the engine. Most have the exact layout of the
//! Rust type (checked at compile time), so values pass through without copies.
use ponko::d2::Sprite;
use ponko::d2::Transform as Transform2D;
use ponko::spatial::GridEntry;
use ponko::{Color, DrawParams, Rect, Texture, Vec2};
use ponko_ecs::Entity;

/// A 2D vector.
/// ```
/// let v = ponko_ffi::CrabbyVec2 { x: 1.0, y: 2.0 };
/// assert_eq!(v.x + v.y, 3.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CrabbyVec2 {
    /// Horizontal, right is positive.
    pub x: f32,
    /// Vertical, down is positive.
    pub y: f32,
}

/// RGBA color, each channel 0..1 (same layout as Rust `Color`).
/// ```
/// let white = ponko_ffi::CrabbyColor { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
/// assert_eq!(white.a, 1.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyColor {
    /// Red.
    pub r: f32,
    /// Green.
    pub g: f32,
    /// Blue.
    pub b: f32,
    /// Alpha (1 = opaque).
    pub a: f32,
}

/// A texture handle from `crabby_ctx_load_texture` (same layout as Rust `Texture`).
/// Never make one up: only use values the engine gave you.
/// ```
/// assert_eq!(std::mem::size_of::<ponko_ffi::CrabbyTexture>(), 12);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrabbyTexture {
    /// Engine-internal id.
    pub id: u32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// Position, rotation (radians, clockwise) and scale; the built-in
/// `Transform2D` component, same layout as in Rust.
/// ```
/// let t = ponko_ffi::CrabbyTransform2D::default();
/// assert_eq!(t.scale.x, 1.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyTransform2D {
    /// Where the entity is.
    pub position: CrabbyVec2,
    /// Radians, clockwise on screen.
    pub rotation: f32,
    /// Size multiplier; negative flips.
    pub scale: CrabbyVec2,
}

impl Default for CrabbyTransform2D {
    fn default() -> Self {
        to_c(Transform2D::default())
    }
}

/// An entity as stored in query chunks. `crabby_entity_bits` (or
/// `(uint64_t)generation << 32 | index`) gives the id other calls take.
/// ```
/// let e = ponko_ffi::CrabbyEntity { index: 3, generation: 1 };
/// assert_eq!(ponko_ffi::crabby_entity_bits(e), (1u64 << 32) | 3);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrabbyEntity {
    /// Slot number.
    pub index: u32,
    /// Bumped when the slot is reused, so old ids stay dead.
    pub generation: u32,
}

/// The id of `entity` that other calls take.
/// ```
/// let e = ponko_ffi::CrabbyEntity { index: 7, generation: 0 };
/// assert_eq!(ponko_ffi::crabby_entity_bits(e), 7);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_entity_bits(entity: CrabbyEntity) -> u64 {
    (entity.generation as u64) << 32 | entity.index as u64
}

/// An axis-aligned rectangle from `min` (top-left) to `max`.
/// ```
/// let r = ponko_ffi::CrabbyRect { min: Default::default(), max: ponko_ffi::CrabbyVec2 { x: 16.0, y: 16.0 } };
/// assert_eq!(r.max.x, 16.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CrabbyRect {
    /// Top-left corner.
    pub min: CrabbyVec2,
    /// Bottom-right corner.
    pub max: CrabbyVec2,
}

/// The built-in `Sprite` component as a C value. Get and set it with the
/// sprite component id like any component; it can't be a query column,
/// because the engine stores it in a Rust layout.
/// ```
/// let s = ponko_ffi::CrabbySprite::default();
/// assert_eq!(s.pivot.x, 0.5);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbySprite {
    /// What to draw.
    pub texture: CrabbyTexture,
    /// Tint. Default white.
    pub color: CrabbyColor,
    /// Draw order, higher on top. Default 0.
    pub z: f32,
    /// Point placed at the position: (0.5, 0.5) = center (default).
    pub pivot: CrabbyVec2,
    /// Show only `region` of the texture (texture pixels).
    pub has_region: bool,
    /// Used when `has_region` is true.
    pub region: CrabbyRect,
}

impl Default for CrabbySprite {
    fn default() -> Self {
        sprite_to_c(&Sprite::new(texture_from_c(CrabbyTexture {
            id: 0,
            width: 0,
            height: 0,
        })))
    }
}

/// Options for `crabby_ctx_draw_ex`. Start from `crabby_draw_params_default()`.
/// ```
/// let p = ponko_ffi::crabby_draw_params_default();
/// assert_eq!(p.scale.x, 1.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyDrawParams {
    /// Size multiplier; negative flips. Default (1, 1).
    pub scale: CrabbyVec2,
    /// Radians, clockwise. Default 0.
    pub rotation: f32,
    /// Tint. Default white.
    pub color: CrabbyColor,
    /// Draw order, higher on top. Default 0.
    pub z: f32,
    /// Point placed at the position. Default (0.5, 0.5).
    pub pivot: CrabbyVec2,
    /// Window pixels, ignoring the camera (UI). Default false.
    pub screen: bool,
    /// Draw only `region` of the texture.
    pub has_region: bool,
    /// Used when `has_region` is true.
    pub region: CrabbyRect,
}

/// Default draw options: no scale, rotation or tint, centered.
/// ```
/// assert!(!ponko_ffi::crabby_draw_params_default().screen);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_draw_params_default() -> CrabbyDrawParams {
    let d = DrawParams::default();
    let (has_region, region) = rect_to_c(d.region);
    CrabbyDrawParams {
        scale: to_c(d.scale),
        rotation: d.rotation,
        color: to_c(d.color),
        z: d.z,
        pivot: to_c(d.pivot),
        screen: d.screen,
        has_region,
        region,
    }
}

/// A spatial-grid hit: an entity and its position.
/// ```
/// assert_eq!(std::mem::size_of::<ponko_ffi::CrabbyGridHit>(), 16);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyGridHit {
    /// Who.
    pub entity: CrabbyEntity,
    /// Where it was when the grid was built.
    pub position: CrabbyVec2,
}

/// An item of a `CrabbyGrid`: entity, position and a number of your own
/// (an index into your own arrays, for example).
/// ```
/// assert_eq!(std::mem::size_of::<ponko_ffi::CrabbyGridItem>(), 20);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyGridItem {
    /// Who.
    pub entity: CrabbyEntity,
    /// Where.
    pub position: CrabbyVec2,
    /// Yours.
    pub data: u32,
}

// ---- layout checks and conversions -------------------------------------------

/// `A` and `B` have the same size, alignment and field offsets.
/// # Safety
/// Only for type pairs whose size, alignment and field offsets match (checked by `same_layout!`).
pub(crate) unsafe trait SameLayout<B>: Copy {}
macro_rules! same_layout {
    ($c:ty, $rust:ty $(, $cf:ident = $rf:ident)*) => {
        unsafe impl SameLayout<$rust> for $c {}
        unsafe impl SameLayout<$c> for $rust {}
        const _: () = {
            assert!(size_of::<$c>() == size_of::<$rust>());
            assert!(align_of::<$c>() == align_of::<$rust>());
            $(assert!(std::mem::offset_of!($c, $cf) == std::mem::offset_of!($rust, $rf));)*
        };
    };
}
pub(crate) use same_layout;
same_layout!(CrabbyVec2, Vec2, x = x, y = y);
same_layout!(CrabbyColor, Color, r = r, g = g, b = b, a = a);
same_layout!(
    CrabbyTransform2D,
    Transform2D,
    position = position,
    rotation = rotation,
    scale = scale
);
same_layout!(CrabbyRect, Rect, min = min, max = max);
same_layout!(CrabbyGridHit, GridEntry<()>, entity = entity, position = position);
same_layout!(
    CrabbyGridItem,
    GridEntry<u32>,
    entity = entity,
    position = position,
    data = data
);
// Entity is repr(C) { index, generation }; its fields are private to ponko_ecs.
same_layout!(CrabbyEntity, Entity);
const _: () = assert!(size_of::<CrabbyTexture>() == size_of::<Texture>());

/// Converts between two types with the same layout.
#[inline(always)]
pub(crate) fn to_c<A: SameLayout<B>, B>(a: A) -> B {
    // SAFETY: SameLayout is only implemented for checked pairs of plain f32/u32 structs.
    unsafe { std::mem::transmute_copy(&a) }
}

pub(crate) fn texture_to_c(t: Texture) -> CrabbyTexture {
    // SAFETY: both are repr(C) { u32 id, u32 width, u32 height } (sizes asserted above).
    unsafe { std::mem::transmute(t) }
}
pub(crate) fn texture_from_c(t: CrabbyTexture) -> Texture {
    // SAFETY: as above. Ids the engine doesn't know draw nothing or the wrong texture; they are not UB.
    unsafe { std::mem::transmute(t) }
}

fn rect_to_c(r: Option<Rect>) -> (bool, CrabbyRect) {
    match r {
        Some(r) => (true, to_c(r)),
        None => (false, CrabbyRect::default()),
    }
}
fn rect_from_c(has: bool, r: CrabbyRect) -> Option<Rect> {
    has.then(|| to_c(r))
}

pub(crate) fn sprite_to_c(s: &Sprite) -> CrabbySprite {
    let (has_region, region) = rect_to_c(s.region);
    CrabbySprite {
        texture: texture_to_c(s.texture),
        color: to_c(s.color),
        z: s.z,
        pivot: to_c(s.pivot),
        has_region,
        region,
    }
}
pub(crate) fn sprite_from_c(s: &CrabbySprite) -> Sprite {
    let mut sprite = Sprite::new(texture_from_c(s.texture));
    sprite.color = to_c(s.color);
    sprite.z = s.z;
    sprite.pivot = to_c(s.pivot);
    sprite.region = rect_from_c(s.has_region, s.region);
    sprite
}
impl crate::CMirror for Sprite {
    type C = CrabbySprite;
    fn from_c(c: &CrabbySprite) -> Result<Self, crate::Failure> {
        Ok(sprite_from_c(c))
    }
    fn to_c(&self) -> CrabbySprite {
        sprite_to_c(self)
    }
}
pub(crate) fn draw_params_from_c(p: &CrabbyDrawParams) -> DrawParams {
    DrawParams {
        scale: to_c(p.scale),
        rotation: p.rotation,
        color: to_c(p.color),
        z: p.z,
        pivot: to_c(p.pivot),
        screen: p.screen,
        region: rect_from_c(p.has_region, p.region),
    }
}
