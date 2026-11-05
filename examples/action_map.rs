//! Named actions, shared by keyboard and gamepad. Runs without a window.
use crabbyengine::*;

// ANCHOR: action_map
fn main() {
    let pad = Gamepad::from_slot(0).unwrap();
    let mut bindings = ActionMap::default();
    bindings.set(
        "jump",
        Action::Button(vec![
            ButtonBinding::Key(Key::Space),
            ButtonBinding::Gamepad {
                pad,
                button: GamepadButton::South,
            },
        ]),
    );
    bindings.set(
        "move",
        Action::Axis2(vec![
            Axis2Binding::Keys {
                left: Key::A,
                right: Key::D,
                up: Key::W,
                down: Key::S,
            },
            Axis2Binding::Gamepad {
                pad,
                stick: GamepadStick::Left,
                deadzone: 0.2,
            },
        ]),
    );
    bindings.set(
        "throttle",
        Action::Axis(vec![AxisBinding::Gamepad {
            pad,
            axis: GamepadAxis::RightTrigger,
            deadzone: 0.1,
        }]),
    );
    let saved = bindings.to_json().unwrap();
    let restored = ActionMap::from_json(&saved).unwrap();
    Game::new("actions", 1, 1).run_headless(0, |ctx| {
        ctx.set_actions(restored);
        ctx.feed_input(InputEvent::Key(Key::Space, true));
        assert!(ctx.action_pressed("jump") && ctx.action_down("jump"));
        let movement = ctx.action_axis2("move");
        let throttle = ctx.action_axis("throttle");
        assert_eq!((movement, throttle), (Vec2::ZERO, 0.0));
        ctx.feed_input(InputEvent::Key(Key::Space, false));
        assert!(ctx.action_released("jump"));
        let mut changed = ctx.actions().clone();
        changed.set("jump", Action::Button(vec![ButtonBinding::Key(Key::Enter)]));
        ctx.set_actions(changed);
        assert!(!ctx.action_pressed("jump"));
    });
}
// ANCHOR_END: action_map
