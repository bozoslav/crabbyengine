//! Explicit schemas for built-in serialization contracts. Never infer from sampled values.
#![allow(dead_code)]
use crate::ValueSchema::{self, *};
fn transform2d() -> ValueSchema {
    ValueSchema::object([("position", Vec2), ("rotation", Number), ("scale", Vec2)])
}
fn attached() -> ValueSchema {
    ValueSchema::object([("to", Entity), ("offset", Vec2), ("rotation", Number)])
}
fn transform3d() -> ValueSchema {
    ValueSchema::object([
        ("rotation", Tuple(vec![Number; 4])),
        ("position", Vec3),
        ("scale", Vec3),
    ])
}
fn pointlight3d() -> ValueSchema {
    ValueSchema::object([
        ("color", Color),
        ("intensity", Number),
        ("range", Number),
        ("shadows", Bool),
        ("shadow_resolution", Unsigned),
    ])
}
fn spotlight3d() -> ValueSchema {
    ValueSchema::object([
        ("color", Color),
        ("intensity", Number),
        ("range", Number),
        ("inner_angle", Number),
        ("outer_angle", Number),
        ("shadows", Bool),
        ("shadow_resolution", Unsigned),
    ])
}
fn decal3d() -> ValueSchema {
    ValueSchema::object([
        ("size", Vec3),
        ("image", Unsigned),
        ("normal_image", Unsigned),
        ("uv_min", Vec2),
        ("uv_max", Vec2),
        ("color", Color),
        ("opacity", Number),
        ("normal_strength", Number),
        ("roughness", Number),
        ("metallic", Number),
        ("angle_fade_start", Number),
        ("angle_fade_end", Number),
        ("depth_fade", Number),
        ("max_distance", Number),
        ("order", Integer),
        ("layers", Unsigned),
    ])
}
fn lodhysteresis() -> ValueSchema {
    Number
}
fn receivesdecals() -> ValueSchema {
    Unsigned
}
fn rigidbody() -> ValueSchema {
    ValueSchema::object([
        ("kind", ValueSchema::one_of(["Dynamic", "Fixed", "Kinematic"])),
        ("gravity_scale", Number),
        ("linear_damping", Number),
        ("angular_damping", Number),
        ("lock_rotation", Bool),
        ("ccd", Bool),
    ])
}
fn collider() -> ValueSchema {
    ValueSchema::object([
        ("shape", shape2d()),
        ("density", Number),
        ("friction", Number),
        ("restitution", Number),
        ("sensor", Bool),
        ("offset", Vec2),
    ])
}
fn velocity() -> ValueSchema {
    ValueSchema::object([("linear", Vec2), ("angular", Number)])
}
fn rigidbody3d() -> ValueSchema {
    ValueSchema::object([
        ("kind", ValueSchema::one_of(["Dynamic", "Fixed", "Kinematic"])),
        ("dominance", Integer),
        ("gravity_scale", Number),
        ("linear_damping", Number),
        ("angular_damping", Number),
        ("lock_rotation", Bool),
        ("ccd", Bool),
        ("mass", ValueSchema::optional(Number)),
        ("center_of_mass", ValueSchema::optional(Vec3)),
    ])
}
fn collider3d() -> ValueSchema {
    ValueSchema::object([
        ("shape", shape3d()),
        ("density", Number),
        ("friction", Number),
        ("restitution", Number),
        ("sensor", Bool),
        ("offset", Vec3),
        ("groups", collisiongroups()),
    ])
}
fn velocity3d() -> ValueSchema {
    ValueSchema::object([("linear", Vec3), ("angular", Vec3)])
}
fn character3d() -> ValueSchema {
    ValueSchema::object([
        ("height", Number),
        ("radius", Number),
        ("max_slope", Number),
        ("step_height", Number),
        ("snap_distance", Number),
        ("push_mass", Number),
        ("gravity_scale", Number),
        ("groups", collisiongroups()),
        ("velocity", Vec3),
        ("grounded", Bool),
    ])
}
fn vehicle3d() -> ValueSchema {
    ValueSchema::object([
        ("wheels", Tuple(vec![wheel3d(); 8])),
        ("wheel_count", Unsigned),
        ("groups", collisiongroups()),
    ])
}
fn colliderfocus() -> ValueSchema {
    ValueSchema::object([("radius", Number), ("keep_radius", Number)])
}
fn rect() -> ValueSchema {
    ValueSchema::object([("min", Vec2), ("max", Vec2)])
}
fn collisiongroups() -> ValueSchema {
    ValueSchema::object([("memberships", Unsigned), ("filter", Unsigned)])
}
fn wheel3d() -> ValueSchema {
    ValueSchema::object([
        ("position", Vec3),
        ("radius", Number),
        ("mass", Number),
        ("suspension_distance", Number),
        ("spring", Number),
        ("damper", Number),
        ("target_position", Number),
        ("force_app_point_distance", Number),
        ("damping_rate", Number),
        ("forward_friction", frictioncurve()),
        ("sideways_friction", frictioncurve()),
        ("motor_torque", Number),
        ("brake_torque", Number),
        ("steer_angle", Number),
        ("local_rotation", Tuple(vec![Number; 4])),
        ("grounded", Bool),
        ("contact_point", Vec3),
        ("contact_normal", Vec3),
        ("compression", Number),
        ("suspension_force", Number),
        ("sprung_mass", Number),
        ("angular_velocity", Number),
        ("forward_slip", Number),
        ("sideways_slip", Number),
        ("spin", Number),
        ("local_position", Vec3),
    ])
}
fn frictioncurve() -> ValueSchema {
    ValueSchema::object([
        ("extremum_slip", Number),
        ("extremum_value", Number),
        ("asymptote_slip", Number),
        ("asymptote_value", Number),
        ("stiffness", Number),
    ])
}
pub(super) fn builtin<T: 'static>() -> Option<ValueSchema> {
    let id = std::any::TypeId::of::<T>();
    if id == std::any::TypeId::of::<crate::Transform2D>() {
        return Some(transform2d());
    }
    if id == std::any::TypeId::of::<crate::Attached>() {
        return Some(attached());
    }
    #[cfg(feature = "3d")]
    if id == std::any::TypeId::of::<crate::Transform3D>() {
        return Some(transform3d());
    }
    #[cfg(feature = "3d")]
    if id == std::any::TypeId::of::<crate::PointLight3D>() {
        return Some(pointlight3d());
    }
    #[cfg(feature = "3d")]
    if id == std::any::TypeId::of::<crate::SpotLight3D>() {
        return Some(spotlight3d());
    }
    #[cfg(feature = "3d")]
    if id == std::any::TypeId::of::<crate::Decal3D>() {
        return Some(decal3d());
    }
    #[cfg(feature = "3d")]
    if id == std::any::TypeId::of::<crate::LodHysteresis>() {
        return Some(lodhysteresis());
    }
    #[cfg(feature = "3d")]
    if id == std::any::TypeId::of::<crate::ReceivesDecals>() {
        return Some(receivesdecals());
    }
    #[cfg(feature = "physics")]
    if id == std::any::TypeId::of::<crate::RigidBody>() {
        return Some(rigidbody());
    }
    #[cfg(feature = "physics")]
    if id == std::any::TypeId::of::<crate::Collider>() {
        return Some(collider());
    }
    #[cfg(feature = "physics")]
    if id == std::any::TypeId::of::<crate::Velocity>() {
        return Some(velocity());
    }
    #[cfg(feature = "physics3d")]
    if id == std::any::TypeId::of::<crate::RigidBody3D>() {
        return Some(rigidbody3d());
    }
    #[cfg(feature = "physics3d")]
    if id == std::any::TypeId::of::<crate::Collider3D>() {
        return Some(collider3d());
    }
    #[cfg(feature = "physics3d")]
    if id == std::any::TypeId::of::<crate::Velocity3D>() {
        return Some(velocity3d());
    }
    #[cfg(feature = "physics3d")]
    if id == std::any::TypeId::of::<crate::Character3D>() {
        return Some(character3d());
    }
    #[cfg(feature = "physics3d")]
    if id == std::any::TypeId::of::<crate::Vehicle3D>() {
        return Some(vehicle3d());
    }
    #[cfg(feature = "physics3d")]
    if id == std::any::TypeId::of::<crate::ColliderFocus>() {
        return Some(colliderfocus());
    }
    None
}

pub(super) fn sprite() -> ValueSchema {
    ValueSchema::object([
        ("texture", ValueSchema::optional(String)),
        ("color", Color),
        ("z", Number),
        ("pivot", Vec2),
        ("region", ValueSchema::optional(rect())),
    ])
}
pub(super) fn model() -> ValueSchema {
    ValueSchema::object([("mesh", ValueSchema::optional(String)), ("color", Color)])
}
pub(super) fn shadow() -> ValueSchema {
    ValueSchema::object([("mesh", ValueSchema::optional(String))])
}
pub(super) fn lod() -> ValueSchema {
    ValueSchema::object([
        ("size", Number),
        ("center", Vec3),
        (
            "levels",
            ValueSchema::array(ValueSchema::object([
                ("screen_height", Number),
                ("fade_width", Number),
                (
                    "renderers",
                    ValueSchema::array(ValueSchema::object([
                        ("mesh", ValueSchema::optional(String)),
                        ("offset", transform3d()),
                        ("color", Color),
                    ])),
                ),
                (
                    "shadow",
                    OneOf(vec![
                        ValueSchema::one_of(["Same"]),
                        ValueSchema::object([(
                            "Casters",
                            ValueSchema::array(ValueSchema::object([
                                ("mesh", ValueSchema::optional(String)),
                                ("offset", transform3d()),
                                ("color", Color),
                            ])),
                        )]),
                    ]),
                ),
            ])),
        ),
    ])
}

fn shape2d() -> ValueSchema {
    OneOf(vec![
        ValueSchema::object([("Circle", ValueSchema::object([("radius", Number)]))]),
        ValueSchema::object([("Rect", ValueSchema::object([("size", Vec2)]))]),
        ValueSchema::object([("Capsule", ValueSchema::object([("height", Number), ("radius", Number)]))]),
        ValueSchema::object([(
            "Polygon",
            ValueSchema::object([("points", Tuple(vec![Vec2; 8])), ("count", Unsigned)]),
        )]),
    ])
}
fn shape3d() -> ValueSchema {
    OneOf(vec![
        ValueSchema::object([("Sphere", ValueSchema::object([("radius", Number)]))]),
        ValueSchema::object([("Cuboid", ValueSchema::object([("size", Vec3)]))]),
        ValueSchema::object([("Capsule", ValueSchema::object([("height", Number), ("radius", Number)]))]),
        ValueSchema::object([(
            "Mesh",
            OneOf(vec![
                ValueSchema::object([("index", Unsigned), ("generation", Unsigned)]),
                ValueSchema::object([("file", String), ("scale", Vec3), ("convex", Bool)]),
            ]),
        )]),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn validate(schema: &ValueSchema, value: &serde_json::Value) {
        match schema {
            Object(fields) => {
                let object = value.as_object().expect("object");
                assert_eq!(fields.len(), object.len(), "field drift: {value}");
                for (name, shape) in fields {
                    validate(shape, &object[name]);
                }
            }
            Tuple(items) => {
                let values = value.as_array().expect("tuple");
                assert_eq!(items.len(), values.len());
                for (s, v) in items.iter().zip(values) {
                    validate(s, v);
                }
            }
            Array(item) => {
                for v in value.as_array().expect("array") {
                    validate(item, v);
                }
            }
            Optional(item) => {
                if !value.is_null() {
                    validate(item, value);
                }
            }
            Number => assert!(value.is_number()),
            Integer => assert!(value.is_i64()),
            Unsigned | Entity => assert!(value.is_u64()),
            Bool => assert!(value.is_boolean()),
            String => assert!(value.is_string()),
            Enum(names) => assert!(names.iter().any(|s| value == s)),
            Vec2 => validate(&Tuple(vec![Number; 2]), value),
            Vec3 => validate(&Tuple(vec![Number; 3]), value),
            Color => validate(
                &ValueSchema::object([("r", Number), ("g", Number), ("b", Number), ("a", Number)]),
                value,
            ),
            Null => assert!(value.is_null()),
            OneOf(options) => {
                let selected = options
                    .iter()
                    .find(|s| match s {
                        Object(fields) => value
                            .as_object()
                            .is_some_and(|o| fields.iter().all(|(n, _)| o.contains_key(n))),
                        Enum(names) => names.iter().any(|n| value == n),
                        _ => false,
                    })
                    .expect("known tagged variant");
                validate(selected, value);
            }
            Opaque => panic!("built-in schema must describe every field"),
        }
    }
    #[test]
    fn builtin_shapes_match_serialized_fields_and_value_types() {
        macro_rules! check {
            ($t:ty, $value:expr) => {
                validate(&builtin::<$t>().unwrap(), &serde_json::to_value($value).unwrap());
            };
        }
        validate(
            &sprite(),
            &serde_json::to_value(super::super::SpriteJson::default()).unwrap(),
        );
        #[cfg(feature = "3d")]
        {
            validate(
                &model(),
                &serde_json::to_value(super::super::ModelJson::default()).unwrap(),
            );
            validate(
                &shadow(),
                &serde_json::to_value(super::super::ShadowMeshJson::default()).unwrap(),
            );
        }
        check!(crate::Transform2D, crate::Transform2D::default());
        check!(crate::Attached, crate::Attached::to(crate::Entity::from_bits(0)));
        #[cfg(feature = "3d")]
        {
            check!(crate::Transform3D, crate::Transform3D::default());
            check!(crate::PointLight3D, crate::PointLight3D::default());
            check!(crate::SpotLight3D, crate::SpotLight3D::default());
            check!(crate::Decal3D, crate::Decal3D::default());
            check!(crate::ReceivesDecals, crate::ReceivesDecals::default());
            check!(crate::LodHysteresis, crate::LodHysteresis::default());
        }
        #[cfg(feature = "physics")]
        {
            check!(crate::RigidBody, crate::RigidBody::default());
            check!(crate::Collider, crate::Collider::circle(1.0));
            check!(crate::Velocity, crate::Velocity::default());
        }
        #[cfg(feature = "physics3d")]
        {
            check!(crate::RigidBody3D, crate::RigidBody3D::default());
            check!(crate::Collider3D, crate::Collider3D::sphere(1.0));
            check!(crate::Velocity3D, crate::Velocity3D::default());
            check!(crate::Character3D, crate::Character3D::default());
            check!(crate::Vehicle3D, crate::Vehicle3D::default());
            check!(crate::ColliderFocus, crate::ColliderFocus::default());
        }
    }
}
