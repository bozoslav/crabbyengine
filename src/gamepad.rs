//! Reading gamepads with gilrs, into [`Input`] (the `gamepad` feature).

use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};

use crate::input::{Gamepad, GamepadAxis, GamepadButton, Input, MAX_GAMEPADS};

/// Connects gilrs's pads to the engine's slots.
pub(crate) struct Gamepads {
    gilrs: Option<Gilrs>,
    slots: [Option<GamepadId>; MAX_GAMEPADS],
}

impl Gamepads {
    /// Opens the gamepad system and connects the pads already plugged in.
    /// Without it (no permission, unsupported system), there are no pads.
    pub(crate) fn new(input: &mut Input) -> Self {
        let gilrs = match Gilrs::new() {
            Ok(gilrs) => Some(gilrs),
            Err(gilrs::Error::NotImplemented(gilrs)) => Some(gilrs),
            Err(e) => {
                eprintln!("crabbyengine: gamepads unavailable ({e})");
                None
            }
        };
        let mut pads = Self {
            gilrs,
            slots: [None; MAX_GAMEPADS],
        };
        let ids: Vec<GamepadId> = pads.gilrs.iter().flat_map(|g| g.gamepads().map(|(id, _)| id)).collect();
        for id in ids {
            pads.connect(id, input);
        }
        pads
    }

    fn connect(&mut self, id: GamepadId, input: &mut Input) {
        if self.slot_of(id).is_some() {
            return;
        }
        if let Some(pad) = input.connect_gamepad() {
            self.slots[pad.slot()] = Some(id);
        }
    }

    fn slot_of(&self, id: GamepadId) -> Option<Gamepad> {
        let slot = self.slots.iter().position(|&s| s == Some(id))?;
        Some(Gamepad(slot as u8))
    }

    /// Applies everything the pads did since the last frame.
    pub(crate) fn poll(&mut self, input: &mut Input) {
        while let Some(event) = self.gilrs.as_mut().and_then(Gilrs::next_event) {
            if let EventType::Connected = event.event {
                self.connect(event.id, input);
                continue;
            }
            let Some(pad) = self.slot_of(event.id) else {
                continue;
            };
            match event.event {
                EventType::Disconnected => {
                    input.disconnect_gamepad(pad);
                    self.slots[pad.slot()] = None;
                }
                EventType::ButtonPressed(b, _) => {
                    if let Some(b) = button(b) {
                        input.set_gamepad_button(pad, b, true);
                    }
                }
                EventType::ButtonReleased(b, _) => {
                    if let Some(b) = button(b) {
                        input.set_gamepad_button(pad, b, false);
                    }
                }
                EventType::ButtonChanged(b, value, _) => match b {
                    Button::LeftTrigger2 => input.set_gamepad_axis(pad, GamepadAxis::LeftTrigger, value),
                    Button::RightTrigger2 => input.set_gamepad_axis(pad, GamepadAxis::RightTrigger, value),
                    _ => {}
                },
                EventType::AxisChanged(a, value, _) => {
                    if let Some((axis, sign)) = axis(a) {
                        input.set_gamepad_axis(pad, axis, value * sign);
                    }
                }
                _ => {}
            }
        }
    }
}

/// gilrs's button names (by position) to ours.
fn button(b: Button) -> Option<GamepadButton> {
    use GamepadButton as G;
    Some(match b {
        Button::South => G::South,
        Button::East => G::East,
        Button::North => G::North,
        Button::West => G::West,
        Button::LeftTrigger => G::LeftBumper,
        Button::RightTrigger => G::RightBumper,
        Button::LeftTrigger2 => G::LeftTrigger,
        Button::RightTrigger2 => G::RightTrigger,
        Button::Select => G::Select,
        Button::Start => G::Start,
        Button::LeftThumb => G::LeftStick,
        Button::RightThumb => G::RightStick,
        Button::DPadUp => G::DPadUp,
        Button::DPadDown => G::DPadDown,
        Button::DPadLeft => G::DPadLeft,
        Button::DPadRight => G::DPadRight,
        _ => return None,
    })
}

/// gilrs's axes to ours, with the sign that makes y point down.
fn axis(a: Axis) -> Option<(GamepadAxis, f32)> {
    Some(match a {
        Axis::LeftStickX => (GamepadAxis::LeftStickX, 1.0),
        Axis::LeftStickY => (GamepadAxis::LeftStickY, -1.0),
        Axis::RightStickX => (GamepadAxis::RightStickX, 1.0),
        Axis::RightStickY => (GamepadAxis::RightStickY, -1.0),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_engine_button_has_a_gilrs_button() {
        let all = [
            Button::South,
            Button::East,
            Button::North,
            Button::West,
            Button::LeftTrigger,
            Button::RightTrigger,
            Button::LeftTrigger2,
            Button::RightTrigger2,
            Button::Select,
            Button::Start,
            Button::LeftThumb,
            Button::RightThumb,
            Button::DPadUp,
            Button::DPadDown,
            Button::DPadLeft,
            Button::DPadRight,
        ];
        let mapped: std::collections::HashSet<_> = all.iter().filter_map(|&b| button(b)).collect();
        assert_eq!(mapped.len(), GamepadButton::COUNT);
        assert_eq!(button(Button::Mode), None);
    }

    #[test]
    fn stick_y_points_down() {
        assert_eq!(axis(Axis::LeftStickY), Some((GamepadAxis::LeftStickY, -1.0)));
        assert_eq!(axis(Axis::LeftStickX), Some((GamepadAxis::LeftStickX, 1.0)));
        assert_eq!(axis(Axis::DPadX), None);
    }
}
