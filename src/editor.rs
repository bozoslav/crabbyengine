//! The editor: the inspector's changes to the world, with undo, and Play /
//! Stop (run the game, then put the world back as it was).
//!
//! Kept apart from the egui panels ([`crate::inspector`]) so every edit can
//! be tested without a window. Edits work on reflected components only:
//! they're what the editor can read, write and put back.

use ponko_ecs::Entity;
use serde_json::{Value, json};

use crate::{Ctx, Vec2, WorldReflect};

/// Undo steps kept; older ones are dropped.
const MAX_UNDO: usize = 200;

/// One change, as what puts it back.
#[derive(Debug)]
enum Undo {
    /// Component `name` of `e` was `before` (`None`: it didn't have it).
    Component {
        e: Entity,
        name: String,
        before: Option<Value>,
    },
    /// `e` was spawned: despawn it.
    Spawned(Entity),
    /// `e` was despawned with these (reflected) components: spawn them
    /// again. Handles never come back, so it's a new entity, and the steps
    /// before this one are pointed at it.
    Despawned { e: Entity, components: Value },
}

/// Edits with undo, and the world saved by Play for Stop.
#[derive(Default)]
pub(crate) struct Editor {
    undo: Vec<Undo>,
    /// The world as it was when Play was pressed, and how many undo steps
    /// there were then (steps taken while playing are dropped by Stop).
    played_from: Option<(Value, usize)>,
}

impl Editor {
    /// Whether Play was pressed and Stop wasn't yet.
    pub fn playing(&self) -> bool {
        self.played_from.is_some()
    }

    /// Runs the game, first saving the world for [`stop`](Self::stop).
    /// Pressed again while playing (after a pause), it only resumes.
    pub fn play(&mut self, ctx: &mut Ctx) {
        if self.played_from.is_none() {
            self.played_from = Some((ctx.world.save(), self.undo.len()));
        }
        ctx.time.resume();
    }

    /// Pauses and puts the world back as it was when Play was pressed.
    /// Components that aren't reflected can't be saved, so aren't put back.
    /// If the world can't be put back (a lockstep session is running), the
    /// Play snapshot and the undo history are kept and the simulation keeps
    /// running, so Stop can be retried.
    pub fn stop(&mut self, ctx: &mut Ctx) -> Result<(), String> {
        let Some((save, steps)) = &self.played_from else {
            ctx.time.pause();
            return Ok(());
        };
        let steps = *steps;
        // Pause only once the world is back: a refused Stop changes nothing.
        ctx.replace_world(save)?;
        ctx.time.pause();
        self.played_from = None;
        self.undo.truncate(steps);
        Ok(())
    }

    /// Keeps the world as it is now: Stop won't go back.
    pub fn keep(&mut self) {
        self.played_from = None;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    fn record(&mut self, step: Undo) {
        if self.undo.len() == MAX_UNDO {
            self.undo.remove(0);
        }
        self.undo.push(step);
    }

    /// Writes component `name` of `e` (adding it if missing), with undo.
    pub fn set(&mut self, ctx: &mut Ctx, e: Entity, name: &str, value: Value) -> Result<(), String> {
        let before = ctx.world.get_json(e, name).ok();
        ctx.load_scene_assets(&json!({ name: &value }))?;
        ctx.world.set_json(e, name, value).map_err(|e| e.to_string())?;
        self.record(Undo::Component {
            e,
            name: name.to_string(),
            before,
        });
        Ok(())
    }

    /// Adds component `name` to `e` with its default fields (types whose
    /// fields all have defaults; others say which field is missing).
    pub fn add(&mut self, ctx: &mut Ctx, e: Entity, name: &str) -> Result<(), String> {
        self.set(ctx, e, name, json!({}))
    }

    /// Removes component `name` from `e`, with undo.
    pub fn remove(&mut self, ctx: &mut Ctx, e: Entity, name: &str) -> Result<(), String> {
        let before = ctx.world.get_json(e, name).map_err(|e| e.to_string())?;
        ctx.world.remove_json(e, name).map_err(|e| e.to_string())?;
        self.record(Undo::Component {
            e,
            name: name.to_string(),
            before: Some(before),
        });
        Ok(())
    }

    /// Records that `e`'s transform was `before` a drag that moved it
    /// directly (the drag itself changes the world every frame).
    pub fn moved(&mut self, e: Entity, before: Value) {
        self.record(Undo::Component {
            e,
            name: "Transform2D".into(),
            before: Some(before),
        });
    }

    /// Spawns an entity with only a transform at `at`.
    pub fn spawn_empty(&mut self, ctx: &mut Ctx, at: Vec2) -> Entity {
        let e = ctx.world.spawn((crate::Transform2D::at(at),));
        self.record(Undo::Spawned(e));
        e
    }

    /// Spawns prefab `path` at `at`, with undo.
    pub fn spawn_prefab(&mut self, ctx: &mut Ctx, path: &str, at: Vec2) -> Result<Entity, String> {
        let e = ctx.spawn_prefab(path, at)?;
        self.record(Undo::Spawned(e));
        Ok(e)
    }

    /// A copy of `e`'s reflected components, `offset` away.
    pub fn duplicate(&mut self, ctx: &mut Ctx, e: Entity, offset: Vec2) -> Result<Entity, String> {
        let mut components = ctx.world.entity_json(e).map_err(|e| e.to_string())?;
        if let Some(p) = components["Transform2D"]["position"].as_array_mut()
            && let [x, y] = &mut p[..]
        {
            *x = json!(x.as_f64().unwrap_or(0.0) + f64::from(offset.x));
            *y = json!(y.as_f64().unwrap_or(0.0) + f64::from(offset.y));
        }
        let copy = ctx.world.spawn_json(components).map_err(|e| e.to_string())?;
        self.record(Undo::Spawned(copy));
        Ok(copy)
    }

    /// Despawns `e`, with undo (which brings back its reflected components
    /// as a new entity: references to `e` from other components stay dead).
    pub fn delete(&mut self, ctx: &mut Ctx, e: Entity) -> Result<(), String> {
        let components = ctx.world.entity_json(e).map_err(|e| e.to_string())?;
        ctx.world.despawn(e);
        if components.as_object().is_some_and(|c| !c.is_empty()) {
            self.record(Undo::Despawned { e, components });
        }
        Ok(())
    }

    /// Puts back the last change. `Ok(false)` if there's nothing to undo.
    pub fn undo(&mut self, ctx: &mut Ctx) -> Result<bool, String> {
        let Some(step) = self.undo.pop() else {
            return Ok(false);
        };
        match step {
            Undo::Component { e, name, before } => {
                match before {
                    Some(value) => ctx.world.set_json(e, &name, value).map(drop),
                    None => ctx.world.remove_json(e, &name).map(drop),
                }
                .map_err(|e| e.to_string())?;
            }
            Undo::Spawned(e) => {
                ctx.world.despawn(e);
            }
            Undo::Despawned { e, components } => {
                let back = ctx.world.spawn_json(components).map_err(|e| e.to_string())?;
                for step in &mut self.undo {
                    match step {
                        Undo::Component { e: old, .. } | Undo::Spawned(old) | Undo::Despawned { e: old, .. } => {
                            if *old == e {
                                *old = back;
                            }
                        }
                    }
                }
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Transform2D, vec2};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Debug)]
    #[serde(default)]
    struct Hp(f32);

    fn ctx() -> Ctx {
        let mut ctx = Ctx::new(1.0 / 60.0, vec2(800.0, 600.0));
        ctx.world.reflect::<Hp>();
        ctx
    }
    fn at(ctx: &Ctx, e: Entity) -> Vec2 {
        ctx.world.get::<Transform2D>(e).unwrap().position
    }

    #[test]
    fn every_edit_is_undone_in_reverse_order() {
        let mut ctx = ctx();
        let mut ed = Editor::default();
        let a = ed.spawn_empty(&mut ctx, vec2(1.0, 1.0));
        ed.set(&mut ctx, a, "Hp", json!(5.0)).unwrap();
        let b = ed.duplicate(&mut ctx, a, vec2(10.0, 0.0)).unwrap();
        assert_eq!((at(&ctx, b), ctx.world.get::<Hp>(b)), (vec2(11.0, 1.0), Some(&Hp(5.0))));
        ed.remove(&mut ctx, a, "Hp").unwrap();
        ed.delete(&mut ctx, b).unwrap();
        assert_eq!(ctx.world.len(), 1);

        assert!(ed.undo(&mut ctx).unwrap()); // delete: back as a new entity
        assert!(!ctx.world.is_alive(b), "old handles stay dead");
        let back = ctx
            .world
            .query::<(Entity, &Hp)>()
            .iter()
            .map(|(e, _)| e)
            .next()
            .unwrap();
        assert_eq!(at(&ctx, back), vec2(11.0, 1.0));
        assert!(ed.undo(&mut ctx).unwrap()); // remove
        assert_eq!(ctx.world.get::<Hp>(a), Some(&Hp(5.0)));
        assert!(ed.undo(&mut ctx).unwrap()); // duplicate: despawns the one brought back
        assert!(!ctx.world.is_alive(back));
        assert!(ed.undo(&mut ctx).unwrap()); // set (added)
        assert_eq!(ctx.world.get::<Hp>(a), None);
        assert!(ed.undo(&mut ctx).unwrap()); // spawn
        assert_eq!(ctx.world.len(), 0);
        assert!(!ed.undo(&mut ctx).unwrap());
    }

    #[test]
    fn a_drag_undoes_in_one_step() {
        let mut ctx = ctx();
        let mut ed = Editor::default();
        let e = ctx.world.spawn((Transform2D::at(vec2(1.0, 2.0)),));
        let before = ctx.world.get_json(e, "Transform2D").unwrap();
        for x in 0..10 {
            ctx.world.get_mut::<Transform2D>(e).unwrap().position.x = x as f32; // the drag
        }
        ed.moved(e, before);
        ed.undo(&mut ctx).unwrap();
        assert_eq!(at(&ctx, e), vec2(1.0, 2.0));
    }

    #[test]
    fn stop_puts_the_world_back_as_it_was_at_play() {
        let mut ctx = ctx();
        let mut ed = Editor::default();
        let e = ed.spawn_empty(&mut ctx, vec2(1.0, 2.0));
        ed.play(&mut ctx);
        assert!(ed.playing() && !ctx.time.is_paused());
        // The game runs: things move, spawn and die; the editor edits too.
        ctx.world.get_mut::<Transform2D>(e).unwrap().position = vec2(50.0, 50.0);
        ctx.world.spawn((Transform2D::default(),));
        ed.set(&mut ctx, e, "Hp", json!(1.0)).unwrap();
        ctx.time.pause();
        ed.play(&mut ctx); // resume: still from the first Play
        ed.stop(&mut ctx).unwrap();
        assert!(ctx.time.is_paused() && !ed.playing());
        assert_eq!(ctx.world.len(), 1);
        assert_eq!((at(&ctx, e), ctx.world.get::<Hp>(e)), (vec2(1.0, 2.0), None));
        // Undo steps from before Play still apply; those from playing are gone.
        assert!(ed.undo(&mut ctx).unwrap());
        assert_eq!(ctx.world.len(), 0);
        assert!(!ed.can_undo());
    }

    #[test]
    fn keep_makes_stop_do_nothing() {
        let mut ctx = ctx();
        let mut ed = Editor::default();
        ed.play(&mut ctx);
        ctx.world.spawn((Transform2D::default(),));
        ed.keep();
        ed.stop(&mut ctx).unwrap();
        assert_eq!(ctx.world.len(), 1);
    }

    #[test]
    fn a_failed_edit_changes_nothing_and_is_not_recorded() {
        let mut ctx = ctx();
        let mut ed = Editor::default();
        let e = ctx.world.spawn((Transform2D::default(),));
        assert!(ed.set(&mut ctx, e, "Hp", json!("not a number")).is_err());
        assert!(ed.add(&mut ctx, e, "NoSuchThing").is_err());
        assert!(!ed.can_undo());
    }

    #[test]
    fn a_refused_stop_keeps_the_play_snapshot_and_undo_for_a_retry() {
        let mut ctx = ctx();
        let mut ed = Editor::default();
        let a = ed.spawn_empty(&mut ctx, vec2(1.0, 1.0));
        ed.set(&mut ctx, a, "Hp", json!(5.0)).unwrap();
        ed.play(&mut ctx);
        ed.set(&mut ctx, a, "Hp", json!(9.0)).unwrap(); // while playing
        ctx.world.spawn((Hp(1.0),));
        // A lockstep session: Stop would put the world back on this peer only.
        let session = std::mem::replace(&mut ctx.net, crate::Net::loopback(2).remove(0));
        drop(session);
        let err = ed.stop(&mut ctx).unwrap_err();
        assert!(err.contains("lockstep"), "{err}");
        assert!(ed.playing(), "the Play snapshot is kept");
        assert!(!ctx.time.is_paused(), "a refused Stop doesn't pause the simulation");
        assert_eq!(ed.undo.len(), 3, "and the undo history");
        assert_eq!(ctx.world.len(), 2, "the world is unchanged");
        assert_eq!(ctx.world.get::<Hp>(a), Some(&Hp(9.0)));
        // The session ended: Stop works now.
        ctx.net = crate::Net::default();
        ed.stop(&mut ctx).unwrap();
        assert!(!ed.playing() && ctx.time.is_paused());
        assert_eq!(ed.undo.len(), 2, "edits made while playing are dropped");
        assert_eq!((ctx.world.len(), ctx.world.get::<Hp>(a)), (1, Some(&Hp(5.0))));
    }
}
