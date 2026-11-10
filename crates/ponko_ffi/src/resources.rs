//! Resources from C: one value per world, found by name (a score, settings,
//! a random seed), stored as plain bytes.
use std::alloc::Layout;
use std::ffi::{CStr, c_char, c_void};

use ponko::WorldReflect;

use crate::{Failure, PonkoStatus, PonkoWorld, access, boundary, error, invalid};
use crate::{PonkoField, check_fields, fields_from_c};

fn resource_name<'a>(name: *const c_char) -> Result<&'a str, Failure> {
    if name.is_null() {
        return Err(invalid("name is null"));
    }
    // SAFETY: caller supplies a NUL-terminated name.
    let name = unsafe { CStr::from_ptr(name) }
        .to_str()
        .map_err(|_| invalid("name is not UTF-8"))?;
    if name.is_empty() {
        return Err(invalid("name is empty"));
    }
    Ok(name)
}

fn resource_layout(size: usize, alignment: usize) -> Result<Layout, Failure> {
    let layout = Layout::from_size_align(size, alignment).map_err(|_| invalid("invalid resource layout"))?;
    if !size.is_multiple_of(alignment) {
        return Err(invalid("resource size must be a multiple of alignment"));
    }
    Ok(layout)
}

fn same_layout(world: &ponko::World, name: &str, layout: Layout) -> Result<(), Failure> {
    match world.raw_ref().resource_layout_dynamic(name) {
        Some(existing) if existing != layout => Err(invalid("resource exists with a different size or alignment")),
        _ => Ok(()),
    }
}

/// Sets the resource called `name` to a copy of `size` bytes at `data`,
/// adding it if the world has none. A resource keeps its size and alignment.
/// ```
/// use ponko_ffi::*;
/// let w = ponko_world_new();
/// let score: u32 = 7;
/// let mut out: u32 = 0;
/// unsafe {
///     let size = std::mem::size_of::<u32>();
///     assert_eq!(ponko_resource_set(w, c"Score".as_ptr(), (&raw const score).cast(), size, 4), PonkoStatus::PONKO_OK);
///     assert_eq!(ponko_resource_get(w, c"Score".as_ptr(), (&raw mut out).cast(), size), PonkoStatus::PONKO_OK);
///     assert_eq!(out, 7);
///     ponko_world_free(w);
/// }
/// ```
/// # Safety
/// `world` must be null or live; `name` NUL-terminated; `data` must hold
/// `size` readable bytes of a plain value (no pointers it owns).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ponko_resource_set(
    world: *mut PonkoWorld,
    name: *const c_char,
    data: *const c_void,
    size: usize,
    alignment: usize,
) -> PonkoStatus {
    boundary(|| {
        let name = resource_name(name)?;
        let layout = resource_layout(size, alignment)?;
        if data.is_null() {
            return Err(invalid("data is null"));
        }
        // SAFETY: access validates null and guards the live handle.
        unsafe {
            access(world, |w, _| {
                same_layout(w, name, layout)?;
                // SAFETY: the layout matches, `data` holds `size` bytes of a plain value.
                w.raw().insert_resource_dynamic(name, layout, data.cast());
                Ok(())
            })
        }
    })
}

/// Copies the resource called `name` into `out` (`size` bytes, which must be
/// its size). `PONKO_NOT_FOUND` if the world has none.
/// ```
/// use ponko_ffi::*;
/// let w = ponko_world_new();
/// let mut out: u32 = 0;
/// unsafe {
///     let status = ponko_resource_get(w, c"Score".as_ptr(), (&raw mut out).cast(), 4);
///     assert_eq!(status, PonkoStatus::PONKO_NOT_FOUND);
///     ponko_world_free(w);
/// }
/// ```
/// # Safety
/// `world` must be null or live; `name` NUL-terminated; `out` must hold
/// `size` writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ponko_resource_get(
    world: *mut PonkoWorld,
    name: *const c_char,
    out: *mut c_void,
    size: usize,
) -> PonkoStatus {
    boundary(|| {
        let name = resource_name(name)?;
        if out.is_null() {
            return Err(invalid("output is null"));
        }
        // SAFETY: access validates null and guards the live handle.
        unsafe {
            access(world, |w, _| {
                let layout = w
                    .raw_ref()
                    .resource_layout_dynamic(name)
                    .ok_or_else(|| error(PonkoStatus::PONKO_NOT_FOUND, "resource not found"))?;
                if layout.size() != size {
                    return Err(invalid("resource copy size does not match"));
                }
                let from = w.raw_ref().get_resource_dynamic(name).expect("layout implies presence");
                // SAFETY: both hold `size` bytes; the world's copy is separate memory.
                std::ptr::copy_nonoverlapping(from, out.cast::<u8>(), size);
                Ok(())
            })
        }
    })
}

/// Removes the resource called `name`; `PONKO_NOT_FOUND` if there is none.
/// ```
/// use ponko_ffi::*;
/// let w = ponko_world_new();
/// let seed: u64 = 42;
/// unsafe {
///     ponko_resource_set(w, c"Seed".as_ptr(), (&raw const seed).cast(), 8, 8);
///     assert_eq!(ponko_resource_remove(w, c"Seed".as_ptr()), PonkoStatus::PONKO_OK);
///     assert_eq!(ponko_resource_remove(w, c"Seed".as_ptr()), PonkoStatus::PONKO_NOT_FOUND);
///     ponko_world_free(w);
/// }
/// ```
/// # Safety
/// `world` must be null or live; `name` NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ponko_resource_remove(world: *mut PonkoWorld, name: *const c_char) -> PonkoStatus {
    boundary(|| {
        let name = resource_name(name)?;
        // SAFETY: access validates null and guards the live handle.
        unsafe {
            access(world, |w, _| {
                if w.raw().remove_resource_dynamic(name) {
                    Ok(())
                } else {
                    Err(error(PonkoStatus::PONKO_NOT_FOUND, "resource not found"))
                }
            })
        }
    })
}

/// Makes the resource called `name` readable and writable as JSON by tools
/// and agents, and saved by `ponko_world_save`, by describing its fields.
/// It doesn't have to exist yet: a tool setting it adds it, with bytes no
/// field covers zero. Describing it again replaces the fields. A name a
/// Rust resource is reflected under returns `PONKO_PANIC`, saying so.
/// ```
/// use ponko_ffi::*;
/// let w = ponko_world_new();
/// let field = PonkoField { name: c"red".as_ptr(), kind: PonkoFieldType::PONKO_FIELD_U32 as u32, offset: 0 };
/// unsafe {
///     assert_eq!(ponko_resource_reflect(w, c"Score".as_ptr(), 8, 4, &field, 1), PonkoStatus::PONKO_OK);
///     ponko_world_free(w);
/// }
/// ```
/// # Safety
/// `world` must be null or live; `name` NUL-terminated; `fields` must hold
/// `len` fields with valid names. Any field values must make a valid value
/// of the resource (true for plain C structs of these types).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ponko_resource_reflect(
    world: *mut PonkoWorld,
    name: *const c_char,
    size: usize,
    alignment: usize,
    fields: *const PonkoField,
    len: usize,
) -> PonkoStatus {
    boundary(|| {
        let name = resource_name(name)?;
        let layout = resource_layout(size, alignment)?;
        // SAFETY: forwarded from the caller.
        let list = unsafe { fields_from_c(fields, len)? };
        check_fields(&list, size, "resource")?;
        // SAFETY: access validates null and guards the live handle.
        unsafe {
            access(world, |w, _| {
                same_layout(w, name, layout)?;
                // SAFETY: fields checked; the caller promised field values make valid values.
                w.reflect_resource_fields(name, layout, &list);
                Ok(())
            })
        }
    })
}
