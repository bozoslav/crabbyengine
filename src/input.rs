//! Keyboard and mouse state.
//!
//! The engine records raw window events here; game code reads the result
//! through [`Ctx`](crate::Ctx). State lives in fixed-size arrays indexed by enum
//! discriminant, so lookups are O(1) and nothing allocates per frame.

use glam::Vec2;
#[cfg(feature = "window")]
use winit::keyboard::KeyCode;

macro_rules! define_keys {
    ($($(#[$meta:meta])* $name:ident => $code:ident),* $(,)?) => {
        /// A keyboard key, identified by its **physical position** on a US layout.
        ///
        /// Physical keys keep controls in the same place on every keyboard layout:
        /// `Key::W` is the key above `S` on QWERTY, AZERTY and Dvorak alike.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum Key {
            $($(#[$meta])* $name),*
        }

        impl Key {
            pub(crate) const COUNT: usize = [$(Key::$name),*].len();

            #[cfg(feature = "window")]
            pub(crate) fn from_winit(code: KeyCode) -> Option<Key> {
                match code {
                    $(KeyCode::$code => Some(Key::$name),)*
                    _ => None,
                }
            }
        }
    };
}

define_keys! {
    A => KeyA, B => KeyB, C => KeyC, D => KeyD, E => KeyE, F => KeyF, G => KeyG,
    H => KeyH, I => KeyI, J => KeyJ, K => KeyK, L => KeyL, M => KeyM, N => KeyN,
    O => KeyO, P => KeyP, Q => KeyQ, R => KeyR, S => KeyS, T => KeyT, U => KeyU,
    V => KeyV, W => KeyW, X => KeyX, Y => KeyY, Z => KeyZ,

    /// The `0` key in the number row (not the numpad).
    Num0 => Digit0, Num1 => Digit1, Num2 => Digit2, Num3 => Digit3, Num4 => Digit4,
    Num5 => Digit5, Num6 => Digit6, Num7 => Digit7, Num8 => Digit8, Num9 => Digit9,

    F1 => F1, F2 => F2, F3 => F3, F4 => F4, F5 => F5, F6 => F6,
    F7 => F7, F8 => F8, F9 => F9, F10 => F10, F11 => F11, F12 => F12,

    Up => ArrowUp, Down => ArrowDown, Left => ArrowLeft, Right => ArrowRight,

    Space => Space, Enter => Enter, Escape => Escape, Tab => Tab,
    Backspace => Backspace, CapsLock => CapsLock,
    Insert => Insert, Delete => Delete, Home => Home, End => End,
    PageUp => PageUp, PageDown => PageDown,

    LeftShift => ShiftLeft, RightShift => ShiftRight,
    LeftCtrl => ControlLeft, RightCtrl => ControlRight,
    LeftAlt => AltLeft, RightAlt => AltRight,
    /// Cmd on macOS, the Windows key on Windows.
    LeftSuper => SuperLeft,
    /// Cmd on macOS, the Windows key on Windows.
    RightSuper => SuperRight,

    Minus => Minus, Equal => Equal,
    LeftBracket => BracketLeft, RightBracket => BracketRight, Backslash => Backslash,
    Semicolon => Semicolon, Apostrophe => Quote, Backquote => Backquote,
    Comma => Comma, Period => Period, Slash => Slash,

    Numpad0 => Numpad0, Numpad1 => Numpad1, Numpad2 => Numpad2, Numpad3 => Numpad3,
    Numpad4 => Numpad4, Numpad5 => Numpad5, Numpad6 => Numpad6, Numpad7 => Numpad7,
    Numpad8 => Numpad8, Numpad9 => Numpad9,
    NumpadAdd => NumpadAdd, NumpadSubtract => NumpadSubtract,
    NumpadMultiply => NumpadMultiply, NumpadDivide => NumpadDivide,
    NumpadDecimal => NumpadDecimal, NumpadEnter => NumpadEnter,
}

/// A mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    /// The "back" side button found on many mice.
    Back,
    /// The "forward" side button found on many mice.
    Forward,
}

impl MouseButton {
    pub(crate) const COUNT: usize = 5;

    #[cfg(feature = "window")]
    pub(crate) fn from_winit(button: winit::event::MouseButton) -> Option<MouseButton> {
        use winit::event::MouseButton as W;
        match button {
            W::Left => Some(MouseButton::Left),
            W::Right => Some(MouseButton::Right),
            W::Middle => Some(MouseButton::Middle),
            W::Back => Some(MouseButton::Back),
            W::Forward => Some(MouseButton::Forward),
            W::Other(_) => None,
        }
    }
}

/// Down / pressed-this-frame / released-this-frame flags for `N` buttons.
#[derive(Debug, Clone)]
struct ButtonStates<const N: usize> {
    down: [bool; N],
    pressed: [bool; N],
    released: [bool; N],
}

impl<const N: usize> ButtonStates<N> {
    const fn new() -> Self {
        Self {
            down: [false; N],
            pressed: [false; N],
            released: [false; N],
        }
    }

    fn set(&mut self, i: usize, is_down: bool) {
        match (self.down[i], is_down) {
            (false, true) => self.pressed[i] = true,
            (true, false) => self.released[i] = true,
            _ => {}
        }
        self.down[i] = is_down;
    }

    fn release_all(&mut self) {
        for i in 0..N {
            self.set(i, false);
        }
    }

    fn end_frame(&mut self) {
        self.pressed = [false; N];
        self.released = [false; N];
    }
}

/// Everything the engine knows about keyboard and mouse this frame.
#[derive(Debug, Clone)]
pub(crate) struct Input {
    keys: ButtonStates<{ Key::COUNT }>,
    mouse: ButtonStates<{ MouseButton::COUNT }>,
    mouse_pos: Vec2,
    mouse_pos_at_frame_start: Vec2,
    mouse_wheel: Vec2,
}

impl Input {
    pub fn new() -> Self {
        Self {
            keys: ButtonStates::new(),
            mouse: ButtonStates::new(),
            mouse_pos: Vec2::ZERO,
            mouse_pos_at_frame_start: Vec2::ZERO,
            mouse_wheel: Vec2::ZERO,
        }
    }

    // --- event recording (engine side) ---

    pub fn set_key(&mut self, key: Key, is_down: bool) {
        self.keys.set(key as usize, is_down);
    }

    pub fn set_mouse_button(&mut self, button: MouseButton, is_down: bool) {
        self.mouse.set(button as usize, is_down);
    }

    pub fn set_mouse_pos(&mut self, pos: Vec2) {
        self.mouse_pos = pos;
    }

    pub fn add_mouse_wheel(&mut self, delta: Vec2) {
        self.mouse_wheel += delta;
    }

    /// Called when the window loses focus: we will never see the matching
    /// key-up events, so treat everything as released to avoid stuck keys.
    pub fn release_all(&mut self) {
        self.keys.release_all();
        self.mouse.release_all();
    }

    /// Clears per-frame state. Called after the game's update for the frame.
    pub fn end_frame(&mut self) {
        self.keys.end_frame();
        self.mouse.end_frame();
        self.mouse_wheel = Vec2::ZERO;
        self.mouse_pos_at_frame_start = self.mouse_pos;
    }

    // --- queries (exposed through Ctx) ---

    pub fn key_down(&self, key: Key) -> bool {
        self.keys.down[key as usize]
    }

    pub fn key_pressed(&self, key: Key) -> bool {
        self.keys.pressed[key as usize]
    }

    pub fn key_released(&self, key: Key) -> bool {
        self.keys.released[key as usize]
    }

    pub fn mouse_down(&self, button: MouseButton) -> bool {
        self.mouse.down[button as usize]
    }

    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse.pressed[button as usize]
    }

    pub fn mouse_released(&self, button: MouseButton) -> bool {
        self.mouse.released[button as usize]
    }

    pub fn mouse_pos(&self) -> Vec2 {
        self.mouse_pos
    }

    pub fn mouse_delta(&self) -> Vec2 {
        self.mouse_pos - self.mouse_pos_at_frame_start
    }

    pub fn mouse_wheel(&self) -> Vec2 {
        self.mouse_wheel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_hold_release_cycle() {
        let mut input = Input::new();

        input.set_key(Key::Space, true);
        assert!(input.key_down(Key::Space));
        assert!(input.key_pressed(Key::Space));
        assert!(!input.key_released(Key::Space));
        input.end_frame();

        // Held: still down, no longer "pressed".
        assert!(input.key_down(Key::Space));
        assert!(!input.key_pressed(Key::Space));
        input.end_frame();

        input.set_key(Key::Space, false);
        assert!(!input.key_down(Key::Space));
        assert!(input.key_released(Key::Space));
        input.end_frame();
        assert!(!input.key_released(Key::Space));
    }

    #[test]
    fn tap_within_one_frame_is_not_lost() {
        let mut input = Input::new();
        input.set_key(Key::A, true);
        input.set_key(Key::A, false);
        assert!(input.key_pressed(Key::A));
        assert!(input.key_released(Key::A));
        assert!(!input.key_down(Key::A));
    }

    #[test]
    fn repeated_down_events_do_not_retrigger_pressed() {
        let mut input = Input::new();
        input.set_key(Key::W, true);
        input.end_frame();
        input.set_key(Key::W, true);
        assert!(!input.key_pressed(Key::W));
    }

    #[test]
    fn release_all_clears_held_keys_and_buttons() {
        let mut input = Input::new();
        input.set_key(Key::Left, true);
        input.set_mouse_button(MouseButton::Right, true);
        input.end_frame();

        input.release_all();
        assert!(!input.key_down(Key::Left));
        assert!(input.key_released(Key::Left));
        assert!(!input.mouse_down(MouseButton::Right));
        assert!(input.mouse_released(MouseButton::Right));
        // Keys that were never down are not reported as released.
        assert!(!input.key_released(Key::Right));
    }

    #[test]
    fn mouse_delta_and_wheel_reset_each_frame() {
        let mut input = Input::new();
        input.set_mouse_pos(Vec2::new(10.0, 20.0));
        input.end_frame();

        input.set_mouse_pos(Vec2::new(15.0, 18.0));
        input.add_mouse_wheel(Vec2::new(0.0, 1.0));
        input.add_mouse_wheel(Vec2::new(0.0, 2.0));
        assert_eq!(input.mouse_delta(), Vec2::new(5.0, -2.0));
        assert_eq!(input.mouse_wheel(), Vec2::new(0.0, 3.0));

        input.end_frame();
        assert_eq!(input.mouse_delta(), Vec2::ZERO);
        assert_eq!(input.mouse_wheel(), Vec2::ZERO);
        assert_eq!(input.mouse_pos(), Vec2::new(15.0, 18.0));
    }

    #[test]
    #[cfg(feature = "window")]
    fn winit_mapping() {
        assert_eq!(Key::from_winit(KeyCode::KeyW), Some(Key::W));
        assert_eq!(Key::from_winit(KeyCode::ArrowLeft), Some(Key::Left));
        assert_eq!(Key::from_winit(KeyCode::Digit5), Some(Key::Num5));
        assert_eq!(Key::from_winit(KeyCode::MediaPlayPause), None);
        assert_eq!(
            MouseButton::from_winit(winit::event::MouseButton::Middle),
            Some(MouseButton::Middle)
        );
    }
}
