//! Keyboard keys and mouse buttons as C enums, in the same order as the Rust ones.
use crabbyengine::{Key, MouseButton};

/// A keyboard key, by position on a US layout (`CRABBY_KEY_A`, ...).
/// ```
/// assert_eq!(crabby_ffi::CrabbyKey::CRABBY_KEY_A as u32, 0);
/// ```
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyKey {
    /// `A`.
    CRABBY_KEY_A,
    /// `B`.
    CRABBY_KEY_B,
    /// `C`.
    CRABBY_KEY_C,
    /// `D`.
    CRABBY_KEY_D,
    /// `E`.
    CRABBY_KEY_E,
    /// `F`.
    CRABBY_KEY_F,
    /// `G`.
    CRABBY_KEY_G,
    /// `H`.
    CRABBY_KEY_H,
    /// `I`.
    CRABBY_KEY_I,
    /// `J`.
    CRABBY_KEY_J,
    /// `K`.
    CRABBY_KEY_K,
    /// `L`.
    CRABBY_KEY_L,
    /// `M`.
    CRABBY_KEY_M,
    /// `N`.
    CRABBY_KEY_N,
    /// `O`.
    CRABBY_KEY_O,
    /// `P`.
    CRABBY_KEY_P,
    /// `Q`.
    CRABBY_KEY_Q,
    /// `R`.
    CRABBY_KEY_R,
    /// `S`.
    CRABBY_KEY_S,
    /// `T`.
    CRABBY_KEY_T,
    /// `U`.
    CRABBY_KEY_U,
    /// `V`.
    CRABBY_KEY_V,
    /// `W`.
    CRABBY_KEY_W,
    /// `X`.
    CRABBY_KEY_X,
    /// `Y`.
    CRABBY_KEY_Y,
    /// `Z`.
    CRABBY_KEY_Z,
    /// `Num0`.
    CRABBY_KEY_NUM_0,
    /// `Num1`.
    CRABBY_KEY_NUM_1,
    /// `Num2`.
    CRABBY_KEY_NUM_2,
    /// `Num3`.
    CRABBY_KEY_NUM_3,
    /// `Num4`.
    CRABBY_KEY_NUM_4,
    /// `Num5`.
    CRABBY_KEY_NUM_5,
    /// `Num6`.
    CRABBY_KEY_NUM_6,
    /// `Num7`.
    CRABBY_KEY_NUM_7,
    /// `Num8`.
    CRABBY_KEY_NUM_8,
    /// `Num9`.
    CRABBY_KEY_NUM_9,
    /// `F1`.
    CRABBY_KEY_F1,
    /// `F2`.
    CRABBY_KEY_F2,
    /// `F3`.
    CRABBY_KEY_F3,
    /// `F4`.
    CRABBY_KEY_F4,
    /// `F5`.
    CRABBY_KEY_F5,
    /// `F6`.
    CRABBY_KEY_F6,
    /// `F7`.
    CRABBY_KEY_F7,
    /// `F8`.
    CRABBY_KEY_F8,
    /// `F9`.
    CRABBY_KEY_F9,
    /// `F10`.
    CRABBY_KEY_F10,
    /// `F11`.
    CRABBY_KEY_F11,
    /// `F12`.
    CRABBY_KEY_F12,
    /// `Up`.
    CRABBY_KEY_UP,
    /// `Down`.
    CRABBY_KEY_DOWN,
    /// `Left`.
    CRABBY_KEY_LEFT,
    /// `Right`.
    CRABBY_KEY_RIGHT,
    /// `Space`.
    CRABBY_KEY_SPACE,
    /// `Enter`.
    CRABBY_KEY_ENTER,
    /// `Escape`.
    CRABBY_KEY_ESCAPE,
    /// `Tab`.
    CRABBY_KEY_TAB,
    /// `Backspace`.
    CRABBY_KEY_BACKSPACE,
    /// `CapsLock`.
    CRABBY_KEY_CAPS_LOCK,
    /// `Insert`.
    CRABBY_KEY_INSERT,
    /// `Delete`.
    CRABBY_KEY_DELETE,
    /// `Home`.
    CRABBY_KEY_HOME,
    /// `End`.
    CRABBY_KEY_END,
    /// `PageUp`.
    CRABBY_KEY_PAGE_UP,
    /// `PageDown`.
    CRABBY_KEY_PAGE_DOWN,
    /// `LeftShift`.
    CRABBY_KEY_LEFT_SHIFT,
    /// `RightShift`.
    CRABBY_KEY_RIGHT_SHIFT,
    /// `LeftCtrl`.
    CRABBY_KEY_LEFT_CTRL,
    /// `RightCtrl`.
    CRABBY_KEY_RIGHT_CTRL,
    /// `LeftAlt`.
    CRABBY_KEY_LEFT_ALT,
    /// `RightAlt`.
    CRABBY_KEY_RIGHT_ALT,
    /// `LeftSuper`.
    CRABBY_KEY_LEFT_SUPER,
    /// `RightSuper`.
    CRABBY_KEY_RIGHT_SUPER,
    /// `Minus`.
    CRABBY_KEY_MINUS,
    /// `Equal`.
    CRABBY_KEY_EQUAL,
    /// `LeftBracket`.
    CRABBY_KEY_LEFT_BRACKET,
    /// `RightBracket`.
    CRABBY_KEY_RIGHT_BRACKET,
    /// `Backslash`.
    CRABBY_KEY_BACKSLASH,
    /// `Semicolon`.
    CRABBY_KEY_SEMICOLON,
    /// `Apostrophe`.
    CRABBY_KEY_APOSTROPHE,
    /// `Backquote`.
    CRABBY_KEY_BACKQUOTE,
    /// `Comma`.
    CRABBY_KEY_COMMA,
    /// `Period`.
    CRABBY_KEY_PERIOD,
    /// `Slash`.
    CRABBY_KEY_SLASH,
    /// `Numpad0`.
    CRABBY_KEY_NUMPAD_0,
    /// `Numpad1`.
    CRABBY_KEY_NUMPAD_1,
    /// `Numpad2`.
    CRABBY_KEY_NUMPAD_2,
    /// `Numpad3`.
    CRABBY_KEY_NUMPAD_3,
    /// `Numpad4`.
    CRABBY_KEY_NUMPAD_4,
    /// `Numpad5`.
    CRABBY_KEY_NUMPAD_5,
    /// `Numpad6`.
    CRABBY_KEY_NUMPAD_6,
    /// `Numpad7`.
    CRABBY_KEY_NUMPAD_7,
    /// `Numpad8`.
    CRABBY_KEY_NUMPAD_8,
    /// `Numpad9`.
    CRABBY_KEY_NUMPAD_9,
    /// `NumpadAdd`.
    CRABBY_KEY_NUMPAD_ADD,
    /// `NumpadSubtract`.
    CRABBY_KEY_NUMPAD_SUBTRACT,
    /// `NumpadMultiply`.
    CRABBY_KEY_NUMPAD_MULTIPLY,
    /// `NumpadDivide`.
    CRABBY_KEY_NUMPAD_DIVIDE,
    /// `NumpadDecimal`.
    CRABBY_KEY_NUMPAD_DECIMAL,
    /// `NumpadEnter`.
    CRABBY_KEY_NUMPAD_ENTER,
}

pub(crate) fn key(k: CrabbyKey) -> Key {
    use CrabbyKey::*;
    match k {
        CRABBY_KEY_A => Key::A,
        CRABBY_KEY_B => Key::B,
        CRABBY_KEY_C => Key::C,
        CRABBY_KEY_D => Key::D,
        CRABBY_KEY_E => Key::E,
        CRABBY_KEY_F => Key::F,
        CRABBY_KEY_G => Key::G,
        CRABBY_KEY_H => Key::H,
        CRABBY_KEY_I => Key::I,
        CRABBY_KEY_J => Key::J,
        CRABBY_KEY_K => Key::K,
        CRABBY_KEY_L => Key::L,
        CRABBY_KEY_M => Key::M,
        CRABBY_KEY_N => Key::N,
        CRABBY_KEY_O => Key::O,
        CRABBY_KEY_P => Key::P,
        CRABBY_KEY_Q => Key::Q,
        CRABBY_KEY_R => Key::R,
        CRABBY_KEY_S => Key::S,
        CRABBY_KEY_T => Key::T,
        CRABBY_KEY_U => Key::U,
        CRABBY_KEY_V => Key::V,
        CRABBY_KEY_W => Key::W,
        CRABBY_KEY_X => Key::X,
        CRABBY_KEY_Y => Key::Y,
        CRABBY_KEY_Z => Key::Z,
        CRABBY_KEY_NUM_0 => Key::Num0,
        CRABBY_KEY_NUM_1 => Key::Num1,
        CRABBY_KEY_NUM_2 => Key::Num2,
        CRABBY_KEY_NUM_3 => Key::Num3,
        CRABBY_KEY_NUM_4 => Key::Num4,
        CRABBY_KEY_NUM_5 => Key::Num5,
        CRABBY_KEY_NUM_6 => Key::Num6,
        CRABBY_KEY_NUM_7 => Key::Num7,
        CRABBY_KEY_NUM_8 => Key::Num8,
        CRABBY_KEY_NUM_9 => Key::Num9,
        CRABBY_KEY_F1 => Key::F1,
        CRABBY_KEY_F2 => Key::F2,
        CRABBY_KEY_F3 => Key::F3,
        CRABBY_KEY_F4 => Key::F4,
        CRABBY_KEY_F5 => Key::F5,
        CRABBY_KEY_F6 => Key::F6,
        CRABBY_KEY_F7 => Key::F7,
        CRABBY_KEY_F8 => Key::F8,
        CRABBY_KEY_F9 => Key::F9,
        CRABBY_KEY_F10 => Key::F10,
        CRABBY_KEY_F11 => Key::F11,
        CRABBY_KEY_F12 => Key::F12,
        CRABBY_KEY_UP => Key::Up,
        CRABBY_KEY_DOWN => Key::Down,
        CRABBY_KEY_LEFT => Key::Left,
        CRABBY_KEY_RIGHT => Key::Right,
        CRABBY_KEY_SPACE => Key::Space,
        CRABBY_KEY_ENTER => Key::Enter,
        CRABBY_KEY_ESCAPE => Key::Escape,
        CRABBY_KEY_TAB => Key::Tab,
        CRABBY_KEY_BACKSPACE => Key::Backspace,
        CRABBY_KEY_CAPS_LOCK => Key::CapsLock,
        CRABBY_KEY_INSERT => Key::Insert,
        CRABBY_KEY_DELETE => Key::Delete,
        CRABBY_KEY_HOME => Key::Home,
        CRABBY_KEY_END => Key::End,
        CRABBY_KEY_PAGE_UP => Key::PageUp,
        CRABBY_KEY_PAGE_DOWN => Key::PageDown,
        CRABBY_KEY_LEFT_SHIFT => Key::LeftShift,
        CRABBY_KEY_RIGHT_SHIFT => Key::RightShift,
        CRABBY_KEY_LEFT_CTRL => Key::LeftCtrl,
        CRABBY_KEY_RIGHT_CTRL => Key::RightCtrl,
        CRABBY_KEY_LEFT_ALT => Key::LeftAlt,
        CRABBY_KEY_RIGHT_ALT => Key::RightAlt,
        CRABBY_KEY_LEFT_SUPER => Key::LeftSuper,
        CRABBY_KEY_RIGHT_SUPER => Key::RightSuper,
        CRABBY_KEY_MINUS => Key::Minus,
        CRABBY_KEY_EQUAL => Key::Equal,
        CRABBY_KEY_LEFT_BRACKET => Key::LeftBracket,
        CRABBY_KEY_RIGHT_BRACKET => Key::RightBracket,
        CRABBY_KEY_BACKSLASH => Key::Backslash,
        CRABBY_KEY_SEMICOLON => Key::Semicolon,
        CRABBY_KEY_APOSTROPHE => Key::Apostrophe,
        CRABBY_KEY_BACKQUOTE => Key::Backquote,
        CRABBY_KEY_COMMA => Key::Comma,
        CRABBY_KEY_PERIOD => Key::Period,
        CRABBY_KEY_SLASH => Key::Slash,
        CRABBY_KEY_NUMPAD_0 => Key::Numpad0,
        CRABBY_KEY_NUMPAD_1 => Key::Numpad1,
        CRABBY_KEY_NUMPAD_2 => Key::Numpad2,
        CRABBY_KEY_NUMPAD_3 => Key::Numpad3,
        CRABBY_KEY_NUMPAD_4 => Key::Numpad4,
        CRABBY_KEY_NUMPAD_5 => Key::Numpad5,
        CRABBY_KEY_NUMPAD_6 => Key::Numpad6,
        CRABBY_KEY_NUMPAD_7 => Key::Numpad7,
        CRABBY_KEY_NUMPAD_8 => Key::Numpad8,
        CRABBY_KEY_NUMPAD_9 => Key::Numpad9,
        CRABBY_KEY_NUMPAD_ADD => Key::NumpadAdd,
        CRABBY_KEY_NUMPAD_SUBTRACT => Key::NumpadSubtract,
        CRABBY_KEY_NUMPAD_MULTIPLY => Key::NumpadMultiply,
        CRABBY_KEY_NUMPAD_DIVIDE => Key::NumpadDivide,
        CRABBY_KEY_NUMPAD_DECIMAL => Key::NumpadDecimal,
        CRABBY_KEY_NUMPAD_ENTER => Key::NumpadEnter,
    }
}

/// A mouse button.
/// ```
/// assert_eq!(crabby_ffi::CrabbyMouseButton::CRABBY_MOUSE_LEFT as u32, 0);
/// ```
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyMouseButton {
    /// Left button.
    CRABBY_MOUSE_LEFT,
    /// Right button.
    CRABBY_MOUSE_RIGHT,
    /// Middle button (wheel click).
    CRABBY_MOUSE_MIDDLE,
    /// Side "back" button.
    CRABBY_MOUSE_BACK,
    /// Side "forward" button.
    CRABBY_MOUSE_FORWARD,
}

pub(crate) fn button(b: CrabbyMouseButton) -> MouseButton {
    use CrabbyMouseButton::*;
    match b {
        CRABBY_MOUSE_LEFT => MouseButton::Left,
        CRABBY_MOUSE_RIGHT => MouseButton::Right,
        CRABBY_MOUSE_MIDDLE => MouseButton::Middle,
        CRABBY_MOUSE_BACK => MouseButton::Back,
        CRABBY_MOUSE_FORWARD => MouseButton::Forward,
    }
}

const KEY_COUNT: u32 = CrabbyKey::CRABBY_KEY_NUMPAD_ENTER as u32 + 1;
const _: () = assert!(size_of::<CrabbyKey>() == 4 && size_of::<CrabbyMouseButton>() == 4);

/// C passes enums as plain ints: check the range before turning one into a Rust enum.
pub(crate) fn key_from_c(k: u32) -> Option<Key> {
    // SAFETY: CrabbyKey is a 4-byte fieldless enum with discriminants 0..KEY_COUNT.
    (k < KEY_COUNT).then(|| key(unsafe { std::mem::transmute::<u32, CrabbyKey>(k) }))
}
pub(crate) fn button_from_c(b: u32) -> Option<MouseButton> {
    // SAFETY: as above, with 5 buttons.
    (b <= CrabbyMouseButton::CRABBY_MOUSE_FORWARD as u32)
        .then(|| button(unsafe { std::mem::transmute::<u32, CrabbyMouseButton>(b) }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_c_key_maps_to_a_distinct_rust_key() {
        let keys: std::collections::HashSet<_> = (0..KEY_COUNT).map(|k| key_from_c(k).unwrap()).collect();
        assert_eq!(keys.len(), KEY_COUNT as usize);
        assert_eq!(key_from_c(KEY_COUNT), None);
        assert_eq!(button_from_c(5), None);
        assert_eq!(button_from_c(1), Some(MouseButton::Right));
    }
}
