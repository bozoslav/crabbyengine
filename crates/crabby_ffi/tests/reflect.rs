//! Describing C components for reflection, and world save/load, through the C API.
use crabby_ffi::CrabbyFieldType::*;
use crabby_ffi::*;
use std::ffi::{CStr, CString};
use std::ptr::{null, null_mut};

use CrabbyStatus::*;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Unit {
    hp: f32,
    home: CrabbyVec2,
    target: u64,
    angry: bool,
}

fn field(name: &'static CStr, kind: CrabbyFieldType, offset: usize) -> CrabbyField {
    CrabbyField {
        name: name.as_ptr(),
        kind: kind as u32,
        offset,
    }
}
fn unit_fields() -> [CrabbyField; 4] {
    [
        field(c"hp", CRABBY_FIELD_F32, std::mem::offset_of!(Unit, hp)),
        field(c"home", CRABBY_FIELD_VEC2, std::mem::offset_of!(Unit, home)),
        field(c"target", CRABBY_FIELD_ENTITY, std::mem::offset_of!(Unit, target)),
        field(c"angry", CRABBY_FIELD_BOOL, std::mem::offset_of!(Unit, angry)),
    ]
}
fn last_error() -> String {
    unsafe { CStr::from_ptr(crabby_last_error()) }
        .to_string_lossy()
        .into_owned()
}
/// A world with `Unit` registered (not yet described).
fn world() -> (*mut CrabbyWorld, u32) {
    let w = crabby_world_new();
    let mut id = 0;
    let status =
        unsafe { crabby_component_register(w, c"Unit".as_ptr(), size_of::<Unit>(), align_of::<Unit>(), &mut id) };
    assert_eq!(status, CRABBY_OK);
    (w, id)
}
fn spawn(w: *mut CrabbyWorld, id: u32, unit: Unit) -> u64 {
    let value = CrabbyValue {
        component: id,
        data: (&unit as *const Unit).cast(),
        size: size_of::<Unit>(),
    };
    let mut e = 0;
    assert_eq!(unsafe { crabby_spawn(w, &value, 1, &mut e) }, CRABBY_OK);
    e
}
fn get(w: *mut CrabbyWorld, id: u32, e: u64) -> Unit {
    let mut unit = Unit::default();
    let status = unsafe { crabby_get(w, e, id, (&mut unit as *mut Unit).cast(), size_of::<Unit>()) };
    assert_eq!(status, CRABBY_OK);
    unit
}
fn save(w: *mut CrabbyWorld) -> String {
    let mut len = 0;
    unsafe {
        assert_eq!(crabby_world_save(w, null_mut(), 0, &mut len), CRABBY_OK);
        let mut buf = vec![0xffu8; len + 1];
        assert_eq!(
            crabby_world_save(w, buf.as_mut_ptr().cast(), buf.len(), &mut len),
            CRABBY_OK
        );
        assert_eq!(buf[len], 0, "NUL-terminated");
        buf.truncate(len);
        String::from_utf8(buf).unwrap()
    }
}

#[test]
fn described_c_components_are_saved_and_loaded_with_their_ids() {
    let (w, id) = world();
    let fields = unit_fields();
    assert_eq!(
        unsafe { crabby_component_reflect(w, id, fields.as_ptr(), fields.len()) },
        CRABBY_OK
    );
    let a = spawn(
        w,
        id,
        Unit {
            hp: 3.0,
            ..Default::default()
        },
    );
    let b = spawn(
        w,
        id,
        Unit {
            hp: 5.0,
            home: CrabbyVec2 { x: 1.0, y: 2.0 },
            target: a,
            angry: true,
        },
    );
    let json = save(w);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let saved_b = parsed["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["entity"] == b)
        .unwrap();
    assert_eq!(
        saved_b["components"]["Unit"],
        serde_json::json!({ "hp": 5.0, "home": [1.0, 2.0], "target": a, "angry": true })
    );

    // Change things, then load the save back.
    unsafe { assert_eq!(crabby_despawn(w, a), CRABBY_OK) };
    let c = spawn(w, id, Unit::default());
    let json = CString::new(json).unwrap();
    assert_eq!(unsafe { crabby_world_load(w, json.as_ptr()) }, CRABBY_OK);
    let mut n = 0;
    unsafe { crabby_world_len(w, &mut n) };
    assert_eq!(n, 2);
    assert_eq!(get(w, id, a).hp, 3.0);
    assert_eq!(get(w, id, b).target, a);
    let mut alive = true;
    unsafe { crabby_is_alive(w, c, &mut alive) };
    assert!(!alive, "entities spawned after the save are gone");
    unsafe { crabby_world_free(w) };
}

#[test]
fn small_save_buffers_get_an_empty_string_and_the_size() {
    let (w, id) = world();
    let fields = unit_fields();
    unsafe { crabby_component_reflect(w, id, fields.as_ptr(), fields.len()) };
    spawn(w, id, Unit::default());
    let mut buf = [0x55u8; 8];
    let mut len = 0;
    assert_eq!(
        unsafe { crabby_world_save(w, buf.as_mut_ptr().cast(), buf.len(), &mut len) },
        CRABBY_OK
    );
    assert!(len > buf.len());
    assert_eq!(buf[0], 0);
    assert_eq!(buf[1], 0x55, "nothing else is touched");
    assert_eq!(
        unsafe { crabby_world_save(w, null_mut(), 4, &mut len) },
        CRABBY_INVALID_ARGUMENT
    );
    assert_eq!(
        unsafe { crabby_world_save(w, null_mut(), 0, null_mut()) },
        CRABBY_INVALID_ARGUMENT
    );
    unsafe { crabby_world_free(w) };
}

#[test]
fn bad_saves_are_rejected_with_a_reason_and_change_nothing() {
    let (w, id) = world();
    let fields = unit_fields();
    unsafe { crabby_component_reflect(w, id, fields.as_ptr(), fields.len()) };
    let e = spawn(
        w,
        id,
        Unit {
            hp: 1.0,
            ..Default::default()
        },
    );
    let cases: [(&CStr, &str); 3] = [
        (c"not json", "not JSON"),
        (c"{}", "not a world save"),
        (
            cr#"{"format":"crabby-world","version":1,"entities":[{"entity":1,"components":{"Unit":{"speed":1}}}]}"#,
            "has no field `speed`",
        ),
    ];
    for (json, message) in cases {
        assert_eq!(unsafe { crabby_world_load(w, json.as_ptr()) }, CRABBY_INVALID_ARGUMENT);
        assert!(last_error().contains(message), "{json:?}: {}", last_error());
    }
    assert_eq!(unsafe { crabby_world_load(w, null()) }, CRABBY_INVALID_ARGUMENT);
    assert_eq!(get(w, id, e).hp, 1.0);
    unsafe { crabby_world_free(w) };
}

#[test]
fn field_descriptions_are_checked() {
    let (w, id) = world();
    let mut transform = 0;
    unsafe { crabby_transform_component(w, &mut transform) };
    let too_far = [field(c"hp", CRABBY_FIELD_F64, size_of::<Unit>() - 4)];
    let twice = [field(c"hp", CRABBY_FIELD_F32, 0), field(c"hp", CRABBY_FIELD_F32, 0)];
    let bad_kind = [CrabbyField {
        name: c"hp".as_ptr(),
        kind: 99,
        offset: 0,
    }];
    let no_name = [CrabbyField {
        name: null(),
        kind: 0,
        offset: 0,
    }];
    let overflow = [field(c"hp", CRABBY_FIELD_F32, usize::MAX)];
    let rotation = [field(c"rotation", CRABBY_FIELD_F32, 8)];
    let cases: [(u32, &[CrabbyField], &str); 6] = [
        (id, &too_far, "doesn't fit"),
        (id, &twice, "same name"),
        (id, &bad_kind, "unknown CrabbyFieldType"),
        (id, &no_name, "name is null"),
        (id, &overflow, "overflows"),
        (transform, &rotation, "only components registered from C"),
    ];
    for (component, fields, message) in cases {
        let status = unsafe { crabby_component_reflect(w, component, fields.as_ptr(), fields.len()) };
        assert_eq!(status, CRABBY_INVALID_ARGUMENT, "{message}");
        assert!(last_error().contains(message), "{message}: {}", last_error());
    }
    assert_eq!(
        unsafe { crabby_component_reflect(w, id, null(), 2) },
        CRABBY_INVALID_ARGUMENT
    );
    assert_eq!(
        unsafe { crabby_component_reflect(w, 999, null(), 0) },
        CRABBY_INVALID_ARGUMENT
    );
    // Zero fields is allowed: the component shows up as `{}`.
    assert_eq!(unsafe { crabby_component_reflect(w, id, null(), 0) }, CRABBY_OK);
    unsafe { crabby_world_free(w) };
}
