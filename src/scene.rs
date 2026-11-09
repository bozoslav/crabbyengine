//! Scenes and prefabs: worlds and entities kept in files under `assets/`.
//!
//! A scene is a whole world (the [`WorldReflect::save`] format): a level, a
//! menu, a test setup. A prefab is one entity's components, spawned as many
//! times as you like: a tree, an enemy, a building. Both are JSON, so they
//! diff well, and agents and the editor write them like any other file.

use serde_json::{Value, json};

use crate::{Ctx, Entity, Transform2D, Vec2, WorldReflect, assets};

/// Loads one asset a scene names (`Ctx::scene_asset_field`).
type SceneAssetLoader = std::rc::Rc<dyn Fn(&mut Ctx, &str) -> Result<(), String>>;

/// A field of a reflected component that names an asset, and how to load
/// it (`Ctx::scene_asset_field`).
pub(crate) struct SceneAssetField {
    component: String,
    field: String,
    load: SceneAssetLoader,
}

const PREFAB_FORMAT: &str = "crabby-prefab";
const PREFAB_VERSION: u64 = 1;

impl Ctx {
    /// Replaces the world with the scene in `assets/<path>` (written by
    /// [`save_scene`](Self::save_scene), the editor, or
    /// [`WorldReflect::save`]). Entities keep their ids, so references
    /// between them hold. Components must be [reflected](WorldReflect::reflect).
    /// The scene and the textures it names may also come from an
    /// [asset source](Ctx::add_asset_source).
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
    /// | Other resources | Kept as they were (scores you didn't reflect, tuning loaded with [`load_resource`](Self::load_resource), a baked navigation mesh). |
    /// | Components that aren't reflected | Gone with their entities; [`save_scene`](Self::save_scene) reports them under `"not_saved"`. |
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
        let (_, text) = self.read_asset_text(path)?;
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

    /// Saves the world (its reflected components and resources) as a scene
    /// in `assets/<path>`, over the file if it exists. Not in web builds.
    ///
    /// ```no_run
    /// # use ponko::*;
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
        if !self.prefabs.contains_key(path) {
            let (_, text) = self.read_asset_text(path)?;
            let prefab: Value = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
            let components = components_of(&prefab).map_err(|e| format!("{path}: {e}"))?;
            self.load_scene_assets(&components)
                .map_err(|e| format!("{path}: {e}"))?;
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
    /// # use ponko::*;
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

impl Ctx {
    /// Says that field `field` of reflected component `T` names an asset,
    /// and how to load it: before a scene or prefab is spawned
    /// ([`load_scene`](Self::load_scene), [`spawn_prefab`](Self::spawn_prefab),
    /// the editor and the control endpoint), `load` is called with every
    /// name found there, the way the textures of `Sprite`s and meshes of
    /// `Model`s are loaded first.
    ///
    /// The component keeps the asset's name (a `String`, or a list of
    /// them), not a handle: scenes save it as written, and a system gets
    /// the handle by loading the name again, which returns the cached one.
    /// `load` decides what the name means, so it covers your own asset
    /// types too ([`load_asset_with`](Self::load_asset_with) into a
    /// resource). Names are found through the [asset
    /// sources](Self::add_asset_source) like any other load.
    ///
    /// The field may hold a name or a list of names; other values are
    /// skipped (the component's own deserialization reports them). A
    /// failing `load` fails the scene load before any entity changes, with
    /// the component, field and name in the message. Calling this again
    /// for the same component and field replaces its loader.
    ///
    /// ```
    /// # use ponko::*;
    /// /// A shop item: its icon is an image, saved by name.
    /// #[derive(Clone, serde::Serialize, serde::Deserialize)]
    /// struct ShopItem {
    ///     icon: String,
    /// }
    ///
    /// # let mut red = Vec::new();
    /// # image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]))
    /// #     .write_to(&mut std::io::Cursor::new(&mut red), image::ImageFormat::Png)
    /// #     .unwrap();
    /// Game::new("t", 1, 1).run_headless(0, |ctx| {
    ///     ctx.world.reflect::<ShopItem>();
    ///     ctx.scene_asset_field::<ShopItem>("icon", |ctx, name| {
    ///         ctx.try_load_texture(name).map(|_| ()).map_err(|e| e.to_string())
    ///     });
    ///     // A scene with one shop item, kept in memory with its icon.
    ///     ctx.world.spawn((ShopItem { icon: "icons/apple.png".into() },));
    ///     let shop = serde_json::to_string(&ctx.world.save()).unwrap();
    ///     ctx.add_asset_source(MemorySource::new("shop").with("shop.json", shop).with("icons/apple.png", red));
    ///
    ///     ctx.load_scene("shop.json").unwrap(); // loads "icons/apple.png" first
    ///     let item = ctx.world.query::<&ShopItem>().iter().next().unwrap().clone();
    ///     let icon = ctx.load_texture(&item.icon); // the cached texture
    ///     assert_eq!(icon.width(), 1);
    /// });
    /// ```
    pub fn scene_asset_field<T: 'static>(
        &mut self,
        field: &str,
        load: impl Fn(&mut Ctx, &str) -> Result<(), String> + 'static,
    ) {
        let component = crate::reflect::short_name(std::any::type_name::<T>());
        let entry = SceneAssetField {
            component,
            field: field.to_string(),
            load: std::rc::Rc::new(load),
        };
        match self
            .scene_asset_fields
            .iter_mut()
            .find(|f| f.component == entry.component && f.field == entry.field)
        {
            Some(existing) => *existing = entry,
            None => self.scene_asset_fields.push(entry),
        }
    }

    /// Loads the assets that fields registered with
    /// [`scene_asset_field`](Self::scene_asset_field) name anywhere in `json`.
    pub(crate) fn load_scene_asset_fields(&mut self, json: &Value) -> Result<(), String> {
        for i in 0..self.scene_asset_fields.len() {
            let f = &self.scene_asset_fields[i];
            let (component, field, load) = (f.component.clone(), f.field.clone(), f.load.clone());
            let mut names = Vec::new();
            field_names(json, &component, &field, &mut names);
            names.sort_unstable();
            names.dedup();
            for name in names {
                load(self, &name).map_err(|e| format!("{component}.{field} \"{name}\": {e}"))?;
            }
        }
        Ok(())
    }
}

/// The names (a string, or the strings of a list) under `component`
/// objects' `field` anywhere in `json`.
fn field_names(json: &Value, component: &str, field: &str, out: &mut Vec<String>) {
    match json {
        Value::Object(map) => {
            for (k, value) in map {
                if k == component && value.is_object() {
                    match value.get(field) {
                        Some(Value::String(name)) => out.push(name.clone()),
                        Some(Value::Array(list)) => out.extend(list.iter().filter_map(Value::as_str).map(String::from)),
                        _ => {}
                    }
                } else {
                    field_names(value, component, field, out);
                }
            }
        }
        Value::Array(list) => list.iter().for_each(|v| field_names(v, component, field, out)),
        _ => {}
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
