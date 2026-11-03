//! Text: fonts, a glyph atlas filled on demand, and layout.
//!
//! Glyphs are rasterized at the window's physical pixel size the first time
//! they're drawn, packed into one atlas texture, and drawn as sprites in
//! screen space. Steady-state frames only look glyphs up.

use std::collections::HashMap;

use glam::Vec2;

use crate::color::Color;

/// A loaded font. Get one from [`Ctx::load_font`](crate::Ctx::load_font);
/// [`Font::DEFAULT`] is always there.
///
/// ```
/// # use crabbyengine::*;
/// # fn hud(ctx: &mut Ctx) {
/// ctx.draw_text_ex("Score: 120", vec2(16.0, 16.0), TextParams {
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

/// Optional settings for [`Ctx::draw_text_ex`](crate::Ctx::draw_text_ex).
///
/// ```
/// # use crabbyengine::*;
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
}

pub(crate) struct Text {
    fonts: Vec<Option<fontdue::Font>>,
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
}

impl Text {
    pub fn new() -> Self {
        Self {
            fonts: vec![None],
            glyphs: HashMap::new(),
            cursor: [PAD, PAD],
            row: 0,
            full: false,
            uploads: Vec::new(),
            placed: Vec::new(),
        }
    }

    pub const ATLAS_SIZE: u32 = ATLAS;

    pub fn add_font(&mut self, bytes: &[u8]) -> Result<Font, String> {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(|e| format!("not a font file this engine can read: {e}"))?;
        self.fonts.push(Some(font));
        Ok(Font(self.fonts.len() as u32 - 1))
    }

    /// Replaces a loaded font's data (hot reload); its glyphs are drawn again.
    pub fn replace_font(&mut self, font: Font, bytes: &[u8]) -> Result<(), String> {
        let new = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(|e| format!("not a font file this engine can read: {e}"))?;
        self.fonts[font.0 as usize] = Some(new);
        self.glyphs.retain(|k, _| k.0 != font.0);
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
        let mut placed = std::mem::take(&mut self.placed);
        placed.clear();
        let lines = self.font(font).horizontal_line_metrics(px);
        let (ascent, line_height) = lines.map_or((px * 0.8, px), |m| (m.ascent, m.new_line_size));
        let key_px = px.round().max(1.0) as u32;
        let (mut x, mut y, mut width) = (0.0f32, 0.0f32, 0.0f32);
        let mut prev: Option<char> = None;
        for c in text.chars() {
            if c == '\n' {
                width = width.max(x);
                x = 0.0;
                y += line_height;
                prev = None;
                continue;
            }
            let f = self.font(font);
            if let Some(p) = prev {
                x += f.horizontal_kern(p, c, px).unwrap_or(0.0);
            }
            let advance = f.metrics(c, px).advance_width;
            let glyph = match self.glyphs.get(&(font.0, c, key_px)) {
                Some(g) => Some(*g),
                None if rasterize => self.rasterize(font, c, px, key_px),
                None => None,
            };
            if let Some(glyph) = glyph {
                let pos = Vec2::new(
                    (x + glyph.offset[0]).round(),
                    (y + ascent - glyph.offset[1] - glyph.size[1] as f32).round(),
                );
                placed.push(Placed { pos, glyph });
            }
            x += advance;
            prev = Some(c);
        }
        width = width.max(x);
        self.placed = placed;
        Vec2::new(width, y + line_height)
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
    fn spaces_take_room_but_no_atlas_space() {
        let mut t = Text::new();
        let w = t.layout("a a", Font::DEFAULT, 20.0, true).x;
        assert!(w > t.layout("aa", Font::DEFAULT, 20.0, true).x);
        assert!(t.uploads.iter().all(|(_, size, _)| size[0] > 0));
    }
}
