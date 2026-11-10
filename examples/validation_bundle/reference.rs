//! The reference interaction: hold a jump button for a long time.
//!
//! A tick-indexed input scenario presses the "jump" action at tick 10 and
//! releases it at tick 110. A correct game jumps once for that press. The
//! deliberately broken variant jumps whenever the button is *held* and the
//! body is on the ground, so it bounces again after landing.

use crate::bundle::Assertion;
use ponko::*;
use std::cell::RefCell;
use std::rc::Rc;

pub const SCENARIO_ID: &str = "hold-jump";
pub const TICKS: u64 = 120;
/// (tick, pressed) steps of the "jump" key.
pub const STEPS: [(u64, bool); 2] = [(10, true), (110, false)];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variant {
    /// Jumps on the pressed edge of the action.
    Fixed,
    /// Jumps while the action is held: the bug.
    Buggy,
}

impl Variant {
    pub fn name(self) -> &'static str {
        match self {
            Variant::Fixed => "fixed",
            Variant::Buggy => "buggy",
        }
    }
}

#[derive(Clone, Copy)]
struct Body {
    y: f32,
    vy: f32,
    jumps: u32,
}

/// What was observed on one tick, after the game systems ran.
#[derive(Clone, Copy, Debug)]
pub struct Row {
    pub tick: u64,
    pub pressed: bool,
    pub down: bool,
    pub y: f32,
    pub vy: f32,
    pub jumps: u32,
}

pub struct Run {
    pub trace: Vec<Row>,
    pub log: Vec<String>,
    pub assertions: Vec<Assertion>,
}

pub fn scenario() -> InputScenario {
    STEPS.iter().fold(InputScenario::new(), |s, &(tick, pressed)| {
        s.at(tick, InputEvent::Key(Key::Space, pressed))
    })
}

/// Runs the scenario headlessly and checks it.
pub fn run(variant: Variant, seed: u64) -> Run {
    let trace = Rc::new(RefCell::new(Vec::<Row>::new()));
    let log = Rc::new(RefCell::new(vec![format!(
        "scenario {SCENARIO_ID}: variant={} seed={seed} ticks={TICKS}",
        variant.name()
    )]));
    let (jump_log, trace_sink) = (log.clone(), trace.clone());

    Game::new("validation-reference", 1, 1)
        .seed(seed)
        .sim_system(move |ctx: &mut Ctx| {
            let dt = ctx.dt();
            let tick = ctx.time.sim_tick();
            let want_jump = match variant {
                Variant::Fixed => ctx.input.action_pressed("jump"),
                Variant::Buggy => ctx.input.action_down("jump"),
            };
            let drift = ctx.random().stream(0).range(-0.01, 0.01);
            ctx.world.query::<&mut Body>().for_each(|b| {
                if want_jump && b.y <= 0.0 {
                    b.vy = 5.0;
                    b.jumps += 1;
                    jump_log
                        .borrow_mut()
                        .push(format!("tick {tick}: jump #{} started", b.jumps));
                }
                b.vy -= 9.81 * dt;
                b.y = (b.y + (b.vy + drift) * dt).max(0.0);
            });
        })
        .sim_system(move |ctx: &mut Ctx| {
            let tick = ctx.time.sim_tick();
            let (pressed, down) = (ctx.input.action_pressed("jump"), ctx.input.action_down("jump"));
            let body = *ctx.world.query::<&Body>().iter().next().expect("the body exists");
            trace_sink.borrow_mut().push(Row {
                tick,
                pressed,
                down,
                y: body.y,
                vy: body.vy,
                jumps: body.jumps,
            });
        })
        .input_scenario(scenario())
        .setup(|ctx| {
            let mut map = ActionMap::default();
            map.set("jump", Action::Button(vec![ButtonBinding::Key(Key::Space)]));
            ctx.input.set_actions(map);
            ctx.world.spawn((Body {
                y: 0.0,
                vy: 0.0,
                jumps: 0,
            },));
        })
        .run_headless(TICKS);

    let trace = Rc::try_unwrap(trace).expect("systems dropped").into_inner();
    let mut log = Rc::try_unwrap(log).expect("systems dropped").into_inner();
    let assertions = check(&trace);
    for a in &assertions {
        log.push(format!(
            "assertion {}: {}{}",
            a.id,
            if a.passed { "passed" } else { "FAILED" },
            a.tick.map(|t| format!(" at tick {t}")).unwrap_or_default()
        ));
    }
    Run { trace, log, assertions }
}

fn check(trace: &[Row]) -> Vec<Assertion> {
    let (press_tick, release_tick) = (STEPS[0].0, STEPS[1].0);
    let press_seen = trace.iter().find(|r| r.pressed).map(|r| r.tick);
    let apex = trace.iter().map(|r| r.y).fold(0.0f32, f32::max);
    let extra_jump = trace.iter().find(|r| r.jumps > 1);
    let last = trace.last().expect("the run has ticks");
    vec![
        Assertion {
            id: "scenario-press-applied".into(),
            description: "the scenario's key press reaches the jump action on exactly its tick".into(),
            passed: press_seen == Some(press_tick),
            tick: press_seen.or(Some(press_tick)),
            expected: format!("pressed edge at tick {press_tick}"),
            actual: press_seen.map_or("no pressed edge".into(), |t| format!("pressed edge at tick {t}")),
        },
        Assertion {
            id: "jump-reaches-apex".into(),
            description: "a jump rises at least 1 unit".into(),
            passed: apex >= 1.0,
            tick: None,
            expected: "apex >= 1.0".into(),
            actual: format!("{apex:.3}"),
        },
        Assertion {
            id: "one-jump-per-press".into(),
            description: format!("holding the button from tick {press_tick} to {release_tick} makes one jump"),
            passed: extra_jump.is_none(),
            tick: extra_jump.map(|r| r.tick),
            expected: "1 jump".into(),
            actual: format!("{} jumps", extra_jump.map_or(last.jumps, |r| r.jumps)),
        },
        Assertion {
            id: "lands-by-the-end".into(),
            description: "the body is back on the ground when the run ends".into(),
            passed: last.y == 0.0,
            tick: Some(last.tick),
            expected: "y == 0".into(),
            actual: format!("{:.3}", last.y),
        },
    ]
}
