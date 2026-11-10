use super::*;
use crate::WorldReflect;

#[test]
fn protocol_describes_every_registered_builtin_without_sampling_instances() {
    let mut ctx = Ctx::new(1.0 / 60.0, glam::Vec2::ZERO);
    #[cfg(feature = "physics")]
    {
        ctx.world.reflect::<crate::RigidBody>();
        ctx.world.reflect::<crate::Collider>();
        ctx.world.reflect::<crate::Velocity>();
    }
    #[cfg(feature = "physics3d")]
    {
        ctx.world.reflect::<crate::RigidBody3D>();
        crate::reflect::reflect_collider(&mut ctx.world);
        ctx.world.reflect::<crate::Velocity3D>();
        ctx.world.reflect::<crate::Character3D>();
        ctx.world.reflect::<crate::Vehicle3D>();
        ctx.world.reflect::<crate::ColliderFocus>();
    }
    let response = crate::control::handle(&mut ctx, "capabilities", &json!({})).unwrap();
    for component in response["reflection"]["components"].as_array().unwrap() {
        assert_eq!(component["schema"]["status"], "described", "{component}");
    }
    let sprite = response["reflection"]["components"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "Sprite")
        .unwrap();
    assert_eq!(
        sprite["schema"]["json_schema"]["properties"]["texture"]["anyOf"][0]["type"],
        "string"
    );
}

#[test]
fn custom_type_named_like_builtin_remains_opaque() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Velocity(u32);
    let mut ctx = Ctx::new(1.0 / 60.0, glam::Vec2::ZERO);
    ctx.world.reflect::<Velocity>();
    let response = crate::control::handle(&mut ctx, "capabilities", &json!({})).unwrap();
    let component = response["reflection"]["components"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "Velocity")
        .unwrap();
    assert_eq!(component["schema"]["status"], "opaque");
}

#[cfg(all(feature = "window", feature = "3d"))]
#[test]
fn offscreen_device_uses_the_instance_report_path_and_preserves_gpu_fallbacks() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&Default::default())).expect("offscreen adapter required");
    let (device, _) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let gpu = crate::render::capabilities::of_device(&adapter.get_info(), &device, (1, 1));
    let mut ctx = Ctx::new(1.0 / 60.0, glam::Vec2::ZERO);
    ctx.headless = false;
    // Offscreen substitution at the report's GPU input; no surface or window is created.
    let response = report_with_gpu(&ctx, Some(&gpu));
    assert_eq!(response["mode"], "windowed");
    assert_eq!(response["subsystems"]["renderer"]["adapter"], json!(gpu.adapter));
    assert_eq!(response["subsystems"]["renderer"]["gpu"], json!(gpu));
    assert!(response["subsystems"]["renderer"]["adapter_reason"].is_null());
    ctx.headless = true;
    let minimal = crate::control::handle(&mut ctx, "capabilities", &json!({})).unwrap();
    assert_eq!(minimal["mode"], "headless");
    assert_eq!(minimal["subsystems"]["renderer"]["active"], false);
}

#[test]
fn protocol_resource_descriptions_replace_metadata_without_changing_values() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Score {
        total: u32,
    }
    let mut ctx = Ctx::new(1.0 / 60.0, glam::Vec2::ZERO);
    ctx.world.reflect_resource::<Score>();
    ctx.world.insert_resource(Score { total: 7 });
    let schema = crate::ValueSchema::object([("total", crate::ValueSchema::Unsigned)]);
    ctx.world.describe_resource("Score", schema.clone()).unwrap();
    let response = crate::control::handle(&mut ctx, "capabilities", &json!({})).unwrap();
    let resource = response["reflection"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "Score")
        .unwrap();
    assert_eq!(resource["schema"]["json_schema"], schema.to_json_schema());
    assert_eq!(ctx.world.resource::<Score>().total, 7);
    assert!(ctx.world.describe_resource("missing", schema).is_err());
    let again = crate::control::handle(&mut ctx, "capabilities", &json!({})).unwrap();
    assert_eq!(again["reflection"], response["reflection"]);
}
