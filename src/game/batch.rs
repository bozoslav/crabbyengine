//! Runs the stages of a [`SchedulePlan`]: systems that run alone, and
//! batches of view systems that share the world.
//!
//! A batch runs the same way whether its systems go to other threads or
//! stay on this one: every system gets its view of the shared world, its
//! own command buffer and an outbox for its events, and all of that is
//! delivered when the batch ends, in registration order. So the choice
//! between threads and this thread only changes speed.

use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::time::Duration;

use ponko_ecs::ViewGroup;
use web_time::Instant;

use super::{ChangesSeen, Run, System, run_one};
use crate::ctx::Ctx;
use crate::events::EventSlot;
use crate::random::Random;
use crate::schedule::{DEFAULT_PARALLEL_THRESHOLD, SchedulePlan};
use crate::system_view::{Clock, Shared, ViewRunner};

/// Turns the scheduler on or off whatever the game says: `1`, `0`, or
/// `always` (on, and every batch goes to the thread pool).
const ENV_PARALLEL_SYSTEMS: &str = "PONKO_PARALLEL_SYSTEMS";

/// How the systems run, from the builder and the environment.
#[derive(Clone, Copy)]
pub(super) struct Settings {
    /// Batches exist: view systems that don't conflict share the world.
    pub(super) on: bool,
    /// A batch goes to other threads when the usual run time of its
    /// systems, without the slowest, is at least this many microseconds.
    threshold_us: f32,
    /// There are other threads to go to.
    threads: bool,
}

impl Settings {
    /// `env`: whether the environment variable may override the builder.
    pub(super) fn new(on: bool, threshold: Option<Duration>, env: bool) -> Self {
        let value = env
            .then(|| crate::environment::var(ENV_PARALLEL_SYSTEMS).ok())
            .flatten();
        let (on, threshold) = resolve(on, threshold, value.as_deref());
        Settings {
            on,
            threshold_us: threshold.as_secs_f32() * 1e6,
            // Only asked when the scheduler is on: the question starts
            // rayon's global thread pool, which a game that never turns
            // the scheduler on must not get from here. And the web has no
            // threads; asking there would set up a pool of the page's own.
            threads: on && !cfg!(target_arch = "wasm32") && rayon::current_num_threads() > 1,
        }
    }
}

/// The builder's settings after the environment variable's `value`.
pub(super) fn resolve(on: bool, threshold: Option<Duration>, value: Option<&str>) -> (bool, Duration) {
    let threshold = threshold.unwrap_or(DEFAULT_PARALLEL_THRESHOLD);
    match value {
        Some("1") => (true, threshold),
        Some("0") => (false, threshold),
        Some("always") => (true, Duration::ZERO),
        _ => (on, threshold),
    }
}

/// One stage of one system list: systems `first..first + len` of the list.
struct Stage {
    first: usize,
    len: usize,
    /// For a batch: the proof that its view states may share the world.
    group: Option<ViewGroup>,
}

/// The stages of the game's four system lists, and what running batches
/// reuses from tick to tick.
#[derive(Default)]
pub(super) struct Exec {
    /// `None` until built, and again after the systems or settings change.
    built: Option<Built>,
    /// The view systems of the batch that is running. Boxed as their
    /// systems hold them: a batch moves pointers, not runners.
    #[allow(clippy::vec_box)]
    runners: Vec<Box<ViewRunner>>,
    /// The event slots of the batch that is running. Empty between
    /// batches: only its allocation is kept.
    slots: Vec<EventSlot<'static>>,
}

struct Built {
    settings: Settings,
    /// How many systems each list had when this was built.
    lens: [usize; 4],
    /// Per list; empty when the scheduler is off.
    stages: [Vec<Stage>; 4],
}

impl Exec {
    /// The systems or the settings changed: build again before running.
    pub(super) fn invalidate(&mut self) {
        self.built = None;
    }

    pub(super) fn is_current(&self, lens: [usize; 4]) -> bool {
        self.built.as_ref().is_some_and(|b| b.lens == lens)
    }

    /// Takes the stages from `plan` (`None`: the scheduler is off, every
    /// system runs alone). `lists` are the game's system lists, in the
    /// plan's order.
    pub(super) fn build(&mut self, settings: Settings, plan: Option<&SchedulePlan>, lists: [&[System]; 4]) {
        let lens = lists.map(<[System]>::len);
        let mut stages: [Vec<Stage>; 4] = Default::default();
        if let Some(plan) = plan {
            let mut offset = 0;
            for (list, systems) in lists.into_iter().enumerate() {
                let end = offset + systems.len();
                for stage in plan.stages.iter().filter(|s| (offset..end).contains(&s.first)) {
                    let first = stage.first - offset;
                    let members = &systems[first..first + stage.len];
                    // If the states can't be shown to be disjoint (the plan
                    // says they are), the systems simply run alone.
                    let group = (stage.len > 1)
                        .then(|| ViewGroup::new(members.iter().filter_map(runner).map(|r| &r.state)).ok())
                        .flatten()
                        .filter(|g| g.len() == stage.len);
                    stages[list].push(Stage {
                        first,
                        len: stage.len,
                        group,
                    });
                }
                offset = end;
            }
        }
        self.built = Some(Built { settings, lens, stages });
    }

    /// Runs the systems of list number `list`, whose first system has
    /// timing index `base`. `tick` is the simulation tick for the list of
    /// simulation systems, whose interval systems skip ticks.
    pub(super) fn run_list(
        &mut self,
        systems: &mut [System],
        list: usize,
        ctx: &mut Ctx,
        base: usize,
        tick: Option<u64>,
    ) {
        let alone = |s: &mut System, ctx: &mut Ctx, index: usize| {
            if let Some(tick) = tick {
                if tick % u64::from(s.every) != u64::from(s.phase) {
                    return;
                }
                ctx.set_sim_steps(s.every);
            }
            run_one(s, ctx, index);
        };
        let Exec { built, runners, slots } = self;
        let stages = built
            .as_ref()
            .filter(|b| b.settings.on && b.lens[list] == systems.len());
        let Some(built) = stages else {
            for (i, s) in systems.iter_mut().enumerate() {
                alone(s, ctx, base + i);
            }
            return;
        };
        for stage in &built.stages[list] {
            let members = &mut systems[stage.first..stage.first + stage.len];
            match &stage.group {
                Some(group) => {
                    if tick.is_some() {
                        ctx.set_sim_steps(1);
                    }
                    run_batch(members, group, ctx, base + stage.first, built.settings, runners, slots);
                }
                None => {
                    for (i, s) in members.iter_mut().enumerate() {
                        alone(s, ctx, base + stage.first + i);
                    }
                }
            }
        }
    }
}

fn runner(s: &System) -> Option<&ViewRunner> {
    match &s.run {
        Run::View(Some(runner)) => Some(runner),
        _ => None,
    }
}

/// `slots` emptied, with its allocation kept for a batch that borrows
/// something else. Collecting an emptied vector of the same layout reuses
/// its buffer.
fn recycle(mut slots: Vec<EventSlot<'_>>) -> Vec<EventSlot<'static>> {
    slots.clear();
    slots
        .into_iter()
        .map(|_| -> EventSlot<'static> { unreachable!("the vector is empty") })
        .collect()
}

/// Runs the batch `systems` (all view systems, checked by `group`), whose
/// first system has timing index `base`.
#[allow(clippy::vec_box)] // see `Exec::runners`
fn run_batch(
    systems: &mut [System],
    group: &ViewGroup,
    ctx: &mut Ctx,
    base: usize,
    settings: Settings,
    runners: &mut Vec<Box<ViewRunner>>,
    slot_store: &mut Vec<EventSlot<'static>>,
) {
    ctx.sync_services();
    let clock = Clock::of(ctx);
    let tick = ctx.time.sim_tick();
    let profiling = ctx.perf.state.on;
    // Run one at a time, every system has a change tick of its own. The
    // batch keeps that: its systems write in the ticks they would have had
    // (the world moves past all but the last first), so change stamps and
    // the world's tick are the same as with the scheduler off.
    let first_tick = ctx.world.change_tick().to_bits();
    for _ in 1..systems.len() {
        ctx.world.advance_change_tick();
    }
    runners.clear();
    let mut slots: Vec<EventSlot<'_>> = std::mem::take(slot_store);
    // What the batch could save by using other threads: everything but its
    // slowest system, which this thread would run anyway.
    let (mut work_us, mut slowest_us) = (0.0f32, 0.0f32);
    for (i, s) in systems.iter_mut().enumerate() {
        let Run::View(taken) = &mut s.run else {
            unreachable!("a batch only has view systems");
        };
        let mut runner = taken.take().expect("a batch gives its systems back");
        runner.task.clock = clock;
        // The stream `Ctx::random` gives this system.
        runner.task.random = Random::new(ctx.seed, tick, base + i);
        // Each view compares changes against its own system's last run.
        runner
            .state
            .set_changes_since(Some(s.changes_seen.in_world(&ctx.world)));
        runner
            .state
            .set_change_tick(Some(crate::Tick::from_bits(first_tick + i as u64)));
        // The events it may read: what is kept now. Nothing reaches the
        // store before the batch ends.
        let first = slots.len();
        for (id, items) in runner.access().event_reads() {
            if let Some(items) = items(&ctx.events) {
                slots.push(EventSlot {
                    id,
                    items,
                    unread: ctx.events.unread(id, s.id),
                });
            }
        }
        runner.task.slots = first..slots.len();
        work_us += runner.task.usual_us;
        slowest_us = slowest_us.max(runner.task.usual_us);
        runners.push(runner);
    }
    // Other threads only when it pays, and never while profiling: the
    // allocation counter is the whole process's, so only systems that run
    // one at a time can be charged what they allocate.
    let parallel = settings.threads && !profiling && work_us - slowest_us >= settings.threshold_us;
    let shared = Shared {
        slots: &slots,
        input: &ctx.input,
        spatial: &ctx.spatial,
        count_allocs: profiling,
    };
    let world = &mut ctx.world;
    let ran = catch_unwind(AssertUnwindSafe(|| {
        group.run(
            world,
            runners,
            parallel,
            |runner| {
                let runner: &mut ViewRunner = runner;
                (&mut runner.state, &mut runner.task)
            },
            |task, view| task.run_shared(view, &shared),
        );
    }));
    *slot_store = recycle(slots);
    if let Err(panic) = ran {
        // A system panicked (every other system of the batch still ran:
        // see `ViewGroup::run`). The batch gets no barrier: what its
        // systems recorded and sent is dropped and their read positions
        // stay, on threads and in order alike, so nothing is left over for
        // a later batch to deliver. As after a system that ran alone, a
        // system that returned won't be shown its own writes again, and
        // the last system's tick ends. Then give the systems back and pass
        // the panic on; the game is whole if someone catches it.
        let world_id = ctx.world.id();
        for (i, (s, mut runner)) in systems.iter_mut().zip(runners.drain(..)).enumerate() {
            runner.task.commands.clear();
            runner.task.outbox.clear();
            runner.task.read.clear();
            if runner.task.finished {
                s.changes_seen = ChangesSeen {
                    world: world_id,
                    tick: crate::Tick::from_bits(first_tick + i as u64),
                };
            }
            s.run = Run::View(Some(runner));
        }
        ctx.world.advance_change_tick();
        resume_unwind(panic);
    }

    // The barrier. Everything below happens in registration order.
    // First every system's read positions: the store still holds exactly
    // the events the batch saw.
    for (s, runner) in systems.iter().zip(runners.iter()) {
        for &id in &runner.task.read {
            ctx.events.mark_read(id, s.id);
        }
    }
    let world_id = ctx.world.id();
    for (i, (s, mut runner)) in systems.iter_mut().zip(runners.drain(..)).enumerate() {
        let task = &mut runner.task;
        task.outbox.flush(&mut ctx.events);
        let mut ms = task.last_ms;
        // Its writes belong to the tick it wrote in: it won't be shown
        // them again, every later system will.
        s.changes_seen = ChangesSeen {
            world: world_id,
            tick: crate::Tick::from_bits(first_tick + i as u64),
        };
        if !task.commands.is_empty() {
            // Only the last system of a batch records commands, and the
            // world is in that system's tick: its commands are its own
            // changes, as when it runs alone; the batch's other systems
            // see them on their next run.
            let start = Instant::now();
            ctx.world.apply(&mut task.commands);
            ms += start.elapsed().as_secs_f32() * 1000.0;
        }
        let us = task.last_ms * 1000.0;
        task.usual_us = if task.usual_us == 0.0 {
            us
        } else {
            task.usual_us + (us - task.usual_us) * 0.1
        };
        ctx.perf.timings[base + i].last_ms += ms;
        if profiling {
            ctx.perf.state.add_system_allocs(base + i, task.allocs);
        }
        s.run = Run::View(Some(runner));
    }
    // The last system's tick ends here, as after a system that ran alone.
    ctx.world.advance_change_tick();
    ctx.perf.entities = ctx.world.len();
}
