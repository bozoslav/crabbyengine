//! Action queries: by `&str` name (a lookup per call) vs by resolved `ActionId`.
use std::hint::black_box;
use std::time::Instant;

use ponko::{Action, ActionMap, ButtonBinding, Ctx, Game, Key};

use crate::{Bench, report};

/// Queries per timed run.
const CALLS: usize = 100_000;
/// Actions installed: a game's whole map, not a toy of one.
const ACTIONS: usize = 40;

fn names() -> Vec<String> {
    (0..ACTIONS).map(|i| format!("action_{i:02}")).collect()
}

fn install(ctx: &mut Ctx, names: &[String]) {
    let mut map = ActionMap::default();
    for name in names {
        map.set(name.clone(), Action::Button(vec![ButtonBinding::Key(Key::Space)]));
    }
    ctx.input.set_actions(map);
}

/// Times `op` like [`Bench::steady`], inside a headless game (a `Ctx` can't leave it).
fn timed(b: &mut Bench, name: &str, mut op: impl FnMut(&Ctx, &[String], &[ponko::ActionId]) -> usize) {
    if b.skips(name) {
        return;
    }
    const RUNS: usize = 30;
    const WARMUP: usize = 5;
    let names = names();
    let mut times = Vec::new();
    Game::new("bench", 1, 1).run_headless(0, |ctx| {
        install(ctx, &names);
        let ids: Vec<_> = names.iter().map(|n| ctx.input.action_id(n)).collect();
        for _ in 0..WARMUP {
            black_box(op(ctx, &names, &ids));
        }
        for _ in 0..RUNS {
            let t = Instant::now();
            black_box(op(ctx, &names, &ids));
            times.push(t.elapsed());
        }
    });
    b.report(report::Record::new(name, report::Kind::Steady, WARMUP, &mut times));
}

pub fn run(b: &mut Bench) {
    // `CALLS` `down` queries per run, cycling through every action.
    timed(b, "actions/down-str-100k", |ctx, names, _| {
        let mut held = 0;
        for i in 0..CALLS {
            held += usize::from(ctx.input.action_down(names[i % ACTIONS].as_str()));
        }
        held
    });
    timed(b, "actions/down-id-100k", |ctx, _, ids| {
        let mut held = 0;
        for i in 0..CALLS {
            held += usize::from(ctx.input.action_down(ids[i % ACTIONS]));
        }
        held
    });
    // The baseline: the loop and indexing with no query at all.
    timed(b, "actions/loop-only-100k", |_, names, ids| {
        let mut held = 0;
        for i in 0..CALLS {
            held += black_box(names[i % ACTIONS].as_str()).len() + black_box(ids[i % ACTIONS]).is_none() as usize;
        }
        held
    });
}
