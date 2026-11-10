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

/// An action resolved once, so a query is an array index instead of a string
/// lookup. Get one from [`Input::action_id`](crate::Input::action_id) (or
/// [`actions!`](crate::actions!) for a set), keep it, and pass it where the
/// `&str` forms take a name.
///
/// An id stays valid when bindings are replaced with
/// [`Input::set_actions`](crate::Input::set_actions) (rebinding, loading a file):
/// the name keeps its slot. [`ActionId::NONE`] is what resolving an unknown
/// name gives; it reads false/zero. An id belongs to the context it came from.
/// ```
/// # use ponko::*;
/// Game::new("ids", 1, 1).run_headless(0, |ctx| {
///     let mut map = ActionMap::default();
///     map.set("jump", Action::Button(vec![ButtonBinding::Key(Key::Space)]));
///     ctx.input.set_actions(map);
///     let jump = ctx.input.action_id("jump");
///     ctx.input.feed(InputEvent::Key(Key::Space, true));
///     assert!(ctx.input.action_down(jump));
/// });
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionId(u32);
impl ActionId {
    /// The id of no action: reads false/zero. Resolving an unknown name
    /// returns it (after logging an error).
    pub const NONE: ActionId = ActionId(u32::MAX);
    /// Whether this is [`NONE`](Self::NONE).
    pub fn is_none(self) -> bool {
        self == Self::NONE
    }
}
impl Default for ActionId {
    fn default() -> Self {
        Self::NONE
    }
}

/// Resolving a name that no installed action has. Its message names the
/// known actions, which is usually enough to spot the typo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownAction {
    name: String,
    known: Vec<String>,
}
impl UnknownAction {
    /// The name that was asked for.
    pub fn name(&self) -> &str {
        &self.name
    }
}
impl std::fmt::Display for UnknownAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown action {:?}", self.name)?;
        if self.known.is_empty() {
            write!(f, " (no actions are installed; see Ctx::set_actions)")
        } else {
            write!(f, " (known: {})", self.known.join(", "))
        }
    }
}
impl std::error::Error for UnknownAction {}

/// Something that names an action in a query: an [`ActionId`] (resolved once,
/// the fast form) or a `&str` name (a lookup per call). Implemented for
/// both so `ctx.input.action_down("jump")` and `ctx.input.action_down(jump)` work.
pub trait ActionRef: sealed::Sealed + Copy {
    #[doc(hidden)]
    fn slot(self, map: &ActionMap) -> Option<usize>;
}
mod sealed {
    pub trait Sealed {}
    impl Sealed for super::ActionId {}
    impl Sealed for &str {}
    impl Sealed for &String {}
}
impl ActionRef for ActionId {
    #[inline]
    fn slot(self, map: &ActionMap) -> Option<usize> {
        let i = self.0 as usize;
        (i < map.states.len()).then_some(i)
    }
}
impl ActionRef for &str {
    #[inline]
    fn slot(self, map: &ActionMap) -> Option<usize> {
        map.slots.get(self).map(|&i| i as usize)
    }
}
impl ActionRef for &String {
    #[inline]
    fn slot(self, map: &ActionMap) -> Option<usize> {
        self.as_str().slot(map)
    }
}

/// Declares a set of actions once: a `Copy` struct with one [`ActionId`] per
/// field, resolved in one call, and the names as a constant. No proc macro,
/// and a mistyped field is a compile error instead of a silent `false`.
/// ```
/// # use ponko::*;
/// actions! {
///     /// The game's actions.
///     pub struct Act { jump = "jump", steer = "steer" }
/// }
/// Game::new("act", 1, 1).run_headless(0, |ctx| {
///     let mut map = ActionMap::default();
///     for name in Act::NAMES {
///         map.set(*name, Action::Button(vec![]));
///     }
///     ctx.input.set_actions(map);
///     let act = Act::resolve(ctx);
///     assert!(!ctx.input.action_down(act.jump));
/// });
/// ```
#[macro_export]
macro_rules! actions {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident { $($field:ident = $text:literal),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy)]
        $vis struct $name {
            $(
                #[doc = concat!("The action `", $text, "`.")]
                $vis $field: $crate::ActionId,
            )+
        }
        impl $name {
            /// The action names, in declaration order.
            $vis const NAMES: &'static [&'static str] = &[$($text),+];
            /// Resolves every action once; each unknown name logs an error
            /// and reads false/zero.
            $vis fn resolve(ctx: &$crate::Ctx) -> Self {
                Self { $($field: ctx.input.action_id($text)),+ }
            }
        }
    };
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
/// Unknown names and queries of the wrong kind return false/zero; resolve a
/// name once with [`ActionMap::resolve`] or `Input::action_id` to learn of a typo.
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
    // Runtime state lives in `states`, one slot per name ever installed (an
    // `ActionId` is the slot). `order` is the slot of each binding, in
    // `bindings` order, so refreshing needs no name lookups.
    #[serde(skip)]
    slots: BTreeMap<String, u32>,
    #[serde(skip)]
    states: Vec<State>,
    #[serde(skip)]
    order: Vec<u32>,
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
        self.bindings.insert(name.clone(), action);
        self.sync();
        let slot = self.slots[&name] as usize;
        self.states[slot] = State::default();
    }
    /// Remove a binding.
    /// ```
    /// # use ponko::*;
    /// let mut map = ActionMap::default();
    /// map.remove("jump");
    /// ```
    pub fn remove(&mut self, name: &str) {
        self.bindings.remove(name);
        if let Some(&slot) = self.slots.get(name) {
            self.states[slot as usize] = State::default();
        }
        self.sync();
    }
    /// Resolves an action name once, for the id-based queries. An unknown
    /// name is an error that lists the known actions.
    /// ```
    /// # use ponko::*;
    /// let mut map = ActionMap::default();
    /// map.set("jump", Action::Button(vec![]));
    /// assert!(map.resolve("jump").is_ok());
    /// assert!(map.resolve("jmup").is_err());
    /// ```
    pub fn resolve(&self, name: &str) -> Result<ActionId, UnknownAction> {
        match self.slots.get(name) {
            Some(&slot) if self.bindings.contains_key(name) => Ok(ActionId(slot)),
            _ => Err(UnknownAction {
                name: name.to_string(),
                known: self.bindings.keys().cloned().collect(),
            }),
        }
    }
    /// Gives every binding a state slot (new names get the next free one)
    /// and rebuilds the per-binding slot order.
    fn sync(&mut self) {
        for name in self.bindings.keys() {
            if !self.slots.contains_key(name) {
                self.slots.insert(name.clone(), self.states.len() as u32);
                self.states.push(State::default());
            }
        }
        self.order = self.bindings.keys().map(|n| self.slots[n]).collect();
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
    /// Installs over `old`: names that had a slot keep it (so ids resolved
    /// earlier stay valid), new names get the next ones; all state restarts.
    pub(crate) fn prepare_after(&mut self, old: &ActionMap) {
        self.slots = old.slots.clone();
        self.states = vec![State::default(); old.states.len()];
        self.sync();
    }
    pub(crate) fn end_frame(&mut self) {
        for s in &mut self.states {
            s.pressed = false;
            s.released = false;
        }
    }
    #[inline]
    pub(crate) fn down(&self, action: impl ActionRef) -> bool {
        action.slot(self).is_some_and(|i| self.states[i].down)
    }
    #[inline]
    pub(crate) fn pressed(&self, action: impl ActionRef) -> bool {
        action.slot(self).is_some_and(|i| self.states[i].pressed)
    }
    #[inline]
    pub(crate) fn released(&self, action: impl ActionRef) -> bool {
        action.slot(self).is_some_and(|i| self.states[i].released)
    }
    #[inline]
    pub(crate) fn axis(&self, action: impl ActionRef) -> f32 {
        action.slot(self).map_or(0.0, |i| self.states[i].axis)
    }
    #[inline]
    pub(crate) fn axis2(&self, action: impl ActionRef) -> Vec2 {
        action.slot(self).map_or(Vec2::ZERO, |i| self.states[i].axis2)
    }
    pub(crate) fn refresh(
        &mut self,
        keys: &ButtonStates<{ Key::COUNT }>,
        mouse: &ButtonStates<{ MouseButton::COUNT }>,
        pads: &[PadState; MAX_GAMEPADS],
    ) {
        let key = |k: Key| f32::from(keys.down[k as usize]);
        let pad_axis = |p: Gamepad, a: GamepadAxis| pads.get(p.slot()).map_or(0.0, |s| s.axes[a as usize]);
        for (action, &slot) in self.bindings.values().zip(&self.order) {
            let s = &mut self.states[slot as usize];
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
