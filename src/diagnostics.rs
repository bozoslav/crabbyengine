//! ECS diagnostics for tools and agents: archetype layouts, component
//! storage and the system schedule, as stable, machine-readable data.
//!
//! Nothing here is collected while a game runs. [`Ctx::ecs_diagnostics`]
//! builds a snapshot from the world and the schedule only when asked (by
//! game code, a test, or the control endpoint's `ecs.diagnostics`).

use serde::{Deserialize, Serialize};

use crate::ctx::Ctx;
use crate::game::Phase;
use crate::perf::SystemTiming;

/// The schema version of [`EcsDiagnostics`]. Field names are stable; new
/// fields may be added without a bump. Renaming or removing one bumps it.
pub const ECS_DIAGNOSTICS_VERSION: u32 = 1;

/// A snapshot of the ECS: archetypes, component storage and the systems in
/// run order with their last timings. See [`Ctx::ecs_diagnostics`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EcsDiagnostics {
    /// [`ECS_DIAGNOSTICS_VERSION`] of the producer.
    pub version: u32,
    /// Live entities.
    pub entities: usize,
    /// Bytes allocated for component storage in all archetypes.
    pub allocated_bytes: usize,
    /// Every archetype, empty ones included, in the world's order.
    pub archetypes: Vec<ArchetypeDiagnostics>,
    /// Every registered component type.
    pub components: Vec<ComponentDiagnostics>,
    /// Every system, in the order the schedule runs them.
    pub systems: Vec<SystemDiagnostics>,
}

/// One archetype: entities sharing exactly one set of component types.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ArchetypeDiagnostics {
    /// The archetype's index in the world (stable while the world lives).
    pub index: usize,
    /// Full type names of its components, in storage order.
    pub components: Vec<String>,
    /// Entities stored in it now.
    pub entities: usize,
    /// Bytes per entity: every component plus the entity handle.
    pub bytes_per_entity: usize,
    /// Bytes its component columns have allocated (capacity, not length).
    pub allocated_bytes: usize,
}

/// One component type and how much of it is stored.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ComponentDiagnostics {
    /// Full type name (`my_game::units::Health`), or the registered name.
    pub name: String,
    /// The last path segment (`Health`), as the control endpoint's other
    /// methods name components.
    pub short_name: String,
    /// Size of one value in bytes.
    pub size: usize,
    /// Alignment in bytes.
    pub align: usize,
    /// Archetypes that contain it.
    pub archetypes: usize,
    /// Entities that have it.
    pub entities: usize,
    /// `size * entities`: bytes of live values (excluding spare capacity).
    pub bytes: usize,
}

/// One registered system.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SystemDiagnostics {
    /// The system's function name (`my_game::movement`).
    pub name: String,
    /// Its [`Phase`], spelled like the variant (`AfterPhysics`).
    pub phase: String,
    /// Position in the schedule: systems run in increasing `order`
    /// (simulation systems once per tick, the rest once per frame).
    pub order: usize,
    /// `true` for fixed-tick simulation systems.
    pub sim: bool,
    /// Simulation systems run on ticks where `tick % every == offset`.
    pub every: u32,
    /// See `every`.
    pub offset: u32,
    /// What the system may touch. Today always `exclusive`: every system
    /// gets `&mut Ctx`, runs alone, and its deferred commands are applied
    /// right after it (a barrier after every system).
    pub access: String,
    /// Time in the last frame (all of its ticks together), in ms.
    pub last_ms: f32,
    /// Smoothed average of `last_ms`.
    pub avg_ms: f32,
}

/// How many archetypes and entities have a set of components.
/// See [`EcsDiagnostics::matching`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MatchCount {
    /// Non-empty archetypes that have every named component.
    pub archetypes: usize,
    /// Entities in them.
    pub entities: usize,
}

impl EcsDiagnostics {
    /// How many archetypes and entities have all of `components`, named by
    /// full or short name: what a query with those components would visit.
    /// An unknown name is an error that lists the known ones.
    ///
    /// ```
    /// use ponko::prelude::*;
    /// struct Hp(u32);
    /// fn check(ctx: &mut Ctx) {
    ///     let m = ctx.ecs_diagnostics().matching(&["Hp"]).unwrap();
    ///     assert_eq!((m.archetypes, m.entities), (1, 1));
    /// }
    /// Game::new("t", 1, 1).sim_system(check).run_headless(1, |ctx| {
    ///     ctx.world.spawn((Hp(3),));
    /// });
    /// ```
    pub fn matching(&self, components: &[&str]) -> Result<MatchCount, String> {
        let mut full: Vec<&str> = Vec::with_capacity(components.len());
        for &wanted in components {
            let found = self
                .components
                .iter()
                .find(|c| c.name == wanted)
                .or_else(|| self.components.iter().find(|c| c.short_name == wanted));
            match found {
                Some(c) => full.push(&c.name),
                None => {
                    let known: Vec<&str> = self.components.iter().map(|c| c.short_name.as_str()).collect();
                    return Err(format!("unknown component `{wanted}` (known: {})", known.join(", ")));
                }
            }
        }
        let mut count = MatchCount::default();
        for a in self.archetypes.iter().filter(|a| a.entities > 0) {
            if full.iter().all(|n| a.components.iter().any(|c| c == n)) {
                count.archetypes += 1;
                count.entities += a.entities;
            }
        }
        Ok(count)
    }
}

/// Where a system sits in the schedule, recorded when the game lists its
/// systems (parallel to [`Perf::system_timings`]).
#[derive(Debug, Clone, Copy)]
pub(crate) struct ScheduleSlot {
    pub phase: Phase,
    pub every: u32,
    pub offset: u32,
}

/// `Phase::AfterPhysics` → `"AfterPhysics"`, a stable spelling for tools.
pub(crate) fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Input => "Input",
        Phase::BeforePhysics => "BeforePhysics",
        Phase::Physics => "Physics",
        Phase::AfterPhysics => "AfterPhysics",
        Phase::SimLast => "SimLast",
        Phase::Frame => "Frame",
        Phase::Animate => "Animate",
        Phase::Late => "Late",
        Phase::Draw => "Draw",
    }
}

/// `crate::units::Health` → `Health`.
pub(crate) fn short_name(type_name: &str) -> &str {
    let end = type_name.find('<').unwrap_or(type_name.len());
    type_name[..end].rfind("::").map_or(type_name, |i| &type_name[i + 2..])
}

pub(crate) fn snapshot(ctx: &Ctx) -> EcsDiagnostics {
    let world = &ctx.world;
    let infos = world.components();
    let mut components: Vec<ComponentDiagnostics> = infos
        .iter()
        .map(|info| ComponentDiagnostics {
            name: info.name().to_string(),
            short_name: short_name(info.name()).to_string(),
            size: info.layout().size(),
            align: info.layout().align(),
            ..ComponentDiagnostics::default()
        })
        .collect();
    let archetypes = world
        .raw_ref()
        .archetypes()
        .iter()
        .enumerate()
        .map(|(index, a)| {
            for id in a.component_ids() {
                let c = &mut components[id.index()];
                c.archetypes += 1;
                c.entities += a.len();
            }
            ArchetypeDiagnostics {
                index,
                components: a
                    .component_ids()
                    .iter()
                    .map(|&id| world.component_info(id).name().to_string())
                    .collect(),
                entities: a.len(),
                bytes_per_entity: a.bytes_per_entity(),
                allocated_bytes: a.allocated_bytes(),
            }
        })
        .collect();
    for c in &mut components {
        c.bytes = c.size * c.entities;
    }
    EcsDiagnostics {
        version: ECS_DIAGNOSTICS_VERSION,
        entities: world.len(),
        allocated_bytes: world.allocated_bytes(),
        archetypes,
        components,
        systems: systems(&ctx.perf.timings, &ctx.perf.schedule),
    }
}

fn systems(timings: &[SystemTiming], schedule: &[ScheduleSlot]) -> Vec<SystemDiagnostics> {
    timings
        .iter()
        .enumerate()
        .map(|(order, t)| {
            // A game that hasn't listed its systems yet (or a bare `Ctx`)
            // has timings without slots; report the compatibility default.
            let slot = schedule.get(order).copied().unwrap_or(ScheduleSlot {
                phase: if t.sim { Phase::AfterPhysics } else { Phase::Frame },
                every: 1,
                offset: 0,
            });
            SystemDiagnostics {
                name: t.name.to_string(),
                phase: phase_name(slot.phase).to_string(),
                order,
                sim: t.sim,
                every: slot.every,
                offset: slot.offset,
                access: "exclusive".to_string(),
                last_ms: t.last_ms,
                avg_ms: t.avg_ms,
            }
        })
        .collect()
}
