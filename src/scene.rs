//! Scenes and prefabs: worlds and entities kept in files under `assets/`.
//!
//! A scene is a whole world (the [`WorldReflect::save`] format): a level, a
//! menu, a test setup. A prefab is one entity's components, spawned as many
//! times as you like: a tree, an enemy, a building. Both are JSON, so they
//! diff well, and agents and the editor write them like any other file.

use serde_json::{Value, json};

use crate::{Ctx, Entity, Transform2D, Vec2, WorldReflect};

pub(crate) const PREFAB_FORMAT: &str = "crabby-prefab";
pub(crate) const PREFAB_VERSION: u64 = 1;

impl Ctx {
    /// Replaces the world with the scene in `assets/<path>` (written by
    /// [`save_scene`](crate::Assets::save_scene), the editor, or
    /// [`WorldReflect::save`]). Entities keep their ids, so references
    /// between them hold. Components must be [reflected](WorldReflect::reflect).
    /// The scene and the textures it names may also come from an
    /// [asset source](crate::Assets::add_source).
    ///
    /// ```no_run
    /// # use ponko::*;
    /// Game::new("Crabs", 800, 600).run(|ctx| {
    ///     ctx.world.reflect::<Transform2D>();
    ///     ctx.load_scene("levels/beach.json").expect("level");
    /// });
    /// ```
    ///
    /// # What a load replaces and what it keeps
    ///
    /// A scene is a world snapshot, not a whole application save: it holds
    /// entities with reflected components and reflected resources. The
    /// game's services live outside the world; a load treats them like this:
    ///
    /// | State | After `load_scene` |
    /// |---|---|
    /// | Entities | Replaced by the scene's, with the scene's ids. An old [`Entity`] handle may now name a different entity of the new scene. |
    /// | Reflected resources | Those in the scene are written; reflected ones it doesn't name stay. |
    /// | Other resources | Kept as they were (scores you didn't reflect, tuning loaded with [`load_resource`](crate::Assets::load_resource), a baked navigation mesh). |
    /// | Components that aren't reflected | Gone with their entities; [`save_scene`](crate::Assets::save_scene) reports them under `"not_saved"`. |
    /// | [`commands`](Self::commands) | Commands queued earlier in the running system are dropped: they were meant for the old entities. Commands queued after the load apply as usual. |
    /// | [`events`](Self::events) | Kept: events sent before the load can still be read for their usual lifetime. Entity ids inside them (such as [`CollisionStarted`](crate::CollisionStarted)) refer to the old world. |
    /// | 2D and 3D physics | Bodies, characters and joints are removed at once (queries find nothing), then built from the new components at the next physics step, from rest unless the scene has a velocity component. Gravity, collision meshes, contact hooks and settings are kept. |
    /// | [`spatial`](Self::spatial) neighbor grid | Refilled at the start of the next simulation tick. |
    /// | Assets | Textures, meshes and collision files the scene names are loaded first and stay cached. Loaded assets are never unloaded by a load. |
    /// | Positional sounds | A sound playing from an entity stops at the next audio update unless that entity still has the same playing source; other sounds and music keep playing. |
    /// | Retained UI, camera, lighting, input, time, random seed | Kept: they belong to the game, not the scene. |
    /// | Lockstep networking ([`net`](Self::net)) | Not synchronized: a load changes only this peer. In a session, every peer must make the same load from a simulation system at the same tick. |
    ///
    /// The physics rule also holds for [`WorldReflect::load`] called
    /// directly on `ctx.world`, except that bodies are removed at the next
    /// physics step instead of at once.
    ///
    /// # Errors
    ///
    /// If the file is missing or isn't a scene, names a component or
    /// resource that isn't reflected, or names an asset that can't be
    /// loaded. The world, commands and physics are unchanged then; assets
    /// the scene named that did load before the failure stay cached.
    pub fn load_scene(&mut self, path: &str) -> Result<(), String> {
        let (_, text) = self.assets.read_text(path)?;
        let scene: Value = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
        self.replace_world(&scene).map_err(|e| format!("{path}: {e}"))
    }

    /// Replaces the world with `save` (a scene or world save), as
    /// [`load_scene`](Self::load_scene) describes: its assets first, then
    /// the entities, then the services that follow entity ids.
    pub(crate) fn replace_world(&mut self, save: &Value) -> Result<(), String> {
        self.load_scene_assets(save)?;
        self.world.load(save).map_err(|e| e.to_string())?;
        self.commands = crate::Commands::new();
        #[cfg(feature = "physics")]
        self.physics.forget_bodies();
        #[cfg(feature = "physics3d")]
        self.physics3d.forget_bodies();
        Ok(())
    }

    /// Spawns the prefab in `assets/<path>` with its position at `at` (its
    /// rotation and scale as saved). The file is read once, then kept.
    ///
    /// ```no_run
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// for x in 0..10 {
    ///     ctx.spawn_prefab("prefabs/palm.json", vec2(x as f32 * 64.0, 400.0)).expect("palm");
    /// }
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// If the file is missing or isn't a prefab, or a component isn't
    /// reflected or doesn't fit; nothing is spawned then.
    pub fn spawn_prefab(&mut self, path: &str, at: Vec2) -> Result<Entity, String> {
        if !self.assets.prefabs.contains_key(path) {
            let (_, text) = self.assets.read_text(path)?;
            let prefab: Value = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
            let components = components_of(&prefab).map_err(|e| format!("{path}: {e}"))?;
            self.load_scene_assets(&components)
                .map_err(|e| format!("{path}: {e}"))?;
            self.assets.prefabs.insert(path.to_string(), components);
        }
        let components = self.assets.prefabs[path].clone();
        spawn_at(&mut self.world, components, at).map_err(|e| format!("{path}: {e}"))
    }
}

/// A prefab file's components, checked.
fn components_of(prefab: &Value) -> Result<Value, String> {
    if prefab["format"] != PREFAB_FORMAT {
        return Err(format!("not a prefab (expected \"format\": \"{PREFAB_FORMAT}\")"));
    }
    if prefab["version"].as_u64() != Some(PREFAB_VERSION) {
        return Err(format!(
            "unsupported prefab version {} (this engine reads {PREFAB_VERSION})",
            prefab["version"]
        ));
    }
    match &prefab["components"] {
        Value::Object(c) if !c.is_empty() => Ok(prefab["components"].clone()),
        _ => Err("a prefab needs a non-empty \"components\" object".into()),
    }
}

/// Spawns `components` with the position set to `at` (adding a transform if
/// there's none).
fn spawn_at(world: &mut crate::World, mut components: Value, at: Vec2) -> Result<Entity, String> {
    match components.get_mut("Transform2D") {
        Some(t) => t["position"] = json!([at.x, at.y]),
        None => components["Transform2D"] = serde_json::to_value(Transform2D::at(at)).map_err(|e| e.to_string())?,
    }
    world.spawn_json(components).map_err(|e| e.to_string())
}
