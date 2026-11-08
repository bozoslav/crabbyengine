//! Scenes and prefabs flattened: every GameObject, nested prefab
//! instances expanded with their overrides applied.

use std::collections::HashMap;

use crate::{Error, ObjRef, Project, UnityFile, UnityObject, Value};

const GAME_OBJECT: i32 = 1;
const TRANSFORM: i32 = 4;
const RECT_TRANSFORM: i32 = 224;
const PREFAB_INSTANCE: i32 = 1001;

/// A scene or prefab as plain GameObjects: what Unity shows in its
/// hierarchy, with every nested prefab expanded and its overrides applied.
///
/// ```
/// let text = "--- !u!1 &1\nGameObject:\n  m_Name: Lamp\n  m_IsActive: 1\n--- !u!4 &2\nTransform:\n  m_GameObject: {fileID: 1}\n  m_LocalPosition: {x: 1, y: 2, z: 3}\n  m_Father: {fileID: 0}\n";
/// let file = ponko_unity::UnityFile::parse(text).unwrap();
/// let scene = ponko_unity::Hierarchy::build(&file, &ponko_unity::Project::default()).unwrap();
/// assert_eq!(scene.objects[0].name, "Lamp");
/// assert_eq!(scene.objects[0].local_position(), [1.0, 2.0, 3.0]);
/// ```
#[derive(Debug, Clone, Default)]
pub struct Hierarchy {
    /// Every GameObject; parents before their children.
    pub objects: Vec<GameObject>,
}

/// One GameObject.
#[derive(Debug, Clone)]
pub struct GameObject {
    /// Its id in the file the hierarchy was built from (objects of nested
    /// prefabs get the ids Unity gives them there).
    pub id: i64,
    /// `m_Name`.
    pub name: String,
    /// `m_IsActive`.
    pub active: bool,
    /// `m_Layer`.
    pub layer: i32,
    /// Its parent's index in [`Hierarchy::objects`].
    pub parent: Option<usize>,
    /// Its components, the transform first, with overrides applied.
    pub components: Vec<UnityObject>,
    /// For an instance of an imported model (FBX, glTF, ...): the model's
    /// reference. Its own hierarchy lives in the model file; this object
    /// stands for its root.
    pub model: Option<ObjRef>,
    /// For a model instance: the overrides of objects inside the model
    /// (`(target, property path, value)`), for the importer to apply.
    pub model_overrides: Vec<(ObjRef, String, Value)>,
    /// When it hangs from an object inside an imported model: that
    /// object (its [`parent`](Self::parent) is then the model's root).
    pub model_attach: Option<ObjRef>,
}

impl GameObject {
    /// Its transform's fields (`Transform` or `RectTransform`).
    #[must_use]
    pub fn transform(&self) -> Option<&Value> {
        self.components
            .iter()
            .find(|c| c.class_id == TRANSFORM || c.class_id == RECT_TRANSFORM)
            .map(|c| &c.body)
    }

    /// Position relative to its parent, in Unity's space.
    #[must_use]
    pub fn local_position(&self) -> [f32; 3] {
        self.transform()
            .and_then(|t| t["m_LocalPosition"].vec3())
            .unwrap_or([0.0; 3])
    }

    /// Rotation relative to its parent (x, y, z, w), in Unity's space.
    #[must_use]
    pub fn local_rotation(&self) -> [f32; 4] {
        self.transform()
            .and_then(|t| t["m_LocalRotation"].vec4())
            .unwrap_or([0.0, 0.0, 0.0, 1.0])
    }

    /// Scale relative to its parent.
    #[must_use]
    pub fn local_scale(&self) -> [f32; 3] {
        self.transform()
            .and_then(|t| t["m_LocalScale"].vec3())
            .unwrap_or([1.0; 3])
    }

    /// Its first component of this class (23 MeshRenderer, 33 MeshFilter, ...).
    #[must_use]
    pub fn component(&self, class_id: i32) -> Option<&UnityObject> {
        self.components.iter().find(|c| c.class_id == class_id)
    }
}

/// Every object of a file with its prefab instances expanded, by id.
#[derive(Clone, Default)]
struct Flat {
    objects: Vec<Obj>,
    by_id: HashMap<i64, usize>,
}

#[derive(Clone)]
struct Obj {
    object: UnityObject,
    model: Option<ObjRef>,
    model_overrides: Vec<(ObjRef, String, Value)>,
    /// For a transform under an object inside an imported model: that object.
    model_attach: Option<ObjRef>,
}

impl Flat {
    fn insert(&mut self, obj: Obj) {
        let id = obj.object.file_id;
        if let Some(&i) = self.by_id.get(&id) {
            self.objects[i] = obj;
        } else {
            self.by_id.insert(id, self.objects.len());
            self.objects.push(obj);
        }
    }

    fn get_mut(&mut self, id: i64) -> Option<&mut Obj> {
        self.by_id.get(&id).map(|&i| &mut self.objects[i])
    }
}

/// The id Unity gives object `inner` of the prefab that instance
/// `instance` places.
fn nested_id(instance: i64, inner: i64) -> i64 {
    (instance ^ inner) & i64::MAX
}

impl Hierarchy {
    /// Flattens a scene or prefab. Prefab instances are loaded through
    /// `project` (by GUID); instances of imported models stay one object
    /// each (see [`GameObject::model`]).
    ///
    /// # Errors
    ///
    /// If a nested prefab can't be loaded, or prefabs nest in a loop.
    pub fn build(file: &UnityFile, project: &Project) -> Result<Self, Error> {
        let mut cache = HashMap::new();
        let flat = flatten(file, project, &mut cache, &mut Vec::new())?;
        Ok(Self::from_flat(&flat))
    }

    /// Loads the scene or prefab with this GUID and flattens it.
    ///
    /// # Errors
    ///
    /// See [`Hierarchy::build`].
    pub fn load(project: &Project, guid: &str) -> Result<Self, Error> {
        Self::build(&project.load(guid)?, project)
    }

    fn from_flat(flat: &Flat) -> Self {
        // Components by the GameObject they're on.
        let mut components: HashMap<i64, Vec<&Obj>> = HashMap::new();
        for o in &flat.objects {
            if o.object.class_id != GAME_OBJECT
                && let Some(go) = o.object.body["m_GameObject"].obj_ref()
                && !go.is_none()
            {
                components.entry(go.file_id).or_default().push(o);
            }
        }
        let mut gos: Vec<&Obj> = flat
            .objects
            .iter()
            .filter(|o| o.object.class_id == GAME_OBJECT && !o.object.stripped)
            .collect();
        // Which GameObject owns each transform, and each one's parent.
        let mut owner_of_transform = HashMap::new();
        for go in &gos {
            for c in components.get(&go.object.file_id).into_iter().flatten() {
                if matches!(c.object.class_id, TRANSFORM | RECT_TRANSFORM) {
                    owner_of_transform.insert(c.object.file_id, go.object.file_id);
                }
            }
        }
        let parent_of = |go: i64| -> Option<i64> {
            let t = components
                .get(&go)?
                .iter()
                .find(|c| matches!(c.object.class_id, TRANSFORM | RECT_TRANSFORM))?;
            let father = t.object.body["m_Father"].obj_ref()?;
            owner_of_transform.get(&father.file_id).copied()
        };
        // Parents first: sort by depth, keeping file order otherwise.
        let depth = |mut go: i64| {
            let mut d = 0;
            while let Some(p) = parent_of(go) {
                d += 1;
                go = p;
                if d > 1000 {
                    break; // a loop in broken data
                }
            }
            d
        };
        gos.sort_by_key(|o| depth(o.object.file_id));
        let transform_of = |go: i64| {
            components
                .get(&go)?
                .iter()
                .find(|c| matches!(c.object.class_id, TRANSFORM | RECT_TRANSFORM))
                .copied()
        };
        let index: HashMap<i64, usize> = gos.iter().enumerate().map(|(i, o)| (o.object.file_id, i)).collect();
        let objects = gos
            .iter()
            .map(|o| {
                let b = &o.object.body;
                let mut comps: Vec<UnityObject> = components
                    .get(&o.object.file_id)
                    .into_iter()
                    .flatten()
                    .map(|c| c.object.clone())
                    .collect();
                comps.sort_by_key(|c| !matches!(c.class_id, TRANSFORM | RECT_TRANSFORM));
                GameObject {
                    id: o.object.file_id,
                    name: b["m_Name"].as_str().unwrap_or_default().to_owned(),
                    active: b["m_IsActive"].as_bool().unwrap_or(true),
                    layer: b["m_Layer"].as_i64().unwrap_or(0) as i32,
                    parent: parent_of(o.object.file_id).and_then(|p| index.get(&p).copied()),
                    components: comps,
                    model: o.model.clone(),
                    model_overrides: o.model_overrides.clone(),
                    model_attach: transform_of(o.object.file_id).and_then(|t| t.model_attach.clone()),
                }
            })
            .collect();
        Self { objects }
    }
}

fn flatten(
    file: &UnityFile,
    project: &Project,
    cache: &mut HashMap<String, Flat>,
    loading: &mut Vec<String>,
) -> Result<Flat, Error> {
    let mut flat = Flat::default();
    for o in &file.objects {
        if o.class_id != PREFAB_INSTANCE && !o.stripped {
            flat.insert(Obj {
                object: o.clone(),
                model: None,
                model_overrides: Vec::new(),
                model_attach: None,
            });
        }
    }
    let mut model_instances = std::collections::HashSet::new();
    for instance in file.of_class(PREFAB_INSTANCE) {
        if expand(instance, project, cache, loading, &mut flat)? {
            model_instances.insert(instance.file_id);
        }
    }
    // A stripped object stands for one placed by a prefab instance:
    // references to it point at that object's copy.
    let mut aliases = HashMap::new();
    let mut inner_targets = HashMap::new();
    for o in file.objects.iter().filter(|o| o.stripped) {
        if let (Some(source), Some(instance)) = (
            o.body["m_CorrespondingSourceObject"].obj_ref(),
            o.body["m_PrefabInstance"].obj_ref(),
        ) {
            // Inside an imported model: its root stands in (the model's
            // own hierarchy isn't expanded), and what hangs there keeps
            // the inner object in `model_attach`.
            let real = if model_instances.contains(&instance.file_id) {
                inner_targets.insert(o.file_id, source.clone());
                nested_id(instance.file_id, if o.class_id == GAME_OBJECT { 1 } else { 2 })
            } else {
                nested_id(instance.file_id, source.file_id)
            };
            if real != o.file_id {
                aliases.insert(o.file_id, real);
            }
        }
    }
    for o in &mut flat.objects {
        if let Some(father) = o.object.body["m_Father"].obj_ref()
            && let Some(inner) = inner_targets.get(&father.file_id)
        {
            o.model_attach = Some(inner.clone());
        }
    }
    if !aliases.is_empty() {
        for o in &mut flat.objects {
            rename(&mut o.object.body, &aliases);
        }
    }
    Ok(flat)
}

/// Swaps local references to stripped stand-ins for the objects they
/// stand for.
fn rename(v: &mut Value, aliases: &HashMap<i64, i64>) {
    match v {
        Value::Map(entries) => {
            let local = entries.iter().all(|(k, _)| k != "guid");
            for (k, c) in entries.iter_mut() {
                if k == "fileID" && local {
                    if let Value::Scalar(s) = c
                        && let Some(real) = s.parse::<i64>().ok().and_then(|id| aliases.get(&id))
                    {
                        *s = real.to_string();
                    }
                } else {
                    rename(c, aliases);
                }
            }
        }
        Value::Seq(items) => items.iter_mut().for_each(|c| rename(c, aliases)),
        _ => {}
    }
}

/// Adds the objects that prefab instance `instance` places to `flat`;
/// true when it's an imported model's instance.
fn expand(
    instance: &UnityObject,
    project: &Project,
    cache: &mut HashMap<String, Flat>,
    loading: &mut Vec<String>,
    flat: &mut Flat,
) -> Result<bool, Error> {
    let b = &instance.body;
    let modification = &b["m_Modification"];
    let source = b["m_SourcePrefab"]
        .obj_ref()
        .and_then(|r| r.guid)
        .ok_or_else(|| Error::new(format!("prefab instance {} has no source prefab", instance.file_id)))?;
    let parent = modification["m_TransformParent"].obj_ref().unwrap_or(ObjRef {
        file_id: 0,
        guid: None,
        kind: None,
    });
    let modifications: Vec<(ObjRef, String, Value)> = modification["m_Modifications"]
        .as_seq()
        .unwrap_or_default()
        .iter()
        .filter_map(|m| {
            let target = m["target"].obj_ref()?;
            let path = m["propertyPath"].as_str()?.to_owned();
            let value = match m["objectReference"].obj_ref() {
                Some(r) if !r.is_none() => m["objectReference"].clone(),
                _ => m["value"].clone(),
            };
            Some((target, path, value))
        })
        .collect();
    let is_prefab = project
        .path_of(&source)
        .and_then(|p| p.extension())
        .is_some_and(|e| e == "prefab");
    if !is_prefab {
        // An imported model: one GameObject and transform for its root.
        let go_id = nested_id(instance.file_id, 1);
        let t_id = nested_id(instance.file_id, 2);
        let mut transform = Value::Map(vec![
            ("m_GameObject".into(), id_ref(go_id)),
            ("m_LocalRotation".into(), xyzw(0.0, 0.0, 0.0, 1.0)),
            ("m_LocalPosition".into(), xyzw(0.0, 0.0, 0.0, f32::NAN)),
            ("m_LocalScale".into(), xyzw(1.0, 1.0, 1.0, f32::NAN)),
            ("m_Father".into(), id_ref(parent.file_id)),
        ]);
        let mut go = Value::Map(vec![
            ("m_Name".into(), Value::Scalar(String::new())),
            ("m_IsActive".into(), Value::Scalar("1".into())),
        ]);
        // The root's overrides are the ones that set its place (Unity
        // always writes them); the rest belong to objects inside.
        let root = modifications
            .iter()
            .find(|(_, p, _)| p == "m_LocalPosition.x")
            .map(|(t, ..)| t.file_id);
        let mut inside = Vec::new();
        for (target, path, value) in &modifications {
            if Some(target.file_id) == root && path.starts_with("m_Local") {
                set_path(&mut transform, path, value.clone());
            } else if path == "m_Name" && root.is_some() && name_target(&modifications, target) {
                set_path(&mut go, path, value.clone());
            } else {
                inside.push((target.clone(), path.clone(), value.clone()));
            }
        }
        if go["m_Name"].as_str() == Some("") {
            let name = project
                .path_of(&source)
                .and_then(|p| p.file_stem())
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            set_path(&mut go, "m_Name", Value::Scalar(name));
        }
        flat.insert(Obj {
            object: UnityObject {
                class_id: GAME_OBJECT,
                file_id: go_id,
                stripped: false,
                type_name: "GameObject".into(),
                body: go,
            },
            model: b["m_SourcePrefab"].obj_ref(),
            model_overrides: inside,
            model_attach: None,
        });
        flat.insert(Obj {
            object: UnityObject {
                class_id: TRANSFORM,
                file_id: t_id,
                stripped: false,
                type_name: "Transform".into(),
                body: transform,
            },
            model: None,
            model_overrides: Vec::new(),
            model_attach: None,
        });
        return Ok(true);
    }
    // A prefab: its own flattened objects, overridden, then re-keyed.
    if loading.contains(&source) {
        return Err(Error::new(format!("prefabs nest in a loop through {source}")));
    }
    let mut inner = match cache.get(&source) {
        Some(f) => f.clone(),
        None => {
            loading.push(source.clone());
            let file = project.load(&source)?;
            let f = flatten(&file, project, cache, loading)?;
            loading.pop();
            cache.insert(source.clone(), f.clone());
            f
        }
    };
    for (target, path, value) in &modifications {
        if let Some(o) = inner.get_mut(target.file_id) {
            set_path(&mut o.object.body, path, value.clone());
        }
    }
    let removed: Vec<i64> = ["m_RemovedComponents", "m_RemovedGameObjects"]
        .iter()
        .flat_map(|k| modification[*k].as_seq().unwrap_or_default())
        .filter_map(|r| r.obj_ref().map(|r| r.file_id))
        .collect();
    let roots: Vec<i64> = inner
        .objects
        .iter()
        .filter(|o| {
            matches!(o.object.class_id, TRANSFORM | RECT_TRANSFORM)
                && o.object.body["m_Father"].obj_ref().is_none_or(|f| f.is_none())
        })
        .map(|o| o.object.file_id)
        .collect();
    let ids: std::collections::HashSet<i64> = inner.by_id.keys().copied().collect();
    for mut o in inner.objects {
        if removed.contains(&o.object.file_id) {
            continue;
        }
        let was_root = roots.contains(&o.object.file_id);
        remap(&mut o.object.body, &ids, instance.file_id);
        if was_root {
            set_path(&mut o.object.body, "m_Father", id_ref(parent.file_id));
        }
        o.object.file_id = nested_id(instance.file_id, o.object.file_id);
        o.object.stripped = false;
        flat.insert(o);
    }
    Ok(false)
}

/// Whether the `m_Name` override `target` is the model root's GameObject:
/// Unity writes it for the root only, so the one `m_Name` target is it.
fn name_target(mods: &[(ObjRef, String, Value)], target: &ObjRef) -> bool {
    mods.iter().filter(|(_, p, _)| p == "m_Name").count() == 1
        && mods.iter().any(|(t, p, _)| t == target && p == "m_Name")
}

fn id_ref(id: i64) -> Value {
    Value::Map(vec![("fileID".into(), Value::Scalar(id.to_string()))])
}

fn xyzw(x: f32, y: f32, z: f32, w: f32) -> Value {
    let mut entries = vec![
        ("x".into(), Value::Scalar(x.to_string())),
        ("y".into(), Value::Scalar(y.to_string())),
        ("z".into(), Value::Scalar(z.to_string())),
    ];
    if !w.is_nan() {
        entries.push(("w".into(), Value::Scalar(w.to_string())));
    }
    Value::Map(entries)
}

/// Re-keys references within a nested prefab's objects to their ids in
/// the outer file.
fn remap(v: &mut Value, ids: &std::collections::HashSet<i64>, instance: i64) {
    match v {
        Value::Map(entries) => {
            let local = entries.iter().all(|(k, _)| k != "guid");
            for (k, c) in entries.iter_mut() {
                if k == "fileID" && local {
                    if let Value::Scalar(s) = c
                        && let Ok(id) = s.parse::<i64>()
                        && ids.contains(&id)
                    {
                        *s = nested_id(instance, id).to_string();
                    }
                } else {
                    remap(c, ids, instance);
                }
            }
        }
        Value::Seq(items) => items.iter_mut().for_each(|c| remap(c, ids, instance)),
        _ => {}
    }
}

/// Sets a field by Unity's property path: `m_LocalPosition.x`,
/// `m_Materials.Array.data[1]`, `m_Materials.Array.size`.
pub(crate) fn set_path(v: &mut Value, path: &str, value: Value) {
    let mut parts = path.split('.').peekable();
    let mut at = v;
    while let Some(part) = parts.next() {
        if part == "Array" {
            let Some(next) = parts.next() else { return };
            if !matches!(at, Value::Seq(_)) {
                *at = Value::Seq(Vec::new());
            }
            let Value::Seq(items) = at else { unreachable!() };
            if next == "size" {
                if let Some(n) = value.as_i64() {
                    items.resize(n.max(0) as usize, Value::Null);
                }
                return;
            }
            let Some(i) = next
                .strip_prefix("data[")
                .and_then(|s| s.strip_suffix(']'))
                .and_then(|s| s.parse::<usize>().ok())
            else {
                return;
            };
            if items.len() <= i {
                items.resize(i + 1, Value::Null);
            }
            at = &mut items[i];
            if parts.peek().is_none() {
                *at = value;
                return;
            }
            continue;
        }
        if !matches!(at, Value::Map(_)) {
            *at = Value::Map(Vec::new());
        }
        let Value::Map(entries) = at else { unreachable!() };
        let i = match entries.iter().position(|(k, _)| k == part) {
            Some(i) => i,
            None => {
                entries.push((part.to_owned(), Value::Null));
                entries.len() - 1
            }
        };
        if parts.peek().is_none() {
            entries[i].1 = value;
            return;
        }
        at = &mut entries[i].1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> Value {
        Value::Scalar(x.into())
    }

    #[test]
    fn property_paths_set_fields_and_arrays() {
        let mut v = Value::Map(vec![("m_LocalPosition".into(), xyzw(1.0, 2.0, 3.0, f32::NAN))]);
        set_path(&mut v, "m_LocalPosition.y", s("9"));
        assert_eq!(v["m_LocalPosition"].vec3(), Some([1.0, 9.0, 3.0]));
        set_path(&mut v, "m_Materials.Array.data[1]", s("b"));
        assert_eq!(v["m_Materials"].as_seq().unwrap().len(), 2);
        assert_eq!(v["m_Materials"][1].as_str(), Some("b"));
        set_path(&mut v, "m_Materials.Array.size", s("1"));
        assert_eq!(v["m_Materials"].as_seq().unwrap().len(), 1);
        set_path(&mut v, "m_New.inner", s("x"));
        assert_eq!(v["m_New"]["inner"].as_str(), Some("x"));
    }

    #[test]
    fn nested_ids_follow_unitys_rule() {
        assert_eq!(nested_id(0b1100, 0b1010), 0b0110);
        assert!(nested_id(-1, 5) >= 0, "the sign bit is cleared");
    }
}
