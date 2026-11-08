//! Named, rebindable input. Configuration allocates; evaluating input does not.
use crate::input::{ButtonStates, PadState};
use crate::{Gamepad, GamepadAxis, GamepadButton, GamepadStick, Key, MAX_GAMEPADS, MouseButton};
use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A digital source. Gamepad slots are stable while connected.
/// ```
/// use ponko::{ButtonBinding, Key};
/// let binding = ButtonBinding::Key(Key::Space);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ButtonBinding {
    /// Physical keyboard key.
    Key(Key),
    /// Mouse button.
    Mouse(MouseButton),
    /// Gamepad slot and button.
    Gamepad {
        /// Slot in `0..MAX_GAMEPADS`.
        pad: Gamepad,
        /// Button to read.
        button: GamepadButton,
    },
}

/// A scalar source. Gamepad values inside the deadzone read zero; outside
/// they are rescaled continuously to full range. Invalid deadzones read zero.
/// ```
/// use ponko::{AxisBinding, Key};
/// let binding = AxisBinding::Keys { negative: Key::A, positive: Key::D };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AxisBinding {
    /// Opposing keys cancel.
    Keys {
        /// Key for -1.
        negative: Key,
        /// Key for +1.
        positive: Key,
    },
    /// Stick component or trigger, with a deadzone in `0..1`.
    Gamepad {
        /// Controller slot.
        pad: Gamepad,
        /// Component or trigger.
        axis: GamepadAxis,
        /// Deadzone in `0..1`.
        deadzone: f32,
    },
}

/// A vector source. Keyboard diagonals are normalized; sticks use a radial
/// deadzone and retain their direction. Y is positive down.
/// ```
/// use ponko::{Axis2Binding, Key};
/// let binding = Axis2Binding::Keys { left: Key::A, right: Key::D, up: Key::W, down: Key::S };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Axis2Binding {
    /// Four physical keys (for example WASD).
    Keys {
        /// Negative x.
        left: Key,
        /// Positive x.
        right: Key,
        /// Negative y.
        up: Key,
        /// Positive y.
        down: Key,
    },
    /// Gamepad stick with a radial deadzone in `0..1`.
    Gamepad {
        /// Controller slot.
        pad: Gamepad,
        /// Stick to read.
        stick: GamepadStick,
        /// Deadzone in `0..1`.
        deadzone: f32,
    },
}

/// An action's kind and sources. Buttons OR together; axes select the source
/// with greatest magnitude (first wins ties), avoiding doubled movement.
/// ```
/// use ponko::{Action, ButtonBinding, Key};
/// let jump = Action::Button(vec![ButtonBinding::Key(Key::Space)]);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    /// Digital sources.
    Button(Vec<ButtonBinding>),
    /// Scalar sources.
    Axis(Vec<AxisBinding>),
    /// Vector sources.
    Axis2(Vec<Axis2Binding>),
}

#[derive(Debug, Clone, Default)]
struct State {
    down: bool,
    pressed: bool,
    released: bool,
    axis: f32,
    axis2: Vec2,
}

/// Serializable bindings, ordered by name. Runtime state is never serialized.
/// Unknown names and queries of the wrong kind return false/zero.
/// ```
/// use ponko::{ActionMap, Action, ButtonBinding, Key};
/// let mut map = ActionMap::default();
/// map.set("jump", Action::Button(vec![ButtonBinding::Key(Key::Space)]));
/// let json = map.to_json().unwrap();
/// let restored = ActionMap::from_json(&json).unwrap();
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionMap {
    bindings: BTreeMap<String, Action>,
    #[serde(skip)]
    states: BTreeMap<String, State>,
}
impl ActionMap {
    /// Add or replace an action. Installation on a context starts with no edges.
    /// ```
    /// # use ponko::*;
    /// let mut map = ActionMap::default();
    /// map.set("jump", Action::Button(vec![]));
    /// ```
    pub fn set(&mut self, name: impl Into<String>, action: Action) {
        let name = name.into();
        self.states.insert(name.clone(), State::default());
        self.bindings.insert(name, action);
    }
    /// Remove a binding.
    /// ```
    /// # use ponko::*;
    /// let mut map = ActionMap::default();
    /// map.remove("jump");
    /// ```
    pub fn remove(&mut self, name: &str) {
        self.bindings.remove(name);
        self.states.remove(name);
    }
    /// Save bindings to JSON. Use `std::fs::write` to persist them.
    /// ```
    /// # use ponko::*;
    /// let json = ActionMap::default().to_json().unwrap();
    /// ```
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
    /// Load bindings from JSON. A malformed document returns an error.
    /// ```
    /// # use ponko::*;
    /// let map = ActionMap::from_json(r#"{"bindings":{}}"#).unwrap();
    /// ```
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
    pub(crate) fn prepare(&mut self) {
        self.states = self.bindings.keys().map(|n| (n.clone(), State::default())).collect();
    }
    pub(crate) fn end_frame(&mut self) {
        for s in self.states.values_mut() {
            s.pressed = false;
            s.released = false;
        }
    }
    pub(crate) fn down(&self, name: &str) -> bool {
        self.states.get(name).is_some_and(|s| s.down)
    }
    pub(crate) fn pressed(&self, name: &str) -> bool {
        self.states.get(name).is_some_and(|s| s.pressed)
    }
    pub(crate) fn released(&self, name: &str) -> bool {
        self.states.get(name).is_some_and(|s| s.released)
    }
    pub(crate) fn axis(&self, name: &str) -> f32 {
        self.states.get(name).map_or(0.0, |s| s.axis)
    }
    pub(crate) fn axis2(&self, name: &str) -> Vec2 {
        self.states.get(name).map_or(Vec2::ZERO, |s| s.axis2)
    }
    pub(crate) fn refresh(
        &mut self,
        keys: &ButtonStates<{ Key::COUNT }>,
        mouse: &ButtonStates<{ MouseButton::COUNT }>,
        pads: &[PadState; MAX_GAMEPADS],
    ) {
        let key = |k: Key| f32::from(keys.down[k as usize]);
        let pad_axis = |p: Gamepad, a: GamepadAxis| pads.get(p.slot()).map_or(0.0, |s| s.axes[a as usize]);
        for (name, action) in &self.bindings {
            let s = self.states.get_mut(name).expect("installed action state");
            match action {
                Action::Button(bindings) => {
                    let down = bindings.iter().any(|b| match *b {
                        ButtonBinding::Key(k) => keys.down[k as usize],
                        ButtonBinding::Mouse(b) => mouse.down[b as usize],
                        ButtonBinding::Gamepad { pad, button } => {
                            pads.get(pad.slot()).is_some_and(|s| s.buttons.down[button as usize])
                        }
                    });
                    s.pressed |= !s.down && down;
                    s.released |= s.down && !down;
                    s.down = down;
                }
                Action::Axis(bindings) => {
                    s.axis = 0.0;
                    for b in bindings {
                        let v = match *b {
                            AxisBinding::Keys { negative, positive } => key(positive) - key(negative),
                            AxisBinding::Gamepad { pad, axis, deadzone } => {
                                deadzone_value(pad_axis(pad, axis), deadzone)
                            }
                        };
                        if v.abs() > s.axis.abs() {
                            s.axis = v;
                        }
                    }
                }
                Action::Axis2(bindings) => {
                    s.axis2 = Vec2::ZERO;
                    for b in bindings {
                        let v = match *b {
                            Axis2Binding::Keys { left, right, up, down } => {
                                Vec2::new(key(right) - key(left), key(down) - key(up)).clamp_length_max(1.0)
                            }
                            Axis2Binding::Gamepad { pad, stick, deadzone } => {
                                let (x, y) = match stick {
                                    GamepadStick::Left => (GamepadAxis::LeftStickX, GamepadAxis::LeftStickY),
                                    GamepadStick::Right => (GamepadAxis::RightStickX, GamepadAxis::RightStickY),
                                };
                                let v = Vec2::new(pad_axis(pad, x), pad_axis(pad, y));
                                let len = v.length();
                                if len.is_finite() && len > 0.0 {
                                    v / len * deadzone_value(len, deadzone)
                                } else {
                                    Vec2::ZERO
                                }
                            }
                        };
                        if v.length_squared() > s.axis2.length_squared() {
                            s.axis2 = v;
                        }
                    }
                }
            }
        }
    }
}
fn deadzone_value(value: f32, deadzone: f32) -> f32 {
    if !value.is_finite() || !(0.0..1.0).contains(&deadzone) {
        return 0.0;
    }
    value.signum() * ((value.abs().min(1.0) - deadzone).max(0.0) / (1.0 - deadzone))
}

/// Raw input for headless tests and replay. Feed before querying actions.
/// ```
/// use ponko::{InputEvent, Key};
/// let event = InputEvent::Key(Key::Space, true);
/// ```
#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    /// Key and held state.
    Key(Key, bool),
    /// Mouse button and held state.
    Mouse(MouseButton, bool),
    /// Gamepad slot, button and held state.
    GamepadButton(Gamepad, GamepadButton, bool),
    /// Gamepad slot, axis and value.
    GamepadAxis(Gamepad, GamepadAxis, f32),
    /// Release all keyboard and mouse buttons (focus loss).
    FocusLost,
    /// Release a controller's buttons and center its axes.
    GamepadDisconnected(Gamepad),
}
