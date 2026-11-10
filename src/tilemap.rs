//! Tilemaps: grids of tiles from a [`SpriteSheet`], drawn only where the
//! camera looks.

use glam::{UVec2, Vec2};

use crate::color::Color;
use crate::ctx::Ctx;
use crate::ecs::Transform2D;
use crate::render::batch::{SpriteInstance, uv_region};
use crate::sheet::SpriteSheet;

/// A grid of tiles, each a frame of `sheet` or empty. Put it on an entity
/// with a [`Transform2D`]: its position is the map's top-left corner (the
/// rotation and scale are ignored). The built-in [`draw_tilemaps`] system
/// draws the tiles the camera can see, so maps can be huge.
///
/// ```
/// # use ponko::*;
/// # fn setup(ctx: &mut Ctx, tiles: SpriteSheet) {
/// let mut map = TileMap::new(tiles, 200, 100); // 200 × 100 tiles, all empty
/// for x in 0..200 {
///     map.set(x, 99, Some(0)); // a floor of tile 0
/// }
/// ctx.world.spawn((Transform2D::at(vec2(0.0, 0.0)), map));
/// # }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TileMap {
    /// The tiles' images: tile `n` is frame `n` of this sheet.
    pub sheet: SpriteSheet,
    /// Map size in tiles.
    pub size: UVec2,
    /// Size of one tile in world units. Default: the sheet's frame size.
    pub tile_size: Vec2,
    /// Draw order. Default `0`: maps are drawn before sprites, so sprites
    /// with the same `z` are on top. (Keeping every `z` equal also keeps
    /// drawing on its fast path, with no sorting.)
    pub z: f32,
    /// Tint for every tile. Default white.
    pub color: Color,
    /// Frame index per tile, row by row; [`TileMap::EMPTY`] draws nothing.
    pub tiles: Vec<u16>,
}

impl TileMap {
    /// The tile value for "nothing here".
    pub const EMPTY: u16 = u16::MAX;

    /// An empty `width` × `height` map using `sheet`'s frames as tiles.
    #[must_use]
    pub fn new(sheet: SpriteSheet, width: u32, height: u32) -> Self {
        Self {
            sheet,
            size: UVec2::new(width, height),
            tile_size: sheet.frame_size,
            z: 0.0,
            color: Color::WHITE,
            tiles: vec![Self::EMPTY; width as usize * height as usize],
        }
    }

    /// Sets tile `(x, y)` to frame `frame`, or empties it with `None`.
    /// Out-of-bounds positions are ignored.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(map: &mut TileMap) {
    /// map.set(3, 4, Some(7));
    /// map.set(3, 5, None);
    /// # }
    /// ```
    pub fn set(&mut self, x: u32, y: u32, frame: Option<u16>) {
        if x < self.size.x && y < self.size.y {
            self.tiles[(y * self.size.x + x) as usize] = frame.unwrap_or(Self::EMPTY);
        }
    }

    /// The frame at tile `(x, y)`, or `None` if empty or out of bounds.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(map: &TileMap) {
    /// let solid = map.get(10, 2).is_some();
    /// # }
    /// ```
    #[must_use]
    pub fn get(&self, x: u32, y: u32) -> Option<u16> {
        if x >= self.size.x || y >= self.size.y {
            return None;
        }
        let t = self.tiles[(y * self.size.x + x) as usize];
        (t != Self::EMPTY).then_some(t)
    }

    /// The tile containing world point `world`, for a map whose top-left is
    /// at `origin`; `None` outside the map.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn paint(ctx: &mut Ctx, map: &mut TileMap) {
    /// if let Some(tile) = map.tile_at(Vec2::ZERO, ctx.mouse_world()) {
    ///     map.set(tile.x, tile.y, Some(1));
    /// }
    /// # }
    /// ```
    #[must_use]
    pub fn tile_at(&self, origin: Vec2, world: Vec2) -> Option<UVec2> {
        let t = ((world - origin) / self.tile_size).floor();
        (t.x >= 0.0 && t.y >= 0.0 && t.x < self.size.x as f32 && t.y < self.size.y as f32)
            .then(|| UVec2::new(t.x as u32, t.y as u32))
    }
}

/// Built-in system: draws the visible tiles of every [`TileMap`].
/// [`Game`](crate::Game) runs it every frame, before
/// [`draw_sprites`](crate::systems::draw_sprites).
///
/// ```no_run
/// # use ponko::*;
/// Game::new("No maps here", 800, 600).remove(Systems::DRAW_TILEMAPS).run(|_| {});
/// ```
pub fn draw_tilemaps(ctx: &mut Ctx) {
    let view = ctx.camera.visible_rect();
    let list = &mut ctx.gfx.draw_list;
    ctx.world.query::<(&Transform2D, &TileMap)>().for_each(|(t, map)| {
        let ts = map.tile_size;
        let lo = ((view.min - t.position) / ts).floor().max(Vec2::ZERO);
        let hi = ((view.max - t.position) / ts).ceil().min(map.size.as_vec2());
        if lo.x >= hi.x || lo.y >= hi.y {
            return;
        }
        let tex = map.sheet.texture;
        let tex_size = tex.size().max(Vec2::ONE);
        let color = map.color.to_rgba8();
        let uv_size = (map.sheet.frame_size / tex_size).into();
        for y in lo.y as u32..hi.y as u32 {
            let row = (y * map.size.x) as usize;
            for x in lo.x as u32..hi.x as u32 {
                let tile = map.tiles[row + x as usize];
                if tile == TileMap::EMPTY {
                    continue;
                }
                let frame = map.sheet.frame(u32::from(tile));
                let pos = t.position + Vec2::new(x as f32, y as f32) * ts;
                let instance = SpriteInstance::new(pos.into(), ts.into(), [0.0, 0.0], 0.0, color)
                    .with_uv(uv_region((frame.min / tex_size).into(), uv_size));
                list.push(tex.id, map.z, instance);
            }
        }
    });
}
