//! `Input::pressed_this_frame` driven by tick-indexed input scenarios: the
//! query a rebinding screen uses, checked tick by tick.
use ponko::actions::InputEvent;
use ponko::input::{Gamepad, GamepadAxis, GamepadButton};
use ponko::prelude::*;
use ponko::scenario::InputScenario;
use std::cell::RefCell;
use std::rc::Rc;

fn pad0() -> Gamepad {
    Gamepad::from_slot(0).unwrap()
}

/// What the query returned on each sim tick.
fn record(scenario: InputScenario, ticks: u64) -> Vec<Vec<PressedInput>> {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = seen.clone();
    Game::new("pressed", 1, 1)
        .sim_system(move |ctx: &mut Ctx| log.borrow_mut().push(ctx.input.pressed_this_frame().collect()))
        .input_scenario(scenario)
        .run_headless(ticks, |_| {});
    Rc::try_unwrap(seen).unwrap().into_inner()
}

fn key(k: Key) -> PressedInput {
    PressedInput::Button(ButtonBinding::Key(k))
}

#[test]
fn each_device_is_reported_on_exactly_its_tick() {
    let scenario = InputScenario::new()
        .at(1, InputEvent::Key(Key::F, true))
        .at(2, InputEvent::Mouse(MouseButton::Middle, true))
        .at(3, InputEvent::GamepadButton(pad0(), GamepadButton::North, true))
        .at(4, InputEvent::GamepadAxis(pad0(), GamepadAxis::LeftTrigger, 0.8));
    let trace = record(scenario, 6);
    assert_eq!(
        trace,
        [
            vec![],
            vec![key(Key::F)],
            vec![PressedInput::Button(ButtonBinding::Mouse(MouseButton::Middle))],
            vec![PressedInput::Button(ButtonBinding::Gamepad {
                pad: pad0(),
                button: GamepadButton::North
            })],
            vec![PressedInput::Axis(AxisBinding::Gamepad {
                pad: pad0(),
                axis: GamepadAxis::LeftTrigger,
                deadzone: ponko::actions::DEFAULT_DEADZONE
            })],
            vec![],
        ]
    );
}

#[test]
fn held_inputs_and_releases_are_not_presses() {
    let scenario = InputScenario::new()
        .at(0, InputEvent::Key(Key::F, true))
        .at(3, InputEvent::Key(Key::F, false))
        .at(4, InputEvent::Key(Key::F, true));
    let trace = record(scenario, 6);
    assert_eq!(
        trace,
        [
            vec![key(Key::F)],
            vec![],
            vec![],
            vec![], // released
            vec![key(Key::F)],
            vec![],
        ]
    );
}

#[test]
fn several_presses_on_one_tick_come_back_in_a_fixed_order() {
    // Fed in two different orders, the same list comes back.
    let a = InputScenario::new()
        .at(0, InputEvent::Mouse(MouseButton::Left, true))
        .at(0, InputEvent::Key(Key::X, true))
        .at(0, InputEvent::Key(Key::C, true));
    let b = InputScenario::new()
        .at(0, InputEvent::Key(Key::C, true))
        .at(0, InputEvent::Key(Key::X, true))
        .at(0, InputEvent::Mouse(MouseButton::Left, true));
    let expected = vec![
        key(Key::C),
        key(Key::X),
        PressedInput::Button(ButtonBinding::Mouse(MouseButton::Left)),
    ];
    assert_eq!(record(a, 1)[0], expected);
    assert_eq!(record(b, 1)[0], expected);
}

#[test]
fn a_stick_reports_once_when_it_crosses_the_threshold() {
    let stick = |x| InputEvent::GamepadAxis(pad0(), GamepadAxis::LeftStickX, x);
    let scenario = InputScenario::new()
        .at(0, stick(0.2))
        .at(1, stick(0.7)) // crosses
        .at(2, stick(1.0)) // held
        .at(3, stick(0.0))
        .at(4, stick(-0.6)); // crosses again, the other way
    let counts: Vec<usize> = record(scenario, 6).iter().map(Vec::len).collect();
    assert_eq!(counts, [0, 1, 0, 0, 1, 0]);
}

/// A rebinding screen: bind "jump" to whatever is pressed next, ignoring the
/// press that opened the prompt.
#[derive(Default)]
struct Prompt {
    open_at: u64,
    bound: Option<ButtonBinding>,
}

fn rebinding_run(scenario: InputScenario) -> (Option<ButtonBinding>, Vec<(u64, bool)>) {
    let prompt = Rc::new(RefCell::new(Prompt {
        open_at: 2,
        bound: None,
    }));
    let jumps = Rc::new(RefCell::new(Vec::new()));
    let (p, j) = (prompt.clone(), jumps.clone());
    Game::new("rebind", 1, 1)
        .sim_system(move |ctx: &mut Ctx| {
            let tick = ctx.time.sim_tick();
            let mut p = p.borrow_mut();
            if tick > p.open_at && p.bound.is_none() {
                // The prompt takes the first button-like press and binds it.
                let pressed = ctx.input.pressed_this_frame().find_map(|p| p.button().cloned());
                if let Some(binding) = pressed {
                    ctx.input.set_action("jump", Action::Button(vec![binding.clone()]));
                    p.bound = Some(binding);
                }
            }
            j.borrow_mut().push((tick, ctx.input.action_pressed("jump")));
        })
        .input_scenario(scenario)
        .run_headless(10, |ctx| {
            let mut map = ActionMap::default();
            map.set("jump", Action::Button(vec![ButtonBinding::Key(Key::Space)]));
            ctx.input.set_actions(map);
        });
    let bound = prompt.borrow().bound.clone();
    let jumps = jumps.borrow().clone();
    (bound, jumps)
}

#[test]
fn a_rebinding_prompt_binds_the_next_press_and_the_new_binding_works() {
    let scenario = InputScenario::new()
        // Before the prompt: Space still jumps.
        .at(1, InputEvent::Key(Key::Space, true))
        .at(1, InputEvent::Key(Key::Space, false))
        // The player presses a gamepad button in the prompt.
        .at(4, InputEvent::GamepadButton(pad0(), GamepadButton::West, true))
        .at(5, InputEvent::GamepadButton(pad0(), GamepadButton::West, false))
        // Later, that button jumps and Space no longer does.
        .at(7, InputEvent::GamepadButton(pad0(), GamepadButton::West, true))
        .at(8, InputEvent::Key(Key::Space, true));
    let (bound, jumps) = rebinding_run(scenario);
    assert_eq!(
        bound,
        Some(ButtonBinding::Gamepad {
            pad: pad0(),
            button: GamepadButton::West
        })
    );
    let jumped: Vec<u64> = jumps.iter().filter(|(_, j)| *j).map(|(t, _)| *t).collect();
    assert_eq!(
        jumped,
        [1, 7],
        "Space on tick 1 and the new button on tick 7, not Space on tick 8"
    );
}

#[test]
fn the_same_scenario_gives_the_same_trace_every_run() {
    let scenario = || {
        InputScenario::new()
            .at(1, InputEvent::Key(Key::Num1, true))
            .at(1, InputEvent::GamepadAxis(pad0(), GamepadAxis::RightStickY, 0.9))
            .at(3, InputEvent::Mouse(MouseButton::Back, true))
    };
    assert_eq!(record(scenario(), 5), record(scenario(), 5));
}
