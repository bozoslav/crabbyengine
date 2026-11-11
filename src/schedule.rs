//! The schedule of a game's systems: which run alone and which share a
//! batch, and why. From [`Game::schedule_plan`](crate::Game::schedule_plan).
//!
//! Systems run phase after phase, and inside a phase in registration order.
//! The plan cuts that order into **stages**: a stage is either one system
//! that runs alone, or a **batch** of view systems
//! ([`Game::add_view`](crate::Game::add_view)) that are next to each other
//! in the order and whose declarations don't conflict
//! ([`SystemAccess::conflicts_before`](crate::access::SystemAccess::conflicts_before)).
//! Stages run one after another; with
//! [`Game::parallel_systems`](crate::Game::parallel_systems) on, the systems
//! of a batch may run at the same time. A system never moves past another:
//! two systems that conflict keep their registration order, because they are
//! in different stages.
//!
//! # Barriers
//!
//! The end of every stage is a barrier. There, and only there:
//!
//! - the **commands** recorded in the stage are applied, in registration
//!   order;
//! - the **events** sent in the stage become visible to other systems, in
//!   registration order;
//! - a new change tick starts, so later systems' `Added` and `Changed`
//!   filters see the stage's writes.
//!
//! A system that runs alone is a stage of its own, so this is the contract
//! systems always had: commands apply and events are visible right after
//! each system. For a batch, the conflict rules make sure that no system in
//! it could have seen the difference: a system that sends an event type
//! never shares a batch with a later reader of it, and a system that
//! records commands is always the last of its batch.
//!
//! A new stage starts ([`Boundary`]):
//!
//! - at every [`Phase`],
//! - before and after every system that runs alone ([`Alone`]): every
//!   `&mut Ctx` system, and view systems that hold a service only one
//!   system may hold, send and read the same event type, or run at an
//!   interval,
//! - where a system conflicts with one already in the batch,
//! - at an explicit [`Game::barrier`](crate::Game::barrier),
//! - where [`Game::order`](crate::Game::order) says one system must see
//!   another's work.
//!
//! # Determinism
//!
//! The plan depends only on the registered systems and their declarations.
//! Whether a batch really runs on several threads (it doesn't when its
//! systems are too cheap to be worth it, while profiling, on the web, or on
//! a machine with one core) changes nothing a system can observe: in both
//! cases the batch's systems share the world through their views, get their
//! own command buffer and event outbox, and those are delivered at the
//! barrier in registration order. Each system's random stream depends on
//! the seed, the tick and the system, as always.
//!
//! Change ticks are the same too. Run one at a time, every system has a
//! change tick of its own; the systems of a batch write in exactly those
//! ticks (the world moves past them before the batch starts), so
//! `change_tick()` reads the same number in a system, change stamps
//! compare the same against a tick a system kept, and the world's tick
//! after any stage is the same with the scheduler on or off.
//!
//! # Panics
//!
//! If a system of a batch panics, the other systems of the batch still
//! run to their end: on threads they may be running already, so the batch
//! does the same in order. Then the panic is passed on; if several systems
//! panicked, the one registered first. A batch that ends this way has no
//! barrier: the commands its systems recorded and the events they sent are
//! dropped, and the events they read stay unread. What the systems wrote
//! to components and resources stays, as it does when a system that runs
//! alone panics halfway. Nothing is kept for a later batch to deliver, and
//! all of this is the same on threads and in order. (With the scheduler
//! off, systems after the panicking one never start: that difference
//! can't be avoided once systems run at the same time.) A game that
//! catches the panic and goes on runs every system again normally: as
//! after a run alone, a system that returned isn't shown its own writes of
//! that batch as changes, and the batch's change ticks have ended. The
//! system that panicked is shown its own writes once more, as it is when
//! it panics running alone.
//!
//! ```
//! use ponko::access::SystemAccess;
//! use ponko::prelude::*;
//! use ponko::system_view::SystemView;
//!
//! struct Pos(f32);
//! struct Vel(f32);
//! struct Health(i32);
//!
//! fn movement(sys: &mut SystemView) {
//!     sys.world.query::<(&mut Pos, &Vel)>().for_each(|(p, v)| p.0 += v.0);
//! }
//! fn regenerate(sys: &mut SystemView) {
//!     sys.world.query::<&mut Health>().for_each(|h| h.0 += 1);
//! }
//! fn stop(sys: &mut SystemView) {
//!     sys.world.query::<&mut Vel>().for_each(|v| v.0 = 0.0);
//! }
//!
//! let game = Game::new("plan", 1, 1)
//!     .parallel_systems(true)
//!     .add_view(Phase::AfterPhysics, SystemAccess::new().write::<Pos>().read::<Vel>(), movement)
//!     .add_view(Phase::AfterPhysics, SystemAccess::new().write::<Health>(), regenerate)
//!     .add_view(Phase::AfterPhysics, SystemAccess::new().write::<Vel>(), stop);
//! let plan = game.schedule_plan();
//! // `movement` and `regenerate` share a batch; `stop` writes what
//! // `movement` reads, so it waits for the batch to end.
//! let stages = plan.stages_in(Phase::AfterPhysics);
//! assert_eq!((stages[0].len, stages[1].len), (2, 1));
//! ```

use std::fmt;

use crate::Phase;
use crate::access::{AccessConflict, Service, SystemAccess, Unbatchable};
use crate::game::SystemId;

/// How much work a batch needs before it goes to other threads, unless
/// [`Game::parallel_threshold`](crate::Game::parallel_threshold) says
/// otherwise: the usual run time of its systems, without the slowest one.
/// From the `schedule/` benchmarks (see the guide's "Parallel systems").
pub const DEFAULT_PARALLEL_THRESHOLD: std::time::Duration = std::time::Duration::from_micros(50);

/// Why a system runs alone, as a stage of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Alone {
    /// It has no declaration: it may use anything.
    Undeclared,
    /// It is declared but takes `&mut Ctx`
    /// ([`Game::add_with_access`](crate::Game::add_with_access), C and C++
    /// systems): its declaration is only checked in debug builds, so it
    /// isn't trusted to share the world.
    CtxSystem,
    /// A view system that declares this service, which only one system at a
    /// time may hold (sound, the cameras, the physics worlds, networking).
    Service(Service),
    /// A view system that sends and reads this event type: alone, it sees
    /// its own events at once.
    SendsAndReads(&'static str),
    /// A view system that runs at an interval (not every tick): a batch
    /// runs as a whole, so a system that skips ticks stays out of it.
    Interval,
}

impl fmt::Display for Alone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Alone::Undeclared => f.write_str("undeclared"),
            Alone::CtxSystem => f.write_str("takes `&mut Ctx`"),
            Alone::Service(s) => write!(f, "holds service `{s}`"),
            Alone::SendsAndReads(n) => write!(f, "sends and reads events `{n}`"),
            Alone::Interval => f.write_str("runs at an interval"),
        }
    }
}

/// One system of a [`SchedulePlan`].
#[derive(Clone, Debug)]
pub struct PlannedSystem {
    /// The system's name (its function path).
    pub name: &'static str,
    /// The phase it runs in.
    pub phase: Phase,
    /// Why it runs alone; `None` for a view system that can share a batch.
    pub alone: Option<Alone>,
}

/// Why a stage starts where it does: what keeps its first system out of
/// the stage before it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Boundary {
    /// It is the first stage of its phase.
    Phase,
    /// Its system runs alone, or the system before it does.
    Alone,
    /// Its first system conflicts with system `with` (an index into
    /// [`SchedulePlan::systems`]) of the batch before it.
    Conflict {
        /// The earlier system.
        with: usize,
        /// What they conflict on.
        conflicts: Vec<AccessConflict>,
    },
    /// An explicit [`Game::barrier`](crate::Game::barrier).
    Barrier,
    /// [`Game::order`](crate::Game::order) puts its first system after
    /// system `after` of the batch before it.
    Order {
        /// The earlier system.
        after: usize,
    },
}

/// One stage of a [`SchedulePlan`]: systems `first..first + len` of
/// [`SchedulePlan::systems`]. More than one system is a batch.
#[derive(Clone, Debug)]
pub struct Stage {
    /// The phase of its systems.
    pub phase: Phase,
    /// Index of its first system.
    pub first: usize,
    /// How many systems it has.
    pub len: usize,
    /// Why it starts here.
    pub boundary: Boundary,
}

impl Stage {
    /// The indexes of its systems in [`SchedulePlan::systems`].
    #[must_use]
    pub fn systems(&self) -> std::ops::Range<usize> {
        self.first..self.first + self.len
    }

    /// `true` if more than one system shares the stage.
    #[must_use]
    pub fn is_batch(&self) -> bool {
        self.len > 1
    }
}

/// Which systems run alone and which share a batch: from
/// [`Game::schedule_plan`](crate::Game::schedule_plan). See the
/// [module documentation](self) for the rules.
///
/// The plan is the same whether or not
/// [`Game::parallel_systems`](crate::Game::parallel_systems) is on: it shows
/// what turning it on would run together. Printing it (`{plan}`) lists every
/// stage with the reason it starts there.
#[derive(Clone, Debug, Default)]
pub struct SchedulePlan {
    /// Every registered system, in run order.
    pub systems: Vec<PlannedSystem>,
    /// The stages, in run order; together they cover every system once.
    pub stages: Vec<Stage>,
}

impl SchedulePlan {
    /// The stages of `phase`, in run order.
    #[must_use]
    pub fn stages_in(&self, phase: Phase) -> Vec<&Stage> {
        self.stages.iter().filter(|s| s.phase == phase).collect()
    }

    /// The stages with more than one system.
    pub fn batches(&self) -> impl Iterator<Item = &Stage> + '_ {
        self.stages.iter().filter(|s| s.is_batch())
    }

    /// The names of `stage`'s systems, in registration order.
    #[must_use]
    pub fn names(&self, stage: &Stage) -> Vec<&'static str> {
        self.systems[stage.systems()].iter().map(|s| s.name).collect()
    }

    /// The index of the stage that runs the system named `name` (the first
    /// one, if several systems share the name).
    #[must_use]
    pub fn stage_of(&self, name: &str) -> Option<usize> {
        let system = self.systems.iter().position(|s| s.name == name)?;
        self.stages.iter().position(|s| s.systems().contains(&system))
    }
}

impl fmt::Display for SchedulePlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut phase = None;
        for stage in &self.stages {
            if phase != Some(stage.phase) {
                phase = Some(stage.phase);
                writeln!(f, "{:?}", stage.phase)?;
            }
            let names = self.names(stage).join(" + ");
            let first = &self.systems[stage.first];
            let kind = match (first.alone, stage.is_batch()) {
                (Some(why), _) => format!("alone: {why}"),
                (None, true) => "batch".into(),
                (None, false) => "batch of one".into(),
            };
            let name = |i: usize| self.systems[i].name;
            let after = match &stage.boundary {
                Boundary::Phase | Boundary::Alone => String::new(),
                Boundary::Conflict { with, conflicts } => {
                    let why: Vec<String> = conflicts.iter().map(ToString::to_string).collect();
                    format!("; after `{}`: conflict on {}", name(*with), why.join(", "))
                }
                Boundary::Barrier => "; after a barrier".into(),
                Boundary::Order { after } => format!("; ordered after `{}`", name(*after)),
            };
            writeln!(f, "  {names} ({kind}{after})")?;
        }
        Ok(())
    }
}

/// One registered system, as the planner sees it.
pub(crate) struct PlanInput<'a> {
    pub(crate) name: &'static str,
    pub(crate) phase: Phase,
    /// Which of the game's system lists it is in: stages don't cross lists.
    pub(crate) list: u8,
    pub(crate) id: SystemId,
    pub(crate) access: Option<&'a SystemAccess>,
    /// A view system: it can't reach anything undeclared.
    pub(crate) view: bool,
    /// It runs at an interval, not every tick.
    pub(crate) interval: bool,
    /// An explicit barrier was placed before it.
    pub(crate) barrier: bool,
}

fn why_alone(s: &PlanInput<'_>) -> Option<Alone> {
    let Some(access) = s.access else {
        return Some(Alone::Undeclared);
    };
    if access.is_exclusive() {
        return Some(Alone::Undeclared);
    }
    if !s.view {
        return Some(Alone::CtxSystem);
    }
    if s.interval {
        return Some(Alone::Interval);
    }
    access.batch_problem().map(|p| match p {
        Unbatchable::Service(s) => Alone::Service(s),
        Unbatchable::SendsAndReads(n) => Alone::SendsAndReads(n),
    })
}

/// What is wrong with `order`, given the systems' ids and names in run
/// order: the first pair that names two registered systems the wrong way
/// round. Checked whether the scheduler is on or not, so that turning it
/// on can't be what makes a game panic. Allocates nothing unless there is
/// a problem.
pub(crate) fn order_problem(
    systems: impl Iterator<Item = (SystemId, &'static str)> + Clone,
    order: &[(SystemId, SystemId)],
) -> Option<String> {
    order.iter().find_map(|&(first, second)| {
        let find = |id: SystemId| systems.clone().enumerate().find(|(_, s)| s.0 == id);
        let ((a, (_, first)), (b, (_, second))) = (find(first)?, find(second)?);
        (a >= b).then(|| {
            format!(
                "Game::order: `{second}` already runs before `{first}` (an earlier phase, or registered first in \
                 the same phase); order can keep two systems apart, not swap them"
            )
        })
    })
}

/// Cuts `systems` (in run order) into stages. `order` lists pairs of
/// systems where the second must see the first's work.
///
/// # Panics
/// If a pair in `order` names two registered systems the wrong way round.
#[track_caller]
pub(crate) fn plan(systems: &[PlanInput<'_>], order: &[(SystemId, SystemId)]) -> SchedulePlan {
    if let Some(problem) = order_problem(systems.iter().map(|s| (s.id, s.name)), order) {
        panic!("{problem}");
    }
    let mut out = SchedulePlan::default();
    // Whether the last stage is a batch that the next system may join.
    let mut open = false;
    for (i, s) in systems.iter().enumerate() {
        let alone = why_alone(s);
        out.systems.push(PlannedSystem {
            name: s.name,
            phase: s.phase,
            alone,
        });
        let new_phase = i == 0 || systems[i - 1].phase != s.phase || systems[i - 1].list != s.list;
        let boundary = if new_phase {
            Some(Boundary::Phase)
        } else if s.barrier {
            Some(Boundary::Barrier)
        } else if alone.is_some() || !open {
            Some(Boundary::Alone)
        } else {
            let stage = out.stages.last().expect("an open batch");
            let access = s.access.expect("a batchable system is declared");
            stage.systems().find_map(|m| {
                if order.contains(&(systems[m].id, s.id)) {
                    return Some(Boundary::Order { after: m });
                }
                let earlier = systems[m].access.expect("a batchable system is declared");
                let conflicts = earlier.conflicts_before(access);
                (!conflicts.is_empty()).then_some(Boundary::Conflict { with: m, conflicts })
            })
        };
        match boundary {
            Some(boundary) => out.stages.push(Stage {
                phase: s.phase,
                first: i,
                len: 1,
                boundary,
            }),
            None => out.stages.last_mut().expect("an open batch").len += 1,
        }
        open = alone.is_none();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    struct A;
    struct B;
    struct C;
    struct Hit;

    /// A view system in `AfterPhysics`.
    fn view<'a>(name: &'static str, access: &'a SystemAccess) -> PlanInput<'a> {
        PlanInput {
            name,
            phase: Phase::AfterPhysics,
            list: 0,
            id: SystemId::from_bits(name.as_bytes()[0].into()),
            access: Some(access),
            view: true,
            interval: false,
            barrier: false,
        }
    }

    fn stages(plan: &SchedulePlan) -> Vec<String> {
        plan.stages.iter().map(|s| plan.names(s).join("+")).collect()
    }

    #[test]
    fn independent_view_systems_share_one_batch() {
        let (a, b, c) = (
            SystemAccess::new().write::<A>(),
            SystemAccess::new().write::<B>(),
            SystemAccess::new().write::<C>(),
        );
        let plan = plan(&[view("a", &a), view("b", &b), view("c", &c)], &[]);
        assert_eq!(stages(&plan), ["a+b+c"]);
        assert_eq!(plan.stages[0].boundary, Boundary::Phase);
        assert_eq!(plan.batches().count(), 1);
    }

    #[test]
    fn a_chain_of_conflicts_is_one_stage_per_system() {
        // Each reads what the one before writes.
        let (a, b, c) = (
            SystemAccess::new().write::<A>(),
            SystemAccess::new().read::<A>().write::<B>(),
            SystemAccess::new().read::<B>().write::<C>(),
        );
        let plan = plan(&[view("a", &a), view("b", &b), view("c", &c)], &[]);
        assert_eq!(stages(&plan), ["a", "b", "c"]);
        assert_eq!(
            plan.stages[1].boundary,
            Boundary::Conflict {
                with: 0,
                conflicts: vec![AccessConflict::Component(std::any::type_name::<A>())]
            }
        );
        assert_eq!(plan.batches().count(), 0);
    }

    #[test]
    fn readers_fan_out_after_their_writer_and_join_again_before_the_next() {
        let w = SystemAccess::new().write::<A>();
        let r = SystemAccess::new().read::<A>();
        let rb = SystemAccess::new().read::<A>().write::<B>();
        let end = SystemAccess::new().write::<A>().read::<B>();
        let systems = [
            view("w", &w),
            view("x", &r),
            view("y", &rb),
            view("z", &r),
            view("e", &end),
        ];
        let plan = plan(&systems, &[]);
        assert_eq!(stages(&plan), ["w", "x+y+z", "e"]);
        // `e` is kept out by the first batch member it conflicts with.
        assert!(matches!(plan.stages[2].boundary, Boundary::Conflict { with: 1, .. }));
    }

    #[test]
    fn a_system_is_checked_against_every_member_of_the_batch() {
        // `c` is fine next to `b` but not next to `a`.
        let (a, b, c) = (
            SystemAccess::new().write::<A>(),
            SystemAccess::new().write::<B>(),
            SystemAccess::new().read::<A>(),
        );
        let plan = plan(&[view("a", &a), view("b", &b), view("c", &c)], &[]);
        assert_eq!(stages(&plan), ["a+b", "c"]);
    }

    #[test]
    fn systems_that_run_alone_cut_the_batches_around_them() {
        let (a, b) = (SystemAccess::new().write::<A>(), SystemAccess::new().write::<B>());
        let ctx_declared = SystemAccess::new().write::<C>();
        let mut undeclared = view("u", &a);
        undeclared.access = None;
        let mut ctx_system = view("d", &ctx_declared);
        ctx_system.view = false;
        let systems = [view("a", &a), undeclared, view("b", &b), ctx_system, view("c", &a)];
        let plan = plan(&systems, &[]);
        assert_eq!(stages(&plan), ["a", "u", "b", "d", "c"]);
        let alone: Vec<_> = plan.systems.iter().map(|s| s.alone).collect();
        assert_eq!(
            alone,
            [None, Some(Alone::Undeclared), None, Some(Alone::CtxSystem), None]
        );
        assert!(plan.stages[1..].iter().all(|s| s.boundary == Boundary::Alone));
    }

    #[test]
    fn exclusive_services_and_self_read_events_run_alone() {
        let audio = SystemAccess::new().write_service(Service::Audio);
        let camera = SystemAccess::new().read_service(Service::Camera);
        let echo = SystemAccess::new().send_events::<Hit>().read_events::<Hit>();
        let shared = SystemAccess::new()
            .read_service(Service::Input)
            .read_service(Service::Time)
            .read_service(Service::Random)
            .read_service(Service::Spatial);
        let systems = [
            view("a", &audio),
            view("c", &camera),
            view("e", &echo),
            view("s", &shared),
            view("t", &shared),
        ];
        let plan = plan(&systems, &[]);
        assert_eq!(stages(&plan), ["a", "c", "e", "s+t"]);
        assert_eq!(plan.systems[0].alone, Some(Alone::Service(Service::Audio)));
        assert_eq!(plan.systems[1].alone, Some(Alone::Service(Service::Camera)));
        assert_eq!(
            plan.systems[2].alone,
            Some(Alone::SendsAndReads(std::any::type_name::<Hit>()))
        );
    }

    #[test]
    fn events_and_commands_follow_the_ordered_rules() {
        let send = SystemAccess::new().send_events::<Hit>();
        let read = SystemAccess::new().read_events::<Hit>();
        let record = SystemAccess::new().write::<A>().commands();
        let other = SystemAccess::new().write::<B>();
        // Reader first, two senders: one batch. A later reader waits.
        let systems = [view("r", &read), view("s", &send), view("t", &send), view("u", &read)];
        assert_eq!(stages(&plan(&systems, &[])), ["r+s+t", "u"]);
        // A recorder is the last of its batch.
        let third = SystemAccess::new().write::<C>();
        let systems = [view("o", &other), view("c", &record), view("p", &third)];
        let p = plan(&systems, &[]);
        assert_eq!(stages(&p), ["o+c", "p"]);
        assert!(matches!(
            &p.stages[1].boundary,
            Boundary::Conflict { with: 1, conflicts } if conflicts.contains(&AccessConflict::Commands)
        ));
    }

    #[test]
    fn an_explicit_barrier_starts_a_new_stage() {
        let (a, b, c) = (
            SystemAccess::new().write::<A>(),
            SystemAccess::new().write::<B>(),
            SystemAccess::new().write::<C>(),
        );
        let mut after_barrier = view("b", &b);
        after_barrier.barrier = true;
        let plan = plan(&[view("a", &a), after_barrier, view("c", &c)], &[]);
        assert_eq!(stages(&plan), ["a", "b+c"]);
        assert_eq!(plan.stages[1].boundary, Boundary::Barrier);
    }

    #[test]
    fn explicit_order_keeps_two_compatible_systems_apart() {
        let (a, b, c) = (
            SystemAccess::new().write::<A>(),
            SystemAccess::new().write::<B>(),
            SystemAccess::new().write::<C>(),
        );
        let systems = [view("a", &a), view("b", &b), view("c", &c)];
        let order = [(systems[0].id, systems[2].id)];
        let plan = plan(&systems, &order);
        assert_eq!(stages(&plan), ["a+b", "c"]);
        assert_eq!(plan.stages[1].boundary, Boundary::Order { after: 0 });
    }

    #[test]
    #[should_panic(expected = "`a` already runs before `c`")]
    fn explicit_order_against_the_run_order_panics() {
        let (a, c) = (SystemAccess::new().write::<A>(), SystemAccess::new().write::<C>());
        let systems = [view("a", &a), view("c", &c)];
        let _ = plan(&systems, &[(systems[1].id, systems[0].id)]);
    }

    #[test]
    fn phases_and_lists_are_barriers() {
        let (a, b) = (SystemAccess::new().write::<A>(), SystemAccess::new().write::<B>());
        let mut later = view("b", &b);
        later.phase = Phase::SimLast;
        let mut other_list = view("c", &a);
        other_list.phase = Phase::SimLast;
        other_list.list = 1;
        let plan = plan(&[view("a", &a), later, other_list], &[]);
        assert_eq!(stages(&plan), ["a", "b", "c"]);
        assert!(plan.stages.iter().all(|s| s.boundary == Boundary::Phase));
        assert_eq!(plan.stages_in(Phase::SimLast).len(), 2);
    }

    #[test]
    fn a_view_system_with_an_interval_runs_alone_and_says_why() {
        let (a, b, c) = (
            SystemAccess::new().write::<A>(),
            SystemAccess::new().write::<B>(),
            SystemAccess::new().write::<C>(),
        );
        let every_other_tick = PlanInput {
            interval: true,
            ..view("b", &b)
        };
        let plan = plan(&[view("a", &a), every_other_tick, view("c", &c)], &[]);
        assert_eq!(stages(&plan), ["a", "b", "c"]);
        assert_eq!(plan.systems[1].alone, Some(Alone::Interval));
        // Not "takes `&mut Ctx`": it is a view system.
        assert!(plan.to_string().contains("b (alone: runs at an interval)"), "{plan}");
    }

    #[test]
    fn a_wrong_order_is_found_without_a_plan() {
        let ids = |names: &[&'static str]| -> Vec<(SystemId, &'static str)> {
            names
                .iter()
                .map(|&n| (SystemId::from_bits(n.as_bytes()[0].into()), n))
                .collect()
        };
        let systems = ids(&["a", "b", "c"]);
        let id = |i: usize| systems[i].0;
        let check = |order: &[(SystemId, SystemId)]| order_problem(systems.iter().copied(), order);
        assert_eq!(check(&[]), None);
        assert_eq!(check(&[(id(0), id(1)), (id(1), id(2))]), None);
        let problem = check(&[(id(0), id(1)), (id(2), id(0))]).unwrap();
        assert!(problem.contains("`a` already runs before `c`"), "{problem}");
        // A system can't be ordered after itself either.
        assert!(check(&[(id(1), id(1))]).is_some());
        // A pair that names a removed system is ignored.
        assert_eq!(check(&[(id(2), SystemId::from_bits(9999))]), None);
    }

    #[test]
    fn the_plan_prints_every_stage_with_its_reason() {
        let (a, b, c) = (
            SystemAccess::new().write::<A>(),
            SystemAccess::new().write::<B>(),
            SystemAccess::new().read::<A>(),
        );
        let mut alone = view("u", &a);
        alone.access = None;
        let text = plan(&[view("a", &a), view("b", &b), view("c", &c), alone], &[]).to_string();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "AfterPhysics");
        assert_eq!(lines[1], "  a + b (batch)");
        assert!(lines[2].starts_with("  c (batch of one; after `a`: conflict on component"));
        assert_eq!(lines[3], "  u (alone: undeclared)");
    }
}
