//! Component reflection: read and write components by name, as JSON.
//!
//! Reflection is opt-in per component type and uses serde, so any type with
//! `#[derive(Serialize, Deserialize)]` can be reflected. It is what tools use
//! to look inside a running game: the agent (MCP) bridge, the inspector, and
//! save/load.

use std::collections::HashMap;
use std::fmt;

use crabby_ecs::{Component, Entity, World};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Why a reflection call failed.
///
/// ```
/// # use crabbyengine::*;
/// let world = World::new();
/// let e = Entity::from_bits(0);
/// let err = world.get_json(e, "Health").unwrap_err();
/// assert_eq!(err, ReflectError::UnknownComponent("Health".into()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReflectError {
    /// No reflected component has this name.
    UnknownComponent(String),
    /// The entity was despawned (or never existed).
    NoSuchEntity(Entity),
    /// The entity is alive but doesn't have this component.
    MissingComponent { entity: Entity, component: String },
    /// The JSON doesn't fit the component type, or the value can't be written as JSON.
    BadValue { component: String, message: String },
}

impl fmt::Display for ReflectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownComponent(name) => write!(
                f,
                "no reflected component named `{name}` (register it with `world.reflect::<T>()`)"
            ),
            Self::NoSuchEntity(e) => write!(f, "entity {} does not exist", e.to_bits()),
            Self::MissingComponent { entity, component } => {
                write!(f, "entity {} has no `{component}`", entity.to_bits())
            }
            Self::BadValue { component, message } => write!(f, "bad value for `{component}`: {message}"),
        }
    }
}

impl std::error::Error for ReflectError {}

type GetFn = fn(&World, Entity) -> Option<Result<Value, String>>;
type SetFn = fn(&mut World, Entity, Value) -> Result<(), String>;
type SpawnFn = fn(&mut World, Value) -> Result<Entity, String>;
type RemoveFn = fn(&mut World, Entity) -> bool;

/// What reflection knows about one component type.
#[derive(Clone)]
struct Reflected {
    name: String,
    type_name: &'static str,
    get: GetFn,
    set: SetFn,
    spawn: SpawnFn,
    remove: RemoveFn,
}

/// The reflected components of a world, stored in it as a resource.
#[derive(Default)]
struct Registry {
    list: Vec<Reflected>,
    by_name: HashMap<String, usize>,
}

fn get<T: Component + Serialize>(world: &World, e: Entity) -> Option<Result<Value, String>> {
    world
        .get::<T>(e)
        .map(|v| serde_json::to_value(v).map_err(|err| err.to_string()))
}

fn set<T: Component + DeserializeOwned>(world: &mut World, e: Entity, value: Value) -> Result<(), String> {
    let value: T = serde_json::from_value(value).map_err(|err| err.to_string())?;
    world.insert(e, value);
    Ok(())
}

fn spawn<T: Component + DeserializeOwned>(world: &mut World, value: Value) -> Result<Entity, String> {
    let value: T = serde_json::from_value(value).map_err(|err| err.to_string())?;
    Ok(world.spawn((value,)))
}

fn remove<T: Component>(world: &mut World, e: Entity) -> bool {
    world.remove::<T>(e).is_some()
}

/// `crate::game::Score<u32>` → `Score<u32>`: the last path segment, keeping generics.
fn short_name(type_name: &str) -> String {
    let end = type_name.find('<').unwrap_or(type_name.len());
    let start = type_name[..end].rfind("::").map_or(0, |i| i + 2);
    type_name[start..].to_string()
}

/// Fields present in `patch` replace those in `base`; nested objects merge
/// the same way. Anything else (numbers, arrays, ...) is replaced whole.
fn merge(base: &mut Value, patch: Value) {
    match (base, patch) {
        (Value::Object(base), Value::Object(patch)) => {
            for (k, v) in patch {
                match base.get_mut(&k) {
                    Some(slot) => merge(slot, v),
                    None => {
                        base.insert(k, v);
                    }
                }
            }
        }
        (base, patch) => *base = patch,
    }
}

/// Reflection methods on [`World`]: read and write components by name as JSON.
///
/// Comes in with `use crabbyengine::*;`.
pub trait WorldReflect {
    /// Makes component `T` visible to reflection under its type name without
    /// the module path (`my_game::Health` becomes `"Health"`).
    /// Registering the same type again does nothing.
    ///
    /// `Transform2D` is reflected in every game by default.
    ///
    /// ```
    /// use crabbyengine::*;
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct Health { current: f32, max: f32 }
    ///
    /// let mut world = World::new();
    /// world.reflect::<Health>();
    /// let e = world.spawn((Health { current: 3.0, max: 10.0 },));
    /// assert_eq!(world.get_json(e, "Health").unwrap()["current"], 3.0);
    /// ```
    ///
    /// # Panics
    /// If a different type with the same short name is already reflected.
    fn reflect<T: Component + Serialize + DeserializeOwned>(&mut self);

    /// Names of all reflected components, in registration order.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let mut world = World::new();
    /// world.reflect::<Transform2D>();
    /// assert_eq!(world.reflected_components(), ["Transform2D"]);
    /// ```
    fn reflected_components(&self) -> Vec<&str>;

    /// Component `name` of entity `e`, as JSON.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let mut world = World::new();
    /// world.reflect::<Transform2D>();
    /// let e = world.spawn((Transform2D::at(vec2(1.0, 2.0)),));
    /// let t = world.get_json(e, "Transform2D").unwrap();
    /// assert_eq!(t["position"], serde_json::json!([1.0, 2.0]));
    /// ```
    fn get_json(&self, e: Entity, name: &str) -> Result<Value, ReflectError>;

    /// Writes component `name` of entity `e` from JSON, adding the component
    /// if the entity doesn't have it.
    ///
    /// If the entity has it and `value` is an object, only the fields in
    /// `value` change; the rest keep their values. To add a component,
    /// give every field (or the ones the type has `#[serde(default)]` for).
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # use serde_json::json;
    /// let mut world = World::new();
    /// world.reflect::<Transform2D>();
    /// let e = world.spawn((Transform2D::at(vec2(1.0, 2.0)).rotated(0.5),));
    /// world.set_json(e, "Transform2D", json!({ "position": [5.0, 5.0] })).unwrap();
    /// let t = world.get::<Transform2D>(e).unwrap();
    /// assert_eq!((t.position, t.rotation), (vec2(5.0, 5.0), 0.5));
    /// ```
    fn set_json(&mut self, e: Entity, name: &str, value: Value) -> Result<(), ReflectError>;

    /// Removes component `name` from entity `e`. Returns whether it had it.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let mut world = World::new();
    /// world.reflect::<Transform2D>();
    /// let e = world.spawn((Transform2D::default(),));
    /// assert_eq!(world.remove_json(e, "Transform2D"), Ok(true));
    /// assert_eq!(world.remove_json(e, "Transform2D"), Ok(false));
    /// ```
    fn remove_json(&mut self, e: Entity, name: &str) -> Result<bool, ReflectError>;

    /// Every reflected component of entity `e`, as one JSON object keyed by
    /// component name. Components that aren't reflected are left out.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let mut world = World::new();
    /// world.reflect::<Transform2D>();
    /// let e = world.spawn((Transform2D::default(),));
    /// assert!(world.entity_json(e).unwrap().get("Transform2D").is_some());
    /// ```
    fn entity_json(&self, e: Entity) -> Result<Value, ReflectError>;

    /// Spawns an entity from a JSON object keyed by component name, the
    /// format [`entity_json`](Self::entity_json) returns. On error nothing
    /// is left spawned.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # use serde_json::json;
    /// let mut world = World::new();
    /// world.reflect::<Transform2D>();
    /// let e = world
    ///     .spawn_json(json!({ "Transform2D": { "position": [3.0, 4.0], "rotation": 0.0, "scale": [1.0, 1.0] } }))
    ///     .unwrap();
    /// assert_eq!(world.get::<Transform2D>(e).unwrap().position, vec2(3.0, 4.0));
    /// ```
    fn spawn_json(&mut self, components: Value) -> Result<Entity, ReflectError>;
}

fn registry(world: &World) -> Option<&Registry> {
    world.get_resource::<Registry>()
}

fn find(world: &World, name: &str) -> Result<Reflected, ReflectError> {
    registry(world)
        .and_then(|r| r.by_name.get(name).map(|&i| r.list[i].clone()))
        .ok_or_else(|| ReflectError::UnknownComponent(name.to_string()))
}

fn bad_value(component: &str) -> impl FnOnce(String) -> ReflectError + '_ {
    move |message| ReflectError::BadValue {
        component: component.to_string(),
        message,
    }
}

impl WorldReflect for World {
    #[track_caller]
    fn reflect<T: Component + Serialize + DeserializeOwned>(&mut self) {
        let type_name = std::any::type_name::<T>();
        let name = short_name(type_name);
        if self.get_resource::<Registry>().is_none() {
            self.insert_resource(Registry::default());
        }
        let reg = self.resource_mut::<Registry>();
        if let Some(&i) = reg.by_name.get(&name) {
            let existing = reg.list[i].type_name;
            assert_eq!(
                existing, type_name,
                "World::reflect: `{type_name}` and `{existing}` would both be named `{name}`"
            );
            return;
        }
        reg.by_name.insert(name.clone(), reg.list.len());
        reg.list.push(Reflected {
            name,
            type_name,
            get: get::<T>,
            set: set::<T>,
            spawn: spawn::<T>,
            remove: remove::<T>,
        });
    }

    fn reflected_components(&self) -> Vec<&str> {
        registry(self).map_or_else(Vec::new, |r| r.list.iter().map(|c| c.name.as_str()).collect())
    }

    fn get_json(&self, e: Entity, name: &str) -> Result<Value, ReflectError> {
        let c = find(self, name)?;
        if !self.is_alive(e) {
            return Err(ReflectError::NoSuchEntity(e));
        }
        match (c.get)(self, e) {
            Some(v) => v.map_err(bad_value(name)),
            None => Err(ReflectError::MissingComponent {
                entity: e,
                component: name.to_string(),
            }),
        }
    }

    fn set_json(&mut self, e: Entity, name: &str, value: Value) -> Result<(), ReflectError> {
        let c = find(self, name)?;
        if !self.is_alive(e) {
            return Err(ReflectError::NoSuchEntity(e));
        }
        let value = match (c.get)(self, e) {
            Some(current) if value.is_object() => {
                let mut current = current.map_err(bad_value(name))?;
                merge(&mut current, value);
                current
            }
            _ => value,
        };
        (c.set)(self, e, value).map_err(bad_value(name))
    }

    fn remove_json(&mut self, e: Entity, name: &str) -> Result<bool, ReflectError> {
        let c = find(self, name)?;
        if !self.is_alive(e) {
            return Err(ReflectError::NoSuchEntity(e));
        }
        Ok((c.remove)(self, e))
    }

    fn entity_json(&self, e: Entity) -> Result<Value, ReflectError> {
        if !self.is_alive(e) {
            return Err(ReflectError::NoSuchEntity(e));
        }
        let mut out = serde_json::Map::new();
        for c in registry(self).map_or(&[][..], |r| &r.list) {
            if let Some(v) = (c.get)(self, e) {
                out.insert(c.name.clone(), v.map_err(bad_value(&c.name))?);
            }
        }
        Ok(Value::Object(out))
    }

    fn spawn_json(&mut self, components: Value) -> Result<Entity, ReflectError> {
        let Value::Object(map) = components else {
            return Err(ReflectError::BadValue {
                component: "<entity>".into(),
                message: "expected an object keyed by component name".into(),
            });
        };
        // Look every name up first, so an unknown name spawns nothing.
        let parts = map
            .into_iter()
            .map(|(name, v)| find(self, &name).map(|c| (c, v)))
            .collect::<Result<Vec<_>, _>>()?;
        let mut parts = parts.into_iter();
        let Some((first, value)) = parts.next() else {
            return Err(ReflectError::BadValue {
                component: "<entity>".into(),
                message: "an entity needs at least one component".into(),
            });
        };
        let e = (first.spawn)(self, value).map_err(bad_value(&first.name))?;
        for (c, value) in parts {
            if let Err(message) = (c.set)(self, e, value) {
                self.despawn(e);
                return Err(ReflectError::BadValue {
                    component: c.name,
                    message,
                });
            }
        }
        Ok(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn short_names_drop_the_path_but_keep_generics() {
        assert_eq!(short_name("my_game::units::Health"), "Health");
        assert_eq!(short_name("Health"), "Health");
        assert_eq!(short_name("a::Wrapper<b::Inner>"), "Wrapper<b::Inner>");
    }

    #[test]
    fn merge_replaces_only_given_fields() {
        let mut base = json!({ "a": 1, "b": { "x": 1, "y": 2 }, "c": [1, 2] });
        merge(&mut base, json!({ "b": { "y": 9 }, "c": [3] }));
        assert_eq!(base, json!({ "a": 1, "b": { "x": 1, "y": 9 }, "c": [3] }));
    }
}
