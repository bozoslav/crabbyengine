### Added
- ECS diagnostics explain the schedule (#145). `Ctx::ecs_diagnostics()`,
  the control endpoint's `ecs.diagnostics` and the `ecs_diagnostics` MCP
  tool now list, for every system, its declared access by name
  (`declared`: components, resources, events and services read and
  written, commands), its `stage` and why it runs `alone`; and a
  `schedule` with the stages (which systems share a batch, why each stage
  starts where it does, how often a batch went to other threads or ran
  inline) and the `dependencies` between declared systems, plus
  `query_cache` with the archetype index's policy and counters.
  `EcsDiagnostics::system(name)` finds a system by name. All of it is
  worked out when asked; a running game only adds two counters per batch.
- Guide: "ECS diagnostics" documents the new fields, with a worked example
  of finding out why systems don't run together.

### Changed
- `SystemDiagnostics::access` is `declared` for a declared `&mut Ctx`
  system and `view` for a view system; undeclared and exclusive systems
  stay `exclusive`. The schema version stays 1: fields were only added.
