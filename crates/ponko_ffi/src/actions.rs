//! Named actions and their serializable binding configuration.
use crate::ctx::{ctx_access, out};
use crate::keys::{button_from_c, gamepad_axis_from_c, gamepad_button_from_c, key_from_c};
use crate::types::{CrabbyVec2, to_c};
use crate::{CrabbyCtx, CrabbyStatus, Failure, boundary, invalid, invalid_owned};
use ponko::{Action, ActionMap, Axis2Binding, AxisBinding, ButtonBinding, Gamepad, GamepadStick, InputEvent};
use std::ffi::{CStr, c_char};

/// Owned binding configuration. Install a copy with `crabby_ctx_set_actions`.
/// ```
/// let map = ponko_ffi::crabby_action_map_new();
/// unsafe { ponko_ffi::crabby_action_map_free(map); }
/// ```
pub struct CrabbyActionMap(ActionMap);

/// Source kinds for `CrabbyActionBinding::kind`.
/// ```
/// assert_eq!(ponko_ffi::CrabbyActionSource::CRABBY_ACTION_KEY as u32, 0);
/// ```
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum CrabbyActionSource {
    /// Digital key (`first` = CrabbyKey).
    CRABBY_ACTION_KEY = 0,
    /// Digital mouse button (`first` = CrabbyMouseButton).
    CRABBY_ACTION_MOUSE = 1,
    /// Digital gamepad button (`first` = CrabbyGamepadButton).
    CRABBY_ACTION_GAMEPAD_BUTTON = 2,
    /// Scalar key pair (`first` negative, `second` positive).
    CRABBY_ACTION_KEYS = 3,
    /// Scalar gamepad axis (`first` = CrabbyGamepadAxis).
    CRABBY_ACTION_GAMEPAD_AXIS = 4,
    /// Vector keys (`first..fourth` = left, right, up, down).
    CRABBY_ACTION_KEYS2 = 5,
    /// Vector gamepad stick (`first` = 0 left, 1 right).
    CRABBY_ACTION_GAMEPAD_STICK = 6,
}

/// One source: initialize unused fields to zero. `pad` is a controller slot.
/// ```
/// let binding = ponko_ffi::CrabbyActionBinding { kind: 0, pad: 0, first: 0, second: 0, third: 0, fourth: 0, deadzone: 0.0 };
/// assert_eq!(binding.kind, 0);
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CrabbyActionBinding {
    /// CrabbyActionSource.
    pub kind: u32,
    /// Gamepad slot, for gamepad sources.
    pub pad: u32,
    /// First source parameter (see CrabbyActionSource).
    pub first: u32,
    /// Second source parameter.
    pub second: u32,
    /// Third source parameter.
    pub third: u32,
    /// Fourth source parameter.
    pub fourth: u32,
    /// Gamepad deadzone in `0..1`.
    pub deadzone: f32,
}

unsafe fn text<'a>(ptr: *const c_char) -> Result<&'a str, Failure> {
    if ptr.is_null() {
        return Err(invalid("string is null"));
    }
    // SAFETY: caller supplies a NUL-terminated string, alive for the call.
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|_| invalid("string is not UTF-8"))
}
unsafe fn map_mut<'a>(map: *mut CrabbyActionMap) -> Result<&'a mut ActionMap, Failure> {
    // SAFETY: caller supplies a live, exclusively accessed handle.
    unsafe { map.as_mut() }
        .map(|m| &mut m.0)
        .ok_or_else(|| invalid("action map is null"))
}
fn pad(slot: u32) -> Result<Gamepad, Failure> {
    Gamepad::from_slot(slot as usize).ok_or_else(|| invalid("gamepad slot out of range"))
}
fn key(value: u32) -> Result<ponko::Key, Failure> {
    key_from_c(value).ok_or_else(|| invalid("unknown key"))
}
fn mouse(value: u32) -> Result<ponko::MouseButton, Failure> {
    button_from_c(value).ok_or_else(|| invalid("unknown mouse button"))
}
fn button(value: u32) -> Result<ponko::GamepadButton, Failure> {
    gamepad_button_from_c(value).ok_or_else(|| invalid("unknown gamepad button"))
}
fn axis(value: u32) -> Result<ponko::GamepadAxis, Failure> {
    gamepad_axis_from_c(value).ok_or_else(|| invalid("unknown gamepad axis"))
}

/// Create an empty map. Free once with `crabby_action_map_free`.
/// ```
/// let m = ponko_ffi::crabby_action_map_new();
/// unsafe { ponko_ffi::crabby_action_map_free(m); }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn crabby_action_map_new() -> *mut CrabbyActionMap {
    let mut result = std::ptr::null_mut();
    boundary(|| {
        result = Box::into_raw(Box::new(CrabbyActionMap(ActionMap::default())));
        Ok(())
    });
    result
}

/// Free a map; null is ignored.
/// ```
/// unsafe { ponko_ffi::crabby_action_map_free(std::ptr::null_mut()); }
/// ```
/// # Safety
/// Handle must be null or owned, live, and not used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_action_map_free(map: *mut CrabbyActionMap) {
    if !map.is_null() {
        // SAFETY: caller transfers the owned handle exactly once.
        drop(unsafe { Box::from_raw(map) });
    }
}

/// Replace a named action. `kind`: 0 button, 1 scalar, 2 vector.
/// Sources must match the action kind. Invalid configuration leaves the map unchanged.
/// ```
/// assert_ne!(unsafe { ponko_ffi::crabby_action_map_set(std::ptr::null_mut(), c"jump".as_ptr(), 0, std::ptr::null(), 0) } as u32, 0);
/// ```
/// # Safety
/// Handle must be live or null, name NUL-terminated, bindings readable for count
/// elements (or null with count 0); pointers must not alias the handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_action_map_set(
    map: *mut CrabbyActionMap,
    name: *const c_char,
    kind: u32,
    bindings: *const CrabbyActionBinding,
    count: usize,
) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: caller supplies live non-overlapping handles and strings.
        let (map, name) = unsafe { (map_mut(map)?, text(name)?) };
        if bindings.is_null() && count != 0 {
            return Err(invalid("bindings is null"));
        }
        let bindings = if count == 0 {
            &[]
        } else {
            // SAFETY: caller supplies count readable descriptions.
            unsafe { std::slice::from_raw_parts(bindings, count) }
        };
        let mut buttons = Vec::new();
        let mut axes = Vec::new();
        let mut vectors = Vec::new();
        for b in bindings {
            match (kind, b.kind) {
                (0, 0) => buttons.push(ButtonBinding::Key(key(b.first)?)),
                (0, 1) => buttons.push(ButtonBinding::Mouse(mouse(b.first)?)),
                (0, 2) => buttons.push(ButtonBinding::Gamepad {
                    pad: pad(b.pad)?,
                    button: button(b.first)?,
                }),
                (1, 3) => axes.push(AxisBinding::Keys {
                    negative: key(b.first)?,
                    positive: key(b.second)?,
                }),
                (1, 4) => axes.push(AxisBinding::Gamepad {
                    pad: pad(b.pad)?,
                    axis: axis(b.first)?,
                    deadzone: b.deadzone,
                }),
                (2, 5) => vectors.push(Axis2Binding::Keys {
                    left: key(b.first)?,
                    right: key(b.second)?,
                    up: key(b.third)?,
                    down: key(b.fourth)?,
                }),
                (2, 6) => vectors.push(Axis2Binding::Gamepad {
                    pad: pad(b.pad)?,
                    stick: match b.first {
                        0 => GamepadStick::Left,
                        1 => GamepadStick::Right,
                        _ => return Err(invalid("unknown stick")),
                    },
                    deadzone: b.deadzone,
                }),
                _ => return Err(invalid("source does not match action kind")),
            }
        }
        let action = match kind {
            0 => Action::Button(buttons),
            1 => Action::Axis(axes),
            2 => Action::Axis2(vectors),
            _ => return Err(invalid("unknown action kind")),
        };
        map.set(name, action);
        Ok(())
    })
}

/// Remove a named action.
/// ```
/// assert_ne!(unsafe { ponko_ffi::crabby_action_map_remove(std::ptr::null_mut(), c"jump".as_ptr()) } as u32, 0);
/// ```
/// # Safety
/// Handle must be live or null, name readable NUL-terminated and non-overlapping.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_action_map_remove(map: *mut CrabbyActionMap, name: *const c_char) -> CrabbyStatus {
    // SAFETY: requirements are forwarded to the helpers.
    boundary(|| unsafe {
        map_mut(map)?.remove(text(name)?);
        Ok(())
    })
}

/// Load Rust-compatible JSON. Errors leave the map unchanged.
/// ```
/// assert_ne!(unsafe { ponko_ffi::crabby_action_map_load(std::ptr::null_mut(), c"{}".as_ptr()) } as u32, 0);
/// ```
/// # Safety
/// Handle must be live or null, JSON readable NUL-terminated and non-overlapping.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_action_map_load(map: *mut CrabbyActionMap, json: *const c_char) -> CrabbyStatus {
    boundary(|| {
        // SAFETY: caller supplies live, non-overlapping handle and string.
        let (map, json) = unsafe { (map_mut(map)?, text(json)?) };
        *map = ActionMap::from_json(json).map_err(|e| invalid_owned(e.to_string()))?;
        Ok(())
    })
}

/// Save JSON. `out_len` excludes NUL. With capacity <= length, writes an empty
/// string (if capacity > 0); allocate length+1 and call again. Null buffer is
/// accepted only with capacity 0.
/// ```
/// assert_ne!(unsafe { ponko_ffi::crabby_action_map_save(std::ptr::null_mut(), std::ptr::null_mut(), 0, &mut 0) } as u32, 0);
/// ```
/// # Safety
/// Handle must be live or null; buffer writable for capacity bytes and out_len
/// writable, with no overlapping pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_action_map_save(
    map: *mut CrabbyActionMap,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    boundary(|| {
        if out_len.is_null() || (buffer.is_null() && capacity != 0) {
            return Err(invalid("output is null"));
        }
        // SAFETY: caller supplies live handle.
        let json = unsafe { map_mut(map)? }
            .to_json()
            .map_err(|e| invalid_owned(e.to_string()))?;
        // SAFETY: validated non-null outputs; caller supplies stated capacity.
        unsafe {
            out_len.write(json.len());
            if json.len() < capacity {
                std::ptr::copy_nonoverlapping(json.as_ptr(), buffer.cast(), json.len());
                buffer.add(json.len()).write(0);
            } else if capacity != 0 {
                buffer.write(0);
            }
        }
        Ok(())
    })
}

/// Install a copy of the bindings; rebinding generates no edges. Map remains owned by caller.
/// ```
/// assert_ne!(unsafe { ponko_ffi::crabby_ctx_set_actions(std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 0);
/// ```
/// # Safety
/// Context must be live or null, map live, exclusively accessed and non-overlapping.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_actions(ctx: *mut CrabbyCtx, map: *mut CrabbyActionMap) -> CrabbyStatus {
    // SAFETY: caller supplies callback context and independent map.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.input.set_actions(map_mut(map)?.clone());
            Ok(())
        })
    })
}

/// Copy current bindings into a map.
/// ```
/// assert_ne!(unsafe { ponko_ffi::crabby_ctx_actions(std::ptr::null_mut(), std::ptr::null_mut()) } as u32, 0);
/// ```
/// # Safety
/// Context and destination must be live or null, exclusively accessed, non-overlapping.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_actions(ctx: *mut CrabbyCtx, map: *mut CrabbyActionMap) -> CrabbyStatus {
    // SAFETY: caller supplies callback context and independent map.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            *map_mut(map)? = c.input.actions().clone();
            Ok(())
        })
    })
}

/// Feed manual input: kind 0 key, 1 mouse, 2 gamepad button, 3 gamepad axis,
/// 4 focus lost, 5 gamepad disconnected. Digital value is 0 released, nonzero held.
/// ```
/// assert_ne!(unsafe { ponko_ffi::crabby_ctx_feed_input(std::ptr::null_mut(), 0, 0, 0, 1.0) } as u32, 0);
/// ```
/// # Safety
/// Context must be live in its callback, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_feed_input(
    ctx: *mut CrabbyCtx,
    kind: u32,
    slot: u32,
    input: u32,
    value: f32,
) -> CrabbyStatus {
    // SAFETY: caller supplies callback context.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            c.input.feed(match kind {
                0 => InputEvent::Key(key(input)?, value != 0.0),
                1 => InputEvent::Mouse(mouse(input)?, value != 0.0),
                2 => InputEvent::GamepadButton(pad(slot)?, button(input)?, value != 0.0),
                3 => InputEvent::GamepadAxis(pad(slot)?, axis(input)?, value),
                4 => InputEvent::FocusLost,
                5 => InputEvent::GamepadDisconnected(pad(slot)?),
                _ => return Err(invalid("unknown input event kind")),
            });
            Ok(())
        })
    })
}

/// Query a named action's down. Unknown/wrong-kind actions read zero.
/// ```
/// assert_ne!(unsafe { ponko_ffi::crabby_ctx_action_down(std::ptr::null_mut(), c"move".as_ptr(), &mut false) } as u32, 0);
/// ```
/// # Safety
/// Context must be live or null, name NUL-terminated, output writable and non-overlapping.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_action_down(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    value: *mut bool,
) -> CrabbyStatus {
    // SAFETY: caller supplies callback context, string and independent output.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let name = text(name)?;
            out(value, c.input.action_down(name))
        })
    })
}

/// Query a named action's pressed. Unknown/wrong-kind actions read zero.
/// ```
/// assert_ne!(unsafe { ponko_ffi::crabby_ctx_action_pressed(std::ptr::null_mut(), c"move".as_ptr(), &mut false) } as u32, 0);
/// ```
/// # Safety
/// Context must be live or null, name NUL-terminated, output writable and non-overlapping.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_action_pressed(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    value: *mut bool,
) -> CrabbyStatus {
    // SAFETY: caller supplies callback context, string and independent output.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let name = text(name)?;
            out(value, c.input.action_pressed(name))
        })
    })
}

/// Query a named action's released. Unknown/wrong-kind actions read zero.
/// ```
/// assert_ne!(unsafe { ponko_ffi::crabby_ctx_action_released(std::ptr::null_mut(), c"move".as_ptr(), &mut false) } as u32, 0);
/// ```
/// # Safety
/// Context must be live or null, name NUL-terminated, output writable and non-overlapping.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_action_released(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    value: *mut bool,
) -> CrabbyStatus {
    // SAFETY: caller supplies callback context, string and independent output.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let name = text(name)?;
            out(value, c.input.action_released(name))
        })
    })
}

/// Query a named action's axis. Unknown/wrong-kind actions read zero.
/// ```
/// assert_ne!(unsafe { ponko_ffi::crabby_ctx_action_axis(std::ptr::null_mut(), c"move".as_ptr(), &mut 0.0) } as u32, 0);
/// ```
/// # Safety
/// Context must be live or null, name NUL-terminated, output writable and non-overlapping.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_action_axis(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    value: *mut f32,
) -> CrabbyStatus {
    // SAFETY: caller supplies callback context, string and independent output.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let name = text(name)?;
            out(value, c.input.action_axis(name))
        })
    })
}

/// Query a named action's axis2. Unknown/wrong-kind actions read zero.
/// ```
/// assert_ne!(unsafe { ponko_ffi::crabby_ctx_action_axis2(std::ptr::null_mut(), c"move".as_ptr(), &mut ponko_ffi::CrabbyVec2 { x: 0.0, y: 0.0 }) } as u32, 0);
/// ```
/// # Safety
/// Context must be live or null, name NUL-terminated, output writable and non-overlapping.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_action_axis2(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    value: *mut CrabbyVec2,
) -> CrabbyStatus {
    // SAFETY: caller supplies callback context, string and independent output.
    boundary(|| unsafe {
        ctx_access(ctx, |c| {
            let name = text(name)?;
            out(value, to_c(c.input.action_axis2(name)))
        })
    })
}
