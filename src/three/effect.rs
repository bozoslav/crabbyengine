//! Screen effects: a game's WGSL run over the whole lit 3D frame, before
//! bloom and tone mapping (sun shafts, underwater, heat haze, outlines).

use super::Material;
use crate::Texture;
use crate::gfx::Graphics;

/// A screen effect shader, made with [`Graphics::load_effect`](crate::Graphics::load_effect) or
/// [`Graphics::create_effect`](crate::Graphics::create_effect); a cheap handle. Run it by adding a
/// [`ScreenEffect`] to [`Ctx::effects`](crate::Ctx::effects).
///
/// ```no_run
/// # use ponko::*;
/// # fn f(ctx: &mut Ctx) {
/// let underwater = ctx.gfx.load_effect("effects/underwater.wgsl");
/// ctx.effects.push(ScreenEffect::new(underwater));
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)] // shared with C/C++ (`CrabbyEffect`)
pub struct Effect {
    pub(crate) id: u32,
}

impl Effect {
    /// The engine's id of this effect (for the C API).
    #[doc(hidden)]
    #[must_use]
    pub fn id(self) -> u32 {
        self.id
    }

    /// An effect handle from an id (for the C API; check it with `Graphics::has_effect`).
    #[doc(hidden)]
    #[must_use]
    pub fn from_id(id: u32) -> Self {
        Self { id }
    }
}

/// One effect in [`Ctx::effects`](crate::Ctx::effects): its shader, and a
/// material whose custom values and textures the shader reads
/// (`material.custom[0]` to `[3]`, `custom_texture0`, ...).
///
/// ```no_run
/// # use ponko::*;
/// # fn f(ctx: &mut Ctx, tint: Effect) {
/// let strength = ctx.gfx.create_material(MaterialDesc { custom: [vec4(0.5, 0.0, 0.0, 0.0), Vec4::ZERO, Vec4::ZERO, Vec4::ZERO], ..Default::default() });
/// ctx.effects.push(ScreenEffect::new(tint).with_material(strength));
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenEffect {
    /// The shader.
    pub effect: Effect,
    /// Where its values and textures come from. `None`: the default
    /// material (all custom values 0).
    pub material: Option<Material>,
}

impl ScreenEffect {
    /// Runs `effect` with no values of its own.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx, e: Effect) {
    /// ctx.effects.push(ScreenEffect::new(e));
    /// # }
    /// ```
    #[must_use]
    pub fn new(effect: Effect) -> Self {
        Self { effect, material: None }
    }

    /// The same, reading its values and textures from `material`; change
    /// them with [`Graphics::update_material`](crate::Graphics::update_material) to animate the effect.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(e: Effect, m: Material) {
    /// let fx = ScreenEffect::new(e).with_material(m);
    /// # }
    /// ```
    #[must_use]
    pub fn with_material(self, material: Material) -> Self {
        Self {
            material: Some(material),
            ..self
        }
    }
}

/// One entry of [`Ctx::target_effects`](crate::Ctx::target_effects): an
/// effect shader run over the lit 3D frame that writes its result into a
/// render target of its own, leaving the frame unchanged.
///
/// It reads the frame as a [`ScreenEffect`] does (`p.color`, `p.world`,
/// `p.distance`, `p.sky`, `scene_color`, `scene_position`) and writes `effect(p)`
/// into each texel of `target`. A target of another size reads the frame at
/// the matching point (the nearest pixel, no filtering): a small target is a
/// cheap, coarse copy.
///
/// **When it runs.** Once per frame, after the opaque and transparent
/// surfaces and temporal antialiasing, and before [`Ctx::effects`](crate::Ctx::effects),
/// bloom and tone mapping. Target effects run in list order. So in one frame:
/// - screen effects (and later target effects) that sample `target` see
///   this frame's result;
/// - materials, target cameras and compute passes, which run earlier, see the
///   one from the frame before (all 0 at first).
///
/// **Resources.** `target` must be made by [`Graphics::create_render_target`](crate::Graphics::create_render_target);
/// other textures are skipped. Its whole contents are replaced each frame. The
/// game owns it and keeps it until it stops using it; the engine only writes
/// it. An entry whose material samples its own `target` can't run (one pass
/// can't read and write the same texture): it is skipped with a warning.
/// Without a 3D scene nothing runs.
///
/// ```no_run
/// # use ponko::*;
/// # fn f(ctx: &mut Ctx) {
/// // A quarter-size map of how far each pixel is, for a later effect to blur.
/// let far = ctx.gfx.create_effect("far", "fn effect(p: Pixel) -> vec3<f32> { return vec3<f32>(p.distance); }");
/// let depth = ctx.gfx.create_render_target(320, 180);
/// ctx.target_effects.push(TargetEffect::new(far, depth));
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetEffect {
    /// The shader, made like a screen effect's.
    pub effect: Effect,
    /// What it writes: made by [`Graphics::create_render_target`](crate::Graphics::create_render_target).
    pub target: Texture,
    /// Where its values and textures come from. `None`: the default
    /// material (all custom values 0).
    pub material: Option<Material>,
}

impl TargetEffect {
    /// Runs `effect` into `target` with no values of its own.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx, e: Effect, t: Texture) {
    /// ctx.target_effects.push(TargetEffect::new(e, t));
    /// # }
    /// ```
    #[must_use]
    pub fn new(effect: Effect, target: Texture) -> Self {
        Self {
            effect,
            target,
            material: None,
        }
    }

    /// The same, reading its values and textures from `material`.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(e: Effect, t: Texture, m: Material) {
    /// let pass = TargetEffect::new(e, t).with_material(m);
    /// # }
    /// ```
    #[must_use]
    pub fn with_material(self, material: Material) -> Self {
        Self {
            material: Some(material),
            ..self
        }
    }
}

impl Graphics {
    /// Loads a screen effect shader (WGSL) from an [added asset
    /// source](crate::Assets::add_source) or `assets/`. It defines
    /// `fn effect(p: Pixel) -> vec3<f32>`: the new light at each pixel of
    /// the 3D frame, from what's there (`p.color`, `p.world`, `p.distance`,
    /// `p.sky`, `p.screen`, `p.uv`). It can call `scene_color(screen)` and
    /// `scene_position(screen)` for other pixels, and read `frame` and
    /// `material` as material shaders do. With hot reload on, saving the
    /// file in `assets/` updates the game (an error keeps the old one). Loading the same
    /// file again returns the same effect.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let haze = ctx.gfx.load_effect("effects/haze.wgsl");
    /// ctx.effects.push(ScreenEffect::new(haze));
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// If the file can't be read or doesn't compile (the message says where).
    /// With hot reload on, a file that doesn't compile doesn't panic: it
    /// gets a stand-in and stays watched until a saved fix compiles
    /// ([`ShaderOutcome::Failed`](crate::ShaderOutcome::Failed), [`Graphics::shader_status`](crate::Graphics::shader_status)).
    pub fn load_effect(&mut self, path: &str) -> Effect {
        self.try_load_effect(path).unwrap_or_else(|e| panic!("ponko: {e}"))
    }

    /// [`Graphics::load_effect`](crate::Graphics::load_effect) that returns errors (for the C API).
    #[doc(hidden)]
    pub fn try_load_effect(&mut self, path: &str) -> Result<Effect, String> {
        let found = match self
            .asset_sources
            .lookup(path, |origin| self.assets3d.loaded_effects.get(origin).copied())?
        {
            crate::assets::Lookup::Cached(_, effect) => return Ok(effect),
            crate::assets::Lookup::Found(found) => found,
        };
        let origin = found.origin.clone();
        let source = found.into_text(path)?;
        let effect = match self.try_create_effect(path, &source) {
            Ok(effect) => effect,
            // With hot reload on, it shows the frame as it is until a saved
            // fix compiles; its status says what's wrong.
            Err(e) if self.keeps_failed_load(&origin) => {
                Self::report_stand_in(path, &e);
                let id = self.assets3d.effects;
                self.compile_effect(path, super::EFFECT_STAND_IN, None).map_err(|_| e)?;
                self.assets3d.effects += 1;
                self.assets3d.effects_live.insert(id);
                Effect { id }
            }
            Err(e) => return Err(e),
        };
        self.assets3d.loaded_effects.insert(origin.clone(), effect);
        if let crate::assets::Origin::File(file) = origin {
            self.watcher
                .watch(path, file, crate::hot_reload::Asset::Effect(effect.id));
        }
        Ok(effect)
    }

    /// Makes a screen effect shader from WGSL source; `name` is for error
    /// messages. See [`Graphics::load_effect`](crate::Graphics::load_effect) for what it defines.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let grey = ctx.gfx.create_effect("grey", "
    ///     fn effect(p: Pixel) -> vec3<f32> {
    ///         return vec3<f32>(dot(p.color, vec3<f32>(0.2126, 0.7152, 0.0722)));
    ///     }");
    /// ctx.effects.push(ScreenEffect::new(grey));
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// If it doesn't compile (the message says where).
    pub fn create_effect(&mut self, name: &str, source: &str) -> Effect {
        self.try_create_effect(name, source)
            .unwrap_or_else(|e| panic!("ponko: {e}"))
    }

    /// [`Graphics::create_effect`](crate::Graphics::create_effect) that returns errors (for the C API).
    #[doc(hidden)]
    pub fn try_create_effect(&mut self, name: &str, source: &str) -> Result<Effect, String> {
        let id = self.assets3d.effects;
        let compiled = self.compile_effect(name, source, None);
        self.record_shader(name, super::ShaderKind::Effect, false, compiled)?;
        self.assets3d.effects += 1;
        self.assets3d.effects_live.insert(id);
        Ok(Effect { id })
    }

    /// Checks effect `source` and builds its pipeline, as `replace` or as the next one.
    fn compile_effect(
        &mut self,
        name: &str,
        source: &str,
        replace: Option<u32>,
    ) -> Result<(), Box<super::ShaderDiagnostic>> {
        #[cfg(feature = "window")]
        crate::render::diagnose(
            name,
            super::ShaderKind::Effect,
            crate::render::EFFECT_VARIANT,
            &crate::render::compose_effect(source),
        )?;
        if let Some(renderer) = self.renderer.as_mut()
            && !self.headless
        {
            let built = match replace {
                Some(id) => renderer.replace_effect(id, name, source),
                None => renderer
                    .create_effect(name, source)
                    .map(|gpu_id| debug_assert_eq!(gpu_id, self.assets3d.effects)),
            };
            built?;
        }
        let _ = (name, source, replace);
        Ok(())
    }

    /// Recompiles an effect whose file changed (hot reload).
    pub(crate) fn reload_effect(&mut self, id: u32, name: &str, source: &str) -> Result<(), String> {
        if !self.assets3d.effects_live.contains(id) {
            return Ok(());
        }
        let compiled = self.compile_effect(name, source, Some(id));
        self.record_shader(name, super::ShaderKind::Effect, true, compiled)
    }

    /// Whether this game made `effect` and hasn't freed it (for the C API,
    /// which takes ids): [`Ctx::is_live`](crate::Ctx::is_live).
    #[doc(hidden)]
    #[must_use]
    pub fn has_effect(&self, effect: Effect) -> bool {
        self.assets3d.effects_live.contains(effect.id)
    }

    /// Frees screen effect `id` (see [`Ctx::free`](crate::Ctx::free)): entries of
    /// [`Ctx::effects`](crate::Ctx::effects) showing it are skipped from now on.
    pub(crate) fn free_effect_id(&mut self, id: u32) {
        if !self.assets3d.effects_live.remove(id) {
            return;
        }
        self.assets3d.loaded_effects.retain(|_, e| e.id != id);
        self.watcher
            .forget(|a| matches!(a, crate::hot_reload::Asset::Effect(e) if *e == id));
    }
}
