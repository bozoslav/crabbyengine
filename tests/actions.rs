use ponko::*;

fn bindings() -> ActionMap {
    let mut map = ActionMap::default();
    let pad = Gamepad::from_slot(0).unwrap();
    map.set(
        "jump",
        Action::Button(vec![
            ButtonBinding::Key(Key::Space),
            ButtonBinding::Mouse(MouseButton::Left),
            ButtonBinding::Gamepad {
                pad,
                button: GamepadButton::South,
            },
        ]),
    );
    map.set(
        "throttle",
        Action::Axis(vec![
            AxisBinding::Keys {
                negative: Key::S,
                positive: Key::W,
            },
            AxisBinding::Gamepad {
                pad,
                axis: GamepadAxis::RightTrigger,
                deadzone: 0.2,
            },
        ]),
    );
    map.set(
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
    map
}

#[test]
fn aggregate_edges_survive_taps_and_clear_between_headless_frames() {
    let mut frame = 0;
    Game::new("actions", 1, 1)
        .sim_system(move |ctx: &mut Ctx| {
            match frame {
                0 => {
                    ctx.input.feed(InputEvent::Key(Key::Space, true));
                    assert!(ctx.input.action_pressed("jump"));
                }
                1 => {
                    assert!(ctx.input.action_down("jump"));
                    assert!(!ctx.input.action_pressed("jump"));
                    ctx.input.feed(InputEvent::Mouse(MouseButton::Left, true));
                    ctx.input.feed(InputEvent::Key(Key::Space, false));
                    assert!(ctx.input.action_down("jump"));
                    assert!(!ctx.input.action_pressed("jump"));
                    assert!(!ctx.input.action_released("jump"));
                }
                2 => {
                    ctx.input.feed(InputEvent::Mouse(MouseButton::Left, false));
                    assert!(ctx.input.action_released("jump"));
                    ctx.input.feed(InputEvent::Key(Key::Space, true));
                    ctx.input.feed(InputEvent::Key(Key::Space, false));
                    assert!(ctx.input.action_pressed("jump"));
                    assert!(ctx.input.action_released("jump"));
                    assert!(!ctx.input.action_down("jump"));
                }
                3 => {
                    assert!(!ctx.input.action_pressed("jump"));
                    assert!(!ctx.input.action_released("jump"));
                    let pad = Gamepad::from_slot(0).unwrap();
                    ctx.input
                        .feed(InputEvent::GamepadButton(pad, GamepadButton::South, true));
                    assert!(ctx.input.action_pressed("jump"));
                    ctx.input.feed(InputEvent::GamepadDisconnected(pad));
                    assert!(ctx.input.action_released("jump"));
                    assert!(!ctx.input.action_down("jump"));
                }
                _ => unreachable!(),
            }
            frame += 1;
        })
        .run_headless(4, |ctx| ctx.input.set_actions(bindings()));
}

#[test]
fn axes_cancel_normalize_select_strongest_and_apply_deadzones() {
    Game::new("axes", 1, 1).run_headless(0, |ctx| {
        ctx.input.set_actions(bindings());
        ctx.input.feed(InputEvent::Key(Key::W, true));
        ctx.input.feed(InputEvent::Key(Key::D, true));
        let movement = ctx.input.action_axis2("move");
        assert!((movement.length() - 1.0).abs() < 1e-6);
        assert!(movement.x > 0.0 && movement.y < 0.0);
        assert_eq!(ctx.input.action_axis("throttle"), 1.0);
        ctx.input.feed(InputEvent::Key(Key::S, true));
        assert_eq!(ctx.input.action_axis("throttle"), 0.0);
        ctx.input.feed(InputEvent::FocusLost);
        assert_eq!(ctx.input.action_axis2("move"), Vec2::ZERO);
        let pad = Gamepad::from_slot(0).unwrap();
        ctx.input
            .feed(InputEvent::GamepadAxis(pad, GamepadAxis::RightTrigger, 0.1));
        assert_eq!(ctx.input.action_axis("throttle"), 0.0);
        ctx.input
            .feed(InputEvent::GamepadAxis(pad, GamepadAxis::RightTrigger, 0.6));
        assert!((ctx.input.action_axis("throttle") - 0.5).abs() < 1e-6);
        ctx.input
            .feed(InputEvent::GamepadAxis(pad, GamepadAxis::LeftStickX, 0.6));
        ctx.input
            .feed(InputEvent::GamepadAxis(pad, GamepadAxis::LeftStickY, 0.8));
        assert!((ctx.input.action_axis2("move") - Vec2::new(0.6, 0.8)).length() < 1e-6);
        ctx.input
            .feed(InputEvent::GamepadAxis(pad, GamepadAxis::LeftStickX, f32::NAN));
        assert_eq!(ctx.input.action_axis2("move"), Vec2::ZERO);
        ctx.input.feed(InputEvent::GamepadDisconnected(pad));
        assert_eq!(ctx.input.action_axis("throttle"), 0.0);
        assert_eq!(ctx.input.action_axis2("move"), Vec2::ZERO);
        assert!(!ctx.input.action_down("absent"));
        assert_eq!(ctx.input.action_axis("move"), 0.0);
        assert_eq!(ctx.input.action_axis2("jump"), Vec2::ZERO);
    });
}

#[test]
fn json_round_trip_and_rebinding_reset_edges_without_releasing_held_sources() {
    let json = bindings().to_json().unwrap();
    let map = ActionMap::from_json(&json).unwrap();
    assert_eq!(map.to_json().unwrap(), json);
    assert!(ActionMap::from_json("bad").is_err());
    assert!(
        ActionMap::from_json(r#"{"bindings":{"bad":{"Button":[{"Gamepad":{"pad":8,"button":"South"}}]}}}"#).is_err()
    );
    Game::new("rebind", 1, 1).run_headless(0, |ctx| {
        ctx.input.feed(InputEvent::Key(Key::Space, true));
        ctx.input.set_actions(map);
        assert!(ctx.input.action_down("jump"));
        assert!(!ctx.input.action_pressed("jump"));
        let mut changed = ctx.input.actions().clone();
        changed.set("jump", Action::Button(vec![ButtonBinding::Key(Key::Enter)]));
        changed.remove("move");
        ctx.input.set_actions(changed);
        assert!(!ctx.input.action_down("jump"));
        assert!(!ctx.input.action_released("jump"));
        ctx.input.feed(InputEvent::Key(Key::Enter, true));
        assert!(ctx.input.action_pressed("jump"));
        ctx.input.feed(InputEvent::FocusLost);
        assert!(ctx.input.action_released("jump"));
        assert_eq!(ctx.input.action_axis2("move"), Vec2::ZERO);
    });
}
