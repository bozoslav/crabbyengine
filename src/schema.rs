//! Opt-in structural schemas for reflected types, so tools can learn a
//! component's or resource's fields before reading or writing it.
//!
//! `world.reflect::<T>()` accepts any Serde type, and Serde can't say what
//! shape a type serializes to (custom serializers, `#[serde(flatten)]`,
//! untagged enums...). So a type's schema is only reported when it is known
//! for sure: described with [`WorldReflect::describe_component`] /
//! [`WorldReflect::describe_resource`], or derived from the fields of
//! [`WorldReflect::reflect_fields`]. Everything else is reported as opaque.
//!
//! [`WorldReflect::describe_component`]: crate::WorldReflect::describe_component
//! [`WorldReflect::describe_resource`]: crate::WorldReflect::describe_resource
//! [`WorldReflect::reflect_fields`]: crate::WorldReflect::reflect_fields

use serde_json::{Value, json};

use crate::reflect::{Field, FieldType};

/// The JSON shape of a reflected value, described by the game.
///
/// [`to_json_schema`](Self::to_json_schema) turns it into a JSON Schema
/// fragment, which is what tools receive.
///
/// This type itself serializes using Serde's externally tagged enum encoding,
/// shared with the C/C++ description APIs: `"Number"`,
/// `{"Optional":"String"}`, or `{"Object":[["hp","Number"]]}`.
/// This metadata encoding is distinct from the JSON Schema returned by discovery.
///
/// ```
/// use ponko::*;
///
/// let health = ValueSchema::object([
///     ("current", ValueSchema::Number),
///     ("max", ValueSchema::Number),
/// ]);
/// assert_eq!(health.to_json_schema()["properties"]["max"]["type"], "number");
/// ```
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ValueSchema {
    /// Shape not known; any JSON may appear here.
    Opaque,
    /// JSON null (including unit marker components).
    Null,
    /// A fixed-length sequence of possibly different shapes.
    Tuple(Vec<ValueSchema>),
    /// Any one of these shapes.
    OneOf(Vec<ValueSchema>),
    /// `true` or `false`.
    Bool,
    /// A whole number.
    Integer,
    /// A whole number, 0 or more.
    Unsigned,
    /// Any number.
    Number,
    /// A string.
    String,
    /// An entity id (a non-negative integer).
    Entity,
    /// `[x, y]`, like [`Vec2`](crate::Vec2).
    Vec2,
    /// `[x, y, z]`, like [`Vec3`](crate::Vec3).
    Vec3,
    /// `{ "r", "g", "b", "a" }`, like [`Color`](crate::Color).
    Color,
    /// One of these strings (a fieldless enum).
    Enum(Vec<String>),
    /// A list of values of one shape.
    Array(Box<ValueSchema>),
    /// The value, or `null`.
    Optional(Box<ValueSchema>),
    /// An object with these fields, in order.
    Object(Vec<(String, ValueSchema)>),
}

impl ValueSchema {
    /// An object with these fields.
    pub fn object<N: Into<String>>(fields: impl IntoIterator<Item = (N, ValueSchema)>) -> Self {
        Self::Object(fields.into_iter().map(|(n, s)| (n.into(), s)).collect())
    }

    /// One of these strings.
    pub fn one_of<N: Into<String>>(names: impl IntoIterator<Item = N>) -> Self {
        Self::Enum(names.into_iter().map(Into::into).collect())
    }

    /// A list of `item`.
    #[must_use]
    pub fn array(item: ValueSchema) -> Self {
        Self::Array(Box::new(item))
    }

    /// `inner`, or `null`.
    #[must_use]
    pub fn optional(inner: ValueSchema) -> Self {
        Self::Optional(Box::new(inner))
    }

    /// A JSON Schema fragment for this shape. [`Opaque`](Self::Opaque)
    /// parts become `{}` (anything) marked `"x-ponko-opaque": true`.
    #[must_use]
    pub fn to_json_schema(&self) -> Value {
        let numbers =
            |n: usize| json!({ "type": "array", "items": { "type": "number" }, "minItems": n, "maxItems": n });
        match self {
            Self::Null => json!({"type": "null"}),
            Self::Tuple(items) => {
                json!({"type": "array", "prefixItems": items.iter().map(Self::to_json_schema).collect::<Vec<_>>(), "minItems": items.len(), "maxItems": items.len()})
            }
            Self::OneOf(items) => json!({"anyOf": items.iter().map(Self::to_json_schema).collect::<Vec<_>>()}),
            Self::Opaque => json!({ "x-ponko-opaque": true }),
            Self::Bool => json!({ "type": "boolean" }),
            Self::Integer => json!({ "type": "integer" }),
            Self::Unsigned => json!({ "type": "integer", "minimum": 0 }),
            Self::Number => json!({ "type": "number" }),
            Self::String => json!({ "type": "string" }),
            Self::Entity => json!({ "type": "integer", "minimum": 0, "format": "entity" }),
            Self::Vec2 => numbers(2),
            Self::Vec3 => numbers(3),
            Self::Color => Self::object(["r", "g", "b", "a"].map(|c| (c, Self::Number))).to_json_schema(),
            Self::Enum(names) => json!({ "type": "string", "enum": names }),
            Self::Array(item) => json!({ "type": "array", "items": item.to_json_schema() }),
            Self::Optional(inner) => json!({ "anyOf": [inner.to_json_schema(), { "type": "null" }] }),
            Self::Object(fields) => {
                let props: serde_json::Map<String, Value> =
                    fields.iter().map(|(n, s)| (n.clone(), s.to_json_schema())).collect();
                json!({ "type": "object", "properties": props })
            }
        }
    }

    /// The schema of a component or resource described by `fields`.
    pub(crate) fn from_fields(fields: &[Field]) -> Self {
        Self::Object(
            fields
                .iter()
                .map(|f| {
                    let s = match f.ty {
                        FieldType::F32 | FieldType::F64 => Self::Number,
                        FieldType::I8 | FieldType::I16 | FieldType::I32 | FieldType::I64 => Self::Integer,
                        FieldType::U8 | FieldType::U16 | FieldType::U32 | FieldType::U64 => Self::Unsigned,
                        FieldType::Bool => Self::Bool,
                        FieldType::Vec2 => Self::Vec2,
                        FieldType::Color => Self::Color,
                        FieldType::Entity => Self::Entity,
                    };
                    (f.name.clone(), s)
                })
                .collect(),
        )
    }
}

/// Where a reflected type's schema came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SchemaSource {
    /// `describe_component` / `describe_resource`.
    Described,
    /// Derived from `reflect_fields` / `reflect_resource_fields`.
    Fields,
}

/// The `schema` entry tools get for one reflected type: the JSON Schema
/// when it is known, else an explicit `"opaque"` status and why.
pub(crate) fn schema_json(known: Option<&(ValueSchema, SchemaSource)>) -> Value {
    match known {
        Some((schema, source)) => json!({
            "status": "described",
            "source": match source {
                SchemaSource::Described => "described",
                SchemaSource::Fields => "fields",
            },
            "json_schema": schema.to_json_schema(),
        }),
        None => json!({
            "status": "opaque",
            "reason": "reflected through Serde without schema metadata; its JSON shape is not known (describe it with describe_component / describe_resource)",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_schemas_become_json_schema() {
        let s = ValueSchema::object([
            ("tags", ValueSchema::array(ValueSchema::String)),
            ("target", ValueSchema::optional(ValueSchema::Entity)),
            ("team", ValueSchema::one_of(["Red", "Blue"])),
            ("extra", ValueSchema::Opaque),
        ]);
        let j = s.to_json_schema();
        assert_eq!(j["properties"]["tags"]["items"]["type"], "string");
        assert_eq!(j["properties"]["target"]["anyOf"][1]["type"], "null");
        assert_eq!(j["properties"]["team"]["enum"], json!(["Red", "Blue"]));
        assert_eq!(j["properties"]["extra"]["x-ponko-opaque"], true);
    }

    #[test]
    fn field_types_map_to_json_types() {
        let s = ValueSchema::from_fields(&[
            Field::new("hp", FieldType::F32, 0),
            Field::new("count", FieldType::U32, 4),
            Field::new("delta", FieldType::I16, 8),
            Field::new("alive", FieldType::Bool, 10),
            Field::new("at", FieldType::Vec2, 12),
        ]);
        let p = &s.to_json_schema()["properties"];
        assert_eq!(p["hp"]["type"], "number");
        assert_eq!(
            (p["count"]["type"].clone(), p["count"]["minimum"].clone()),
            (json!("integer"), json!(0))
        );
        assert_eq!(p["delta"]["type"], "integer");
        assert_eq!(p["alive"]["type"], "boolean");
        assert_eq!(p["at"]["maxItems"], 2);
    }
}
