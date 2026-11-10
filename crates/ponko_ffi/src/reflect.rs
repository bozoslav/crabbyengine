//! Reflection of C components (their fields as JSON, for tools and agents)
//! and world save/load.
use std::ffi::{CStr, c_char};

use crate::ctx::{ctx_access, text};
use ponko::reflect::{Field, FieldType, WorldReflect};

use crate::{CrabbyStatus, CrabbyWorld, Failure, access, boundary, component, invalid, invalid_owned};

/// Field types for `CrabbyField::kind`, and how they look in JSON.
/// ```
/// assert_eq!(ponko_ffi::CrabbyFieldType::CRABBY_FIELD_COLOR as u32, 12);
/// ```
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyFieldType {
    /// `float`, a JSON number.
    CRABBY_FIELD_F32 = 0,
    /// `double`.
    CRABBY_FIELD_F64 = 1,
    /// `int8_t`.
    CRABBY_FIELD_I8 = 2,
    /// `int16_t`.
    CRABBY_FIELD_I16 = 3,
    /// `int32_t`.
    CRABBY_FIELD_I32 = 4,
    /// `int64_t`.
    CRABBY_FIELD_I64 = 5,
    /// `uint8_t`.
    CRABBY_FIELD_U8 = 6,
    /// `uint16_t`.
    CRABBY_FIELD_U16 = 7,
    /// `uint32_t`.
    CRABBY_FIELD_U32 = 8,
    /// `uint64_t`.
    CRABBY_FIELD_U64 = 9,
    /// `bool`, `true`/`false`.
    CRABBY_FIELD_BOOL = 10,
    /// `CrabbyVec2`, `[x, y]`.
    CRABBY_FIELD_VEC2 = 11,
    /// `CrabbyColor`, `{"r", "g", "b", "a"}`.
    CRABBY_FIELD_COLOR = 12,
    /// An entity id (`uint64_t`), a JSON number.
    CRABBY_FIELD_ENTITY = 13,
}

const TYPES: [FieldType; 14] = [
    FieldType::F32,
    FieldType::F64,
    FieldType::I8,
    FieldType::I16,
    FieldType::I32,
    FieldType::I64,
    FieldType::U8,
    FieldType::U16,
    FieldType::U32,
    FieldType::U64,
    FieldType::Bool,
    FieldType::Vec2,
    FieldType::Color,
    FieldType::Entity,
];

/// One field of a C component: its JSON name, a `CrabbyFieldType`, and its
/// byte offset (`offsetof`).
/// ```
/// let f = ponko_ffi::CrabbyField { name: c"hp".as_ptr(), kind: 0, offset: 0 };
/// assert_eq!(f.offset, 0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CrabbyField {
    /// NUL-terminated UTF-8; copied.
    pub name: *const c_char,
    /// A `CrabbyFieldType`.
    pub kind: u32,
    /// Byte offset in the component.
    pub offset: usize,
}

/// Reads `len` C field descriptions.
///
/// # Safety
/// `fields` holds `len` readable fields with NUL-terminated names (or is
/// null with `len` 0).
pub(crate) unsafe fn fields_from_c(fields: *const CrabbyField, len: usize) -> Result<Vec<Field>, Failure> {
    if fields.is_null() && len > 0 {
        return Err(invalid("fields is null"));
    }
    let raw = if len == 0 {
        &[][..]
    } else {
        // SAFETY: caller supplies `len` readable fields.
        unsafe { std::slice::from_raw_parts(fields, len) }
    };
    let mut list = Vec::with_capacity(raw.len());
    for f in raw {
        if f.name.is_null() {
            return Err(invalid("a field name is null"));
        }
        // SAFETY: caller supplies NUL-terminated names.
        let name = unsafe { CStr::from_ptr(f.name) }
            .to_str()
            .map_err(|_| invalid("a field name is not UTF-8"))?;
        let ty = *TYPES
            .get(f.kind as usize)
            .ok_or_else(|| invalid("unknown CrabbyFieldType"))?;
        if f.offset.checked_add(ty.size()).is_none() {
            return Err(invalid("field offset overflows"));
        }
        list.push(Field::new(name, ty, f.offset));
    }
    Ok(list)
}

/// Every field fits in `size` bytes and has its own name.
pub(crate) fn check_fields(list: &[Field], size: usize, what: &str) -> Result<(), Failure> {
    let mut names = std::collections::HashSet::new();
    for f in list {
        if f.offset + f.ty.size() > size {
            return Err(invalid_owned(format!("a field doesn't fit in the {what}")));
        }
        if !names.insert(f.name.as_str()) {
            return Err(invalid("two fields have the same name"));
        }
    }
    Ok(())
}

/// Makes a C component readable and writable as JSON by tools and agents
/// (the control endpoint and `ponko-mcp`) and saved by `crabby_world_save`,
/// by describing its fields. Bytes no field covers are zero in values made
/// from JSON. Describing it again replaces the fields.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe {
///     let mut id = 0;
///     crabby_component_register(w, c"Health".as_ptr(), 4, 4, &mut id);
///     let field = CrabbyField { name: c"current".as_ptr(), kind: CrabbyFieldType::CRABBY_FIELD_F32 as u32, offset: 0 };
///     assert_eq!(crabby_component_reflect(w, id, &field, 1), CrabbyStatus::CRABBY_OK);
///     crabby_world_free(w);
/// }
/// ```
/// # Safety
/// `world` must be null or live; `fields` must hold `len` fields with valid
/// names. Any field values must make a valid value of the component (true
/// for plain C structs of these types).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_component_reflect(
    world: *mut CrabbyWorld,
    component_id: u32,
    fields: *const CrabbyField,
    len: usize,
) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: forwarded from the caller.
        let list = unsafe { fields_from_c(fields, len)? };
        // SAFETY: access validates null and guards the live handle.
        unsafe {
            access(world, |w, _| {
                let (id, kind) = component(w, component_id)?;
                if !kind.is_plain() || w.component_info(id).type_id().is_some() {
                    return Err(invalid("only components registered from C can be described"));
                }
                check_fields(&list, w.component_info(id).layout().size(), "component")?;
                // SAFETY: checked above; the caller promised field values make valid values.
                w.reflect_fields(id, &list);
                Ok(())
            })
        }
    })
}

/// Writes the world as JSON (the format of Rust's `World::save`: every entity
/// with reflected components, with its id) into `buffer`, NUL-terminated.
/// `*out_len` is set to the JSON's length without the NUL. If `capacity` is
/// not more than that, only an empty string is written: grow the buffer to
/// `*out_len + 1` and call again.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// let mut len = 0;
/// unsafe {
///     assert_eq!(crabby_world_save(w, std::ptr::null_mut(), 0, &mut len), CrabbyStatus::CRABBY_OK);
///     let mut buf = vec![1u8; len + 1];
///     assert_eq!(crabby_world_save(w, buf.as_mut_ptr().cast(), buf.len(), &mut len), CrabbyStatus::CRABBY_OK);
///     assert_eq!(buf[len], 0);
///     crabby_world_free(w);
/// }
/// ```
/// # Safety
/// `world` must be null or live; `buffer` must hold `capacity` bytes (or be
/// null with capacity 0); `out_len` writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_save(
    world: *mut CrabbyWorld,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    boundary(|| {
        if out_len.is_null() {
            return Err(invalid("output is null"));
        }
        if buffer.is_null() && capacity > 0 {
            return Err(invalid("buffer is null"));
        }
        // SAFETY: access validates null and guards the live handle.
        let json = unsafe { access(world, |w, _| Ok(w.save().to_string()))? };
        // SAFETY: checked non-null; `buffer` holds `capacity` bytes.
        unsafe {
            out_len.write(json.len());
            if json.len() < capacity {
                std::ptr::copy_nonoverlapping(json.as_ptr(), buffer.cast::<u8>(), json.len());
                buffer.add(json.len()).write(0);
            } else if capacity > 0 {
                buffer.write(0);
            }
        }
        Ok(())
    })
}

/// Replaces every entity with those in a save from `crabby_world_save`,
/// with the same ids. Everything is checked first: on error the world is
/// unchanged and `crabby_last_error()` says what was wrong.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe {
///     assert_eq!(crabby_world_load(w, c"{}".as_ptr()), CrabbyStatus::CRABBY_INVALID_ARGUMENT);
///     crabby_world_free(w);
/// }
/// ```
/// # Safety
/// `world` must be null or live; `json` NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_load(world: *mut CrabbyWorld, json: *const c_char) -> CrabbyStatus {
    boundary(|| {
        if json.is_null() {
            return Err(invalid("json is null"));
        }
        // SAFETY: caller supplies a NUL-terminated string.
        let text = unsafe { CStr::from_ptr(json) }
            .to_str()
            .map_err(|_| invalid("json is not UTF-8"))?;
        let save: serde_json::Value =
            serde_json::from_str(text).map_err(|e| crate::invalid_owned(format!("not JSON: {e}")))?;
        // SAFETY: access validates null and guards the live handle.
        unsafe {
            access(world, |w, _| {
                w.load(&save).map_err(|e| crate::invalid_owned(e.to_string()))
            })
        }
    })
}

/// Attach opt-in schema metadata to an already reflected component.
/// `schema` is JSON encoding of Rust's ValueSchema: e.g. `"Number"`,
/// `{"Object":[["health","Number"]]}`, `{"Optional":"String"}`.
/// This describes discovery only; it does not change serialization or validate values.
/// Invalid JSON or an unknown name fails without changing the previous schema.
/// # Safety
/// `world` must be null or live. `name` and `schema` must be null or valid
/// NUL-terminated UTF-8 strings for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_describe_component(
    world: *mut CrabbyWorld,
    name: *const c_char,
    schema: *const c_char,
) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: caller promises live strings; text checks null and UTF-8.
        let (name, schema) = unsafe { (text(name)?, text(schema)?) };
        let schema = serde_json::from_str::<ponko::ValueSchema>(schema).map_err(|e| invalid_owned(e.to_string()))?;
        // SAFETY: access checks the handle and guards its world borrow.
        unsafe {
            access(world, |w, _| {
                w.describe_component(name, schema)
                    .map_err(|e| invalid_owned(e.to_string()))
            })
        }
    })
}

/// Attach opt-in schema metadata to an already reflected resource.
/// Uses the same schema encoding as `crabby_world_describe_component`.
/// # Safety
/// `world` must be null or live; strings must be null or valid NUL-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_describe_resource(
    world: *mut CrabbyWorld,
    name: *const c_char,
    schema: *const c_char,
) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: caller promises live strings; text checks null and UTF-8.
        let (name, schema) = unsafe { (text(name)?, text(schema)?) };
        let schema = serde_json::from_str::<ponko::ValueSchema>(schema).map_err(|e| invalid_owned(e.to_string()))?;
        // SAFETY: access checks the handle and guards its world borrow.
        unsafe {
            access(world, |w, _| {
                w.describe_resource(name, schema)
                    .map_err(|e| invalid_owned(e.to_string()))
            })
        }
    })
}

/// What this running game can do, as JSON: the control endpoint's
/// `capabilities` answer (Rust `Ctx::capabilities_json`), including every
/// reflected component and resource with the schema described for it.
/// Writes into `buffer` as `crabby_ctx_perf_report` does: the length goes
/// to `out_len`, and the text (NUL-terminated) only if it fits.
/// # Safety
/// `ctx` null or live; `buffer` holds `capacity` bytes (or is null with
/// capacity 0); `out_len` writable.
/// ```
/// let mut len = 0usize;
/// assert_eq!(unsafe { ponko_ffi::crabby_ctx_capabilities_json(std::ptr::null(), std::ptr::null_mut(), 0, &mut len) } as u32, 1);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_capabilities_json(
    ctx: *const crate::CrabbyCtx,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: forwarded caller contract.
    boundary(|| unsafe {
        crate::shader_status::text_output(buffer, capacity, out_len)?;
        let json = ctx_access(ctx, |c| Ok(c.capabilities_json()))?;
        crate::perf::write_text(buffer, capacity, out_len, &json);
        Ok(())
    })
}
