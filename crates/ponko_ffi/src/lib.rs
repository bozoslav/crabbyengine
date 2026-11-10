//! The C ABI of Ponko; `include/crabby.h` is generated from it and
//! `include/crabby.hpp` wraps it for C++.
//!
//! Handles are confined to the thread that made them, except inside
//! `crabby_query_par` callbacks, which run on many threads and may only touch
//! their chunk, read-only context values and spatial queries. Callers supply
//! valid, suitably aligned memory; null pointers are diagnosed, but dangling,
//! forged, overlapping, or undersized memory is a caller error. Components
//! made from C must be memcpy-safe. Callbacks must return normally: neither
//! C++ exceptions nor Rust panics may cross the C ABI. Chunk memory is
//! borrowed only until its callback returns.
//!
//! ```
//! use ponko_ffi::{crabby_world_new, crabby_world_free, CrabbyStatus};
//! let world = crabby_world_new();
//! assert!(!world.is_null());
//! unsafe { assert_eq!(crabby_world_free(world), CrabbyStatus::CRABBY_OK); }
//! ```

#![warn(missing_docs)]

mod access;
mod actions;
mod animator;
mod attach;
mod audio3d;
mod changes;
mod contacts3d;
mod ctx;
mod flow;
mod game;
mod grid;
mod handles;
mod keys;
mod media;
mod messages;
mod morph;
mod nav3d;
mod net;
mod particles;
mod perf;
mod physics;
mod physics3d;
mod physics3d_stream;
mod ragdoll;
mod reflect;
mod replicate;
mod residency;
mod resources;
mod scene;
mod shader_status;
mod skin;
mod steam;
mod steam_lobby;
mod stream;
mod strips;
mod text3d;
mod three;
// After `three`: it uses the quality struct, which the header defines there.
mod gpu;
mod tiles;
mod types;
mod ui_panel3d;
mod ui_retained;
mod vehicle3d;
pub use access::*;
pub use actions::*;
pub use animator::*;
/// Counts the library's heap allocations while profiling is on (one atomic
/// load per allocation otherwise), so a C or C++ game's reports show how many
/// the engine makes. A Rust program that links this crate as a library and
/// wants its own allocator should depend on the engine instead.
#[global_allocator]
static ALLOCATOR: ponko::CountingAllocator = ponko::CountingAllocator;

pub use attach::*;
pub use audio3d::*;
pub use changes::*;
pub use contacts3d::*;
pub use ctx::*;
pub use flow::*;
pub use game::*;
pub use gpu::*;
pub use grid::*;
pub use handles::*;
pub use keys::*;
pub use media::*;
pub use messages::*;
pub use morph::*;
pub use nav3d::*;
pub use net::*;
pub use particles::*;
pub use perf::*;
pub use physics::*;
pub use physics3d::*;
pub use physics3d_stream::*;
pub use ragdoll::*;
pub use reflect::*;
pub use replicate::*;
pub use residency::*;
pub use resources::*;
pub use scene::*;
pub use shader_status::*;
pub use skin::*;
pub use steam::*;
pub use steam_lobby::*;
pub use stream::*;
pub use strips::*;
pub use text3d::*;
pub use three::*;
pub use tiles::*;
pub use types::*;
pub use ui_panel3d::*;
pub use ui_retained::*;
pub use vehicle3d::*;

use ponko::{Sprite, Transform2D, spatial_key};
use ponko_ecs::{ComponentId, Entity, World};
use std::alloc::Layout;
use std::any::TypeId;
use std::cell::RefCell;
use std::ffi::{CStr, c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};

/// Result of an ABI operation. Inspect `crabby_last_error` on failure.
/// ```
/// assert_eq!(ponko_ffi::CrabbyStatus::CRABBY_OK as u32, 0);
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

/// Opaque world handle. Never copy, dereference, or free it yourself.
/// ```
/// let handle = ponko_ffi::crabby_world_new();
/// unsafe { ponko_ffi::crabby_world_free(handle); }
/// ```
pub struct CrabbyWorld {
    world: *mut World,
    /// Set while an operation (or a query callback) uses the world.
    busy: AtomicBool,
    /// Set while a parallel query runs: commands are off then (one queue, many threads).
    parallel: AtomicBool,
    owned: bool,
}

/// A component value copied into the world; `size` must match registration.
/// A non-null readable pointer is required even for a zero-sized value.
/// ```
/// let byte = 1u8;
/// let value = ponko_ffi::CrabbyValue { component: 0, data: (&byte as *const u8).cast(), size: 1 };
/// assert_eq!(value.size, 1);
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CrabbyValue {
    /// World-local registered component id.
    pub component: u32,
    /// Readable component bytes, not overlapping world storage.
    pub data: *const c_void,
    /// Exact registered component size (`sizeof(CrabbySprite)` for sprites).
    pub size: usize,
}

/// Borrowed archetype columns in query order. Valid only during the callback.
/// Column elements are tightly packed with the registered size as their stride.
/// ```
/// assert!(std::mem::size_of::<ponko_ffi::CrabbyChunk>() >= 4 * std::mem::size_of::<usize>());
/// ```
#[repr(C)]
pub struct CrabbyChunk {
    /// Number of rows.
    pub count: usize,
    /// Number of columns, in the requested component order.
    pub column_count: usize,
    /// Array of mutable column pointers; never retain it after the callback.
    pub columns: *const *mut c_void,
    /// The entity of each row.
    pub entities: *const CrabbyEntity,
    /// Where row 0 is in the whole query: rows are numbered `0..total` in
    /// the same order every time, so a parallel query can fill row
    /// `first + i` of a buffer sized to the query's count.
    pub first: usize,
}

/// Callback invoked for each nonempty matching archetype (or slice of one,
/// for `crabby_query_par`). Must return normally.
/// ```
/// let callback: ponko_ffi::CrabbyQueryFn = None;
/// assert!(callback.is_none());
/// ```
pub type CrabbyQueryFn = Option<unsafe extern "C" fn(*const CrabbyChunk, *mut c_void)>;

thread_local! {
    static LAST_ERROR: RefCell<[u8; 512]> = const { RefCell::new([0; 512]) };
}

pub(crate) struct Failure(CrabbyStatus, std::borrow::Cow<'static, str>);
pub(crate) fn error(status: CrabbyStatus, message: &'static str) -> Failure {
    Failure(status, message.into())
}
pub(crate) fn invalid(message: &'static str) -> Failure {
    error(CRABBY_INVALID_ARGUMENT, message)
}
pub(crate) fn invalid_owned(message: String) -> Failure {
    Failure(CRABBY_INVALID_ARGUMENT, message.into())
}
pub(crate) fn save_error(message: &str) {
    LAST_ERROR.with(|cell| {
        let mut buffer = cell.borrow_mut();
        let mut n = message.len().min(buffer.len() - 1);
        while !message.is_char_boundary(n) {
            n -= 1;
        }
        buffer[..n].copy_from_slice(&message.as_bytes()[..n]);
        buffer[n] = 0;
    });
}
pub(crate) fn boundary(f: impl FnOnce() -> Result<(), Failure>) -> CrabbyStatus {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => CRABBY_OK,
        Ok(Err(Failure(status, message))) => {
            save_error(&message);
            status
        }
        Err(payload) => {
            let text = payload
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("unknown panic payload");
            save_error(&format!("Rust panic caught at C ABI boundary: {text}"));
            // Drop ordinary payloads, but contain a payload destructor that panics.
            // A second payload is intentionally retained: its destructor could
            // panic recursively, and no unwind may escape this ABI boundary.
            if let Err(secondary) = catch_unwind(AssertUnwindSafe(|| drop(payload))) {
                std::mem::forget(secondary);
            }
            CRABBY_PANIC
        }
    }
}

pub(crate) fn owned_world(world: World) -> *mut CrabbyWorld {
    Box::into_raw(Box::new(CrabbyWorld::new(Box::into_raw(Box::new(world)), true)))
}

impl CrabbyWorld {
    pub(crate) fn new(world: *mut World, owned: bool) -> Self {
        Self {
            world,
            busy: AtomicBool::new(false),
            parallel: AtomicBool::new(false),
            owned,
        }
    }
    /// Takes the busy flag; `None` if something else holds it.
    pub(crate) fn lock(&self) -> Option<BusyGuard<'_>> {
        (!self.busy.swap(true, Ordering::Acquire)).then(|| BusyGuard(&self.busy))
    }
}

pub(crate) struct BusyGuard<'a>(&'a AtomicBool);
impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
pub(crate) unsafe fn access<R>(
    handle: *mut CrabbyWorld,
    f: impl FnOnce(&mut World, &CrabbyWorld) -> Result<R, Failure>,
) -> Result<R, Failure> {
    if handle.is_null() {
        return Err(invalid("world is null"));
    }
    // SAFETY: caller supplies a live handle. Shared access only touches the
    // atomic guard before creating any unique world reference.
    let handle = unsafe { &*handle };
    let Some(_guard) = handle.lock() else {
        return Err(error(CRABBY_BUSY, "world is borrowed by another operation"));
    };
    // SAFETY: the guard grants exclusive world access until it drops.
    f(unsafe { &mut *handle.world }, handle)
}

/// How the C side sees a component's bytes.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Kind {
    /// Same bytes in C and in the world: C-registered components, Transform2D.
    Plain,
    /// A Rust type seen in C as a mirror struct (`CrabbySprite`, ...),
    /// converted on every spawn/get/set.
    Mirror(&'static Mirror),
}
impl Kind {
    pub(crate) fn is_plain(self) -> bool {
        matches!(self, Kind::Plain)
    }
}
const _: () = assert!(!std::mem::needs_drop::<ponko::Animation>());

/// A Rust component and the C struct that stands for it.
pub(crate) trait CMirror: Sized + 'static {
    type C: Copy;
    /// Checks a value from C; bad fields are `CRABBY_INVALID_ARGUMENT`.
    fn from_c(c: &Self::C) -> Result<Self, Failure>;
    fn to_c(&self) -> Self::C;
}

/// Type-erased conversions of a [`CMirror`].
#[derive(Debug)]
pub(crate) struct Mirror {
    c_size: usize,
    /// Reads a C value (maybe unaligned) and writes the Rust value to a [`Slot`].
    from_c: unsafe fn(*const c_void, *mut Slot) -> Result<(), Failure>,
    /// Reads a stored Rust value and writes the C value (maybe unaligned).
    to_c: unsafe fn(*const u8, *mut c_void),
}

/// Room for one converted Rust value, without a heap allocation per value
/// (the largest is `Joint3D`, about 320 bytes).
#[repr(C, align(16))]
pub(crate) struct Slot(std::mem::MaybeUninit<[u8; 512]>);
impl Slot {
    pub(crate) const fn new() -> Self {
        Self(std::mem::MaybeUninit::uninit())
    }
}

pub(crate) const fn mirror<T: CMirror>() -> Mirror {
    // Values are moved as raw bytes and never dropped.
    assert!(!std::mem::needs_drop::<T>());
    assert!(size_of::<T>() <= size_of::<Slot>() && align_of::<T>() <= align_of::<Slot>());
    unsafe fn from_c<T: CMirror>(src: *const c_void, dst: *mut Slot) -> Result<(), Failure> {
        // SAFETY: the caller checked `src` holds `size_of::<T::C>()` readable
        // bytes; `dst` fits `T` (asserted above).
        unsafe {
            let value = T::from_c(&src.cast::<T::C>().read_unaligned())?;
            dst.cast::<T>().write(value);
        }
        Ok(())
    }
    unsafe fn to_c<T: CMirror>(src: *const u8, dst: *mut c_void) {
        // SAFETY: `src` is a stored `T`; the caller checked `dst` holds
        // `size_of::<T::C>()` writable bytes.
        unsafe { dst.cast::<T::C>().write_unaligned((*src.cast::<T>()).to_c()) }
    }
    Mirror {
        c_size: size_of::<T::C>(),
        from_c: from_c::<T>,
        to_c: to_c::<T>,
    }
}
impl Mirror {
    /// Converts the C value at `src` into `slot`; returns the Rust value's address.
    /// # Safety
    /// `src` readable for `c_size` bytes.
    pub(crate) unsafe fn convert(&self, src: *const c_void, slot: &mut Slot) -> Result<*const u8, Failure> {
        let slot: *mut Slot = slot;
        // SAFETY: forwarded caller contract.
        unsafe { (self.from_c)(src, slot)? };
        Ok(slot.cast_const().cast())
    }
}

static SPRITE: Mirror = mirror::<Sprite>();

pub(crate) fn component(world: &World, id: u32) -> Result<(ComponentId, Kind), Failure> {
    let id = world
        .component_at(id as usize)
        .ok_or_else(|| invalid("component id is not registered in this world"))?;
    let info = world.component_info(id);
    let kind = match info.type_id() {
        // Raw byte copies into a value with a destructor would corrupt memory.
        None if !info.needs_drop() => Kind::Plain,
        Some(t) if t == TypeId::of::<Transform2D>() => Kind::Plain,
        // repr(C), layout checked against CrabbyAttached; any bits are a valid value.
        Some(t) if t == TypeId::of::<ponko::Attached>() => Kind::Plain,
        // repr(C), layout checked against CrabbyAnimation; `looping` must be 0 or 1.
        Some(t) if t == TypeId::of::<ponko::Animation>() => Kind::Plain,
        Some(t) if t == TypeId::of::<Sprite>() => Kind::Mirror(&SPRITE),
        // repr(C), layout checked against CrabbyTransform3D / CrabbyModel;
        // any bits are a valid value (unknown meshes draw nothing).
        Some(t) if t == TypeId::of::<ponko::Transform3D>() => Kind::Plain,
        Some(t) if t == TypeId::of::<ponko::Model>() => Kind::Plain,
        Some(t) if t == TypeId::of::<ponko::PointLight3D>() => Kind::Plain,
        Some(t) if t == TypeId::of::<ponko::SpotLight3D>() => Kind::Plain,
        // repr(C), layout checked against CrabbyReflectionProbe3D; `content`
        // is checked when a bake or the renderer uses it, and non-finite
        // sizes or intensities give the probe no influence.
        Some(t) if t == TypeId::of::<ponko::ReflectionProbe3D>() => Kind::Plain,
        // repr(C), layouts checked against CrabbyDecal3D / CrabbyReceivesDecals;
        // any bits are valid (non-finite or empty values draw no decal, and
        // images outside the decal textures draw as flat color).
        Some(t) if t == TypeId::of::<ponko::Decal3D>() => Kind::Plain,
        Some(t) if t == TypeId::of::<ponko::ReceivesDecals>() => Kind::Plain,
        // repr(C), layout checked against CrabbyTerrainTile; unknown meshes
        // draw nothing and every level index is clamped.
        Some(t) if t == TypeId::of::<ponko::TerrainTile>() => Kind::Plain,
        Some(t) => match crate::physics::kind(t)
            .or_else(|| crate::physics3d::kind3d(t))
            .or_else(|| crate::physics3d_stream::stream_kind(t))
            .or_else(|| crate::contacts3d::contact_kind(t))
        {
            Some(kind) => kind,
            None => {
                return Err(invalid(
                    "component has no C layout (a Rust type or one with a destructor)",
                ));
            }
        },
        None => {
            return Err(invalid(
                "component has no C layout (a Rust type or one with a destructor)",
            ));
        }
    };
    Ok((id, kind))
}
pub(crate) fn c_size(world: &World, id: ComponentId, kind: Kind) -> usize {
    match kind {
        Kind::Plain => world.component_info(id).layout().size(),
        Kind::Mirror(m) => m.c_size,
    }
}
pub(crate) fn value_layout(
    world: &World,
    id: u32,
    data: *const c_void,
    size: usize,
) -> Result<(ComponentId, Kind), Failure> {
    let (id, kind) = component(world, id)?;
    if data.is_null() {
        return Err(invalid("component data is null"));
    }
    if size != c_size(world, id, kind) {
        return Err(invalid("component copy size does not match registration"));
    }
    Ok((id, kind))
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
/// let world = ponko_ffi::crabby_world_new();
/// unsafe { ponko_ffi::crabby_world_free(world); }
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
/// `world` must be null or a live handle, used at most once after success.
/// ```
/// let w = ponko_ffi::crabby_world_new();
/// unsafe { assert_eq!(ponko_ffi::crabby_world_free(w), ponko_ffi::CrabbyStatus::CRABBY_OK); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_free(world: *mut CrabbyWorld) -> CrabbyStatus {
    boundary(|| {
        if world.is_null() {
            return Err(invalid("world is null"));
        }
        // SAFETY: live handle guaranteed by caller; inspect before taking ownership.
        let h = unsafe { &*world };
        if !h.owned {
            return Err(error(CRABBY_BUSY, "world is borrowed"));
        }
        let Some(guard) = h.lock() else {
            return Err(error(CRABBY_BUSY, "world is borrowed"));
        };
        std::mem::forget(guard);
        // SAFETY: owned, locked handle and its World each came from Box::into_raw.
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
/// assert!(!ponko_ffi::crabby_last_error().is_null());
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_last_error() -> *const c_char {
    LAST_ERROR.with(|cell| cell.as_ptr().cast())
}

/// ABI version, currently 2.
/// ```
/// assert_eq!(ponko_ffi::crabby_abi_version(), 2);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_abi_version() -> u32 {
    2
}

/// Registers a POD component. Repeating a name and layout returns the same id.
/// Size must be a multiple of nonzero power-of-two alignment, including size zero.
/// Names are nonempty UTF-8, NUL-terminated strings, copied by this call.
/// # Safety
/// Live handle; valid readable name and writable output pointer.
/// Component values must be memcpy-safe, thread-safe bytes without destructors.
/// ```
/// use ponko_ffi::*;
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
            access(world, |w, _| {
                if let Some(id) = w.component_by_name(name) {
                    let info = w.component_info(id);
                    if info.layout() != layout || info.type_id().is_some() || info.needs_drop() {
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

/// The id of the built-in `Transform2D` component (`CrabbyTransform2D` in C),
/// registering it if needed. It can be a query column.
/// # Safety
/// Live handle and writable output.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe { let mut id = 99; assert_eq!(crabby_transform_component(w, &mut id), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_transform_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: caller supplies a live handle and writable output (checked non-null).
    unsafe { builtin::<Transform2D>(world, out_id) }
}

/// The id of the built-in `Sprite` component, registering it if needed. Values
/// are `CrabbySprite` in spawn/get/set; sprites can't be query columns.
/// # Safety
/// Live handle and writable output.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe { let mut id = 99; assert_eq!(crabby_sprite_component(w, &mut id), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_sprite_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: as above.
    unsafe { builtin::<Sprite>(world, out_id) }
}

pub(crate) unsafe fn builtin<T: ponko_ecs::Component>(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    boundary(|| {
        if out_id.is_null() {
            return Err(invalid("output is null"));
        }
        // SAFETY: caller supplies a live handle and writable output.
        unsafe {
            access(world, |w, _| {
                *out_id = w.register::<T>().index() as u32;
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
/// use ponko_ffi::*;
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
            access(world, |w, _| {
                let values = if count == 0 {
                    &[]
                } else {
                    std::slice::from_raw_parts(values, count)
                };
                *out_entity = spawn_values(w, values)?.to_bits();
                Ok(())
            })
        }
    })
}

/// Validates and spawns; shared by `crabby_spawn` and `crabby_ctx_spawn_later`.
pub(crate) unsafe fn spawn_values(w: &mut World, values: &[CrabbyValue]) -> Result<Entity, Failure> {
    if values.len() > w.components().len() {
        return Err(invalid("more values than registered components"));
    }
    // Mirrored values are converted to their Rust form here. Never grown
    // past its capacity, so the pointers into it stay valid.
    let mut slots: Vec<Slot> = Vec::with_capacity(values.len());
    let mut components = Vec::with_capacity(values.len());
    for v in values {
        let (id, kind) = value_layout(w, v.component, v.data, v.size)?;
        if components.iter().any(|(previous, _)| *previous == id) {
            return Err(invalid("duplicate component id"));
        }
        let ptr: *const u8 = match kind {
            Kind::Plain => v.data.cast(),
            Kind::Mirror(m) => {
                slots.push(Slot::new());
                let slot = slots.last_mut().expect("just pushed");
                // SAFETY: size checked; the caller promises readable bytes (maybe unaligned).
                unsafe { m.convert(v.data, slot)? }
            }
        };
        components.push((id, ptr));
    }
    // SAFETY: every pointer refers to a value of the registered layout.
    Ok(unsafe { w.spawn_dynamic(&components) })
}

/// Despawns an entity; stale or missing entities return `CRABBY_NOT_FOUND`.
/// # Safety
/// `world` must be null or a live handle.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe { assert_eq!(crabby_despawn(w, 42), CrabbyStatus::CRABBY_NOT_FOUND); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_despawn(world: *mut CrabbyWorld, entity: u64) -> CrabbyStatus {
    // SAFETY: caller guarantees live handle, access enforces exclusive use.
    boundary(|| unsafe { access(world, |w, _| present(w.despawn(Entity::from_bits(entity)))) })
}

/// Tests whether a generational entity handle is alive.
/// # Safety
/// Live world and valid writable output, or null pointers.
/// ```
/// use ponko_ffi::*;
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
            access(world, |w, _| {
                *out_alive = w.is_alive(Entity::from_bits(entity));
                Ok(())
            })
        }
    })
}

/// Returns the number of live entities.
/// # Safety
/// Live world and valid writable output, or null pointers.
/// ```
/// use ponko_ffi::*;
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
            access(world, |w, _| {
                *out_len = w.len();
                Ok(())
            })
        }
    })
}

/// Copies one component into caller storage of the exact registered size.
/// # Safety
/// Live handle and valid writable, non-overlapping output bytes.
/// ```
/// use ponko_ffi::*;
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
        access(world, |w, _| {
            let (id, kind) = value_layout(w, id, out_data, size)?;
            let src = w
                .get_dynamic(Entity::from_bits(entity), id)
                .ok_or_else(|| error(CRABBY_NOT_FOUND, "entity or component not found"))?;
            match kind {
                Kind::Plain => std::ptr::copy_nonoverlapping(src, out_data.cast(), size),
                Kind::Mirror(m) => (m.to_c)(src, out_data),
            }
            Ok(())
        })
    })
}

/// Adds or replaces a component by copying its exact registered size.
/// # Safety
/// Live handle and readable POD bytes, not overlapping world storage.
/// ```
/// use ponko_ffi::*;
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
        access(world, |w, _| {
            let (id, kind) = value_layout(w, id, data, size)?;
            present(insert_value(w, Entity::from_bits(entity), id, kind, data)?)
        })
    })
}

/// Inserts a validated value; shared by `crabby_set` and `crabby_ctx_set_later`.
/// `Ok(false)`: no such entity.
pub(crate) unsafe fn insert_value(
    w: &mut World,
    e: Entity,
    id: ComponentId,
    kind: Kind,
    data: *const c_void,
) -> Result<bool, Failure> {
    // SAFETY: the caller validated the layout of `data`.
    unsafe {
        Ok(match kind {
            Kind::Plain => w.insert_dynamic(e, id, data.cast()),
            Kind::Mirror(m) => {
                let mut slot = Slot::new();
                let value = m.convert(data, &mut slot)?;
                w.insert_dynamic(e, id, value)
            }
        })
    }
}

/// Removes a component, returning `CRABBY_NOT_FOUND` if entity/component is absent.
/// # Safety
/// `world` must be null or a live handle.
/// ```
/// use ponko_ffi::*;
/// unsafe { assert_eq!(crabby_remove(std::ptr::null_mut(), 0, 0), CrabbyStatus::CRABBY_INVALID_ARGUMENT); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_remove(world: *mut CrabbyWorld, entity: u64, id: u32) -> CrabbyStatus {
    // SAFETY: caller supplies a live handle; component id is checked before ECS access.
    boundary(|| unsafe {
        access(world, |w, _| {
            let (id, _) = component(w, id)?;
            present(w.remove_dynamic(Entity::from_bits(entity), id))
        })
    })
}

/// Reorders entities that have a `Transform2D` so that entities close in
/// space are close in memory (Morton order of `cell_size` cells). Entity ids
/// stay valid. Neighbor queries get much faster on big worlds; doing it
/// every 32 ticks or so is plenty.
/// # Safety
/// `world` must be null or a live handle.
/// ```
/// use ponko_ffi::*;
/// let w = crabby_world_new();
/// unsafe { assert_eq!(crabby_world_sort_spatial(w, 32.0), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_sort_spatial(world: *mut CrabbyWorld, cell_size: f32) -> CrabbyStatus {
    boundary(|| {
        if !(cell_size > 0.0 && cell_size.is_finite()) {
            return Err(invalid("cell size must be positive"));
        }
        // SAFETY: caller supplies a live handle.
        unsafe {
            access(world, |w, _| {
                w.sort_by::<Transform2D>(|t| spatial_key(t.position, cell_size));
                Ok(())
            })
        }
    })
}

/// Checks query ids and resolves them; also returns each column's stride.
pub(crate) fn resolve(w: &World, input: &[u32]) -> Result<([ComponentId; 32], [usize; 32]), Failure> {
    let mut resolved = [ComponentId::default(); 32];
    let mut sizes = [0; 32];
    for (i, &raw) in input.iter().enumerate() {
        if input[..i].contains(&raw) {
            return Err(invalid("duplicate query component id"));
        }
        let (id, kind) = component(w, raw)?;
        if !kind.is_plain() {
            return Err(invalid(
                "sprites and physics bodies/colliders can't be query columns: use crabby_get/crabby_set",
            ));
        }
        resolved[i] = id;
        sizes[i] = w.component_info(id).layout().size();
    }
    Ok((resolved, sizes))
}

pub(crate) fn query_input<'a>(components: *const u32, count: usize) -> Result<&'a [u32], Failure> {
    if count > 32 || (components.is_null() && count != 0) {
        return Err(invalid("query requires at most 32 component ids and a valid array"));
    }
    // SAFETY: caller supplies `count` readable ids (checked non-null when count > 0).
    Ok(if count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(components, count) }
    })
}

/// Visits each nonempty matching archetype, with columns in requested order.
/// At most 32 distinct ids; zero ids visit all entities. Successful traversal
/// allocates no memory. World operations (including reads/free) during a callback
/// return `CRABBY_BUSY`; edit only the provided columns, and use the context's
/// `*_later` commands to spawn or despawn.
///
/// The columns are writable, so every row of a column whose changes are
/// tracked (`crabby_component_track_changes`) counts as changed. To read
/// without that, or to visit only changed rows, use `crabby_query_with`.
/// # Safety
/// Live handle; readable ids; callback and user pointer remain valid.
/// Callbacks must return normally and preserve valid POD values in all columns.
/// ```
/// use ponko_ffi::*;
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
        let input = query_input(components, count)?;
        let callback = callback.ok_or_else(|| invalid("query callback is null"))?;
        // SAFETY: caller supplies live handle, input ids and a callable callback.
        unsafe {
            access(world, |w, _| {
                let (resolved, _) = resolve(w, input)?;
                let mut first = 0;
                w.for_each_chunk(&resolved[..count], |chunk| {
                    let view = CrabbyChunk {
                        count: chunk.entities.len(),
                        column_count: count,
                        columns: chunk.columns.as_ptr().cast(),
                        entities: chunk.entities.as_ptr().cast(),
                        first,
                    };
                    first += view.count;
                    callback(&view, user);
                });
                Ok(())
            })
        }
    })
}

/// Like `crabby_query`, but splits archetypes into slices and calls
/// `callback` for them **on all CPU cores at once**. The callback must be
/// thread-safe: touch only its own chunk's rows, read-only data, the context's
/// read-only values (dt, tick, random bits) and spatial queries. World calls
/// return `CRABBY_BUSY` and commands are off until it returns.
/// # Safety
/// As `crabby_query`, plus: `callback` and `user` must be safe to use from
/// several threads at the same time.
/// ```
/// use ponko_ffi::*;
/// unsafe extern "C" fn visit(_: *const CrabbyChunk, _: *mut std::ffi::c_void) {}
/// let w = crabby_world_new();
/// unsafe { assert_eq!(crabby_query_par(w, std::ptr::null(), 0, Some(visit), std::ptr::null_mut()), CrabbyStatus::CRABBY_OK); crabby_world_free(w); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_query_par(
    world: *mut CrabbyWorld,
    components: *const u32,
    count: usize,
    callback: CrabbyQueryFn,
    user: *mut c_void,
) -> CrabbyStatus {
    boundary(|| {
        let input = query_input(components, count)?;
        let callback = callback.ok_or_else(|| invalid("query callback is null"))?;
        // SAFETY: caller supplies live handle, input ids and a thread-safe callback.
        unsafe {
            access(world, |w, handle| {
                let (resolved, sizes) = resolve(w, input)?;
                let _commands_off = ParallelQuery::begin(handle);
                let threads = rayon::current_num_threads().max(1);
                let mut first = 0;
                w.for_each_chunk(&resolved[..count], |chunk| {
                    chunk_in_parallel(&chunk, &sizes, first, threads, callback, user);
                    first += chunk.entities.len();
                });
                Ok(())
            })
        }
    })
}

/// Marks a world as running a parallel query until dropped: commands are
/// off meanwhile (one queue, many threads).
pub(crate) struct ParallelQuery<'a>(&'a AtomicBool);

impl<'a> ParallelQuery<'a> {
    pub(crate) fn begin(handle: &'a CrabbyWorld) -> Self {
        handle.parallel.store(true, Ordering::Release);
        ParallelQuery(&handle.parallel)
    }
}

impl Drop for ParallelQuery<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// Splits one chunk into slices of rows and calls `callback` for them on all
/// CPU cores. `first` is the chunk's position in the whole query, `sizes`
/// the stride of each column, and `threads` the size of the thread pool.
///
/// # Safety
/// `callback` and `user` must be safe to use from several threads at once,
/// and `sizes` must hold the stride of every column of `chunk`.
pub(crate) unsafe fn chunk_in_parallel(
    chunk: &ponko_ecs::Chunk<'_>,
    sizes: &[usize; 32],
    first: usize,
    threads: usize,
    callback: unsafe extern "C" fn(*const CrabbyChunk, *mut c_void),
    user: *mut c_void,
) {
    use rayon::prelude::*;
    /// Raw pointers handed to worker threads. Each task gets a disjoint row range.
    #[derive(Clone, Copy)]
    struct Shared {
        columns: [*mut u8; 32],
        entities: *const Entity,
        user: *mut c_void,
    }
    // SAFETY: the caller promises the callback and `user` are thread-safe;
    // columns are only touched in disjoint row ranges.
    unsafe impl Send for Shared {}
    unsafe impl Sync for Shared {}
    const MIN_ROWS: usize = if cfg!(miri) { 16 } else { 512 };
    const MAX_ROWS: usize = if cfg!(miri) { 16 } else { 8192 };

    let count = chunk.columns.len();
    let len = chunk.entities.len();
    let mut columns = [std::ptr::null_mut(); 32];
    columns[..count].copy_from_slice(chunk.columns);
    let shared = Shared {
        columns,
        entities: chunk.entities.as_ptr(),
        user,
    };
    let sizes = *sizes;
    let rows = (len / (threads * 4)).clamp(MIN_ROWS, MAX_ROWS);
    let task = move |t: usize| {
        let s = shared;
        let start = t * rows;
        let mut ptrs = [std::ptr::null_mut::<c_void>(); 32];
        for i in 0..count {
            // SAFETY: start < len, so the offset stays inside the column.
            ptrs[i] = unsafe { s.columns[i].add(start * sizes[i]) }.cast();
        }
        let view = CrabbyChunk {
            count: rows.min(len - start),
            column_count: count,
            columns: ptrs.as_ptr(),
            // SAFETY: start < len.
            entities: unsafe { s.entities.add(start) }.cast(),
            first: first + start,
        };
        // SAFETY: the caller's contract for `callback`; the view outlives the call.
        unsafe { callback(&view, s.user) };
    };
    if len <= rows {
        task(0);
    } else {
        (0..len.div_ceil(rows)).into_par_iter().for_each(task);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn panics_are_contained_and_release_the_borrow_guard() {
        let world = crabby_world_new();
        let result =
            boundary(|| unsafe { access(world, |_, _| -> Result<(), Failure> { panic!("injected ABI panic") }) });
        assert_eq!(result, CRABBY_PANIC);
        let message = unsafe { CStr::from_ptr(crabby_last_error()) }.to_str().unwrap();
        assert!(message.ends_with("injected ABI panic"), "{message}");
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

    fn lend<R>(world: &mut World, f: impl FnOnce(*mut CrabbyWorld) -> R) -> R {
        let mut handle = CrabbyWorld::new(world, false);
        f(&mut handle)
    }

    #[test]
    fn lent_handles_access_the_original_world_and_cannot_be_freed() {
        let mut world = World::new();
        lend(&mut world, |handle| unsafe {
            assert_eq!(crabby_world_free(handle), CRABBY_BUSY);
            assert_eq!(crabby_spawn(handle, std::ptr::null(), 0, &mut 0), CRABBY_OK);
        });
        assert_eq!(world.len(), 1);
    }

    #[test]
    fn rust_components_without_a_c_layout_are_rejected() {
        let mut world = World::new();
        let e = world.spawn((String::from("rust owned"), 5u64));
        for id in [
            world.component_id::<String>().unwrap(),
            world.component_id::<u64>().unwrap(),
        ] {
            let size = world.component_info(id).layout().size();
            let id = id.index() as u32;
            let bytes = [0u8; std::mem::size_of::<String>()];
            lend(&mut world, |handle| unsafe {
                let mut out = [0u8; std::mem::size_of::<String>()];
                let get = crabby_get(handle, e.to_bits(), id, out.as_mut_ptr().cast(), size);
                assert_eq!(get, CRABBY_INVALID_ARGUMENT);
                let set = crabby_set(handle, e.to_bits(), id, bytes.as_ptr().cast(), size);
                assert_eq!(set, CRABBY_INVALID_ARGUMENT);
                assert_eq!(crabby_remove(handle, e.to_bits(), id), CRABBY_INVALID_ARGUMENT);
                let value = CrabbyValue {
                    component: id,
                    data: bytes.as_ptr().cast(),
                    size,
                };
                assert_eq!(crabby_spawn(handle, &value, 1, &mut 0), CRABBY_INVALID_ARGUMENT);
                unsafe extern "C" fn never(_: *const CrabbyChunk, _: *mut c_void) {
                    panic!("query must be rejected before visiting");
                }
                let query = crabby_query(handle, &id, 1, Some(never), std::ptr::null_mut());
                assert_eq!(query, CRABBY_INVALID_ARGUMENT);
            });
        }
        assert_eq!(world.get::<String>(e).unwrap(), "rust owned");
    }
}
