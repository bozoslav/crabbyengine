//! Named actions, shared by keyboard and gamepad. Runs without a window.
#[allow(unused_imports)]
use ponko::input::{Gamepad, GamepadAxis, GamepadButton, GamepadStick};
use ponko::prelude::*;

// ANCHOR: action_map
// The names live in one place; `Act::resolve` turns them into `Copy` ids once.
actions! {
    struct Act { jump = "jump", movement = "move", throttle = "throttle" }
}

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
    Game::new("actions", 1, 1)
        .setup(|ctx| {
            ctx.input.set_actions(restored);
            // Resolve once, after installing the bindings (an unknown name logs an
            // error here instead of reading false every frame). Ids stay valid
            // when the bindings are replaced below.
            let act = Act::resolve(ctx);
            ctx.input.feed(actions::InputEvent::Key(Key::Space, true));
            assert!(ctx.input.action_pressed(act.jump) && ctx.input.action_down(act.jump));
            let movement = ctx.input.action_axis2(act.movement);
            let throttle = ctx.input.action_axis(act.throttle);
            assert_eq!((movement, throttle), (Vec2::ZERO, 0.0));
            ctx.input.feed(actions::InputEvent::Key(Key::Space, false));
            assert!(ctx.input.action_released(act.jump));
            let mut changed = ctx.input.actions().clone();
            changed.set("jump", Action::Button(vec![ButtonBinding::Key(Key::Enter)]));
            ctx.input.set_actions(changed);
            assert!(!ctx.input.action_pressed(act.jump));
            // The name form still works, for one-off checks and tools.
            assert!(!ctx.input.action_down("jump"));
        })
        .run_headless(0);
}
// ANCHOR_END: action_map
