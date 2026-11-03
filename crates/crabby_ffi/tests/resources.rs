//! Resources set, read, removed and described from C.
use crabby_ffi::CrabbyFieldType::*;
use crabby_ffi::*;
use std::ffi::{CStr, c_void};
use std::ptr::null_mut;

use CrabbyStatus::*;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Score {
    red: u32,
    blue: u32,
}
const SIZE: usize = size_of::<Score>();
const ALIGN: usize = align_of::<Score>();

fn last_error() -> String {
    unsafe { CStr::from_ptr(crabby_last_error()) }
        .to_string_lossy()
        .into_owned()
}
unsafe fn set(w: *mut CrabbyWorld, score: Score) -> CrabbyStatus {
    unsafe { crabby_resource_set(w, c"Score".as_ptr(), (&raw const score).cast(), SIZE, ALIGN) }
}
unsafe fn get(w: *mut CrabbyWorld) -> Result<Score, CrabbyStatus> {
    let mut out = Score::default();
    match unsafe { crabby_resource_get(w, c"Score".as_ptr(), (&raw mut out).cast(), SIZE) } {
        CRABBY_OK => Ok(out),
        status => Err(status),
    }
}
fn score_fields() -> [CrabbyField; 2] {
    [
        CrabbyField {
            name: c"red".as_ptr(),
            kind: CRABBY_FIELD_U32 as u32,
            offset: 0,
        },
        CrabbyField {
            name: c"blue".as_ptr(),
            kind: CRABBY_FIELD_U32 as u32,
            offset: 4,
        },
    ]
}
fn save(w: *mut CrabbyWorld) -> serde_json::Value {
    let mut len = 0;
    unsafe {
        assert_eq!(crabby_world_save(w, null_mut(), 0, &mut len), CRABBY_OK);
        let mut buf = vec![0u8; len + 1];
        assert_eq!(
            crabby_world_save(w, buf.as_mut_ptr().cast(), buf.len(), &mut len),
            CRABBY_OK
        );
        serde_json::from_slice(&buf[..len]).unwrap()
    }
}

#[test]
fn resources_are_set_read_replaced_and_removed_by_name() {
    let w = crabby_world_new();
    unsafe {
        assert_eq!(get(w), Err(CRABBY_NOT_FOUND));
        assert_eq!(set(w, Score { red: 1, blue: 2 }), CRABBY_OK);
        assert_eq!(get(w), Ok(Score { red: 1, blue: 2 }));
        assert_eq!(set(w, Score { red: 5, blue: 2 }), CRABBY_OK);
        assert_eq!(get(w), Ok(Score { red: 5, blue: 2 }));
        assert_eq!(crabby_resource_remove(w, c"Score".as_ptr()), CRABBY_OK);
        assert_eq!(get(w), Err(CRABBY_NOT_FOUND));
        assert_eq!(crabby_resource_remove(w, c"Score".as_ptr()), CRABBY_NOT_FOUND);
        crabby_world_free(w);
    }
}

#[test]
fn a_resource_keeps_its_size() {
    let w = crabby_world_new();
    unsafe {
        assert_eq!(set(w, Score::default()), CRABBY_OK);
        let small: u32 = 3;
        let status = crabby_resource_set(w, c"Score".as_ptr(), (&raw const small).cast(), 4, 4);
        assert_eq!(status, CRABBY_INVALID_ARGUMENT);
        assert!(last_error().contains("different size"), "{}", last_error());
        let mut out = 0u32;
        let status = crabby_resource_get(w, c"Score".as_ptr(), (&raw mut out).cast(), 4);
        assert_eq!(status, CRABBY_INVALID_ARGUMENT);
        assert_eq!(get(w), Ok(Score::default()));
        crabby_world_free(w);
    }
}

#[test]
fn bad_resource_arguments_are_errors_not_crashes() {
    let w = crabby_world_new();
    let value = 0u32;
    let data: *const c_void = (&raw const value).cast();
    unsafe {
        assert_eq!(
            crabby_resource_set(w, std::ptr::null(), data, 4, 4),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_resource_set(w, c"".as_ptr(), data, 4, 4),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_resource_set(w, c"X".as_ptr(), std::ptr::null(), 4, 4),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_resource_set(w, c"X".as_ptr(), data, 4, 3),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_resource_set(w, c"X".as_ptr(), data, 2, 4),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_resource_set(null_mut(), c"X".as_ptr(), data, 4, 4),
            CRABBY_INVALID_ARGUMENT
        );
        assert_eq!(
            crabby_resource_get(w, c"X".as_ptr(), null_mut(), 4),
            CRABBY_INVALID_ARGUMENT
        );
        crabby_world_free(w);
    }
}

#[test]
fn zero_size_resources_work_as_flags() {
    let w = crabby_world_new();
    let nothing = ();
    unsafe {
        let status = crabby_resource_set(w, c"Paused".as_ptr(), (&raw const nothing).cast(), 0, 1);
        assert_eq!(status, CRABBY_OK);
        let mut out = ();
        assert_eq!(
            crabby_resource_get(w, c"Paused".as_ptr(), (&raw mut out).cast(), 0),
            CRABBY_OK
        );
        assert_eq!(crabby_resource_remove(w, c"Paused".as_ptr()), CRABBY_OK);
        crabby_world_free(w);
    }
}

#[test]
fn described_resources_are_saved_and_loaded() {
    let w = crabby_world_new();
    let fields = score_fields();
    unsafe {
        let status = crabby_resource_reflect(w, c"Score".as_ptr(), SIZE, ALIGN, fields.as_ptr(), fields.len());
        assert_eq!(status, CRABBY_OK, "{}", last_error());
        assert_eq!(set(w, Score { red: 3, blue: 9 }), CRABBY_OK);
        let saved = save(w);
        assert_eq!(
            saved["resources"]["Score"],
            serde_json::json!({ "red": 3, "blue": 9 }),
            "{saved}"
        );

        assert_eq!(set(w, Score::default()), CRABBY_OK);
        let text = std::ffi::CString::new(saved.to_string()).unwrap();
        assert_eq!(crabby_world_load(w, text.as_ptr()), CRABBY_OK, "{}", last_error());
        assert_eq!(get(w), Ok(Score { red: 3, blue: 9 }));
        crabby_world_free(w);
    }
}

#[test]
fn resource_descriptions_must_fit() {
    let w = crabby_world_new();
    let fields = score_fields();
    unsafe {
        let status = crabby_resource_reflect(w, c"Score".as_ptr(), 4, 4, fields.as_ptr(), fields.len());
        assert_eq!(status, CRABBY_INVALID_ARGUMENT);
        assert!(last_error().contains("doesn't fit in the resource"), "{}", last_error());
        let twice = [fields[0], fields[0]];
        let status = crabby_resource_reflect(w, c"Score".as_ptr(), SIZE, ALIGN, twice.as_ptr(), 2);
        assert_eq!(status, CRABBY_INVALID_ARGUMENT);
        assert_eq!(set(w, Score::default()), CRABBY_OK);
        let status = crabby_resource_reflect(w, c"Score".as_ptr(), 16, 4, fields.as_ptr(), fields.len());
        assert_eq!(status, CRABBY_INVALID_ARGUMENT);
        crabby_world_free(w);
    }
}
