//! Text: fonts, a glyph atlas filled on demand, and layout.
//!
//! Glyphs are rasterized at the window's physical pixel size the first time
//! they're drawn, packed into one atlas texture, and drawn as sprites in
//! screen space. Steady-state frames only look glyphs up.

use std::collections::HashMap;

use glam::Vec2;

use crate::color::Color;

/// A loaded font. Get one from [`Graphics::load_font`](crate::gfx::Graphics::load_font);
/// [`Font::DEFAULT`] is always there.
///
/// ```
/// # use ponko::prelude::*;
/// # fn hud(ctx: &mut Ctx) {
/// ctx.gfx.draw_text_ex("Score: 120", vec2(16.0, 16.0), TextParams {
///     font: Font::DEFAULT,
///     size: 24.0,
///     ..Default::default()
/// });
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct Font(pub(crate) u32);

impl Font {
    /// The built-in font (Ubuntu Light, Ubuntu Font Licence), used by
    /// [`Ctx::draw_text`](crate::Ctx::draw_text).
    pub const DEFAULT: Font = Font(0);
}

/// Optional settings for [`Graphics::draw_text_ex`](crate::gfx::Graphics::draw_text_ex).
///
/// ```
/// # use ponko::prelude::*;
/// let title = TextParams { size: 48.0, color: Color::hex(0xffcc00), ..Default::default() };
/// # let _ = title;
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextParams {
    /// The font to draw with.
    pub font: Font,
    /// Height of a line in window pixels. Default 16.
    pub size: f32,
    /// Default white.
    pub color: Color,
    /// Draw order among screen-space drawing; higher is on top. Default 0.
    pub z: f32,
}

impl Default for TextParams {
    fn default() -> Self {
        Self {
            font: Font::DEFAULT,
            size: 16.0,
            color: Color::WHITE,
            z: 0.0,
        }
    }
}

/// Atlas side length in pixels.
const ATLAS: u32 = 1024;
/// Pixels of empty space around each glyph, so linear filtering doesn't
/// bleed neighbors in.
const PAD: u32 = 1;

#[derive(Clone, Copy)]
pub(crate) struct Glyph {
    /// Where in the atlas, in pixels (`None` for glyphs with no pixels, like space).
    pub at: Option<[u32; 2]>,
    /// Bitmap size in physical pixels.
    pub size: [u32; 2],
    /// Bitmap offset from the pen position: x right, y up from the baseline.
    pub offset: [f32; 2],
}

/// One glyph to draw, in physical pixels relative to the text's origin.
pub(crate) struct Placed {
    pub pos: Vec2,
    pub glyph: Glyph,
    /// Which line it's on, from 0.
    #[cfg_attr(not(feature = "3d"), allow(dead_code))]
    pub line: u32,
}

/// Line height in pixels SDF glyphs are made at.
#[cfg_attr(not(feature = "3d"), allow(dead_code))]
pub(crate) const SDF_PX: f32 = 64.0;
/// How far, in those pixels, SDF glyphs record the distance to the edge:
/// outlines and soft edges can reach this far out.
#[cfg_attr(not(feature = "3d"), allow(dead_code))]
pub(crate) const SDF_SPREAD: u32 = 8;

/// Signed-distance glyphs, made once per font and character at
/// [`SDF_PX`] and drawn at any size: the alpha channel is the distance to
/// the glyph's edge, 0.5 on it, more inside.
#[cfg_attr(not(feature = "3d"), allow(dead_code))]
#[derive(Default)]
pub(crate) struct SdfAtlas {
    glyphs: HashMap<(u32, char), Glyph>,
    cursor: [u32; 2],
    row: u32,
    /// The atlas is full: start over next frame.
    full: bool,
    /// Glyphs made but not yet on the GPU: atlas position, size, RGBA.
    pub uploads: Vec<([u32; 2], [u32; 2], Vec<u8>)>,
}

pub(crate) struct Text {
    fonts: Vec<Option<fontdue::Font>>,
    /// Which fonts are live (the built-in one always is).
    live: crate::handle::LiveSet,
    glyphs: HashMap<(u32, char, u32), Glyph>,
    /// Shelf packer: next free spot and the current row's height.
    cursor: [u32; 2],
    row: u32,
    /// The atlas is full: start over next frame.
    full: bool,
    /// Glyphs rasterized but not yet on the GPU: atlas position, size, RGBA.
    pub uploads: Vec<([u32; 2], [u32; 2], Vec<u8>)>,
    /// Reused per draw call.
    placed: Vec<Placed>,
    /// Each line's width in the last layout, in its pixels.
    line_widths: Vec<f32>,
    /// Signed-distance glyphs for text drawn at any size (3D text).
    pub sdf: SdfAtlas,
}

impl Text {
    pub fn new() -> Self {
        Self {
            fonts: vec![None],
            live: {
                let mut live = crate::handle::LiveSet::default();
                live.insert(0);
                live
            },
            glyphs: HashMap::new(),
            cursor: [PAD, PAD],
            row: 0,
            full: false,
            uploads: Vec::new(),
            placed: Vec::new(),
            line_widths: Vec::new(),
            sdf: SdfAtlas::default(),
        }
    }

    pub const ATLAS_SIZE: u32 = ATLAS;

    /// Whether `font` is the built-in one or a loaded one not freed.
    pub fn has_font(&self, font: Font) -> bool {
        self.live.contains(font.0)
    }

    /// Fonts loaded and not freed, not counting the built-in one.
    pub fn loaded_fonts(&self) -> usize {
        self.live.len() - 1
    }

    pub fn add_font(&mut self, bytes: &[u8]) -> Result<Font, String> {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(|e| format!("not a font file this engine can read: {e}"))?;
        self.fonts.push(Some(font));
        let id = self.fonts.len() as u32 - 1;
        self.live.insert(id);
        Ok(Font(id))
    }

    /// Drops a loaded font and its glyphs; true if it was live. Text in it
    /// uses the built-in font from now on, which is never freed.
    pub fn free_font(&mut self, font: Font) -> bool {
        if font == Font::DEFAULT || !self.live.remove(font.0) {
            return false;
        }
        self.fonts[font.0 as usize] = None;
        self.glyphs.retain(|k, _| k.0 != font.0);
        self.sdf.glyphs.retain(|k, _| k.0 != font.0);
        true
    }

    /// The font text in `font` is drawn with: itself, or the built-in one
    /// for a freed font or `Font::NONE`.
    fn resolve(&self, font: Font) -> Font {
        if self.has_font(font) { font } else { Font::DEFAULT }
    }

    /// Replaces a loaded font's data (hot reload); its glyphs are drawn again.
    pub fn replace_font(&mut self, font: Font, bytes: &[u8]) -> Result<(), String> {
        let new = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(|e| format!("not a font file this engine can read: {e}"))?;
        if !self.has_font(font) {
            return Ok(());
        }
        self.fonts[font.0 as usize] = Some(new);
        self.glyphs.retain(|k, _| k.0 != font.0);
        self.sdf.glyphs.retain(|k, _| k.0 != font.0);
        Ok(())
    }

    fn font(&mut self, font: Font) -> &fontdue::Font {
        let slot = &mut self.fonts[font.0 as usize];
        slot.get_or_insert_with(|| {
            fontdue::Font::from_bytes(epaint_default_fonts::UBUNTU_LIGHT, fontdue::FontSettings::default())
                .expect("the built-in font is valid")
        })
    }

    /// Call once per frame before drawing: forgets every glyph if the atlas
    /// filled up last frame, so it fills again with what's in use now.
    pub fn begin_frame(&mut self) -> bool {
        if self.sdf.full {
            self.sdf.full = false;
            self.sdf.glyphs.clear();
            self.sdf.cursor = [PAD, PAD];
            self.sdf.row = 0;
        }
        if !self.full {
            return false;
        }
        self.full = false;
        self.glyphs.clear();
        self.cursor = [PAD, PAD];
        self.row = 0;
        true
    }

    /// Lays out `text` at `px` physical pixels per line. With `rasterize`,
    /// missing glyphs are drawn into the atlas (queued in `uploads`).
    /// Returns the size in physical pixels; the glyphs are in `placed()`.
    pub fn layout(&mut self, text: &str, font: Font, px: f32, rasterize: bool) -> Vec2 {
        let font = self.resolve(font);
        let key_px = px.round().max(1.0) as u32;
        self.layout_with(text, font, px, |t, c| match t.glyphs.get(&(font.0, c, key_px)) {
            Some(g) => Some(*g),
            None if rasterize => t.rasterize(font, c, px, key_px),
            None => None,
        })
    }

    /// Lays out `text` with signed-distance glyphs, at [`SDF_PX`] pixels
    /// per line (missing glyphs are made and queued in `sdf.uploads`).
    /// Returns the size in those pixels; the glyphs are in `placed()`,
    /// each line's width in `line_widths()`.
    #[cfg_attr(not(feature = "3d"), allow(dead_code))]
    pub fn layout_sdf(&mut self, text: &str, font: Font) -> Vec2 {
        let font = self.resolve(font);
        self.layout_with(text, font, SDF_PX, |t, c| match t.sdf.glyphs.get(&(font.0, c)) {
            Some(g) => Some(*g),
            None => t.make_sdf(font, c),
        })
    }

    fn layout_with(
        &mut self,
        text: &str,
        font: Font,
        px: f32,
        mut glyph_of: impl FnMut(&mut Self, char) -> Option<Glyph>,
    ) -> Vec2 {
        let mut placed = std::mem::take(&mut self.placed);
        placed.clear();
        self.line_widths.clear();
        let lines = self.font(font).horizontal_line_metrics(px);
        let (ascent, line_height) = lines.map_or((px * 0.8, px), |m| (m.ascent, m.new_line_size));
        let (mut x, mut y, mut width) = (0.0f32, 0.0f32, 0.0f32);
        let mut line = 0;
        let mut prev: Option<char> = None;
        for c in text.chars() {
            if c == '\n' {
                width = width.max(x);
                self.line_widths.push(x);
                x = 0.0;
                y += line_height;
                line += 1;
                prev = None;
                continue;
            }
            let f = self.font(font);
            if let Some(p) = prev {
                x += f.horizontal_kern(p, c, px).unwrap_or(0.0);
            }
            let advance = f.metrics(c, px).advance_width;
            if let Some(glyph) = glyph_of(self, c) {
                let pos = Vec2::new(
                    (x + glyph.offset[0]).round(),
                    (y + ascent - glyph.offset[1] - glyph.size[1] as f32).round(),
                );
                placed.push(Placed { pos, glyph, line });
            }
            x += advance;
            prev = Some(c);
        }
        width = width.max(x);
        self.line_widths.push(x);
        self.placed = placed;
        Vec2::new(width, y + line_height)
    }

    /// Each line's width in the last layout.
    #[cfg_attr(not(feature = "3d"), allow(dead_code))]
    pub fn line_widths(&self) -> &[f32] {
        &self.line_widths
    }

    pub fn placed(&self) -> &[Placed] {
        &self.placed
    }

    fn rasterize(&mut self, font: Font, c: char, px: f32, key_px: u32) -> Option<Glyph> {
        let (m, coverage) = self.font(font).rasterize(c, px);
        let size = [m.width as u32, m.height as u32];
        let offset = [m.xmin as f32, m.ymin as f32];
        let at = if size[0] == 0 || size[1] == 0 {
            None
        } else {
            if self.cursor[0] + size[0] + PAD > ATLAS {
                self.cursor = [PAD, self.cursor[1] + self.row + PAD];
                self.row = 0;
            }
            if self.cursor[1] + size[1] + PAD > ATLAS {
                self.full = true;
                return None;
            }
            let at = self.cursor;
            self.cursor[0] += size[0] + PAD;
            self.row = self.row.max(size[1]);
            // White pixels, coverage as alpha: the sprite's color tints it.
            let rgba = coverage.iter().flat_map(|&a| [255, 255, 255, a]).collect();
            self.uploads.push((at, size, rgba));
            Some(at)
        };
        let glyph = Glyph { at, size, offset };
        self.glyphs.insert((font.0, c, key_px), glyph);
        Some(glyph)
    }
}

impl Text {
    /// Makes `c`'s signed-distance glyph and packs it into the SDF atlas.
    #[cfg_attr(not(feature = "3d"), allow(dead_code))]
    fn make_sdf(&mut self, font: Font, c: char) -> Option<Glyph> {
        let (m, coverage) = self.font(font).rasterize(c, SDF_PX);
        let spread = SDF_SPREAD;
        let (w, h) = (m.width as u32, m.height as u32);
        let size = if w == 0 || h == 0 {
            [0, 0]
        } else {
            [w + 2 * spread, h + 2 * spread]
        };
        let offset = [m.xmin as f32 - spread as f32, m.ymin as f32 - spread as f32];
        let at = if size[0] == 0 {
            None
        } else {
            let atlas = &mut self.sdf;
            if atlas.cursor == [0, 0] {
                atlas.cursor = [PAD, PAD];
            }
            if atlas.cursor[0] + size[0] + PAD > ATLAS {
                atlas.cursor = [PAD, atlas.cursor[1] + atlas.row + PAD];
                atlas.row = 0;
            }
            if atlas.cursor[1] + size[1] + PAD > ATLAS {
                atlas.full = true;
                return None;
            }
            let at = atlas.cursor;
            atlas.cursor[0] += size[0] + PAD;
            atlas.row = atlas.row.max(size[1]);
            let distance = signed_distance(&coverage, w, h, spread);
            let rgba = distance.iter().flat_map(|&a| [255, 255, 255, a]).collect();
            atlas.uploads.push((at, size, rgba));
            Some(at)
        };
        let glyph = Glyph { at, size, offset };
        self.sdf.glyphs.insert((font.0, c), glyph);
        Some(glyph)
    }
}

/// A glyph's coverage (`w` x `h`) as a signed distance field `spread`
/// pixels larger on each side: 128 on the edge, up to 255 `spread` pixels
/// inside, down to 0 as far outside.
#[cfg_attr(not(feature = "3d"), allow(dead_code))]
fn signed_distance(coverage: &[u8], w: u32, h: u32, spread: u32) -> Vec<u8> {
    let (w, h, s) = (w as i32, h as i32, spread as i32);
    let at = |x: i32, y: i32| -> f32 {
        if x < 0 || y < 0 || x >= w || y >= h {
            0.0
        } else {
            f32::from(coverage[(y * w + x) as usize]) / 255.0
        }
    };
    let (out_w, out_h) = (w + 2 * s, h + 2 * s);
    let mut out = Vec::with_capacity((out_w * out_h) as usize);
    for oy in 0..out_h {
        for ox in 0..out_w {
            let (x, y) = (ox - s, oy - s);
            let a = at(x, y);
            let signed = if a > 0.0 && a < 1.0 {
                // On the edge: coverage says how far across it.
                a - 0.5
            } else {
                // The nearest pixel the edge runs through or past; it
                // crosses one half of coverage `b - 0.5` beyond that
                // pixel's center (for inside pixels; mirrored outside).
                let inside = a >= 1.0;
                let mut best = s as f32;
                for dy in -s..=s {
                    for dx in -s..=s {
                        let b = at(x + dx, y + dy);
                        let across = if inside { b - 0.5 } else { 0.5 - b };
                        if (inside && b >= 1.0) || (!inside && b <= 0.0) {
                            continue;
                        }
                        let d = ((dx * dx + dy * dy) as f32).sqrt() + across;
                        best = best.min(d);
                    }
                }
                if inside { best } else { -best }
            };
            let v = 0.5 + signed / (2.0 * s as f32);
            out.push((v.clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_measures_lines_and_kerning() {
        let mut t = Text::new();
        let one = t.layout("Hello", Font::DEFAULT, 20.0, false);
        let two = t.layout("Hello\nHello", Font::DEFAULT, 20.0, false);
        assert!(one.x > 30.0 && one.x < 70.0, "{one}");
        assert_eq!(two.x, one.x);
        assert!((two.y - 2.0 * one.y).abs() < 0.01, "{one} {two}");
        assert_eq!(t.layout("", Font::DEFAULT, 20.0, false).x, 0.0);
    }

    #[test]
    fn glyphs_are_rasterized_once_and_packed_apart() {
        let mut t = Text::new();
        t.layout("abca", Font::DEFAULT, 32.0, true);
        assert_eq!(t.uploads.len(), 3, "a, b, c once each");
        assert_eq!(t.placed().len(), 4);
        let spots: Vec<_> = t.uploads.iter().map(|(at, size, _)| (*at, *size)).collect();
        for (i, (a, sa)) in spots.iter().enumerate() {
            for (b, sb) in &spots[i + 1..] {
                let apart =
                    a[0] + sa[0] <= b[0] || b[0] + sb[0] <= a[0] || a[1] + sa[1] <= b[1] || b[1] + sb[1] <= a[1];
                assert!(apart, "glyphs overlap in the atlas");
            }
        }
        t.uploads.clear();
        t.layout("cab", Font::DEFAULT, 32.0, true);
        assert!(t.uploads.is_empty(), "cached");
    }

    #[test]
    fn a_full_atlas_starts_over_next_frame() {
        let mut t = Text::new();
        // Huge glyphs fill the atlas quickly.
        let many: String = ('A'..='Z').chain('a'..='z').collect();
        t.layout(&many, Font::DEFAULT, 400.0, true);
        assert!(t.full);
        assert!(t.begin_frame());
        assert!(!t.full && t.glyphs.is_empty());
        assert!(!t.begin_frame());
    }

    #[test]
    fn distance_fields_are_half_on_the_edge_and_rise_inside() {
        // A 4 x 4 square, fully covered, with a spread of 3.
        let field = signed_distance(&[255; 16], 4, 4, 3);
        let side = 10;
        let at = |x: usize, y: usize| field[y * side + x];
        // Center of the square: 2 px from the edge, inside.
        assert!(at(5, 5) > 128 + 20, "{}", at(5, 5));
        // Just inside and just outside the left edge (x = 3 is the first
        // covered column) are on either side of one half.
        assert!(at(3, 5) > 128 && at(2, 5) < 128, "{} {}", at(3, 5), at(2, 5));
        // Far outside: as low as the spread allows.
        assert_eq!(at(0, 0), 0);
        // Symmetric.
        assert_eq!(at(3, 5), at(6, 5));
    }

    #[test]
    fn sdf_glyphs_are_made_once_padded_by_the_spread() {
        let mut t = Text::new();
        let size = t.layout_sdf("Hi\nthere", Font::DEFAULT);
        assert_eq!(t.line_widths().len(), 2);
        assert!(size.y > SDF_PX * 1.5, "{size}");
        assert_eq!(t.placed().iter().filter(|p| p.line == 1).count(), 5);
        let made = t.sdf.uploads.len();
        assert_eq!(made, 6, "H, i, t, h, e, r (e twice)");
        let ([w, h], bitmap) = (t.sdf.uploads[0].1, t.font(Font::DEFAULT).rasterize('H', SDF_PX).0);
        assert_eq!(
            [w, h],
            [
                bitmap.width as u32 + 2 * SDF_SPREAD,
                bitmap.height as u32 + 2 * SDF_SPREAD
            ]
        );
        t.layout_sdf("there", Font::DEFAULT);
        assert_eq!(t.sdf.uploads.len(), made, "cached");
    }

    #[test]
    fn spaces_take_room_but_no_atlas_space() {
        let mut t = Text::new();
        let w = t.layout("a a", Font::DEFAULT, 20.0, true).x;
        assert!(w > t.layout("aa", Font::DEFAULT, 20.0, true).x);
        assert!(t.uploads.iter().all(|(_, size, _)| size[0] > 0));
    }
}
