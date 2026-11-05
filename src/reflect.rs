//! Reflection: read and write components and resources by name, as JSON.
//!
//! Reflection is opt-in per component type and uses serde, so any type with
//! `#[derive(Serialize, Deserialize)]` can be reflected. It is what tools use
//! to look inside a running game: the agent (MCP) bridge, the inspector, and
//! save/load.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crabby_ecs::{Component, ComponentId, Entity, World};
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
#[non_exhaustive]
pub enum ReflectError {
    /// No reflected component has this name.
    UnknownComponent(String),
    /// The entity was despawned (or never existed).
    NoSuchEntity(Entity),
    /// The entity is alive but doesn't have this component.
    MissingComponent {
        /// The entity.
        entity: Entity,
        /// The component's reflected name.
        component: String,
    },
    /// The JSON doesn't fit the component type, or the value can't be written as JSON.
    BadValue {
        /// The component's reflected name.
        component: String,
        /// What didn't fit.
        message: String,
    },
    /// No reflected resource has this name.
    UnknownResource(String),
    /// The resource is reflected but not in the world right now.
    MissingResource(String),
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
            Self::UnknownResource(name) => write!(
                f,
                "no reflected resource named `{name}` (register it with `world.reflect_resource::<T>()`)"
            ),
            Self::MissingResource(name) => write!(f, "resource `{name}` is not in the world"),
        }
    }
}

impl std::error::Error for ReflectError {}

type GetFn = Arc<dyn Fn(&World, Entity) -> Option<Result<Value, String>> + Send + Sync>;
type SetFn = Arc<dyn Fn(&mut World, Entity, Value) -> Result<(), String> + Send + Sync>;
type SpawnFn = Arc<dyn Fn(&mut World, Value) -> Result<Entity, String> + Send + Sync>;
type RemoveFn = Arc<dyn Fn(&mut World, Entity) -> bool + Send + Sync>;
type CheckFn = Arc<dyn Fn(&Value) -> Result<(), String> + Send + Sync>;
type SpawnAtFn = Arc<dyn Fn(&mut World, Entity, Value) -> Result<bool, String> + Send + Sync>;

/// `format` field of world saves.
const SAVE_FORMAT: &str = "crabby-world";
/// Version of the save layout; bumped when it changes incompatibly.
const SAVE_VERSION: u64 = 1;

/// What reflection knows about one component type.
#[derive(Clone)]
struct Reflected {
    name: String,
    /// The Rust type name, or the component name for field-described components.
    type_name: String,
    get: GetFn,
    set: SetFn,
    spawn: SpawnFn,
    spawn_at: SpawnAtFn,
    check: CheckFn,
    remove: RemoveFn,
}

type ResGetFn = Arc<dyn Fn(&World) -> Option<Result<Value, String>> + Send + Sync>;
type ResSetFn = Arc<dyn Fn(&mut World, Value) -> Result<(), String> + Send + Sync>;

/// What reflection knows about one resource type.
#[derive(Clone)]
struct ReflectedResource {
    name: String,
    type_name: String,
    get: ResGetFn,
    set: ResSetFn,
    check: CheckFn,
}

/// The reflected components and resources of a world, stored in it as a resource.
#[derive(Default)]
struct Registry {
    list: Vec<Reflected>,
    by_name: HashMap<String, usize>,
    resources: Vec<ReflectedResource>,
    resource_by_name: HashMap<String, usize>,
    /// Field layouts of components described with `reflect_fields`, by id:
    /// how tools read C events of those types.
    layouts: HashMap<ComponentId, Arc<FieldLayout>>,
}

/// The name of foreign component `id`, if it's described with `reflect_fields`.
pub(crate) fn raw_name(world: &World, id: ComponentId) -> Option<&str> {
    Some(registry(world)?.layouts.get(&id)?.component.as_str())
}

/// The name and JSON of a value of foreign component `id` (one described
/// with `reflect_fields`), or `None` if it isn't described.
///
/// # Safety
/// `p` points to a value of that component (its registered size, readable).
pub(crate) unsafe fn raw_json(world: &World, id: ComponentId, p: *const u8) -> Option<(&str, Value)> {
    let layout = registry(world)?.layouts.get(&id)?;
    // SAFETY: forwarded.
    Some((layout.component.as_str(), unsafe { layout.read(p) }))
}

fn get_resource<R: Send + Sync + 'static + Serialize>(world: &World) -> Option<Result<Value, String>> {
    world
        .get_resource::<R>()
        .map(|v| serde_json::to_value(v).map_err(|err| err.to_string()))
}

fn set_resource<R: Send + Sync + 'static + DeserializeOwned>(world: &mut World, value: Value) -> Result<(), String> {
    let value: R = serde_json::from_value(value).map_err(|err| err.to_string())?;
    world.insert_resource(value);
    Ok(())
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

fn spawn_at<T: Component + DeserializeOwned>(world: &mut World, e: Entity, value: Value) -> Result<bool, String> {
    let value: T = serde_json::from_value(value).map_err(|err| err.to_string())?;
    Ok(world.spawn_at(e, (value,)))
}

fn check<T: DeserializeOwned>(value: &Value) -> Result<(), String> {
    T::deserialize(value).map(drop).map_err(|err| err.to_string())
}

fn remove<T: Component>(world: &mut World, e: Entity) -> bool {
    world.remove::<T>(e).is_some()
}

/// `crate::game::Score<u32>` → `Score<u32>`: the last path segment, keeping generics.
pub(crate) fn short_name(type_name: &str) -> String {
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

/// The type of a field in [`WorldReflect::reflect_fields`], and how it looks
/// in JSON.
///
/// ```
/// # use crabbyengine::*;
/// assert_eq!(FieldType::Vec2.size(), 8);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
#[non_exhaustive]
pub enum FieldType {
    /// A JSON number.
    F32,
    /// A JSON number.
    F64,
    /// A JSON integer.
    I8,
    /// A JSON integer.
    I16,
    /// A JSON integer.
    I32,
    /// A JSON integer.
    I64,
    /// A JSON integer, 0 to 255.
    U8,
    /// A JSON integer, 0 to 65535.
    U16,
    /// A JSON integer, 0 or more.
    U32,
    /// A JSON integer, 0 or more.
    U64,
    /// One byte, 0 or 1; `true`/`false` in JSON (other bytes read as `true`).
    Bool,
    /// Two `f32`, `[x, y]` in JSON like [`Vec2`](crate::Vec2).
    Vec2,
    /// Four `f32`, `{ "r", "g", "b", "a" }` in JSON.
    Color,
    /// An entity id as 64 bits, a JSON number like a reflected [`Entity`].
    Entity,
}

impl FieldType {
    /// Bytes the field takes.
    #[must_use]
    pub fn size(self) -> usize {
        match self {
            Self::I8 | Self::U8 | Self::Bool => 1,
            Self::I16 | Self::U16 => 2,
            Self::F32 | Self::I32 | Self::U32 => 4,
            Self::F64 | Self::I64 | Self::U64 | Self::Entity | Self::Vec2 => 8,
            Self::Color => 16,
        }
    }
}

/// One field of a component described with [`WorldReflect::reflect_fields`].
///
/// ```
/// # use crabbyengine::*;
/// let f = Field::new("speed", FieldType::F32, 8);
/// assert_eq!((f.name.as_str(), f.offset), ("speed", 8));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// The JSON key.
    pub name: String,
    /// How the bytes are read and written.
    pub ty: FieldType,
    /// Byte offset in the component.
    pub offset: usize,
}

impl Field {
    /// A field named `name` (the JSON key) of type `ty` at byte `offset`.
    pub fn new(name: impl Into<String>, ty: FieldType, offset: usize) -> Self {
        Self {
            name: name.into(),
            ty,
            offset,
        }
    }
}

/// Checked fields of one component, and its size.
struct FieldLayout {
    component: String,
    size: usize,
    fields: Vec<Field>,
}

macro_rules! num {
    ($p:expr, $t:ty) => {
        // SAFETY: the caller's pointer covers the field (checked in `new`); any bits are a valid $t.
        Value::from(unsafe { $p.cast::<$t>().read_unaligned() })
    };
}

impl FieldLayout {
    #[track_caller]
    fn new(component: &str, size: usize, fields: &[Field]) -> Self {
        let mut names = std::collections::HashSet::new();
        for f in fields {
            assert!(
                f.offset.checked_add(f.ty.size()).is_some_and(|end| end <= size),
                "World::reflect_fields: field `{}` of `{component}` (offset {}, {} bytes) doesn't fit in {size} bytes",
                f.name,
                f.offset,
                f.ty.size()
            );
            assert!(
                names.insert(f.name.as_str()),
                "World::reflect_fields: `{component}` has two fields named `{}`",
                f.name
            );
        }
        Self {
            component: component.to_string(),
            size,
            fields: fields.to_vec(),
        }
    }

    /// The value at `p` as JSON.
    ///
    /// # Safety
    /// `p` points to `size` readable bytes.
    unsafe fn read(&self, p: *const u8) -> Value {
        let mut out = serde_json::Map::with_capacity(self.fields.len());
        for f in &self.fields {
            // SAFETY: inside the value (checked in `new`).
            let p = unsafe { p.add(f.offset) };
            let v = match f.ty {
                FieldType::F32 => num!(p, f32),
                FieldType::F64 => num!(p, f64),
                FieldType::I8 => num!(p, i8),
                FieldType::I16 => num!(p, i16),
                FieldType::I32 => num!(p, i32),
                FieldType::I64 => num!(p, i64),
                FieldType::U8 => num!(p, u8),
                FieldType::U16 => num!(p, u16),
                FieldType::U32 => num!(p, u32),
                FieldType::U64 | FieldType::Entity => num!(p, u64),
                // SAFETY: as in `num!`; read as a byte, so any value is fine.
                FieldType::Bool => Value::from(unsafe { p.read() } != 0),
                FieldType::Vec2 => {
                    // SAFETY: as in `num!`.
                    let [x, y] = [0, 4].map(|o| unsafe { p.add(o).cast::<f32>().read_unaligned() });
                    serde_json::json!([x, y])
                }
                FieldType::Color => {
                    // SAFETY: as in `num!`.
                    let [r, g, b, a] = [0, 4, 8, 12].map(|o| unsafe { p.add(o).cast::<f32>().read_unaligned() });
                    serde_json::json!({ "r": r, "g": g, "b": b, "a": a })
                }
            };
            out.insert(f.name.clone(), v);
        }
        Value::Object(out)
    }

    /// The bytes for `value`: from `current` (or zero) with the given fields written.
    fn bytes(&self, current: Option<*mut u8>, value: &Value) -> Result<Vec<u8>, String> {
        let Value::Object(map) = value else {
            return Err(format!("expected an object with the fields of `{}`", self.component));
        };
        let mut bytes = vec![0u8; self.size];
        if let Some(p) = current {
            // SAFETY: `p` is a live value of this component, `size` bytes.
            unsafe { std::ptr::copy_nonoverlapping(p, bytes.as_mut_ptr(), self.size) };
        }
        for (key, v) in map {
            let Some(f) = self.fields.iter().find(|f| f.name == *key) else {
                let known: Vec<&str> = self.fields.iter().map(|f| f.name.as_str()).collect();
                return Err(format!(
                    "`{}` has no field `{key}` (fields: {})",
                    self.component,
                    known.join(", ")
                ));
            };
            let at = &mut bytes[f.offset..f.offset + f.ty.size()];
            write_field(f, v, at).map_err(|e| format!("field `{key}`: {e}"))?;
        }
        Ok(bytes)
    }
}

fn write_field(f: &Field, v: &Value, at: &mut [u8]) -> Result<(), String> {
    fn int<T: TryFrom<i64> + TryFrom<u64>>(v: &Value) -> Result<T, String> {
        let bad = || format!("expected an integer that fits, got {v}");
        match (v.as_i64(), v.as_u64()) {
            (_, Some(u)) => T::try_from(u).map_err(|_| bad()),
            (Some(i), _) => T::try_from(i).map_err(|_| bad()),
            _ => Err(bad()),
        }
    }
    fn float(v: &Value) -> Result<f64, String> {
        v.as_f64().ok_or_else(|| format!("expected a number, got {v}"))
    }
    match f.ty {
        FieldType::F32 => at.copy_from_slice(&(float(v)? as f32).to_ne_bytes()),
        FieldType::F64 => at.copy_from_slice(&float(v)?.to_ne_bytes()),
        FieldType::I8 => at.copy_from_slice(&int::<i8>(v)?.to_ne_bytes()),
        FieldType::I16 => at.copy_from_slice(&int::<i16>(v)?.to_ne_bytes()),
        FieldType::I32 => at.copy_from_slice(&int::<i32>(v)?.to_ne_bytes()),
        FieldType::I64 => at.copy_from_slice(&int::<i64>(v)?.to_ne_bytes()),
        FieldType::U8 => at.copy_from_slice(&int::<u8>(v)?.to_ne_bytes()),
        FieldType::U16 => at.copy_from_slice(&int::<u16>(v)?.to_ne_bytes()),
        FieldType::U32 => at.copy_from_slice(&int::<u32>(v)?.to_ne_bytes()),
        FieldType::U64 | FieldType::Entity => at.copy_from_slice(&int::<u64>(v)?.to_ne_bytes()),
        FieldType::Bool => at[0] = v.as_bool().ok_or_else(|| format!("expected true or false, got {v}"))? as u8,
        FieldType::Vec2 => {
            let xy = v
                .as_array()
                .filter(|a| a.len() == 2)
                .ok_or_else(|| format!("expected [x, y], got {v}"))?;
            for (i, c) in xy.iter().enumerate() {
                at[i * 4..i * 4 + 4].copy_from_slice(&(float(c)? as f32).to_ne_bytes());
            }
        }
        FieldType::Color => {
            let Value::Object(rgba) = v else {
                return Err(format!("expected {{\"r\", \"g\", \"b\", \"a\"}}, got {v}"));
            };
            for (i, k) in ["r", "g", "b", "a"].iter().enumerate() {
                // Missing channels keep their current value (zero in new values).
                if let Some(c) = rgba.get(*k) {
                    at[i * 4..i * 4 + 4].copy_from_slice(&(float(c)? as f32).to_ne_bytes());
                }
            }
            if let Some(k) = rgba.keys().find(|k| !["r", "g", "b", "a"].contains(&k.as_str())) {
                return Err(format!("colors have r, g, b and a, not `{k}`"));
            }
        }
    }
    Ok(())
}

/// Reflection methods on [`World`]: read and write components by name as JSON.
///
/// Comes in with `use crabbyengine::*;`.
pub trait WorldReflect {
    /// Makes component `T` visible to reflection under its type name without
    /// the module path (`my_game::Health` becomes `"Health"`).
    /// Registering the same type again does nothing.
    ///
    /// `Transform2D`, `Sprite` and `Attached` are reflected in every game by
    /// default (a sprite with its texture's file name), and with 3D on,
    /// `Transform3D` and `Model` too (a model with its mesh's file name).
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

    /// Makes a component that has no Rust type (one registered with
    /// [`World::register_component`], such as a C or C++ component) visible
    /// to reflection, by describing its fields. Its JSON is an object with
    /// one entry per field; bytes no field covers are zero in new values.
    /// Describing the same component again replaces its fields.
    ///
    /// ```
    /// use crabbyengine::*;
    ///
    /// let mut world = World::new();
    /// // A C struct { float current; uint32_t max; }.
    /// // SAFETY: plain bytes, no destructor.
    /// let id = unsafe { world.register_component("Health", 8, 4, None) };
    /// // SAFETY: any values of these fields make a valid Health.
    /// unsafe {
    ///     world.reflect_fields(id, &[
    ///         Field::new("current", FieldType::F32, 0),
    ///         Field::new("max", FieldType::U32, 4),
    ///     ]);
    /// }
    /// let e = world.spawn_json(serde_json::json!({ "Health": { "current": 3.5, "max": 10 } })).unwrap();
    /// assert_eq!(world.get_json(e, "Health").unwrap()["max"], 10);
    /// ```
    ///
    /// # Safety
    /// Any bytes where each field holds a value of its type and every other
    /// byte is zero or unchanged must be a valid value of the component.
    ///
    /// # Panics
    /// If the component has a Rust type or a destructor, a field doesn't fit
    /// in it or two fields share a name, or another type is reflected under
    /// the same name.
    unsafe fn reflect_fields(&mut self, id: ComponentId, fields: &[Field]);

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
    ///
    /// # Errors
    ///
    /// If `name` isn't reflected, `e` is dead or lacks it, or the value can't be
    /// written as JSON.
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
    ///
    /// # Errors
    ///
    /// If `name` isn't reflected, `e` is dead, or `value` doesn't fit the type.
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
    ///
    /// # Errors
    ///
    /// If `name` isn't reflected or `e` is dead.
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
    ///
    /// # Errors
    ///
    /// If `e` is dead or a component can't be written as JSON.
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
    ///
    /// # Errors
    ///
    /// If a name isn't reflected or a value doesn't fit its type; nothing is
    /// spawned then.
    fn spawn_json(&mut self, components: Value) -> Result<Entity, ReflectError>;

    /// The whole world as JSON: every entity that has reflected components,
    /// with its id, sorted by id. Components that aren't reflected are not
    /// saved; `"not_saved"` counts them by name, so nothing goes missing
    /// silently. Reflected resources ([`reflect_resource`](Self::reflect_resource))
    /// are saved under `"resources"`; others are not.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let mut world = World::new();
    /// world.reflect::<Transform2D>();
    /// world.spawn((Transform2D::at(vec2(1.0, 2.0)),));
    /// let save = world.save();
    /// assert_eq!(save["entities"].as_array().unwrap().len(), 1);
    /// ```
    fn save(&self) -> Value;

    /// Replaces every entity with those in `save` (from [`save`](Self::save)),
    /// **with the same ids**, so components that refer to entities stay
    /// valid. Reflected resources in the save are written; all other
    /// resources stay. Everything is checked before anything changes: on
    /// error the world is untouched.
    ///
    /// Old handles may match loaded entities afterwards (the save's ids
    /// take over), which is the point.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// let mut world = World::new();
    /// world.reflect::<Transform2D>();
    /// let e = world.spawn((Transform2D::at(vec2(1.0, 2.0)),));
    /// let save = world.save();
    ///
    /// world.get_mut::<Transform2D>(e).unwrap().position = vec2(9.0, 9.0);
    /// world.load(&save).unwrap();
    /// assert_eq!(world.get::<Transform2D>(e).unwrap().position, vec2(1.0, 2.0));
    /// ```
    ///
    /// # Errors
    ///
    /// If `save` isn't a world save this version reads, names a component or
    /// resource that isn't reflected, or has a value that doesn't fit its
    /// type. Everything is checked first: on an error the world is unchanged.
    fn load(&mut self, save: &Value) -> Result<(), ReflectError>;

    /// Makes resource `R` visible to reflection under its type name without
    /// the module path, like [`reflect`](Self::reflect) does for components.
    /// Tools (agents through `crabby-mcp`, the control endpoint) can then
    /// read and change it, and world saves include it.
    /// Registering the same type again does nothing.
    ///
    /// ```
    /// use crabbyengine::*;
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct Score { red: u32, blue: u32 }
    ///
    /// let mut world = World::new();
    /// world.reflect_resource::<Score>();
    /// world.insert_resource(Score { red: 3, blue: 1 });
    /// assert_eq!(world.resource_json("Score").unwrap()["red"], 3);
    /// ```
    ///
    /// # Panics
    /// If a different type with the same short name is already reflected as a resource.
    fn reflect_resource<R: Send + Sync + 'static + Serialize + DeserializeOwned>(&mut self);

    /// Names of the reflected resources, in registration order (whether or
    /// not they're in the world right now).
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # #[derive(serde::Serialize, serde::Deserialize)] struct Score(u32);
    /// let mut world = World::new();
    /// world.reflect_resource::<Score>();
    /// assert_eq!(world.reflected_resources(), ["Score"]);
    /// ```
    fn reflected_resources(&self) -> Vec<&str>;

    /// A reflected resource as JSON.
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # #[derive(serde::Serialize, serde::Deserialize)] struct Score(u32);
    /// let mut world = World::new();
    /// world.reflect_resource::<Score>();
    /// assert_eq!(world.resource_json("Score"), Err(ReflectError::MissingResource("Score".into())));
    /// world.insert_resource(Score(7));
    /// assert_eq!(world.resource_json("Score").unwrap(), 7);
    /// ```
    ///
    /// # Errors
    ///
    /// If `name` isn't a reflected resource or isn't in the world.
    fn resource_json(&self, name: &str) -> Result<Value, ReflectError>;

    /// Writes a reflected resource from JSON, inserting it if it's missing.
    /// For a resource that exists, an object changes only the fields it
    /// has, like [`set_json`](Self::set_json).
    ///
    /// ```
    /// # use crabbyengine::*;
    /// # use serde_json::json;
    /// # #[derive(serde::Serialize, serde::Deserialize)] struct Score { red: u32, blue: u32 }
    /// let mut world = World::new();
    /// world.reflect_resource::<Score>();
    /// world.insert_resource(Score { red: 3, blue: 1 });
    /// world.set_resource_json("Score", json!({"blue": 9})).unwrap();
    /// assert_eq!(world.resource::<Score>().blue, 9);
    /// assert_eq!(world.resource::<Score>().red, 3);
    /// ```
    ///
    /// # Errors
    ///
    /// If `name` isn't a reflected resource, or `value` doesn't fit its type.
    fn set_resource_json(&mut self, name: &str, value: Value) -> Result<(), ReflectError>;

    /// Makes resource `name`, one with no Rust type (stored with
    /// [`World::insert_resource_dynamic`], the C and C++ path), visible to
    /// reflection by listing its fields, like
    /// [`reflect_fields`](Self::reflect_fields) does for components.
    /// Describing it again replaces the fields.
    ///
    /// ```
    /// use crabbyengine::*;
    /// use std::alloc::Layout;
    ///
    /// let mut world = World::new();
    /// // A C struct { uint32_t red, blue; }.
    /// let layout = Layout::from_size_align(8, 4).unwrap();
    /// // SAFETY: any values of these fields make a valid Score.
    /// unsafe {
    ///     world.reflect_resource_fields("Score", layout, &[
    ///         Field::new("red", FieldType::U32, 0),
    ///         Field::new("blue", FieldType::U32, 4),
    ///     ]);
    /// }
    /// world.set_resource_json("Score", serde_json::json!({ "red": 3, "blue": 1 })).unwrap();
    /// assert_eq!(world.resource_json("Score").unwrap()["red"], 3);
    /// ```
    ///
    /// # Safety
    /// Any bytes where each field holds a value of its type and every other
    /// byte is zero or unchanged must be a valid value of the resource.
    ///
    /// # Panics
    /// If a field doesn't fit, two fields share a name, or a Rust resource
    /// is reflected under the same name.
    unsafe fn reflect_resource_fields(&mut self, name: &str, layout: std::alloc::Layout, fields: &[Field]);
}

fn find_resource(world: &World, name: &str) -> Result<ReflectedResource, ReflectError> {
    registry(world)
        .and_then(|r| r.resource_by_name.get(name).map(|&i| r.resources[i].clone()))
        .ok_or_else(|| ReflectError::UnknownResource(name.to_string()))
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
            let existing = &reg.list[i].type_name;
            assert_eq!(
                existing, type_name,
                "World::reflect: `{type_name}` and `{existing}` would both be named `{name}`"
            );
            return;
        }
        reg.by_name.insert(name.clone(), reg.list.len());
        reg.list.push(Reflected {
            name,
            type_name: type_name.to_string(),
            get: Arc::new(get::<T>),
            set: Arc::new(set::<T>),
            spawn: Arc::new(spawn::<T>),
            spawn_at: Arc::new(spawn_at::<T>),
            check: Arc::new(check::<T>),
            remove: Arc::new(remove::<T>),
        });
    }

    #[track_caller]
    unsafe fn reflect_fields(&mut self, id: ComponentId, fields: &[Field]) {
        let info = self.component_info(id);
        let name = info.name().to_string();
        let size = info.layout().size();
        assert!(
            info.type_id().is_none() && !info.needs_drop(),
            "World::reflect_fields: `{name}` is a Rust type or has a destructor; use `reflect::<T>()`"
        );
        let layout = Arc::new(FieldLayout::new(&name, size, fields));
        if self.get_resource::<Registry>().is_none() {
            self.insert_resource(Registry::default());
        }
        let reg = self.resource_mut::<Registry>();
        reg.layouts.insert(id, layout.clone());
        let entry = |name: &str| {
            let (l, g, k, st) = (layout.clone(), layout.clone(), layout.clone(), layout.clone());
            let sa = layout.clone();
            Reflected {
                name: name.to_string(),
                type_name: name.to_string(),
                get: Arc::new(move |w: &World, e| {
                    // SAFETY: the pointer is to a live value of `id`, `size` bytes.
                    w.get_dynamic(e, id).map(|p| Ok(unsafe { g.read(p) }))
                }),
                set: Arc::new(move |w: &mut World, e, v| {
                    let bytes = l.bytes(w.get_dynamic(e, id), &v)?;
                    // SAFETY: `reflect_fields`' caller promised these bytes are a valid value.
                    unsafe { w.insert_dynamic(e, id, bytes.as_ptr()) };
                    Ok(())
                }),
                spawn: Arc::new(move |w: &mut World, v| {
                    let bytes = st.bytes(None, &v)?;
                    // SAFETY: as above.
                    Ok(unsafe { w.spawn_dynamic(&[(id, bytes.as_ptr())]) })
                }),
                spawn_at: Arc::new(move |w: &mut World, e, v| {
                    let bytes = sa.bytes(None, &v)?;
                    // SAFETY: as above.
                    Ok(unsafe { w.spawn_at_dynamic(e, &[(id, bytes.as_ptr())]) })
                }),
                check: Arc::new(move |v: &Value| k.bytes(None, v).map(drop)),
                remove: Arc::new(move |w: &mut World, e| w.remove_dynamic(e, id)),
            }
        };
        match reg.by_name.get(&name) {
            // Describing the same component again replaces its fields.
            Some(&i) if reg.list[i].type_name == name => reg.list[i] = entry(&name),
            Some(&i) => panic!(
                "World::reflect_fields: `{name}` is already reflected as `{}`",
                reg.list[i].type_name
            ),
            None => {
                reg.by_name.insert(name.clone(), reg.list.len());
                reg.list.push(entry(&name));
            }
        }
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

    #[track_caller]
    fn reflect_resource<R: Send + Sync + 'static + Serialize + DeserializeOwned>(&mut self) {
        let type_name = std::any::type_name::<R>();
        let name = short_name(type_name);
        if self.get_resource::<Registry>().is_none() {
            self.insert_resource(Registry::default());
        }
        let reg = self.resource_mut::<Registry>();
        if let Some(&i) = reg.resource_by_name.get(&name) {
            let existing = &reg.resources[i].type_name;
            assert_eq!(
                existing, type_name,
                "World::reflect_resource: `{type_name}` and `{existing}` would both be named `{name}`"
            );
            return;
        }
        reg.resource_by_name.insert(name.clone(), reg.resources.len());
        reg.resources.push(ReflectedResource {
            name,
            type_name: type_name.to_string(),
            get: Arc::new(get_resource::<R>),
            set: Arc::new(set_resource::<R>),
            check: Arc::new(check::<R>),
        });
    }

    fn reflected_resources(&self) -> Vec<&str> {
        registry(self).map_or_else(Vec::new, |r| r.resources.iter().map(|c| c.name.as_str()).collect())
    }

    fn resource_json(&self, name: &str) -> Result<Value, ReflectError> {
        let r = find_resource(self, name)?;
        match (r.get)(self) {
            Some(v) => v.map_err(bad_value(name)),
            None => Err(ReflectError::MissingResource(name.to_string())),
        }
    }

    fn set_resource_json(&mut self, name: &str, value: Value) -> Result<(), ReflectError> {
        let r = find_resource(self, name)?;
        let value = match (r.get)(self) {
            Some(current) if value.is_object() => {
                let mut current = current.map_err(bad_value(name))?;
                merge(&mut current, value);
                current
            }
            _ => value,
        };
        (r.set)(self, value).map_err(bad_value(name))
    }

    #[track_caller]
    unsafe fn reflect_resource_fields(&mut self, name: &str, layout: std::alloc::Layout, fields: &[Field]) {
        let fields = Arc::new(FieldLayout::new(name, layout.size(), fields));
        if self.get_resource::<Registry>().is_none() {
            self.insert_resource(Registry::default());
        }
        let (read, write, check) = (fields.clone(), fields.clone(), fields);
        let (get_name, set_name) = (name.to_string(), name.to_string());
        let entry = ReflectedResource {
            name: name.to_string(),
            type_name: name.to_string(),
            get: Arc::new(move |w: &World| {
                // SAFETY: the pointer is to the live resource, `layout.size()` bytes.
                w.get_resource_dynamic(&get_name).map(|p| Ok(unsafe { read.read(p) }))
            }),
            set: Arc::new(move |w: &mut World, v| {
                let bytes = write.bytes(w.get_resource_dynamic(&set_name), &v)?;
                // SAFETY: `bytes` holds `layout.size()` bytes, a valid value as
                // `reflect_resource_fields`' caller promised.
                unsafe { w.insert_resource_dynamic(&set_name, layout, bytes.as_ptr()) };
                Ok(())
            }),
            check: Arc::new(move |v: &Value| check.bytes(None, v).map(drop)),
        };
        let reg = self.resource_mut::<Registry>();
        match reg.resource_by_name.get(name) {
            Some(&i) if reg.resources[i].type_name == name => reg.resources[i] = entry,
            Some(&i) => panic!(
                "World::reflect_resource_fields: `{name}` is already reflected as `{}`",
                reg.resources[i].type_name
            ),
            None => {
                reg.resource_by_name.insert(name.to_string(), reg.resources.len());
                reg.resources.push(entry);
            }
        }
    }

    fn save(&self) -> Value {
        let reflected = self.reflected_components();
        let mut entities = Vec::new();
        let mut not_saved: std::collections::BTreeMap<String, usize> = Default::default();
        for a in self.archetypes().iter().filter(|a| !a.is_empty()) {
            for &id in a.component_ids() {
                // Rust components are reflected under their short name, C ones
                // under the name they were registered with ("game::Health").
                let full = self.component_info(id).name();
                let name = short_name(full);
                if !reflected.contains(&name.as_str()) && !reflected.contains(&full) {
                    *not_saved.entry(name).or_default() += a.len();
                }
            }
            for &e in a.entities() {
                if let Ok(Value::Object(components)) = self.entity_json(e)
                    && !components.is_empty()
                {
                    entities.push((e.to_bits(), components));
                }
            }
        }
        entities.sort_unstable_by_key(|(id, _)| *id);
        let entities: Vec<Value> = entities
            .into_iter()
            .map(|(id, components)| serde_json::json!({ "entity": id, "components": components }))
            .collect();
        let mut resources = serde_json::Map::new();
        for r in registry(self).map_or(&[][..], |r| &r.resources) {
            if let Some(Ok(v)) = (r.get)(self) {
                resources.insert(r.name.clone(), v);
            }
        }
        serde_json::json!({
            "format": SAVE_FORMAT,
            "version": SAVE_VERSION,
            "entities": entities,
            "resources": resources,
            "not_saved": not_saved,
        })
    }

    fn load(&mut self, save: &Value) -> Result<(), ReflectError> {
        let bad = |message: String| ReflectError::BadValue {
            component: "<save>".into(),
            message,
        };
        if save["format"] != SAVE_FORMAT {
            return Err(bad(format!(
                "not a world save (expected \"format\": \"{SAVE_FORMAT}\")"
            )));
        }
        if save["version"].as_u64() != Some(SAVE_VERSION) {
            return Err(bad(format!(
                "unsupported save version {} (this engine reads {SAVE_VERSION})",
                save["version"]
            )));
        }
        let list = save["entities"]
            .as_array()
            .ok_or_else(|| bad("missing \"entities\" list".into()))?;

        // Check everything first, so a bad save leaves the world as it was.
        let mut plan = Vec::with_capacity(list.len());
        let mut seen = std::collections::HashSet::with_capacity(list.len());
        for item in list {
            let id = item["entity"]
                .as_u64()
                .ok_or_else(|| bad(format!("entity id missing or not a number: {}", item["entity"])))?;
            if !seen.insert(id) || Entity::from_bits(id).index() == u32::MAX {
                return Err(bad(format!("entity id {id} appears twice or is invalid")));
            }
            let Some(Value::Object(components)) = item.get("components") else {
                return Err(bad(format!("entity {id} has no \"components\" object")));
            };
            if components.is_empty() {
                return Err(bad(format!("entity {id} has no components")));
            }
            let mut parts = Vec::with_capacity(components.len());
            for (name, value) in components {
                let c = find(self, name)?;
                (c.check)(value).map_err(bad_value(name))?;
                parts.push((c, value.clone()));
            }
            plan.push((Entity::from_bits(id), parts));
        }
        // Saves from before resources were saved have none.
        let mut resource_plan = Vec::new();
        if let Some(saved) = save.get("resources") {
            let saved = saved
                .as_object()
                .ok_or_else(|| bad("\"resources\" is not an object".into()))?;
            for (name, value) in saved {
                let r = find_resource(self, name)?;
                (r.check)(value).map_err(bad_value(name))?;
                resource_plan.push((r, value.clone()));
            }
        }

        self.reset();
        for (e, parts) in plan {
            let mut parts = parts.into_iter();
            let (first, value) = parts.next().expect("checked: at least one component");
            let placed = (first.spawn_at)(self, e, value).map_err(bad_value(&first.name))?;
            debug_assert!(placed, "ids are unique and the world was reset");
            for (c, value) in parts {
                (c.set)(self, e, value).map_err(bad_value(&c.name))?;
            }
        }
        for (r, value) in resource_plan {
            (r.set)(self, value).map_err(bad_value(&r.name))?;
        }
        Ok(())
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

// ---- Sprites ------------------------------------------------------------------------

/// The asset names of textures loaded from files (`Ctx::load_texture`), so
/// sprites are saved as `"texture": "crab.png"`. A resource of the game's world.
#[derive(Default)]
pub(crate) struct TextureNames {
    pub(crate) by_id: HashMap<u32, String>,
    pub(crate) by_name: HashMap<String, crate::Texture>,
}

/// A [`Sprite`](crate::Sprite) as JSON: the texture by its file name.
#[derive(Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
struct SpriteJson {
    texture: Option<String>,
    color: crate::Color,
    z: f32,
    pivot: glam::Vec2,
    region: Option<crate::Rect>,
}

impl Default for SpriteJson {
    fn default() -> Self {
        Self {
            texture: None,
            color: crate::Color::WHITE,
            z: 0.0,
            pivot: glam::Vec2::splat(0.5),
            region: None,
        }
    }
}

/// `None` for a texture made in code: it has no file to name, so the sprite
/// isn't saved (the rest of the entity is).
fn sprite_json(world: &World, s: &crate::Sprite) -> Option<Result<Value, String>> {
    let name = world.get_resource::<TextureNames>()?.by_id.get(&s.texture.id)?;
    let json = SpriteJson {
        texture: Some(name.clone()),
        color: s.color,
        z: s.z,
        pivot: s.pivot,
        region: s.region,
    };
    Some(serde_json::to_value(json).map_err(|err| err.to_string()))
}

fn sprite_from_json(world: &World, value: Value) -> Result<crate::Sprite, String> {
    let json: SpriteJson = serde_json::from_value(value).map_err(|err| err.to_string())?;
    let name = json
        .texture
        .ok_or("a sprite needs \"texture\": an image in assets/, like \"crab.png\"")?;
    let texture = world
        .get_resource::<TextureNames>()
        .and_then(|t| t.by_name.get(&name))
        .copied()
        .ok_or_else(|| format!("texture \"{name}\" isn't loaded: ctx.load_texture(\"{name}\") first"))?;
    Ok(crate::Sprite {
        texture,
        color: json.color,
        z: json.z,
        pivot: json.pivot,
        region: json.region,
    })
}

/// Reflects [`Sprite`](crate::Sprite) as `"Sprite"`, its texture saved by
/// file name (see [`TextureNames`]).
pub(crate) fn reflect_sprite(world: &mut World) {
    if world.get_resource::<TextureNames>().is_none() {
        world.insert_resource(TextureNames::default());
    }
    if world.get_resource::<Registry>().is_none() {
        world.insert_resource(Registry::default());
    }
    let reg = world.resource_mut::<Registry>();
    if reg.by_name.contains_key("Sprite") {
        return;
    }
    reg.by_name.insert("Sprite".into(), reg.list.len());
    reg.list.push(Reflected {
        name: "Sprite".into(),
        type_name: std::any::type_name::<crate::Sprite>().into(),
        get: Arc::new(|w: &World, e| w.get::<crate::Sprite>(e).and_then(|s| sprite_json(w, s))),
        set: Arc::new(|w: &mut World, e, v| {
            let sprite = sprite_from_json(w, v)?;
            w.insert(e, sprite);
            Ok(())
        }),
        spawn: Arc::new(|w: &mut World, v| {
            let sprite = sprite_from_json(w, v)?;
            Ok(w.spawn((sprite,)))
        }),
        spawn_at: Arc::new(|w: &mut World, e, v| {
            let sprite = sprite_from_json(w, v)?;
            Ok(w.spawn_at(e, (sprite,)))
        }),
        check: Arc::new(check::<SpriteJson>),
        remove: Arc::new(remove::<crate::Sprite>),
    });
}

// ---- Models ------------------------------------------------------------------------

/// The asset names of meshes loaded from files (`Ctx::load_mesh`), so
/// models are saved as `"mesh": "tree.glb"`. A resource of the game's world.
#[cfg(feature = "3d")]
#[derive(Default)]
pub(crate) struct MeshNames {
    pub(crate) by_id: HashMap<u32, String>,
    pub(crate) by_name: HashMap<String, crate::Mesh>,
}

/// A [`Model`](crate::Model) as JSON: the mesh by its file name.
#[cfg(feature = "3d")]
#[derive(Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ModelJson {
    mesh: Option<String>,
    color: crate::Color,
}

#[cfg(feature = "3d")]
impl Default for ModelJson {
    fn default() -> Self {
        Self {
            mesh: None,
            color: crate::Color::WHITE,
        }
    }
}

/// `None` for a mesh made in code: it has no file to name, so the model
/// isn't saved (the rest of the entity is).
#[cfg(feature = "3d")]
fn model_json(world: &World, m: &crate::Model) -> Option<Result<Value, String>> {
    let name = world.get_resource::<MeshNames>()?.by_id.get(&m.mesh.id)?;
    let json = ModelJson {
        mesh: Some(name.clone()),
        color: m.color,
    };
    Some(serde_json::to_value(json).map_err(|err| err.to_string()))
}

#[cfg(feature = "3d")]
fn model_from_json(world: &World, value: Value) -> Result<crate::Model, String> {
    let json: ModelJson = serde_json::from_value(value).map_err(|err| err.to_string())?;
    let name = json
        .mesh
        .ok_or("a model needs \"mesh\": a glTF file in assets/, like \"tree.glb\"")?;
    let mesh = world
        .get_resource::<MeshNames>()
        .and_then(|t| t.by_name.get(&name))
        .copied()
        .ok_or_else(|| format!("mesh \"{name}\" isn't loaded: ctx.load_mesh(\"{name}\") first"))?;
    Ok(crate::Model {
        mesh,
        color: json.color,
    })
}

#[cfg(feature = "3d")]
#[derive(Serialize, serde::Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ShadowMeshJson {
    mesh: Option<String>,
}

#[cfg(feature = "3d")]
fn shadow_mesh_json(world: &World, s: &crate::ShadowMesh) -> Option<Result<Value, String>> {
    let mesh = match s.0 {
        Some(mesh) => Some(world.get_resource::<MeshNames>()?.by_id.get(&mesh.id)?.clone()),
        None => None,
    };
    Some(serde_json::to_value(ShadowMeshJson { mesh }).map_err(|err| err.to_string()))
}

#[cfg(feature = "3d")]
fn shadow_mesh_from_json(world: &World, value: Value) -> Result<crate::ShadowMesh, String> {
    let json: ShadowMeshJson = serde_json::from_value(value).map_err(|err| err.to_string())?;
    let Some(name) = json.mesh else {
        return Ok(crate::ShadowMesh(None));
    };
    let mesh = world
        .get_resource::<MeshNames>()
        .and_then(|t| t.by_name.get(&name))
        .copied()
        .ok_or_else(|| format!("mesh \"{name}\" isn't loaded: ctx.load_mesh(\"{name}\") first"))?;
    Ok(crate::ShadowMesh(Some(mesh)))
}

/// Reflects [`Model`](crate::Model) as `"Model"`, and
/// [`ShadowMesh`](crate::ShadowMesh) as `"ShadowMesh"`, their meshes saved
/// by file name (see [`MeshNames`]).
#[cfg(feature = "3d")]
pub(crate) fn reflect_model(world: &mut World) {
    if world.get_resource::<MeshNames>().is_none() {
        world.insert_resource(MeshNames::default());
    }
    if world.get_resource::<Registry>().is_none() {
        world.insert_resource(Registry::default());
    }
    let reg = world.resource_mut::<Registry>();
    if reg.by_name.contains_key("Model") {
        return;
    }
    reg.by_name.insert("Model".into(), reg.list.len());
    reg.list.push(Reflected {
        name: "Model".into(),
        type_name: std::any::type_name::<crate::Model>().into(),
        get: Arc::new(|w: &World, e| w.get::<crate::Model>(e).and_then(|m| model_json(w, m))),
        set: Arc::new(|w: &mut World, e, v| {
            let model = model_from_json(w, v)?;
            w.insert(e, model);
            Ok(())
        }),
        spawn: Arc::new(|w: &mut World, v| {
            let model = model_from_json(w, v)?;
            Ok(w.spawn((model,)))
        }),
        spawn_at: Arc::new(|w: &mut World, e, v| {
            let model = model_from_json(w, v)?;
            Ok(w.spawn_at(e, (model,)))
        }),
        check: Arc::new(check::<ModelJson>),
        remove: Arc::new(remove::<crate::Model>),
    });
    reg.by_name.insert("ShadowMesh".into(), reg.list.len());
    reg.list.push(Reflected {
        name: "ShadowMesh".into(),
        type_name: std::any::type_name::<crate::ShadowMesh>().into(),
        get: Arc::new(|w: &World, e| w.get::<crate::ShadowMesh>(e).and_then(|s| shadow_mesh_json(w, s))),
        set: Arc::new(|w: &mut World, e, v| {
            let shadow = shadow_mesh_from_json(w, v)?;
            w.insert(e, shadow);
            Ok(())
        }),
        spawn: Arc::new(|w: &mut World, v| {
            let shadow = shadow_mesh_from_json(w, v)?;
            Ok(w.spawn((shadow,)))
        }),
        spawn_at: Arc::new(|w: &mut World, e, v| {
            let shadow = shadow_mesh_from_json(w, v)?;
            Ok(w.spawn_at(e, (shadow,)))
        }),
        check: Arc::new(check::<ShadowMeshJson>),
        remove: Arc::new(remove::<crate::ShadowMesh>),
    });
}

// ---- Colliders ---------------------------------------------------------------

/// Collision meshes made from mesh files by loading scenes, with the
/// description they were made from (`{"file": "rock.glb", "scale": [1, 1,
/// 1], "convex": false}`), so their colliders are saved the same way. A
/// resource of the game's world.
#[cfg(feature = "physics3d")]
#[derive(Default)]
pub(crate) struct ColliderFiles {
    pub(crate) by_mesh: HashMap<crate::CollisionMesh, Value>,
    pub(crate) by_file: HashMap<String, crate::CollisionMesh>,
}

/// A collider's mesh description as a lookup key.
#[cfg(feature = "physics3d")]
pub(crate) fn collider_file_key(file: &Value) -> String {
    file.to_string()
}

/// A collider described by a mesh file, as JSON: its mesh's description.
#[cfg(feature = "physics3d")]
pub(crate) fn collider_file(value: &Value) -> Option<&Value> {
    value.pointer("/shape/Mesh").filter(|m| m.get("file").is_some())
}

#[cfg(feature = "physics3d")]
fn collider_json(world: &World, c: &crate::Collider3D) -> Option<Result<Value, String>> {
    let mut value = match serde_json::to_value(c) {
        Ok(v) => v,
        Err(err) => return Some(Err(err.to_string())),
    };
    if let crate::Shape3D::Mesh(mesh) = c.shape
        && let Some(file) = world.get_resource::<ColliderFiles>().and_then(|f| f.by_mesh.get(&mesh))
    {
        value["shape"] = serde_json::json!({ "Mesh": file });
    }
    Some(Ok(value))
}

#[cfg(feature = "physics3d")]
fn collider_from_json(world: &World, mut value: Value) -> Result<crate::Collider3D, String> {
    if let Some(file) = collider_file(&value) {
        let mesh = world
            .get_resource::<ColliderFiles>()
            .and_then(|f| f.by_file.get(&collider_file_key(file)))
            .copied()
            .ok_or_else(|| format!("collision mesh {file} isn't loaded: load the scene with ctx.load_scene"))?;
        value["shape"] = serde_json::json!({ "Mesh": mesh });
    }
    serde_json::from_value(value).map_err(|err| err.to_string())
}

/// Reflects [`Collider3D`](crate::Collider3D) as `"Collider3D"`; a mesh
/// made from a file (by loading a scene) is saved by that file.
#[cfg(feature = "physics3d")]
pub(crate) fn reflect_collider(world: &mut World) {
    if world.get_resource::<ColliderFiles>().is_none() {
        world.insert_resource(ColliderFiles::default());
    }
    if world.get_resource::<Registry>().is_none() {
        world.insert_resource(Registry::default());
    }
    let reg = world.resource_mut::<Registry>();
    if reg.by_name.contains_key("Collider3D") {
        return;
    }
    reg.by_name.insert("Collider3D".into(), reg.list.len());
    reg.list.push(Reflected {
        name: "Collider3D".into(),
        type_name: std::any::type_name::<crate::Collider3D>().into(),
        get: Arc::new(|w: &World, e| w.get::<crate::Collider3D>(e).and_then(|c| collider_json(w, c))),
        set: Arc::new(|w: &mut World, e, v| {
            let c = collider_from_json(w, v)?;
            w.insert(e, c);
            Ok(())
        }),
        spawn: Arc::new(|w: &mut World, v| {
            let c = collider_from_json(w, v)?;
            Ok(w.spawn((c,)))
        }),
        spawn_at: Arc::new(|w: &mut World, e, v| {
            let c = collider_from_json(w, v)?;
            Ok(w.spawn_at(e, (c,)))
        }),
        check: Arc::new(|v: &Value| {
            let mut v = v.clone();
            if collider_file(&v).is_some() {
                v["shape"] = serde_json::json!({ "Mesh": crate::CollisionMesh::default() });
            }
            check::<crate::Collider3D>(&v)
        }),
        remove: Arc::new(remove::<crate::Collider3D>),
    });
}

/// The asset names under `key` objects' `field` anywhere in `json` (a
/// scene, a prefab, or an entity's components): the textures of `"Sprite"`s
/// and meshes of `"Model"`s, for loading them first.
pub(crate) fn asset_names(json: &Value, key: &str, field: &str, out: &mut Vec<String>) {
    match json {
        Value::Object(map) => {
            for (k, value) in map {
                if k == key
                    && let Some(name) = value.get(field).and_then(Value::as_str)
                {
                    out.push(name.to_string());
                } else {
                    asset_names(value, key, field, out);
                }
            }
        }
        Value::Array(list) => list.iter().for_each(|v| asset_names(v, key, field, out)),
        _ => {}
    }
}
