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
    /// | 3D navigation (`NavMesh3D`, with the `navmesh` feature) | The baked mesh, a resource, is kept. Crowd agents are components, so they go with their entities; `step_navmesh3d` frees the old agents' crowd slots at its next step, and agents added to the new scene's entities (even with the same ids) start from where they are. |
    /// | [`Attached`](crate::Attached) | Reflected: the scene's attachments, to the scene's entities. [`follow_attachments`](crate::follow_attachments) places them at its next run and despawns those whose entity isn't in the scene. |
    /// | Skeletons, joint attachments, skinned and morphing models | Components: gone with their entities unless the game adds them again. The engine's skinning scratch is rebuilt every frame from the world as it is. |
    /// | Assets | Textures, meshes and collision files the scene names are loaded first and stay cached. Loaded assets are never unloaded by a load. |
    /// | Prefabs | Kept: files [`spawn_prefab`](Self::spawn_prefab) read stay cached, so spawning one after a load doesn't read it again. |
    /// | Positional sounds | A sound playing from an entity stops at the next audio update unless that entity still has the same playing source; other sounds and music keep playing. |
    /// | Retained UI, camera, lighting, input, time, random seed | Kept: they belong to the game, not the scene. |
    /// | 3D UI panels (`UiPanel3D`, with the `3d` feature) | Components: gone with their entities. Which panel had the keyboard is forgotten, so a new panel with the old one's id doesn't get it without a click. |
    /// | Temporal history (TAA, temporal AO, reflections) | Reset, as by `reset_temporal_history` (`3d` feature): the new scene's models have no motion from the old one's. |
    /// | Lockstep networking ([`net`](Self::net)) | Not synchronized: a load changes only this peer. In a session, load from a simulation system (every peer runs it at the same tick) or in setup (every peer runs it before the first tick); [`load_scene_lockstep`](Self::load_scene_lockstep) is the same call, named for that. Loads from frame, late and draw systems, and tools' loads (the editor's Stop, the control endpoint), are refused while a session is connecting or running. After a session is lost, loads are local again. |
    /// | Tools (the inspector) | Its selection is cleared: the id may name another entity now. |
    ///
    /// The physics rule also holds for [`WorldReflect::load`] called
    /// directly on `ctx.world`, except that bodies are removed at the next
    /// physics step instead of at once.
    ///
    /// # Errors
    ///
    /// If the file is missing or isn't a scene, names a component or
    /// resource that isn't reflected, or names an asset that can't be
    /// loaded, or when a lockstep session runs and the load would change
    /// only this peer (see above). The world,
    /// commands, physics and every other service are unchanged then; assets
    /// the scene named that did load before the failure stay cached.
    pub fn load_scene(&mut self, path: &str) -> Result<(), String> {
        self.check_session_load().map_err(|e| format!("{path}: {e}"))?;
        self.read_and_replace(path)
    }

    /// [`load_scene`](Self::load_scene), named for a lockstep game: the
    /// same load on every peer at the same tick. It follows the same rules
    /// (a simulation system or setup in a session, anywhere offline); the
    /// name says at the call site that the load is part of the shared
    /// simulation. The peers' worlds are still compared every
    /// [`checksum_every`](crate::Net::checksum_every) ticks, so a load that
    /// only some peers made shows up as a [`Desync`](crate::Desync).
    ///
    /// ```
    /// # use ponko::*;
    /// # fn next_level(ctx: &mut Ctx) {
    /// // A simulation system: every peer reaches tick 600 with the same world.
    /// if ctx.sim_tick() == 600 {
    ///     ctx.load_scene_lockstep("levels/two.json").expect("level");
    /// }
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// As `load_scene`.
    pub fn load_scene_lockstep(&mut self, path: &str) -> Result<(), String> {
        self.load_scene(path)
    }

    fn read_and_replace(&mut self, path: &str) -> Result<(), String> {
        let (_, text) = self.read_asset_text(path)?;
        let scene: Value = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
        self.replace_world_now(&scene).map_err(|e| format!("{path}: {e}"))
    }

    /// Refuses a game's load that would change only this peer: outside a
    /// simulation system and outside setup while a lockstep session is
    /// connecting or running. Every peer runs its setup before the first
    /// tick and its simulation systems at the same ticks; frame, late and
    /// draw systems run on each peer's own clock.
    fn check_session_load(&self) -> Result<(), String> {
        if !self.session_active() || self.in_sim() || self.in_setup() {
            return Ok(());
        }
        Err(
            "a lockstep session is running and a load outside a simulation system or setup \
             would change only this peer: load from a simulation system, which every peer runs at the same tick"
                .into(),
        )
    }

    fn session_active(&self) -> bool {
        matches!(self.net.state(), crate::NetState::Connecting | crate::NetState::Running)
    }

    /// Replaces the world with `save` (a scene or world save) for tools (the
    /// editor's Stop, the control endpoint): refused while a lockstep
    /// session is connecting or running, since a tool acts on this peer only.
    pub(crate) fn replace_world(&mut self, save: &Value) -> Result<(), String> {
        if self.session_active() {
            return Err("a lockstep session is running and a tool's load would change only this peer".into());
        }
        self.replace_world_now(save)
    }

    /// Replaces the world with `save`, as [`load_scene`](Self::load_scene)
    /// describes: its assets first, then the entities, then the services
    /// that follow entity ids.
    fn replace_world_now(&mut self, save: &Value) -> Result<(), String> {
        self.load_scene_assets(save)?;
        self.world.load(save).map_err(|e| e.to_string())?;
        self.commands = crate::Commands::new();
        self.scene_loads += 1;
        #[cfg(feature = "physics")]
        self.physics.forget_bodies();
        #[cfg(feature = "physics3d")]
        self.physics3d.forget_bodies();
        #[cfg(feature = "3d")]
        {
            // Keyboard focus named an entity of the old world: the same id
            // may be another panel now, which nobody clicked.
            self.ui.panels.forget_focus();
            // Motion history is keyed by entity id: a same-id model of the
            // new scene has no past in the old one.
            self.reset_temporal_history();
        }
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

/// Loads during a lockstep session: refused where only one peer would load,
/// taken where every peer does (setup, simulation systems).
#[cfg(test)]
mod lockstep_tests {
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use crate::{Ctx, Desync, Game, Net, NetState, World, WorldReflect};

    #[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
    struct Tag(u32);

    /// A scene in `assets/` (for `load_scene`) and the same world in a plain
    /// file (for the control endpoint's `world.load`), deleted afterwards.
    struct Files {
        asset: &'static str,
        plain: std::path::PathBuf,
    }
    impl Files {
        fn new(asset: &'static str) -> Self {
            let mut world = World::new();
            world.reflect::<Tag>();
            world.spawn((Tag(7),));
            let text = serde_json::to_string(&world.save()).unwrap();
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
            std::fs::write(root.join("assets").join(asset), &text).unwrap();
            let plain = std::env::temp_dir().join(format!("ponko-{}-{asset}", std::process::id()));
            std::fs::write(&plain, &text).unwrap();
            Self { asset, plain }
        }
    }
    impl Drop for Files {
        fn drop(&mut self) {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
            let _ = std::fs::remove_file(root.join("assets").join(self.asset));
            let _ = std::fs::remove_file(&self.plain);
        }
    }

    fn tags(ctx: &mut Ctx) -> Vec<u32> {
        let mut t: Vec<u32> = ctx.world.query::<&Tag>().iter().map(|t| t.0).collect();
        t.sort_unstable();
        t
    }

    /// Every way to load that only this peer would make: each must fail and
    /// leave the world as it was.
    fn assert_every_local_load_is_refused(ctx: &mut Ctx, files: &Files) {
        let before = WorldReflect::save(&ctx.world);
        let plain = files.plain.to_str().unwrap();
        let errors = [
            ctx.load_scene(files.asset).unwrap_err(),
            ctx.load_scene_lockstep(files.asset).unwrap_err(),
            crate::control::handle(ctx, "scene.load", &json!({ "path": files.asset })).unwrap_err(),
            crate::control::handle(ctx, "world.load", &json!({ "path": plain })).unwrap_err(),
        ];
        for e in &errors {
            assert!(e.contains("lockstep session"), "{e}");
        }
        // The editor's Stop (with the inspector).
        #[cfg(feature = "inspector")]
        {
            let mut editor = crate::editor::Editor::default();
            editor.play(ctx);
            let extra = ctx.world.spawn((Tag(100),));
            assert!(editor.stop(ctx).unwrap_err().contains("lockstep session"));
            assert!(editor.playing(), "Stop can be retried");
            ctx.resume();
            ctx.world.despawn(extra);
        }
        assert_eq!(WorldReflect::save(&ctx.world), before, "nothing changed");
    }

    /// Runs two loopback peers for `ticks`; returns each one's tags and desync.
    fn two_peers(
        ticks: u64,
        setup: fn(&mut Ctx, &Files),
        tick: fn(&mut Ctx, &Files),
        files: &std::sync::Arc<Files>,
    ) -> Vec<(Vec<u32>, Option<Desync>)> {
        let runs: Vec<_> = Net::loopback(2)
            .into_iter()
            .map(|net| {
                let (f1, f2) = (files.clone(), files.clone());
                std::thread::spawn(move || {
                    let mut world = Game::new("t", 1, 1)
                        .lockstep(net.checksum_every(5))
                        .sim_system(move |ctx: &mut Ctx| {
                            ctx.net.set_input(&(ctx.sim_tick() as u32));
                            tick(ctx, &f1);
                            if ctx.sim_tick() + 1 == ticks {
                                let desync = ctx.net.desync().cloned();
                                ctx.world.insert_resource(Some(desync));
                            }
                        })
                        .run_headless(ticks, move |ctx| {
                            ctx.world.reflect::<Tag>();
                            ctx.world.spawn((Tag(1),));
                            setup(ctx, &f2);
                        });
                    let desync = world
                        .remove_resource::<Option<Option<Desync>>>()
                        .expect("every tick ran");
                    let mut t: Vec<u32> = world.query::<&Tag>().iter().map(|t| t.0).collect();
                    t.sort_unstable();
                    (t, desync.flatten())
                })
            })
            .collect();
        runs.into_iter().map(|r| r.join().unwrap()).collect()
    }

    #[test]
    fn loads_outside_simulation_systems_are_refused_while_a_session_runs() {
        let files = std::sync::Arc::new(Files::new("test-scene-lockstep-running.json"));
        let runs = two_peers(
            20,
            |_, _| {},
            |ctx, files| {
                if ctx.sim_tick() == 8 {
                    assert_eq!(ctx.net.state(), NetState::Running);
                    // As in a frame, late or draw system, or a tool between ticks.
                    ctx.set_in_sim(false);
                    assert_every_local_load_is_refused(ctx, files);
                    ctx.set_in_sim(true);
                    assert_eq!(tags(ctx), [1]);
                    ctx.load_scene(files.asset).unwrap();
                }
            },
            &files,
        );
        for (tags, desync) in runs {
            assert_eq!(desync, None);
            assert_eq!(tags, [7], "the scene, on both peers");
        }
    }

    #[test]
    fn a_frame_systems_load_while_connecting_is_refused() {
        let files = std::sync::Arc::new(Files::new("test-scene-lockstep-connecting.json"));
        let runs = two_peers(
            10,
            |ctx, files| {
                assert_eq!(ctx.net.state(), NetState::Connecting);
                // As in a frame system while the peers are still connecting.
                ctx.set_in_setup(false);
                assert_every_local_load_is_refused(ctx, files);
                ctx.set_in_setup(true);
            },
            |_, _| {},
            &files,
        );
        for (tags, desync) in runs {
            assert_eq!(desync, None);
            assert_eq!(tags, [1], "nothing loaded");
        }
    }

    #[test]
    fn setup_and_simulation_loads_still_work_on_every_peer() {
        let files = std::sync::Arc::new(Files::new("test-scene-lockstep-works.json"));
        let runs = two_peers(
            20,
            |ctx, files| {
                assert_eq!(ctx.net.state(), NetState::Connecting);
                ctx.load_scene(files.asset).unwrap();
                assert_eq!(tags(ctx), [7]);
                ctx.world.query::<&mut Tag>().for_each(|t| t.0 = 2);
            },
            |ctx, files| {
                if ctx.sim_tick() == 8 {
                    assert_eq!(tags(ctx), [2]);
                    ctx.load_scene(files.asset).unwrap();
                }
            },
            &files,
        );
        for (tags, desync) in runs {
            assert_eq!(desync, None, "both peers loaded at the same tick");
            assert_eq!(tags, [7]);
        }
    }
}
