//! Screen effects: a game's WGSL run over the whole lit 3D frame, before
//! bloom and tone mapping (sun shafts, underwater, heat haze, outlines).

use super::Material;
use crate::Ctx;

/// A screen effect shader, made with [`Ctx::load_effect`] or
/// [`Ctx::create_effect`]; a cheap handle. Run it by adding a
/// [`ScreenEffect`] to [`Ctx::effects`](crate::Ctx::effects).
///
/// ```no_run
/// # use ponko::*;
/// # fn f(ctx: &mut Ctx) {
/// let underwater = ctx.load_effect("effects/underwater.wgsl");
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

    /// An effect handle from an id (for the C API; check it with `Ctx::has_effect`).
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
/// let strength = ctx.create_material(MaterialDesc { custom: [vec4(0.5, 0.0, 0.0, 0.0), Vec4::ZERO, Vec4::ZERO, Vec4::ZERO], ..Default::default() });
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
    /// them with [`Ctx::update_material`] to animate the effect.
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

impl Ctx {
    /// Loads a screen effect shader (WGSL) from `assets/`. It defines
    /// `fn effect(p: Pixel) -> vec3<f32>`: the new light at each pixel of
    /// the 3D frame, from what's there (`p.color`, `p.world`, `p.distance`,
    /// `p.sky`, `p.screen`, `p.uv`). It can call `scene_color(screen)` and
    /// `scene_position(screen)` for other pixels, and read `frame` and
    /// `material` as material shaders do. With hot reload on, saving the
    /// file updates the game (an error keeps the old one). Loading the same
    /// file again returns the same effect.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let haze = ctx.load_effect("effects/haze.wgsl");
    /// ctx.effects.push(ScreenEffect::new(haze));
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// If the file can't be read or doesn't compile (the message says where).
    pub fn load_effect(&mut self, path: &str) -> Effect {
        self.try_load_effect(path).unwrap_or_else(|e| panic!("ponko: {e}"))
    }

    /// [`Ctx::load_effect`] that returns errors (for the C API).
    #[doc(hidden)]
    pub fn try_load_effect(&mut self, path: &str) -> Result<Effect, String> {
        let file = crate::assets::resolve(path)?;
        if let Some(&effect) = self.assets3d.loaded_effects.get(&file) {
            return Ok(effect);
        }
        let source = crate::assets::read_to_string(&file)?;
        let effect = self.try_create_effect(path, &source)?;
        self.assets3d.loaded_effects.insert(file.clone(), effect);
        self.watcher
            .watch(path, file, crate::hot_reload::Asset::Effect(effect.id));
        Ok(effect)
    }

    /// Makes a screen effect shader from WGSL source; `name` is for error
    /// messages. See [`Ctx::load_effect`] for what it defines.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// let grey = ctx.create_effect("grey", "
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

    /// [`Ctx::create_effect`] that returns errors (for the C API).
    #[doc(hidden)]
    pub fn try_create_effect(&mut self, name: &str, source: &str) -> Result<Effect, String> {
        #[cfg(feature = "window")]
        crate::render::check_shader(name, &crate::render::compose_effect(source))?;
        let id = self.assets3d.effects;
        if let Some(renderer) = self.renderer.as_mut()
            && !self.headless
        {
            let gpu_id = renderer.create_effect(name, source)?;
            debug_assert_eq!(gpu_id, id);
        }
        self.assets3d.effects += 1;
        self.assets3d.effects_live.insert(id);
        Ok(Effect { id })
    }

    /// Recompiles an effect whose file changed (hot reload).
    pub(crate) fn reload_effect(&mut self, id: u32, name: &str, source: &str) -> Result<(), String> {
        if !self.assets3d.effects_live.contains(id) {
            return Ok(());
        }
        #[cfg(feature = "window")]
        crate::render::check_shader(name, &crate::render::compose_effect(source))?;
        if let Some(renderer) = self.renderer.as_mut()
            && !self.headless
        {
            renderer.replace_effect(id, name, source)?;
        }
        Ok(())
    }

    /// Whether this game made `effect` and hasn't freed it (for the C API,
    /// which takes ids): [`Ctx::is_live`].
    #[doc(hidden)]
    #[must_use]
    pub fn has_effect(&self, effect: Effect) -> bool {
        self.is_live(effect)
    }

    /// Frees screen effect `id` (see [`Ctx::free`]): entries of
    /// [`Ctx::effects`] showing it are skipped from now on.
    pub(crate) fn free_effect_id(&mut self, id: u32) {
        if !self.assets3d.effects_live.remove(id) {
            return;
        }
        self.assets3d.loaded_effects.retain(|_, e| e.id != id);
        self.watcher
            .forget(|a| matches!(a, crate::hot_reload::Asset::Effect(e) if *e == id));
    }
}
