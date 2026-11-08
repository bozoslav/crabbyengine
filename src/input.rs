//! Keyboard, mouse and gamepad state.
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
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        #[non_exhaustive]
        #[allow(missing_docs)] // named after the keys
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum MouseButton {
    /// The left (primary) button.
    Left,
    /// The right (secondary) button.
    Right,
    /// The wheel, pressed.
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

/// A gamepad button, named by its position, so it means the same on every
/// brand of controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum GamepadButton {
    /// The bottom face button: A on Xbox, Cross on PlayStation, B on Nintendo.
    South,
    /// The right face button: B on Xbox, Circle on PlayStation, A on Nintendo.
    East,
    /// The top face button: Y on Xbox, Triangle on PlayStation, X on Nintendo.
    North,
    /// The left face button: X on Xbox, Square on PlayStation, Y on Nintendo.
    West,
    /// The upper shoulder button on the left (LB, L1, L).
    LeftBumper,
    /// The upper shoulder button on the right (RB, R1, R).
    RightBumper,
    /// The lower shoulder trigger on the left (LT, L2, ZL), pulled more than
    /// halfway. For how far, read [`GamepadAxis::LeftTrigger`].
    LeftTrigger,
    /// The lower shoulder trigger on the right (RT, R2, ZR), pulled more than
    /// halfway. For how far, read [`GamepadAxis::RightTrigger`].
    RightTrigger,
    /// The small button left of center (View, Share, Minus).
    Select,
    /// The small button right of center (Menu, Options, Plus).
    Start,
    /// Pressing the left stick in.
    LeftStick,
    /// Pressing the right stick in.
    RightStick,
    /// The d-pad, up.
    DPadUp,
    /// The d-pad, down.
    DPadDown,
    /// The d-pad, left.
    DPadLeft,
    /// The d-pad, right.
    DPadRight,
}

impl GamepadButton {
    pub(crate) const COUNT: usize = 16;
}

/// One axis of a gamepad: a stick's x or y (`-1.0..=1.0`, y down is
/// positive like screen coordinates) or a trigger (`0.0..=1.0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum GamepadAxis {
    /// Left stick, left (-1) to right (1).
    LeftStickX,
    /// Left stick, up (-1) to down (1).
    LeftStickY,
    /// Right stick, left (-1) to right (1).
    RightStickX,
    /// Right stick, up (-1) to down (1).
    RightStickY,
    /// Left trigger, released (0) to fully pulled (1).
    LeftTrigger,
    /// Right trigger, released (0) to fully pulled (1).
    RightTrigger,
}

impl GamepadAxis {
    pub(crate) const COUNT: usize = 6;
}

/// Which stick, for [`Ctx::gamepad_stick`](crate::Ctx::gamepad_stick).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum GamepadStick {
    /// The left stick.
    Left,
    /// The right stick.
    Right,
}

/// A connected gamepad, from [`Ctx::gamepads`](crate::Ctx::gamepads). It
/// keeps its slot while connected, so `pads[0]` stays player one; a pad that
/// reconnects takes the lowest free slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct Gamepad(pub(crate) u8);
impl TryFrom<u8> for Gamepad {
    type Error = &'static str;
    fn try_from(slot: u8) -> Result<Self, Self::Error> {
        Self::from_slot(usize::from(slot)).ok_or("gamepad slot out of range")
    }
}
impl From<Gamepad> for u8 {
    fn from(pad: Gamepad) -> u8 {
        pad.0
    }
}

impl Gamepad {
    /// The pad's slot, `0..MAX_GAMEPADS`: a stable number for "player 1, 2, ...".
    ///
    /// ```
    /// # use ponko::*;
    /// # fn f(ctx: &mut Ctx) {
    /// for pad in ctx.gamepads() {
    ///     println!("player {} is connected", pad.slot() + 1);
    /// }
    /// # }
    /// ```
    #[must_use]
    pub fn slot(self) -> usize {
        usize::from(self.0)
    }

    /// The pad in `slot`, whether or not one is connected there (queries
    /// on an empty slot read as nothing pressed). `None` past
    /// [`MAX_GAMEPADS`].
    ///
    /// ```
    /// # use ponko::*;
    /// let player_two = Gamepad::from_slot(1).unwrap();
    /// assert_eq!(player_two.slot(), 1);
    /// assert_eq!(Gamepad::from_slot(MAX_GAMEPADS), None);
    /// ```
    #[must_use]
    pub fn from_slot(slot: usize) -> Option<Gamepad> {
        (slot < MAX_GAMEPADS).then_some(Gamepad(slot as u8))
    }
}

/// How many gamepads the engine tracks at once.
pub const MAX_GAMEPADS: usize = 8;

/// One gamepad's buttons and axes.
#[derive(Debug, Clone)]
pub(super) struct PadState {
    pub(super) buttons: ButtonStates<{ GamepadButton::COUNT }>,
    pub(super) axes: [f32; GamepadAxis::COUNT],
}

impl PadState {
    const fn new() -> Self {
        Self {
            buttons: ButtonStates::new(),
            axes: [0.0; GamepadAxis::COUNT],
        }
    }
}

/// Down / pressed-this-frame / released-this-frame flags for `N` buttons.
#[derive(Debug, Clone)]
pub(super) struct ButtonStates<const N: usize> {
    pub(super) down: [bool; N],
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
    pub(super) keys: ButtonStates<{ Key::COUNT }>,
    pub(super) mouse: ButtonStates<{ MouseButton::COUNT }>,
    mouse_pos: Vec2,
    mouse_pos_at_frame_start: Vec2,
    mouse_wheel: Vec2,
    typed_text: String,
    /// Raw mouse movement this frame (also while the cursor is locked).
    mouse_motion: Vec2,
    pub(super) pads: [PadState; MAX_GAMEPADS],
    /// Connected pads, sorted by slot.
    connected: Vec<Gamepad>,
    pub(crate) actions: crate::ActionMap,
}

impl Input {
    pub fn new() -> Self {
        Self {
            keys: ButtonStates::new(),
            mouse: ButtonStates::new(),
            mouse_pos: Vec2::ZERO,
            mouse_pos_at_frame_start: Vec2::ZERO,
            mouse_wheel: Vec2::ZERO,
            typed_text: String::new(),
            mouse_motion: Vec2::ZERO,
            pads: [const { PadState::new() }; MAX_GAMEPADS],
            connected: Vec::with_capacity(MAX_GAMEPADS),
            actions: crate::ActionMap::default(),
        }
    }

    // --- event recording (engine side) ---

    pub fn set_key(&mut self, key: Key, is_down: bool) {
        self.keys.set(key as usize, is_down);
        self.refresh_actions();
    }

    pub fn set_mouse_button(&mut self, button: MouseButton, is_down: bool) {
        self.mouse.set(button as usize, is_down);
        self.refresh_actions();
    }

    pub fn set_mouse_pos(&mut self, pos: Vec2) {
        self.mouse_pos = pos;
    }

    pub fn add_text(&mut self, text: &str) {
        self.typed_text.push_str(text);
    }

    pub fn typed_text(&self) -> &str {
        &self.typed_text
    }

    pub fn add_mouse_wheel(&mut self, delta: Vec2) {
        self.mouse_wheel += delta;
    }

    pub fn add_mouse_motion(&mut self, delta: Vec2) {
        self.mouse_motion += delta;
    }

    /// A pad was plugged in: it takes the lowest free slot. `None` when all
    /// [`MAX_GAMEPADS`] slots are taken.
    pub fn connect_gamepad(&mut self) -> Option<Gamepad> {
        let slot = (0..MAX_GAMEPADS as u8).find(|&s| !self.connected.contains(&Gamepad(s)))?;
        let pad = Gamepad(slot);
        self.pads[usize::from(slot)] = PadState::new();
        let at = self.connected.partition_point(|p| p.0 < slot);
        self.connected.insert(at, pad);
        self.refresh_actions();
        Some(pad)
    }

    /// A pad was unplugged: its held buttons count as released this frame.
    pub fn disconnect_gamepad(&mut self, pad: Gamepad) {
        self.connected.retain(|&p| p != pad);
        let state = &mut self.pads[pad.slot()];
        state.buttons.release_all();
        state.axes = [0.0; GamepadAxis::COUNT];
        self.refresh_actions();
    }

    pub fn set_gamepad_button(&mut self, pad: Gamepad, button: GamepadButton, is_down: bool) {
        self.pads[pad.slot()].buttons.set(button as usize, is_down);
        self.refresh_actions();
    }

    pub fn set_gamepad_axis(&mut self, pad: Gamepad, axis: GamepadAxis, value: f32) {
        self.pads[pad.slot()].axes[axis as usize] = value;
        self.refresh_actions();
    }

    /// Called when the window loses focus: we will never see the matching
    /// key-up events, so treat everything as released to avoid stuck keys.
    pub fn release_all(&mut self) {
        self.keys.release_all();
        self.mouse.release_all();
        self.refresh_actions();
    }

    /// Clears per-frame state. Called after the game's update for the frame.
    pub fn end_frame(&mut self) {
        self.actions.end_frame();
        self.keys.end_frame();
        self.mouse.end_frame();
        for pad in &mut self.pads {
            pad.buttons.end_frame();
        }
        self.typed_text.clear();
        self.mouse_wheel = Vec2::ZERO;
        self.mouse_motion = Vec2::ZERO;
        self.mouse_pos_at_frame_start = self.mouse_pos;
    }

    pub(crate) fn refresh_actions(&mut self) {
        self.actions.refresh(&self.keys, &self.mouse, &self.pads);
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

    pub fn mouse_motion(&self) -> Vec2 {
        self.mouse_motion
    }

    pub fn gamepads(&self) -> &[Gamepad] {
        &self.connected
    }

    pub fn gamepad_down(&self, pad: Gamepad, button: GamepadButton) -> bool {
        self.pads[pad.slot()].buttons.down[button as usize]
    }

    pub fn gamepad_pressed(&self, pad: Gamepad, button: GamepadButton) -> bool {
        self.pads[pad.slot()].buttons.pressed[button as usize]
    }

    pub fn gamepad_released(&self, pad: Gamepad, button: GamepadButton) -> bool {
        self.pads[pad.slot()].buttons.released[button as usize]
    }

    pub fn gamepad_axis(&self, pad: Gamepad, axis: GamepadAxis) -> f32 {
        self.pads[pad.slot()].axes[axis as usize]
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
    fn raw_mouse_motion_adds_up_and_resets_each_frame() {
        let mut input = Input::new();
        input.add_mouse_motion(Vec2::new(3.0, -1.0));
        input.add_mouse_motion(Vec2::new(2.0, 4.0));
        // The cursor didn't move (it's locked), but the mouse did.
        assert_eq!(input.mouse_delta(), Vec2::ZERO);
        assert_eq!(input.mouse_motion(), Vec2::new(5.0, 3.0));
        input.end_frame();
        assert_eq!(input.mouse_motion(), Vec2::ZERO);
    }

    #[test]
    fn gamepads_take_the_lowest_free_slot() {
        let mut input = Input::new();
        let a = input.connect_gamepad().unwrap();
        let b = input.connect_gamepad().unwrap();
        assert_eq!((a.slot(), b.slot()), (0, 1));
        input.disconnect_gamepad(a);
        assert_eq!(input.gamepads(), [b]);
        let c = input.connect_gamepad().unwrap();
        assert_eq!(c.slot(), 0, "a new pad fills the gap");
        assert_eq!(input.gamepads(), [c, b], "sorted by slot");
        for _ in 2..MAX_GAMEPADS {
            input.connect_gamepad().unwrap();
        }
        assert_eq!(input.connect_gamepad(), None, "all slots taken");
    }

    #[test]
    fn gamepad_buttons_and_axes() {
        let mut input = Input::new();
        let pad = input.connect_gamepad().unwrap();
        input.set_gamepad_button(pad, GamepadButton::South, true);
        input.set_gamepad_axis(pad, GamepadAxis::LeftStickX, -0.5);
        assert!(input.gamepad_pressed(pad, GamepadButton::South));
        assert!(input.gamepad_down(pad, GamepadButton::South));
        assert_eq!(input.gamepad_axis(pad, GamepadAxis::LeftStickX), -0.5);
        input.end_frame();
        assert!(!input.gamepad_pressed(pad, GamepadButton::South));
        assert!(input.gamepad_down(pad, GamepadButton::South));
        assert_eq!(input.gamepad_axis(pad, GamepadAxis::LeftStickX), -0.5, "axes hold");
    }

    #[test]
    fn unplugging_releases_buttons_and_centers_sticks() {
        let mut input = Input::new();
        let pad = input.connect_gamepad().unwrap();
        input.set_gamepad_button(pad, GamepadButton::Start, true);
        input.set_gamepad_axis(pad, GamepadAxis::RightTrigger, 1.0);
        input.end_frame();
        input.disconnect_gamepad(pad);
        assert!(input.gamepad_released(pad, GamepadButton::Start));
        assert!(!input.gamepad_down(pad, GamepadButton::Start));
        assert_eq!(input.gamepad_axis(pad, GamepadAxis::RightTrigger), 0.0);
        assert!(input.gamepads().is_empty());
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
