//! [`Graphics`]: drawing and everything that lives on the GPU, as `ctx.gfx`.
//!
//! **What lives here.** The frame's draw queues (sprites, text, shapes),
//! textures and fonts, screenshots, and with the `3d` feature the meshes,
//! materials, shaders, effects, compute shaders, GPU buffers, instance
//! batches, terrain, render targets, texture arrays and reflection probes.
//! The cameras, lights, sky and post-processing settings are plain fields of
//! [`Ctx`](crate::Ctx) (`ctx.camera3d`, `ctx.lighting`, `ctx.post`): the
//! renderer reads them when the frame is drawn.
//!
//! **When to call it.** Load and create in setup (or any system: a file is
//! read once, then cached by name). Draw calls queue work for the current
//! frame, so call them every frame from a frame system or `.draw(...)`; a
//! draw queued in a simulation system is drawn once per step. Handles
//! (`Texture`, `Mesh`, `Material`, ...) are small `Copy` ids: keep them in
//! components and resources, and free them with
//! [`Ctx::free`](crate::Ctx::free).
//!
//! **What it needs.** A window for anything to appear. In a headless run
//! (tests, servers) loading and creating still give working handles with
//! the right sizes, draws are dropped, and nothing touches a GPU. It never
//! needs the world: the few calls that read or write entities
//! (`write_animated_morphs`, the reflection probe calls) take `&mut World`
//! explicitly, so `ctx.gfx` borrows apart from `ctx.world`.
//!
//! ```
//! use ponko::*;
//!
//! struct Crab(Texture);
//!
//! fn draw(ctx: &mut Ctx) {
//!     // `ctx.gfx` is its own field: draw while a query borrows the world.
//!     for (crab, at) in ctx.world.query::<(&Crab, &Transform2D)>().iter() {
//!         ctx.gfx.draw(&crab.0, at.position);
//!     }
//! }
//!
//! Game::new("gfx", 320, 240).sim_system(draw).run_headless(1, |ctx| {
//!     let pixel = ctx.gfx.create_texture(1, 1, &[255; 4]);
//!     ctx.world.spawn((Crab(pixel), Transform2D::at(vec2(10.0, 20.0))));
//! });
//! ```

use glam::Vec2;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use crate::assets;
use crate::camera::Rect;
use crate::color::Color;
use crate::ctx::{AssetKind, region};
use crate::hot_reload::Asset;
use crate::render::Renderer;
use crate::render::batch::{DrawList, SpriteInstance, uv_region};
use crate::text::{Font, Text, TextParams};
use crate::texture::{DrawParams, Texture, TextureFilter};

/// A screenshot someone asked for: where to save it, and who to tell.
pub(crate) struct ScreenshotRequest {
    pub(crate) path: PathBuf,
    /// Scale down so neither side exceeds this (0: full size).
    pub(crate) max_size: u32,
    pub(crate) reply: Option<Sender<Result<Value, String>>>,
}

/// Side of the circle texture behind [`Graphics::draw_circle`], in pixels.
const CIRCLE: u32 = 128;

/// Which queue a shape goes to: the world (under the camera) or the screen.
#[derive(Clone, Copy)]
enum Layer {
    World,
    Screen,
}

/// Drawing and GPU assets: `ctx.gfx`.
///
/// `ctx.draw`, `ctx.draw_text` and `ctx.load_texture` are shortcuts for
/// [`draw`](Self::draw), [`draw_text`](Self::draw_text) and
/// [`load_texture`](Self::load_texture); everything else is here.
///
/// - **What lives here:** the frame's draw queues (sprites, text, shapes),
///   textures, fonts and screenshots, and with the `3d` feature meshes,
///   materials, shaders, effects, compute shaders, GPU buffers, instance
///   batches, terrain, render targets, texture arrays and reflection
///   probes. Cameras, lights, sky and post-processing are plain fields of
///   [`Ctx`](crate::Ctx).
/// - **When to call it:** load and create in setup (or any system: a file
///   is read once, then cached by name). Draw calls queue work for the
///   current frame, so make them every frame from a frame system; a draw
///   queued in a simulation system is drawn once per step.
/// - **What it needs:** a window for anything to appear. Headless, loading
///   and creating still give working handles with the right sizes, draws
///   are dropped and nothing touches a GPU. The few calls that read or
///   write entities take `&mut World`, so `ctx.gfx` borrows apart from
///   `ctx.world`.
///
/// ```
/// use ponko::*;
///
/// struct Crab(Texture);
///
/// fn draw(ctx: &mut Ctx) {
///     // `ctx.gfx` is its own field: draw while a query borrows the world.
///     for (crab, at) in ctx.world.query::<(&Crab, &Transform2D)>().iter() {
///         ctx.gfx.draw(&crab.0, at.position);
///     }
/// }
///
/// Game::new("gfx", 320, 240).sim_system(draw).run_headless(1, |ctx| {
///     let pixel = ctx.gfx.create_texture(1, 1, &[255; 4]);
///     ctx.world.spawn((Crab(pixel), Transform2D::at(vec2(10.0, 20.0))));
/// });
/// ```
pub struct Graphics {
    /// Assets loaded under residency budgets (`resident_mesh`,
    /// `resident_texture`), by kind. The calls are on `Ctx` (eviction looks
    /// at the world); the bookkeeping is here because `update_texture`
    /// changes it.
    pub(crate) residency: crate::residency::Residency,
    /// The upload queue and streamed assets (`stream_texture`, `stream_mesh`).
    pub(crate) streaming: crate::streaming::Streaming,
    /// The simulation tick when the running system started (residency
    /// orders assets by the frame and tick they were last used in).
    pub(crate) sim_tick: u64,
    /// No window and no GPU: handles are made up, draws are dropped.
    pub(crate) headless: bool,
    /// The frame being built (for shader status reports).
    pub(crate) frame: u64,
    /// Texture ids handed out while headless (no GPU to allocate them).
    pub(crate) headless_textures: u32,
    /// The textures the game made and hasn't freed.
    pub(crate) textures_live: crate::handle::LiveSet,
    /// What the live assets cost (see `Ctx::asset_inventory`).
    pub(crate) asset_ledger: crate::inventory::Ledger,
    /// Shared with the other services that load files.
    pub(crate) asset_sources: assets::SharedSources,
    /// Hot reload's watch list, shared the same way.
    pub(crate) watcher: crate::hot_reload::SharedWatcher,
    /// Loaded textures and fonts by where they came from: index into
    /// `cached_textures`, or the font id.
    pub(crate) loaded: HashMap<(assets::Origin, AssetKind), u64>,
    pub(crate) cached_textures: Vec<Texture>,
    /// Texture file names for scene saves, shared with the world's copy.
    pub(crate) texture_names: crate::reflect::TextureNames,
    /// Mesh file names, the same way.
    #[cfg(feature = "3d")]
    pub(crate) mesh_names: crate::reflect::MeshNames,
    pub(crate) renderer: Option<Renderer>,
    pub(crate) screenshots: Vec<ScreenshotRequest>,
    pub(crate) draw_list: DrawList,
    pub(crate) ui_list: DrawList,
    pub(crate) clear_color: Color,
    pub(crate) text: Text,
    font_atlas: Option<Texture>,
    pub(crate) scale_factor: f32,
    /// A 1x1 white texture, for rectangles and lines.
    white: Option<Texture>,
    /// A white circle with a soft edge, for circles.
    circle: Option<Texture>,
    #[cfg(feature = "3d")]
    pub(crate) assets3d: crate::three::Assets3D,
    #[cfg(feature = "3d")]
    pub(crate) frame3d: crate::three::Frame3D,
}

impl Graphics {
    pub(crate) fn new(
        asset_sources: assets::SharedSources,
        watcher: crate::hot_reload::SharedWatcher,
        texture_names: crate::reflect::TextureNames,
        #[cfg(feature = "3d")] mesh_names: crate::reflect::MeshNames,
    ) -> Self {
        Self {
            residency: Default::default(),
            streaming: Default::default(),
            sim_tick: 0,
            headless: false,
            frame: 0,
            headless_textures: 0,
            textures_live: Default::default(),
            asset_ledger: Default::default(),
            asset_sources,
            watcher,
            loaded: HashMap::new(),
            cached_textures: Vec::new(),
            texture_names,
            #[cfg(feature = "3d")]
            mesh_names,
            renderer: None,
            screenshots: Vec::new(),
            draw_list: DrawList::default(),
            ui_list: DrawList::default(),
            clear_color: Color::PONKO_DARK,
            text: Text::new(),
            font_atlas: None,
            scale_factor: 1.0,
            white: None,
            circle: None,
            #[cfg(feature = "3d")]
            assets3d: Default::default(),
            #[cfg(feature = "3d")]
            frame3d: Default::default(),
        }
    }

    /// Sets the background color. It stays until you change it.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// ctx.gfx.clear(Color::hex(0x203040));
    /// # }
    /// ```
    pub fn clear(&mut self, color: Color) {
        self.clear_color = color;
    }

    /// Draws `texture` centered at `pos`, in world units (window pixels until
    /// you move or zoom [`camera`](crate::Ctx::camera)).
    ///
    /// Drawing is cheap: calls are collected and sent to the GPU in big
    /// batches at the end of the frame. Call it every frame for everything
    /// you want visible.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn update(ctx: &mut Ctx, crab: Texture) {
    /// ctx.gfx.draw(&crab, vec2(400.0, 300.0));
    /// # }
    /// ```
    pub fn draw(&mut self, texture: &Texture, pos: Vec2) {
        push_sprite(
            &mut self.draw_list,
            texture,
            pos,
            Vec2::ONE,
            0.0,
            0.0,
            Vec2::splat(0.5),
            [255; 4],
            None,
        );
    }

    /// Draws `texture` with rotation, scale, tint, draw order or pivot.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn update(ctx: &mut Ctx, crab: Texture) {
    /// ctx.gfx.draw_ex(&crab, vec2(400.0, 300.0), DrawParams {
    ///     rotation: ctx.time.elapsed() as f32,
    ///     color: Color::RED.with_alpha(0.5),
    ///     z: 1.0, // on top of everything at z = 0
    ///     ..Default::default()
    /// });
    /// # }
    /// ```
    pub fn draw_ex(&mut self, texture: &Texture, pos: Vec2, params: DrawParams) {
        push_sprite(
            if params.screen {
                &mut self.ui_list
            } else {
                &mut self.draw_list
            },
            texture,
            pos,
            params.scale,
            params.rotation,
            params.z,
            params.pivot,
            params.color.to_rgba8(),
            params.region,
        );
    }

    /// Draws `text` with its top-left corner at `pos`, in window pixels (on
    /// top of the world, ignoring the camera). `size` is the line height in
    /// pixels. `\n` starts a new line.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn hud(ctx: &mut Ctx) {
    /// let fps = ctx.time.fps();
    /// ctx.gfx.draw_text(&format!("{fps:.0} FPS"), vec2(10.0, 10.0), 18.0, Color::WHITE);
    /// # }
    /// ```
    pub fn draw_text(&mut self, text: &str, pos: Vec2, size: f32, color: Color) {
        self.draw_text_ex(
            text,
            pos,
            TextParams {
                size,
                color,
                ..Default::default()
            },
        );
    }

    /// [`draw_text`](Self::draw_text) with a font and draw order.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn hud(ctx: &mut Ctx, title: Font) {
    /// ctx.gfx.draw_text_ex("Colony", vec2(20.0, 20.0), TextParams {
    ///     font: title,
    ///     size: 40.0,
    ///     z: 1.0,
    ///     ..Default::default()
    /// });
    /// # }
    /// ```
    pub fn draw_text_ex(&mut self, text: &str, pos: Vec2, params: TextParams) {
        if self.headless || text.is_empty() {
            return;
        }
        let atlas = match self.font_atlas {
            Some(t) => t,
            None => {
                let side = Text::ATLAS_SIZE;
                let t = self.make_texture(side, side, &vec![0; (side * side * 4) as usize]);
                self.font_atlas = Some(t);
                t
            }
        };
        let scale = self.scale_factor;
        self.text.layout(text, params.font, params.size * scale, true);
        if !self.text.uploads.is_empty()
            && let Some(renderer) = self.renderer.as_mut()
        {
            for (at, size, rgba) in self.text.uploads.drain(..) {
                renderer.update_texture_region(atlas.id, at, size, &rgba);
            }
        }
        let side = Text::ATLAS_SIZE as f32;
        let color = params.color.to_rgba8();
        for p in self.text.placed() {
            let Some(at) = p.glyph.at else { continue };
            let size = Vec2::new(p.glyph.size[0] as f32, p.glyph.size[1] as f32);
            let uv = uv_region([at[0] as f32 / side, at[1] as f32 / side], (size / side).into());
            self.ui_list.push(
                atlas.id,
                params.z,
                SpriteInstance::new(
                    (pos + p.pos / scale).into(),
                    (size / scale).into(),
                    [0.0, 0.0],
                    0.0,
                    color,
                )
                .with_uv(uv),
            );
        }
    }

    /// The size `text` would take with [`draw_text`](Self::draw_text) at
    /// `size`, in window pixels. Works headless too.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn centered(ctx: &mut Ctx) {
    /// let label = "Paused";
    /// let size = ctx.gfx.measure_text(label, 32.0);
    /// let at = (ctx.window_size() - size) / 2.0;
    /// ctx.gfx.draw_text(label, at, 32.0, Color::WHITE);
    /// # }
    /// ```
    pub fn measure_text(&mut self, text: &str, size: f32) -> Vec2 {
        self.measure_text_ex(text, Font::DEFAULT, size)
    }

    /// [`measure_text`](Self::measure_text) for another font.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx, title: Font) {
    /// let width = ctx.gfx.measure_text_ex("Colony", title, 40.0).x;
    /// # }
    /// ```
    pub fn measure_text_ex(&mut self, text: &str, font: Font, size: f32) -> Vec2 {
        let scale = self.scale_factor;
        self.text.layout(text, font, size * scale, false) / scale
    }

    /// Sets how `texture` is sampled. Use [`TextureFilter::Nearest`] for pixel
    /// art and tilesets; the default is smooth.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// let tiles = ctx.gfx.load_texture("crab.png");
    /// ctx.gfx.set_texture_filter(&tiles, TextureFilter::Nearest);
    /// # }
    /// ```
    pub fn set_texture_filter(&mut self, texture: &Texture, filter: TextureFilter) {
        if !self.textures_live.contains(texture.id) {
            return;
        }
        if let Some(r) = self.renderer.as_mut() {
            r.set_filter(texture.id, filter == TextureFilter::Nearest);
        }
    }

    /// Loads a TrueType or OpenType font from the `assets/` folder.
    /// Loading the same file again returns the same handle.
    ///
    /// # Panics
    /// With a clear message if the file is missing or isn't a font.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// let title = ctx.gfx.load_font("fonts/Title.ttf");
    /// # let _ = title;
    /// # }
    /// ```
    pub fn load_font(&mut self, path: &str) -> Font {
        self.try_load_font(path).unwrap_or_else(|e| panic!("{e}"))
    }

    /// [`load_font`](Self::load_font) with errors as values:
    /// [`ErrorKind::NotFound`](crate::ErrorKind::NotFound),
    /// [`Io`](crate::ErrorKind::Io), or [`Decode`](crate::ErrorKind::Decode)
    /// if it isn't a font.
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     let title = ctx.gfx.try_load_font("fonts/Fancy.ttf").unwrap_or(Font::DEFAULT);
    ///     # let _ = title;
    /// });
    /// ```
    pub fn try_load_font(&mut self, path: &str) -> Result<Font, crate::Error> {
        if let Some((_, id)) = self.cached_in_sources(path, AssetKind::Font) {
            return Ok(Font(id as u32));
        }
        let found = self.asset_sources.find(path)?;
        let origin = found.origin.clone();
        if let Some(&id) = self.loaded.get(&(origin.clone(), AssetKind::Font)) {
            return Ok(Font(id as u32));
        }
        let bytes = found.into_bytes().map_err(|e| e.with_path(path))?;
        let font = self.text.add_font(&bytes).map_err(|e| {
            let what = match &origin {
                assets::Origin::File(file) => file.display().to_string(),
                assets::Origin::Source { .. } => origin.describe(path),
            };
            crate::Error::decode(path, format!("{what}: {e}"))
        })?;
        self.loaded.insert((origin.clone(), AssetKind::Font), u64::from(font.0));
        if let assets::Origin::File(file) = origin {
            self.watcher.watch(path, file, Asset::Font(font.0));
        }
        Ok(font)
    }

    /// A cached `kind` asset called `name` from an added source, searched
    /// in order. Earlier sources win, so this matches what a fresh
    /// [`assets::Sources::find`] would pick for anything still cached.
    fn cached_in_sources(&self, name: &str, kind: AssetKind) -> Option<(assets::Origin, u64)> {
        self.asset_sources
            .origins(name)
            .into_iter()
            .find_map(|origin| self.loaded.get(&(origin.clone(), kind)).map(|&v| (origin, v)))
    }

    /// Whether `font` is the built-in font or one this context loaded and
    /// hasn't freed: the same as [`is_live`](crate::Ctx::is_live). Text in a font
    /// that isn't uses the built-in one.
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     assert!(ctx.gfx.has_font(Font::DEFAULT));
    /// });
    /// ```
    #[must_use]
    pub fn has_font(&self, font: Font) -> bool {
        self.text.has_font(font)
    }

    /// Loads an image from the `assets/` folder (PNG for now).
    ///
    /// Do this once in setup and keep the [`Texture`]; it's a cheap handle.
    /// Loading the same file again returns the same texture (use
    /// [`create_texture`](Self::create_texture) for a copy of your own).
    ///
    /// # Panics
    /// With a clear message if the file is missing or isn't a valid image.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// Game::new("Crabs", 800, 600).run(|ctx| {
    ///     let crab = ctx.gfx.load_texture("crab.png");
    ///     move |ctx: &mut Ctx| ctx.gfx.draw(&crab, vec2(400.0, 300.0))
    /// });
    /// ```
    pub fn load_texture(&mut self, path: &str) -> Texture {
        self.try_load_texture(path).unwrap_or_else(|e| panic!("{e}"))
    }

    /// [`load_texture`](Self::load_texture) with errors as values, for
    /// optional assets: [`ErrorKind::NotFound`](crate::ErrorKind::NotFound)
    /// if no source has it, [`Io`](crate::ErrorKind::Io) if it can't be
    /// read, [`Decode`](crate::ErrorKind::Decode) if it isn't an image.
    /// Like `load_texture`, it also names the texture, so sprites showing it
    /// can be saved.
    ///
    /// ```
    /// # use ponko::*;
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     // A missing optional skin is not a bug: fall back to no skin.
    ///     let skin: Option<Texture> = ctx.gfx.try_load_texture("skins/gold.png").ok();
    ///     assert!(skin.is_none());
    /// });
    /// ```
    pub fn try_load_texture(&mut self, path: &str) -> Result<Texture, crate::Error> {
        let cached = match self.cached_in_sources(path, AssetKind::Texture) {
            Some((_, i)) => Ok(i),
            None => {
                let found = self.asset_sources.find(path)?;
                match self.loaded.get(&(found.origin.clone(), AssetKind::Texture)) {
                    Some(&i) => Ok(i),
                    None => Err(found),
                }
            }
        };
        let texture = match cached {
            Ok(i) => self.cached_textures[i as usize],
            Err(found) => {
                // Acquire, decode (CPU only), then create on the GPU (game thread).
                let origin = found.origin.clone();
                let bytes = found.into_bytes().map_err(|e| e.with_path(path))?;
                let image = assets::decode_image(&bytes).map_err(|e| {
                    crate::Error::decode(path, format!("{} is not a valid image: {e}", origin.describe(path)))
                })?;
                let texture = self.create_texture(image.width, image.height, &image.rgba);
                self.loaded
                    .insert((origin.clone(), AssetKind::Texture), self.cached_textures.len() as u64);
                self.cached_textures.push(texture);
                if let assets::Origin::File(file) = origin {
                    self.watcher.watch(path, file, Asset::Texture(texture));
                }
                texture
            }
        };
        self.texture_names.add(texture.id, path, texture);
        Ok(texture)
    }

    /// Creates a texture from raw RGBA pixels (4 bytes per pixel, row by row,
    /// top row first). Handy for generated images.
    ///
    /// # Panics
    /// If `rgba.len()` isn't `width * height * 4`, or the size is 0 or larger
    /// than the GPU supports.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn setup(ctx: &mut Ctx) {
    /// // A 2x2 checkerboard.
    /// let w = [255, 255, 255, 255];
    /// let b = [0, 0, 0, 255];
    /// let pixels = [w, b, b, w].concat();
    /// let checker = ctx.gfx.create_texture(2, 2, &pixels);
    /// # }
    /// ```
    pub fn create_texture(&mut self, width: u32, height: u32, rgba: &[u8]) -> Texture {
        let texture = self.make_texture(width, height, rgba);
        self.textures_live.insert(texture.id);
        texture
    }

    /// [`create_texture`](Self::create_texture) for the engine's own
    /// textures (UI shapes, glyph atlases): not the game's to free, and not
    /// counted by [`live_counts`](crate::Ctx::live_counts).
    pub(crate) fn make_texture(&mut self, width: u32, height: u32, rgba: &[u8]) -> Texture {
        assert!(
            width > 0 && height > 0,
            "ponko: texture size must not be zero (got {width} x {height})"
        );
        let expected = width as usize * height as usize * 4;
        assert_eq!(
            rgba.len(),
            expected,
            "ponko: a {width} x {height} texture needs {expected} bytes of RGBA, got {}",
            rgba.len()
        );
        let bytes = crate::inventory::rgba8_bytes(width, height);
        if self.headless {
            // No GPU: a handle that remembers its size, so game logic still works.
            self.headless_textures += 1;
            self.asset_ledger.textures.insert(self.headless_textures, bytes);
            return Texture::new(self.headless_textures, width, height);
        }
        let renderer = self.renderer();
        let max = renderer.max_texture_size();
        assert!(
            width <= max && height <= max,
            "ponko: {width} x {height} is larger than this GPU's maximum texture size ({max})"
        );
        let id = renderer.create_texture(width, height, rgba);
        self.asset_ledger.textures.insert(id, bytes);
        Texture::new(id, width, height)
    }

    /// Frees game texture `id` (see [`Ctx::free`]): what still shows it
    /// sees the renderer's white stand-in.
    pub(crate) fn free_texture_id(&mut self, id: u32) {
        if !self.textures_live.remove(id) {
            return;
        }
        self.asset_ledger.textures.remove(&id);
        self.forget_streamed_texture_id(id);
        let cached = &self.cached_textures;
        self.loaded
            .retain(|(_, kind), i| *kind != AssetKind::Texture || cached[*i as usize].id != id);
        self.watcher.forget(|a| matches!(a, Asset::Texture(t) if t.id == id));
        self.texture_names.remove(id);
        #[cfg(feature = "3d")]
        {
            self.assets3d.render_targets.retain(|t| t.id != id);
            self.assets3d.shared_images.retain(|_, t| t.id != id);
        }
        if let Some(renderer) = self.renderer.as_mut()
            && !self.headless
        {
            renderer.free_texture(id);
        }
    }

    /// Frees game font `id` (see [`Ctx::free`]); never the built-in one.
    pub(crate) fn free_font_id(&mut self, id: u32) {
        if !self.text.free_font(Font(id)) {
            return;
        }
        self.loaded
            .retain(|(_, kind), v| *kind != AssetKind::Font || *v != u64::from(id));
        self.watcher.forget(|a| matches!(a, Asset::Font(f) if *f == id));
    }

    /// Replaces every pixel of `texture` with `rgba` (same size as it was
    /// created with, 4 bytes per pixel). For images that change while the
    /// game runs: a minimap, a heat map, a simulated field. Cheap enough to
    /// do every frame for textures up to a few megapixels.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let mut pixels = vec![0u8; 64 * 64 * 4];
    /// let heat = ctx.gfx.create_texture(64, 64, &pixels);
    /// pixels[0..4].copy_from_slice(&[255, 0, 0, 255]); // top-left pixel red
    /// ctx.gfx.update_texture(&heat, &pixels);
    /// # }
    /// ```
    ///
    /// # Panics
    /// If `rgba` isn't `width * height * 4` bytes.
    #[track_caller]
    pub fn update_texture(&mut self, texture: &Texture, rgba: &[u8]) {
        let (w, h) = (texture.width(), texture.height());
        let expected = w as usize * h as usize * 4;
        assert_eq!(
            rgba.len(),
            expected,
            "ponko: a {w} x {h} texture needs {expected} bytes of RGBA, got {}",
            rgba.len()
        );
        #[cfg(feature = "3d")]
        assert!(
            !self.is_render_target(*texture),
            "ponko: a render target is drawn by its target camera, not updated with pixels"
        );
        // A freed texture (or `Texture::NONE`) has no pixels of its own.
        if !self.textures_live.contains(texture.id) {
            return;
        }
        // A streamed texture at less than full resolution: back to full.
        if self.streaming.reduced_size(texture.id).is_some() {
            self.asset_ledger
                .textures
                .insert(texture.id, crate::inventory::rgba8_bytes(w, h));
            if !self.headless {
                self.renderer().replace_texture(texture.id, w, h, rgba);
            }
            self.streamed_texture_restored(texture.id);
            return;
        }
        if self.headless {
            return;
        }
        self.renderer().update_texture(texture.id, w, h, rgba);
    }

    /// How many sprites have been drawn so far this frame.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn update(ctx: &mut Ctx) {
    /// println!("{} sprites", ctx.gfx.sprite_count());
    /// # }
    /// ```
    pub fn sprite_count(&self) -> usize {
        self.draw_list.len() + self.ui_list.len()
    }

    /// Saves the next frame as a PNG at `path`, once it has been drawn.
    /// Does nothing in headless runs (there is no frame). Errors are printed.
    ///
    /// ```
    /// # use ponko::*;
    /// fn photo_mode(ctx: &mut Ctx) {
    ///     if ctx.key_pressed(Key::F12) {
    ///         ctx.gfx.save_screenshot("screenshot.png");
    ///     }
    /// }
    /// # Game::new("t", 1, 1).system(photo_mode).run_headless(1, |_| {});
    /// ```
    pub fn save_screenshot(&mut self, path: impl Into<PathBuf>) {
        if !self.headless {
            self.queue_screenshot(path.into(), 0, None);
        }
    }

    pub(crate) fn queue_screenshot(
        &mut self,
        path: PathBuf,
        max_size: u32,
        reply: Option<Sender<Result<Value, String>>>,
    ) {
        self.screenshots.push(ScreenshotRequest { path, max_size, reply });
    }

    pub(crate) fn renderer(&mut self) -> &mut Renderer {
        self.renderer
            .as_mut()
            .expect("ponko: textures can only be created once the game is running (in setup or update)")
    }

    /// What the GPU can do: the adapter, its main limits, and for each
    /// device-dependent renderer feature (compute passes, TAA, SSR, global
    /// illumination, GPU timestamps, ...) whether it's available and, if
    /// not, the fallback used and why. `None` in headless runs and before
    /// the window opens. See [`GpuCapabilities`](crate::GpuCapabilities).
    #[must_use]
    pub fn gpu_capabilities(&self) -> Option<&crate::GpuCapabilities> {
        self.renderer.as_ref().map(Renderer::capabilities)
    }

    /// Engine side: forget queued draws (headless runs never render).
    pub(crate) fn discard_draws(&mut self) {
        self.draw_list.clear();
        self.ui_list.clear();
    }

    // ----------------------------------------------------------------- shapes

    fn white_texture(&mut self) -> crate::Texture {
        if let Some(t) = self.white {
            return t;
        }
        let t = self.make_texture(1, 1, &[255; 4]);
        self.white = Some(t);
        t
    }

    fn circle_texture(&mut self) -> crate::Texture {
        if let Some(t) = self.circle {
            return t;
        }
        let r = CIRCLE as f32 / 2.0;
        let mut rgba = Vec::with_capacity((CIRCLE * CIRCLE * 4) as usize);
        for y in 0..CIRCLE {
            for x in 0..CIRCLE {
                let d = Vec2::new(x as f32 + 0.5 - r, y as f32 + 0.5 - r).length();
                let alpha = (r - d).clamp(0.0, 1.0); // one pixel of antialiasing
                rgba.extend_from_slice(&[255, 255, 255, (alpha * 255.0) as u8]);
            }
        }
        let t = self.make_texture(CIRCLE, CIRCLE, &rgba);
        self.circle = Some(t);
        t
    }

    #[allow(clippy::too_many_arguments)]
    fn shape(
        &mut self,
        layer: Layer,
        texture: u32,
        pos: Vec2,
        size: Vec2,
        rotation: f32,
        pivot: Vec2,
        color: Color,
        z: f32,
    ) {
        let instance = SpriteInstance::new(pos.into(), size.into(), pivot.into(), rotation, color.to_rgba8());
        match layer {
            Layer::World => self.draw_list.push(texture, z, instance),
            Layer::Screen => self.ui_list.push(texture, z, instance),
        }
    }

    /// Fills `rect` (world units) with `color`.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// ctx.gfx.draw_rect(Rect::new(0.0, 0.0, 200.0, 20.0), Color::hex(0x3a5a40));
    /// # }
    /// ```
    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        let white = self.white_texture().id;
        self.shape(Layer::World, white, rect.min, rect.size(), 0.0, Vec2::ZERO, color, 0.0);
    }

    /// A line from `a` to `b`, `width` world units thick.
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx, from: Vec2, to: Vec2) {
    /// ctx.gfx.draw_line(from, to, 2.0, Color::RED);
    /// # }
    /// ```
    pub fn draw_line(&mut self, a: Vec2, b: Vec2, width: f32, color: Color) {
        let white = self.white_texture().id;
        let d = b - a;
        self.shape(
            Layer::World,
            white,
            a,
            Vec2::new(d.length(), width),
            d.to_angle(),
            Vec2::new(0.0, 0.5),
            color,
            0.0,
        );
    }

    /// A filled circle (world units).
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx, tower: Vec2) {
    /// ctx.gfx.draw_circle(tower, 120.0, Color::BLUE.with_alpha(0.2)); // its range
    /// # }
    /// ```
    pub fn draw_circle(&mut self, center: Vec2, radius: f32, color: Color) {
        let circle = self.circle_texture().id;
        self.shape(
            Layer::World,
            circle,
            center,
            Vec2::splat(radius * 2.0),
            0.0,
            Vec2::splat(0.5),
            color,
            0.0,
        );
    }

    /// A filled screen-space rectangle (UI building block).
    pub(crate) fn ui_fill(&mut self, rect: Rect, color: Color, z: f32) {
        let white = self.white_texture().id;
        self.shape(Layer::Screen, white, rect.min, rect.size(), 0.0, Vec2::ZERO, color, z);
    }
}

#[allow(clippy::too_many_arguments)]
#[inline]
fn push_sprite(
    draw_list: &mut DrawList,
    texture: &Texture,
    pos: Vec2,
    scale: Vec2,
    rotation: f32,
    z: f32,
    pivot: Vec2,
    color: [u8; 4],
    region_px: Option<Rect>,
) {
    let (size, uv) = region(texture, region_px);
    draw_list.push(
        texture.id,
        z,
        SpriteInstance::new(pos.into(), (size * scale).into(), pivot.into(), rotation, color).with_uv(uv),
    );
}
