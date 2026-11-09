//! Tick-indexed input scenarios driving named actions in headless runs.
use ponko::*;
use std::cell::RefCell;
use std::rc::Rc;

/// What a sim system saw of the "jump" action on one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Seen {
    tick: u64,
    pressed: bool,
    down: bool,
    released: bool,
}

fn jump_bindings(ctx: &mut Ctx) {
    let mut map = ActionMap::default();
    map.set("jump", Action::Button(vec![ButtonBinding::Key(Key::Space)]));
    ctx.input.set_actions(map);
}

/// Runs `ticks` headless ticks with `scenario` and records the jump action per tick.
fn record(scenario: InputScenario, ticks: u64) -> Vec<Seen> {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = seen.clone();
    Game::new("scenario", 1, 1)
        .sim_system(move |ctx: &mut Ctx| {
            log.borrow_mut().push(Seen {
                tick: ctx.time.sim_tick(),
                pressed: ctx.input.action_pressed("jump"),
                down: ctx.input.action_down("jump"),
                released: ctx.input.action_released("jump"),
            });
        })
        .input_scenario(scenario)
        .run_headless(ticks, jump_bindings);
    Rc::try_unwrap(seen).unwrap().into_inner()
}

fn seen(tick: u64, pressed: bool, down: bool, released: bool) -> Seen {
    Seen {
        tick,
        pressed,
        down,
        released,
    }
}

#[test]
fn two_transition_scenario_presses_holds_and_releases_on_its_ticks() {
    let scenario = InputScenario::new()
        .at(2, InputEvent::Key(Key::Space, true))
        .at(5, InputEvent::Key(Key::Space, false));
    assert_eq!(
        record(scenario, 7),
        [
            seen(0, false, false, false),
            seen(1, false, false, false),
            seen(2, true, true, false), // pressed edge on exactly one tick
            seen(3, false, true, false),
            seen(4, false, true, false),
            seen(5, false, false, true), // released edge on exactly one tick
            seen(6, false, false, false),
        ]
    );
}

#[test]
fn steps_apply_in_tick_order_and_a_same_tick_tap_keeps_both_edges() {
    // Added out of order; the press and release on tick 3 keep their order.
    let scenario = InputScenario::new()
        .at(3, InputEvent::Key(Key::Space, true))
        .at(1, InputEvent::Key(Key::Space, true))
        .at(3, InputEvent::Key(Key::Space, false))
        .at(2, InputEvent::Key(Key::Space, false));
    assert_eq!(
        record(scenario, 5),
        [
            seen(0, false, false, false),
            seen(1, true, true, false),
            seen(2, false, false, true),
            seen(3, true, false, true),
            seen(4, false, false, false),
        ]
    );
}

#[test]
fn steps_wait_for_their_tick_and_steps_past_the_run_never_apply() {
    let scenario = InputScenario::new()
        .at(0, InputEvent::Key(Key::Space, true))
        .at(100, InputEvent::Key(Key::Space, false));
    let trace = record(scenario, 3);
    assert_eq!(trace[0], seen(0, true, true, false));
    assert!(trace[1..].iter().all(|s| s.down && !s.pressed && !s.released));
}

/// A gameplay interaction: a body that jumps when the action is pressed and
/// falls under gravity.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Body {
    y: f32,
    vy: f32,
    jumps: u32,
}

fn jump_and_fall(ctx: &mut Ctx) {
    let dt = ctx.dt();
    let jump = ctx.input.action_pressed("jump");
    let drift = ctx.random().stream(0).range(-0.01, 0.01);
    ctx.world.query::<&mut Body>().for_each(|b| {
        if jump && b.y <= 0.0 {
            b.vy = 5.0;
            b.jumps += 1;
        }
        b.vy -= 9.81 * dt;
        b.y = (b.y + (b.vy + drift) * dt).max(0.0);
    });
}

fn play(scenario: InputScenario) -> Body {
    let mut world = Game::new("jumping", 1, 1)
        .seed(7)
        .sim_system(jump_and_fall)
        .input_scenario(scenario)
        .run_headless(120, |ctx| {
            jump_bindings(ctx);
            ctx.world.spawn((Body {
                y: 0.0,
                vy: 0.0,
                jumps: 0,
            },));
        });
    *world.query::<&Body>().iter().next().unwrap()
}

fn two_jumps() -> InputScenario {
    InputScenario::new()
        .at(10, InputEvent::Key(Key::Space, true))
        .at(20, InputEvent::Key(Key::Space, false))
        .at(90, InputEvent::Key(Key::Space, true))
        .at(91, InputEvent::Key(Key::Space, false))
}

#[test]
fn same_seed_and_scenario_give_bit_identical_results() {
    let (a, b) = (play(two_jumps()), play(two_jumps()));
    assert_eq!(a.jumps, 2);
    assert_eq!((a.y.to_bits(), a.vy.to_bits()), (b.y.to_bits(), b.vy.to_bits()));
    assert_eq!(a.jumps, b.jumps);
}

#[test]
fn holding_the_action_jumps_once() {
    // Held from tick 10 to the end: one pressed edge, so one jump.
    let body = play(InputScenario::new().at(10, InputEvent::Key(Key::Space, true)));
    assert_eq!(body.jumps, 1);
}

#[test]
fn without_a_scenario_nothing_is_pressed() {
    assert_eq!(play(InputScenario::new()).jumps, 0);
    assert!(record(InputScenario::new(), 3).iter().all(|s| !s.down && !s.pressed));
}
