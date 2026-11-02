//! C-compatible, thread-confined access to dynamic POD components.
//!
//! All handles and callbacks must stay on the creating thread. Callers supply
//! valid, suitably aligned memory; null pointers are diagnosed, but dangling,
//! forged, overlapping, or undersized memory is a caller error. Components must
//! be memcpy-safe, contain no Rust-owned values, and require no destruction.
//! Callbacks must return normally: neither C++ exceptions nor Rust panics may
//! cross the C ABI. Chunk memory is borrowed only until its callback returns.
//!
//! ```
//! use crabby_ffi::{crabby_world_new, crabby_world_free, CrabbyStatus};
//! let world = crabby_world_new();
//! assert!(!world.is_null());
//! unsafe { assert_eq!(crabby_world_free(world), CrabbyStatus::CRABBY_OK); }
//! ```

mod headless;
pub use headless::*;

use crabby_ecs::{ComponentId, Entity, World};
use std::alloc::Layout;
use std::cell::{Cell, RefCell};
use std::ffi::{CStr, c_char, c_void};
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

/// Result of an ABI operation. Inspect `crabby_last_error` on failure.
/// ```
/// assert_eq!(crabby_ffi::CrabbyStatus::CRABBY_OK as u32, 0);
/// ```
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyStatus {
    /// Operation succeeded.
    CRABBY_OK = 0,
    /// An argument, component id, or layout was invalid.
    CRABBY_INVALID_ARGUMENT = 1,
    /// The entity or its requested component does not exist.
    CRABBY_NOT_FOUND = 2,
    /// A callback currently borrows this world, or free targeted a borrowed handle.
    CRABBY_BUSY = 3,
    /// Rust caught a panic. No unwind crossed the ABI.
    CRABBY_PANIC = 4,
}
use CrabbyStatus::*;

/// Opaque, thread-confined world. Never copy, dereference, or free it yourself.
/// ```
/// let handle = crabby_ffi::crabby_world_new();
/// unsafe { crabby_ffi::crabby_world_free(handle); }
/// ```
pub struct CrabbyWorld {
    world: *mut World,
    busy: Cell<bool>,
    owned: bool,
    _thread_confined: PhantomData<Rc<()>>,
}

/// A component value copied into the world; `size` must match registration.
/// A non-null readable pointer is required even for a zero-sized value.
/// ```
/// let byte = 1u8;
/// let value = crabby_ffi::CrabbyValue { component: 0, data: (&byte as *const u8).cast(), size: 1 };
/// assert_eq!(value.size, 1);
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CrabbyValue {
    /// World-local registered component id.
    pub component: u32,
    /// Readable component bytes, not overlapping world storage.
    pub data: *const c_void,
    /// Exact registered component size.
    pub size: usize,
}

/// Borrowed archetype columns in query order. Valid only during the callback.
/// `entities` is opaque: use `crabby_chunk_entity` instead of dereferencing it.
/// Column elements are tightly packed with the registered size as their stride.
/// ```
/// assert!(std::mem::size_of::<crabby_ffi::CrabbyChunk>() >= 4 * std::mem::size_of::<usize>());
/// ```
#[repr(C)]
pub struct CrabbyChunk {
    /// Number of rows.
    pub count: usize,
    /// Number of columns, in the requested component order.
    pub column_count: usize,
    /// Array of mutable column pointers; never retain it after the callback.
    pub columns: *const *mut c_void,
    /// Opaque entity storage; never dereference it in foreign code.
    pub entities: *const c_void,
}

/// Callback invoked once per nonempty matching archetype. Must return normally.
/// ```
/// let callback: crabby_ffi::CrabbyQueryFn = None;
/// assert!(callback.is_none());
/// ```
pub type CrabbyQueryFn = Option<unsafe extern "C" fn(*const CrabbyChunk, *mut c_void)>;

thread_local! {
    static LAST_ERROR: RefCell<[u8; 512]> = const { RefCell::new([0; 512]) };
}

pub(crate) struct Failure(CrabbyStatus, &'static str);
pub(crate) fn error(status: CrabbyStatus, message: &'static str) -> Failure {
    Failure(status, message)
}
fn invalid(message: &'static str) -> Failure {
    error(CRABBY_INVALID_ARGUMENT, message)
}
fn save_error(message: &str) {
    LAST_ERROR.with(|cell| {
        let mut buffer = cell.borrow_mut();
        let n = message.len().min(buffer.len() - 1);
        buffer[..n].copy_from_slice(&message.as_bytes()[..n]);
        buffer[n] = 0;
    });
}
pub(crate) fn boundary(f: impl FnOnce() -> Result<(), Failure>) -> CrabbyStatus {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => CRABBY_OK,
        Ok(Err(Failure(status, message))) => {
            save_error(message);
            status
        }
        Err(payload) => {
            // Drop ordinary payloads, but contain a payload destructor that panics.
            // A second payload is intentionally retained: its destructor could
            // panic recursively, and no unwind may escape this ABI boundary.
            if let Err(secondary) = catch_unwind(AssertUnwindSafe(|| drop(payload))) {
                std::mem::forget(secondary);
            }
            save_error("Rust panic caught at C ABI boundary");
            CRABBY_PANIC
        }
    }
}

pub(crate) fn owned_world(world: World) -> *mut CrabbyWorld {
    Box::into_raw(Box::new(CrabbyWorld {
        world: Box::into_raw(Box::new(world)),
        busy: Cell::new(false),
        owned: true,
        _thread_confined: PhantomData,
    }))
}

pub(crate) fn with_lent_world<R>(world: &mut World, f: impl FnOnce(*mut CrabbyWorld) -> R) -> R {
    let mut handle = CrabbyWorld {
        world,
        busy: Cell::new(false),
        owned: false,
        _thread_confined: PhantomData,
    };
    f(&mut handle)
}

struct BusyGuard<'a>(&'a Cell<bool>);
impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
unsafe fn access<R>(handle: *mut CrabbyWorld, f: impl FnOnce(&mut World) -> Result<R, Failure>) -> Result<R, Failure> {
    if handle.is_null() {
        return Err(invalid("world is null"));
    }
    // SAFETY: caller supplies a live thread-confined handle. Shared access only
    // touches the guard before creating any unique world reference.
    let handle = unsafe { &*handle };
    if handle.busy.replace(true) {
        return Err(error(CRABBY_BUSY, "world is borrowed by another operation"));
    }
    let _guard = BusyGuard(&handle.busy);
    // SAFETY: this guard exclusively owns world access until it drops.
    f(unsafe { &mut *handle.world })
}
fn component(world: &World, id: u32) -> Result<ComponentId, Failure> {
    let info = world
        .components()
        .get(id as usize)
        .ok_or_else(|| invalid("component id is not registered in this world"))?;
    Ok(world.component_by_name(info.name()).expect("registered component name"))
}
fn value_layout(world: &World, id: u32, data: *const c_void, size: usize) -> Result<ComponentId, Failure> {
    let id = component(world, id)?;
    if data.is_null() {
        return Err(invalid("component data is null"));
    }
    if size != world.component_info(id).layout().size() {
        return Err(invalid("component copy size does not match registration"));
    }
    Ok(id)
}
fn present(found: bool) -> Result<(), Failure> {
    if found {
        Ok(())
    } else {
        Err(error(CRABBY_NOT_FOUND, "entity or component not found"))
    }
}

/// Creates an owned empty world; returns null if Rust panics.
/// ```
/// let world = crabby_ffi::crabby_world_new();
/// unsafe { crabby_ffi::crabby_world_free(world); }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_world_new() -> *mut CrabbyWorld {
    let mut result = std::ptr::null_mut();
    boundary(|| {
        result = owned_world(World::new());
        Ok(())
    });
    result
}

/// Frees an owned handle. A busy or borrowed handle returns `CRABBY_BUSY`.
/// # Safety
/// `world` must be null or a live handle on its creating thread, used at most once after success.
/// ```
/// let w = crabby_ffi::crabby_world_new();
/// unsafe { assert_eq!(crabby_ffi::crabby_world_free(w), crabby_ffi::CrabbyStatus::CRABBY_OK); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_free(world: *mut CrabbyWorld) -> CrabbyStatus {
    boundary(|| {
        if world.is_null() {
            return Err(invalid("world is null"));
        }
        // SAFETY: live handle guaranteed by caller; inspect before taking ownership.
        let h = unsafe { &*world };
        if h.busy.get() || !h.owned {
            return Err(error(CRABBY_BUSY, "world is borrowed"));
        }
        // SAFETY: owned, non-busy handle and its World each came from Box::into_raw.
        unsafe {
            drop(Box::from_raw(h.world));
            drop(Box::from_raw(world));
        }
        Ok(())
    })
}

/// Last error on this thread; valid until the next failed ABI call on this thread.
/// Successful calls leave it unchanged. Never free the returned pointer.
/// ```
/// assert!(!crabby_ffi::crabby_last_error().is_null());
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_last_error() -> *const c_char {
    LAST_ERROR.with(|cell| cell.as_ptr().cast())
}

/// ABI version, currently 1.
/// ```
/// assert_eq!(crabby_ffi::crabby_abi_version(), 1);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_abi_version() -> u32 {
    1
}

/// Registers a POD component. Repeating a name and layout returns the same id.
/// Size must be a multiple of nonzero power-of-two alignment, including size zero.
/// Names are nonempty UTF-8, NUL-terminated strings, copied by this call.
/// # Safety
/// Live thread-confined handle; valid readable name and writable output pointer.
/// Component values must be memcpy-safe, thread-safe bytes without destructors.
/// ```
/// use crabby_ffi::*;
/// let w = crabby_world_new();
/// unsafe {
/// let mut id = 0;
/// assert_eq!(crabby_component_register(w, c"value".as_ptr(), 4, 4, &mut id), CrabbyStatus::CRABBY_OK);
/// crabby_world_free(w);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_component_register(
    world: *mut CrabbyWorld,
    name: *const c_char,
    size: usize,
    alignment: usize,
    out_id: *mut u32,
) -> CrabbyStatus {
    boundary(|| {
        if name.is_null() || out_id.is_null() {
            return Err(invalid("name or output is null"));
        }
        let layout = Layout::from_size_align(size, alignment).map_err(|_| invalid("invalid component layout"))?;
        if !size.is_multiple_of(alignment) {
            return Err(invalid("component size must be a multiple of alignment"));
        }
        // SAFETY: caller guarantees NUL-terminated readable name.
        let name = unsafe { CStr::from_ptr(name) }
            .to_str()
            .map_err(|_| invalid("name is not UTF-8"))?;
        if name.is_empty() {
            return Err(invalid("name is empty"));
        }
        // SAFETY: access validates null and guards the live caller-provided handle.
        unsafe {
            access(world, |w| {
                if let Some(id) = w.component_by_name(name) {
                    let info = w.component_info(id);
                    if info.layout() != layout || info.type_id().is_some() {
                        return Err(invalid("name is already registered with another layout or type"));
                    }
                }
                // Caller promises POD values. Layout and collision are checked above.
                let id = w.register_component(name, size, alignment, None);
                *out_id = id.index() as u32;
                Ok(())
            })
        }
    })
}

/// Copies values into a new entity. Null values are allowed only when count is zero.
/// Duplicate component ids are rejected. Output is written only on success.
/// # Safety
/// Live handle, readable array of `count` values and their bytes, writable output.
/// Inputs and output must not overlap world storage or one another.
/// ```
/// use crabby_ffi::*;
/// let w = crabby_world_new();
/// unsafe { let mut e = 0; assert_eq!(crabby_spawn(w, std::ptr::null(), 0, &mut e), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_spawn(
    world: *mut CrabbyWorld,
    values: *const CrabbyValue,
    count: usize,
    out_entity: *mut u64,
) -> CrabbyStatus {
    boundary(|| {
        if out_entity.is_null()
            || (values.is_null() && count != 0)
            || count > isize::MAX as usize / std::mem::size_of::<CrabbyValue>()
        {
            return Err(invalid("invalid values array, count or output"));
        }
        // SAFETY: caller promises valid input memory. Empty null arrays are not dereferenced.
        unsafe {
            access(world, |w| {
                if count > w.components().len() {
                    return Err(invalid("more values than registered components"));
                }
                let values = if count == 0 {
                    &[]
                } else {
                    std::slice::from_raw_parts(values, count)
                };
                let mut components = Vec::with_capacity(count);
                for v in values {
                    let id = value_layout(w, v.component, v.data, v.size)?;
                    if components.iter().any(|(previous, _)| *previous == id) {
                        return Err(invalid("duplicate component id"));
                    }
                    components.push((id, v.data.cast()));
                }
                *out_entity = w.spawn_dynamic(&components).to_bits();
                Ok(())
            })
        }
    })
}

/// Despawns an entity; stale or missing entities return `CRABBY_NOT_FOUND`.
/// # Safety
/// `world` must be null or a live handle on its creating thread.
/// ```
/// use crabby_ffi::*;
/// let w = crabby_world_new();
/// unsafe { assert_eq!(crabby_despawn(w, 42), CrabbyStatus::CRABBY_NOT_FOUND); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_despawn(world: *mut CrabbyWorld, entity: u64) -> CrabbyStatus {
    // SAFETY: caller guarantees live handle, access enforces exclusive use.
    boundary(|| unsafe { access(world, |w| present(w.despawn(Entity::from_bits(entity)))) })
}

/// Tests whether a generational entity handle is alive.
/// # Safety
/// Live thread-confined world and valid writable output, or null pointers.
/// ```
/// use crabby_ffi::*;
/// let w = crabby_world_new();
/// unsafe { let mut alive = true; crabby_is_alive(w, 42, &mut alive); assert!(!alive); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_is_alive(world: *mut CrabbyWorld, entity: u64, out_alive: *mut bool) -> CrabbyStatus {
    boundary(|| {
        if out_alive.is_null() {
            return Err(invalid("output is null"));
        }
        // SAFETY: caller provides writable output and a live handle.
        unsafe {
            access(world, |w| {
                *out_alive = w.is_alive(Entity::from_bits(entity));
                Ok(())
            })
        }
    })
}

/// Returns the number of live entities.
/// # Safety
/// Live thread-confined world and valid writable output, or null pointers.
/// ```
/// use crabby_ffi::*;
/// let w = crabby_world_new();
/// unsafe { let mut len = 1; crabby_world_len(w, &mut len); assert_eq!(len, 0); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_len(world: *mut CrabbyWorld, out_len: *mut usize) -> CrabbyStatus {
    boundary(|| {
        if out_len.is_null() {
            return Err(invalid("output is null"));
        }
        // SAFETY: caller provides writable output and a live handle.
        unsafe {
            access(world, |w| {
                *out_len = w.len();
                Ok(())
            })
        }
    })
}

/// Copies one component into caller storage of the exact registered size.
/// # Safety
/// Live thread-confined handle and valid writable, non-overlapping output bytes.
/// ```
/// use crabby_ffi::*;
/// unsafe { assert_eq!(crabby_get(std::ptr::null_mut(), 0, 0, std::ptr::null_mut(), 0), CrabbyStatus::CRABBY_INVALID_ARGUMENT); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_get(
    world: *mut CrabbyWorld,
    entity: u64,
    id: u32,
    out_data: *mut c_void,
    size: usize,
) -> CrabbyStatus {
    // SAFETY: caller provides live handle and writable output; size/id checked before copy.
    boundary(|| unsafe {
        access(world, |w| {
            let id = value_layout(w, id, out_data, size)?;
            let src = w
                .get_dynamic(Entity::from_bits(entity), id)
                .ok_or_else(|| error(CRABBY_NOT_FOUND, "entity or component not found"))?;
            std::ptr::copy_nonoverlapping(src, out_data.cast(), size);
            Ok(())
        })
    })
}

/// Adds or replaces a component by copying its exact registered size.
/// # Safety
/// Live thread-confined handle and readable POD bytes, not overlapping world storage.
/// ```
/// use crabby_ffi::*;
/// unsafe { assert_eq!(crabby_set(std::ptr::null_mut(), 0, 0, std::ptr::null(), 0), CrabbyStatus::CRABBY_INVALID_ARGUMENT); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_set(
    world: *mut CrabbyWorld,
    entity: u64,
    id: u32,
    data: *const c_void,
    size: usize,
) -> CrabbyStatus {
    // SAFETY: caller provides POD memory and live handle; validate layout before copying.
    boundary(|| unsafe {
        access(world, |w| {
            let id = value_layout(w, id, data, size)?;
            present(w.insert_dynamic(Entity::from_bits(entity), id, data.cast()))
        })
    })
}

/// Removes a component, returning `CRABBY_NOT_FOUND` if entity/component is absent.
/// # Safety
/// `world` must be null or a live handle on its creating thread.
/// ```
/// use crabby_ffi::*;
/// unsafe { assert_eq!(crabby_remove(std::ptr::null_mut(), 0, 0), CrabbyStatus::CRABBY_INVALID_ARGUMENT); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_remove(world: *mut CrabbyWorld, entity: u64, id: u32) -> CrabbyStatus {
    // SAFETY: caller supplies a live handle; component id is checked before ECS access.
    boundary(|| unsafe {
        access(world, |w| {
            let id = component(w, id)?;
            present(w.remove_dynamic(Entity::from_bits(entity), id))
        })
    })
}

/// Visits each nonempty matching archetype, with columns in requested order.
/// At most 32 distinct ids; zero ids visit all entities. Successful traversal
/// allocates no memory. World operations (including reads/free) during a callback
/// return `CRABBY_BUSY`; edit only the provided columns in that callback.
/// # Safety
/// Live thread-confined handle; readable ids; callback and user pointer remain valid.
/// Callbacks must return normally and preserve valid POD values in all columns.
/// ```
/// use crabby_ffi::*;
/// unsafe extern "C" fn visit(_: *const CrabbyChunk, _: *mut std::ffi::c_void) {}
/// let w = crabby_world_new();
/// unsafe { assert_eq!(crabby_query(w, std::ptr::null(), 0, Some(visit), std::ptr::null_mut()), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_query(
    world: *mut CrabbyWorld,
    components: *const u32,
    count: usize,
    callback: CrabbyQueryFn,
    user: *mut c_void,
) -> CrabbyStatus {
    boundary(|| {
        if count > 32 || (components.is_null() && count != 0) {
            return Err(invalid("query requires at most 32 component ids and a valid array"));
        }
        let callback = callback.ok_or_else(|| invalid("query callback is null"))?;
        // SAFETY: caller supplies live handle, input ids and a callable callback.
        unsafe {
            access(world, |w| {
                let input = if count == 0 {
                    &[]
                } else {
                    std::slice::from_raw_parts(components, count)
                };
                if count != 0 {
                    let seed = component(w, input[0])?;
                    let mut resolved = [seed; 32];
                    for (i, &raw) in input.iter().enumerate() {
                        if input[..i].contains(&raw) {
                            return Err(invalid("duplicate query component id"));
                        }
                        resolved[i] = component(w, raw)?;
                    }
                    w.for_each_chunk(&resolved[..count], |chunk| {
                        let view = CrabbyChunk {
                            count: chunk.entities.len(),
                            column_count: chunk.columns.len(),
                            columns: chunk.columns.as_ptr().cast(),
                            entities: chunk.entities.as_ptr().cast(),
                        };
                        callback(&view, user);
                    });
                } else {
                    w.for_each_chunk(&[], |chunk| {
                        let view = CrabbyChunk {
                            count: chunk.entities.len(),
                            column_count: 0,
                            columns: chunk.columns.as_ptr().cast(),
                            entities: chunk.entities.as_ptr().cast(),
                        };
                        callback(&view, user);
                    });
                }
                Ok(())
            })
        }
    })
}

/// Reads a packed entity id from a currently borrowed query chunk.
/// # Safety
/// `chunk` must be an unmodified chunk from the active callback; output must be writable.
/// A fabricated chunk, a retained chunk, or dangling pointers are caller errors.
/// ```
/// use crabby_ffi::*;
/// unsafe { assert_eq!(crabby_chunk_entity(std::ptr::null(), 0, &mut 0), CrabbyStatus::CRABBY_INVALID_ARGUMENT); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_chunk_entity(
    chunk: *const CrabbyChunk,
    index: usize,
    out_entity: *mut u64,
) -> CrabbyStatus {
    boundary(|| {
        if chunk.is_null() || out_entity.is_null() {
            return Err(invalid("chunk or output is null"));
        }
        // SAFETY: caller supplies a live unmodified callback chunk.
        let chunk = unsafe { &*chunk };
        if index >= chunk.count || chunk.entities.is_null() {
            return Err(invalid("chunk entity index is out of bounds"));
        }
        // SAFETY: bounds checked; original chunk contains count initialized Entity entries.
        unsafe {
            *out_entity = (*chunk.entities.cast::<Entity>().add(index)).to_bits();
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn panics_are_contained_and_release_the_borrow_guard() {
        let world = crabby_world_new();
        let result = boundary(|| unsafe { access(world, |_| -> Result<(), Failure> { panic!("injected ABI panic") }) });
        assert_eq!(result, CRABBY_PANIC);
        unsafe {
            assert_eq!(crabby_world_len(world, &mut 0), CRABBY_OK);
            assert_eq!(crabby_world_free(world), CRABBY_OK);
        }
    }
    #[test]
    fn caught_panic_payloads_are_dropped() {
        struct Payload(std::sync::Arc<std::sync::atomic::AtomicBool>);
        impl Drop for Payload {
            fn drop(&mut self) {
                self.0.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }
        let dropped = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let payload = Payload(dropped.clone());
        assert_eq!(boundary(|| std::panic::panic_any(payload)), CRABBY_PANIC);
        assert!(dropped.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn lent_handles_access_the_original_world_and_cannot_be_freed() {
        let mut world = World::new();
        with_lent_world(&mut world, |handle| unsafe {
            assert_eq!(crabby_world_free(handle), CRABBY_BUSY);
            assert_eq!(crabby_spawn(handle, std::ptr::null(), 0, &mut 0), CRABBY_OK);
        });
        assert_eq!(world.len(), 1);
    }
}
