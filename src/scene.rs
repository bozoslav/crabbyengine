//! Scenes and prefabs: worlds and entities kept in files under `assets/`.
//!
//! A scene is a whole world (the [`WorldReflect::save`] format): a level, a
//! menu, a test setup. A prefab is one entity's components, spawned as many
//! times as you like: a tree, an enemy, a building. Both are JSON, so they
//! diff well, and agents and the editor write them like any other file.

use serde_json::{Value, json};

use crate::{Ctx, Entity, Transform2D, Vec2, WorldReflect, assets};

const PREFAB_FORMAT: &str = "crabby-prefab";
const PREFAB_VERSION: u64 = 1;

impl Ctx {
    /// Replaces the world with the scene in `assets/<path>` (written by
    /// [`save_scene`](Self::save_scene), the editor, or
    /// [`WorldReflect::save`]). Entities keep their ids, so references
    /// between them hold. Components must be [reflected](WorldReflect::reflect).
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// Game::new("Crabs", 800, 600).run(|ctx| {
    ///     ctx.world.reflect::<Transform2D>();
    ///     ctx.load_scene("levels/beach.json").expect("level");
    /// });
    /// ```
    ///
    /// # Errors
    ///
    /// If the file is missing or isn't a scene, or names a component that
    /// isn't reflected. The world is unchanged then.
    pub fn load_scene(&mut self, path: &str) -> Result<(), String> {
        let file = assets::resolve(path)?;
        let text = assets::read_to_string(&file)?;
        let scene: Value = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
        self.world.load(&scene).map_err(|e| format!("{path}: {e}"))
    }

    /// Saves the world (its reflected components and resources) as a scene
    /// in `assets/<path>`, over the file if it exists. Not in web builds.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx) {
    /// ctx.save_scene("levels/beach.json").expect("saved");
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// If the file can't be written.
    pub fn save_scene(&mut self, path: &str) -> Result<(), String> {
        let text = serde_json::to_string_pretty(&self.world.save()).map_err(|e| e.to_string())?;
        assets::write(path, text.as_bytes()).map(|_| ())
    }

    /// Spawns the prefab in `assets/<path>` with its position at `at` (its
    /// rotation and scale as saved). The file is read once, then kept.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
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
        if !self.prefabs.contains_key(path) {
            let file = assets::resolve(path)?;
            let text = assets::read_to_string(&file)?;
            let prefab: Value = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
            let components = components_of(&prefab).map_err(|e| format!("{path}: {e}"))?;
            self.prefabs.insert(path.to_string(), components);
        }
        let components = self.prefabs[path].clone();
        spawn_at(&mut self.world, components, at).map_err(|e| format!("{path}: {e}"))
    }

    /// Saves entity `e` (its reflected components) as a prefab in
    /// `assets/<path>`, for [`spawn_prefab`](Self::spawn_prefab). Its
    /// position isn't kept: each spawn says where. Not in web builds.
    ///
    /// ```no_run
    /// # use crabbyengine::*;
    /// # fn f(ctx: &mut Ctx, palm: Entity) {
    /// ctx.save_prefab(palm, "prefabs/palm.json").expect("saved");
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// If `e` is dead or the file can't be written.
    pub fn save_prefab(&mut self, e: Entity, path: &str) -> Result<(), String> {
        let mut components = self.world.entity_json(e).map_err(|e| e.to_string())?;
        if let Some(t) = components.get_mut("Transform2D") {
            t["position"] = json!([0.0, 0.0]);
        }
        let prefab = json!({ "format": PREFAB_FORMAT, "version": PREFAB_VERSION, "components": components });
        let text = serde_json::to_string_pretty(&prefab).map_err(|e| e.to_string())?;
        assets::write(path, text.as_bytes())?;
        self.prefabs.insert(path.to_string(), components);
        Ok(())
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
