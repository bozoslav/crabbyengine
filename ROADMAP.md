# Ponko roadmap

Ponko aims to support complete 2D and 3D games through an integrated, high-performance, code-first engine with useful AI development tools. The direction is broad capability with low incidental complexity, preserving the custom archetype ECS, wgpu renderer, headless simulation and Rust/C++ workflows. See [PROJECT_VISION.md](PROJECT_VISION.md) and the current [ARCHITECTURE.md](ARCHITECTURE.md).

This roadmap is directional. Ordering changes with correctness evidence, measurements and real developer needs; it assigns no release dates. An issue describes intended work, not a delivery promise. P0 establishes foundations before substantial architectural changes; P1 addresses high-impact limitations; P2 follows foundations where useful; P3 is research that may conclude with rejection.

## Current capability inventory

**Implemented** means the mechanism exists in source, not that every use case is production proven. **Partial** means useful building blocks exist with important integration or coverage gaps. **Missing** means the named workflow was not found in the inspected implementation. **Requires verification** marks claims needing platform/runtime evidence beyond source inspection. This documentation change did not execute runtime or GPU validation.

| Complete-game area | Status | Evidence and remaining work |
|---|---|---|
| ECS and fixed-step simulation | Implemented | [ECS](crates/crabby_ecs/src), [game loop](src/game.rs), [determinism tests](tests/determinism.rs). Systems remain sequential; optional cross-system scheduling is planned. |
| World, scene and prefab management | Implemented | [scene.rs](src/scene.rs), [reflection](src/reflect.rs), [scene tests](tests/scenes.rs). Advanced world composition and complete service-state restoration need use-case validation. |
| Asset importing and processing | Partial | [glTF](src/three/gltf.rs), [Unity tooling](crates/crabby_unity), [assets](src/assets.rs). Import support varies by format and feature; it is not source-engine runtime compatibility. |
| Asset streaming and residency | Partial | [Job/CellStreamer](src/stream.rs), [stream tests](tests/stream.rs). Automatic budgeted mip/mesh residency is missing; [#154](https://github.com/ponkoengine/ponko/issues/154). |
| Meshes, materials, animation and skinning | Implemented | [mesh](src/three/mesh.rs), [clips](src/three/clip.rs), [animator](src/three/animator.rs), [skin](src/three/skin.rs), [morph tests](tests/morph3d.rs). Verify particular imported animation workflows; presence is not arbitrary asset parity. |
| Terrain, vegetation, LOD and large worlds | Partial | [terrain](src/three/terrain.rs), [far terrain](src/three/far_terrain.rs), [instances](src/three/instances.rs), [LOD](src/three/lod.rs), [streaming example](examples/streaming.rs). Budgeted residency and broader incremental extraction remain work. |
| Characters, collision, physics and navigation | Implemented | [physics3d](src/physics3d), [navigation](src/nav3d), [vehicle tests](tests/vehicle3d.rs), [navigation tests](tests/navmesh3d.rs). Features are optional; real game integration and target performance still need evidence. |
| UI, text, actions and gamepads | Implemented | [retained UI](src/ui), [text](src/text.rs), [actions](src/actions.rs), [gamepads](src/gamepad.rs), [UI tests](tests/ui_retained.rs). Input backend and native API coverage vary by target. |
| Accessibility | Requires verification | UI and input building blocks exist; no comprehensive assistive-technology, navigation or accessibility conformance claim is established here. Audit with a playable reference workflow before defining additions. |
| Audio and spatial audio | Implemented | [audio](src/audio.rs), [audio3d example](examples/audio3d.rs), [audio tests](tests/audio.rs). Live output/device behavior requires runtime checks. |
| Multiplayer, synchronization and servers | Partial | [lockstep](src/net.rs), [snapshots](src/net/snapshots.rs), [messages](src/net/messages.rs), [replication tests](tests/net_replication.rs), headless runner. Live transport/relay behavior and a complete deployment/authority workflow require verification. |
| Save/load and serialization | Implemented | [reflection](src/reflect.rs), [scenes](src/scene.rs), [stability contract](STABILITY.md). Applications own custom component migrations and external service persistence. |
| GPU compute and custom graphics | Partial | [compute](src/three/compute.rs), [effects](src/three/effect.rs), [shader example](examples/shaders.rs). General safe custom pass insertion is missing; [#155](https://github.com/ponkoengine/ponko/issues/155). |
| Build tooling, packaging and deployment | Partial | [Cargo manifest](Cargo.toml), [web tooling](web), [release workflow](.github/workflows/release.yml). CI/release scripts are not proof that a complete reference game has shipped through every path. |
| Desktop and browser portability | Requires verification | Target-specific implementation and [CI configuration](.github/workflows/ci.yml) exist; see [platform policy](STABILITY.md#platforms). Full visual/runtime parity and individual GPU feature fallbacks need adapter evidence; mobile is not a validated product commitment. |
| Debugging, profiling and automated testing | Implemented | [tests](tests), [GPU tests](src/render), [performance reports](src/perf_report.rs), [inspector](src/inspector.rs). Broader reproducible matrices and diagnostics remain below. |
| AI development and runtime control | Partial | [MCP](crates/crabby_mcp/src/lib.rs), [control](src/control.rs), [agent guide](book/src/agents.md): state, stepping, launch, screenshots, logs and profiling exist. Reliable autonomous complete-game production is not established. |
| End-to-end shipping reference workflow | Missing | Examples exercise many individual systems; a reviewed complete-game acceptance workflow spanning persistence, input/UI, audio, deployment and failure recovery is still an outcome to establish. |

## 1. Performance, correctness and reproducibility

Deliver trusted baselines before optimization or broad refactoring. Existing [benchmarks](crates/crabby_bench), [profiling](book/src/profiling.md), headless tests and GPU regression modules are starting points, not completion of the expanded acceptance matrices.

| Priority / issue | Outcome and dependency |
|---|---|
| P0 [#132](https://github.com/ponkoengine/ponko/issues/132) | ECS iteration, queries, churn, batch spawn and parallel baselines, including a mixed workload, memory/allocations and pinned equivalent `bevy_ecs` comparisons. Export machine-readable results with hardware/toolchain/features and percentiles. Precedes performance-driven ECS redesign. |
| P0 [#133](https://github.com/ponkoengine/ponko/issues/133) | Seeded differential reference-model tests and Miri safety coverage across structural mutations, component layouts, drops and aliasing. Reproduce failures by seed; document parallel-tool limitations. |
| P0 [#134](https://github.com/ponkoengine/ponko/issues/134) | Static/dynamic/material/shadow/alpha/animated rendering workloads separating collect, upload, encode and GPU cost. Export comparable CPU/GPU/memory distributions and adapter/settings metadata; unsupported timestamps stay unavailable. |
| P0 [#135](https://github.com/ponkoengine/ponko/issues/135) | Broader offscreen image and behavioral regressions with tolerance/semantic checks, reviewed reference updates, explicit adapter skips, resize, reload and fallback coverage. Required before major renderer changes. |
| P1 [#142](https://github.com/ponkoengine/ponko/issues/142) | Panic-safe parallel collection and structural mutation audit with drop-count tests and measured normal-path overhead. Builds on #133. |

Do not set flaky wall-clock CI thresholds or infer overall superiority from a microbenchmark. Performance reports need CPU/GPU costs, allocation/memory effects and frame-time tails alongside correctness results.

## 2. ECS architecture and scheduling

Keep `Game::sim_system` and `Ctx` usable. Existing parallel query iteration does not provide parallel execution of independent systems. Preserve the sequential/exclusive path for opaque callbacks; implement a sound access boundary before scheduling concurrent work.

| Priority / issue | Outcome and dependency |
|---|---|
| P1 [#137](https://github.com/ponkoengine/ponko/issues/137) | Access-metadata RFC/prototype covering components, resources, commands, events, physics, hotpatch and FFI. Arbitrary `Ctx` access cannot be made safe by unchecked declarations. |
| P1 [#138](https://github.com/ponkoengine/ponko/issues/138) | Optional dependency-aware scheduler after #137 and #132; conflicting systems retain ordering, opaque systems remain exclusive. Requires sound barriers, lockstep/determinism tests and overhead crossover measurements. |
| P1 [#139](https://github.com/ponkoengine/ponko/issues/139) | Query matching cache with generation invalidation and no retained invalid pointers; measure small-world crossover after #132. |
| P1 [#140](https://github.com/ponkoengine/ponko/issues/140) | Opt-in component change semantics covering every mutation path, dynamic native components, removal and tick wraparound. Baseline tracking overhead; enables #148. |
| P1 [#141](https://github.com/ponkoengine/ponko/issues/141) | Distribute many small archetypes across parallel tasks while preserving indexed output order. Requires #132/#133 and homogeneous-workload regression measurements. |
| P1 [#143](https://github.com/ponkoengine/ponko/issues/143) | More expressive typed filters/larger tuples from actual use cases, preserving simple syntax and alias safety; benchmark against #132. |
| P2 [#144](https://github.com/ponkoengine/ponko/issues/144) | Measure component membership, column lookup and transition edges before adding indexes/bitsets; keep or reject prototypes on time/memory evidence. |
| P2 [#146](https://github.com/ponkoengine/ponko/issues/146) | Explicit commands/events/barrier visibility, deterministic tests and migration guidance. Despite its P2 label, its contract must be designed before general parallel systems are enabled in #138. |
| P3 [#166](https://github.com/ponkoengine/ponko/issues/166) | Isolated sparse-set research only if representative sparse/churn workloads justify it. Compare full memory/mutation/query costs and maintenance; no default storage rewrite. |

## 3. Renderer architecture and scene preparation

Evolve the existing renderer incrementally. Preserve frame output and public APIs at each step; keep native parallel recording and browser behavior. Internal boundaries should reduce coordination cost without exposing a large graph framework to game code.

| Priority / issue | Outcome and dependency |
|---|---|
| P0 [#136](https://github.com/ponkoengine/ponko/issues/136) | Split resource/pipeline ownership, preparation, encoding, lifetimes and post coordination in independently reviewable stages after #134/#135. Existing helper modules do not establish completion. |
| P1 [#147](https://github.com/ponkoengine/ponko/issues/147) | Lightweight internal pass/resource dependencies, hazard/cycle/format validation after #136/#135; preserve pass order and recording performance. |
| P1 [#148](https://github.com/ponkoengine/ponko/issues/148) | Persistent render-object identity and incremental extraction after #140/#134. Reuse static batches; cover edits, removal, despawn and camera changes; measure rebuild cost and memory. |
| P1 [#149](https://github.com/ponkoengine/ponko/issues/149) | Evaluate expanded GPU culling/indirect batching against CPU paths after #134. Existing static compute culling is the baseline; preserve material correctness and downlevel visibility. A measured rejection is valid. |
| P1 [#150](https://github.com/ponkoengine/ponko/issues/150) | Persistent buffers and dirty-range uploads coordinated with #148; expose upload bytes/reallocations and validate edits/streaming without stale allocations. |
| P2 [#156](https://github.com/ponkoengine/ponko/issues/156) | Material-heavy sorting/state-change measurements after #134; preserve transparent ordering and avoid regressions in simple scenes. |
| P2 [#158](https://github.com/ponkoengine/ponko/issues/158) | Test-first resize/history reset audit across scale, MSAA, camera transitions, occlusion and reload; keep hidden/offscreen capture working. Can proceed independently. |
| P2 [#159](https://github.com/ponkoengine/ponko/issues/159) | Investigate dirty shadow cascade/atlas invalidation after baselines and incremental updates; preserve configured refresh and correctness under moving casters/lights. |
| P2 [#160](https://github.com/ponkoengine/ponko/issues/160) | GPU LOD evaluation after #148/#149; measure net cost and validate crossfades, animation, shadows and fast camera motion with baseline fallback. |

## 4. Rendering capabilities and asset management

Powerful graphics need explicit capability, lifetime and customization contracts. Existing shaders, effects, assets and streaming helpers remain the simple entry points.

| Priority / issue | Outcome and dependency |
|---|---|
| P1 [#151](https://github.com/ponkoengine/ponko/issues/151) | Machine-readable capability tiers and fallback reasons for downlevel/WebGL2, baseline WebGPU and advanced desktop paths; limited-device tests before broader GPU features. |
| P1 [#152](https://github.com/ponkoengine/ponko/issues/152) | Typed centralized pipeline/variant cache after #136/#134; correct format/sample/features keys, targeted reload invalidation and observable cache/build costs. |
| P1 [#154](https://github.com/ponkoengine/ponko/issues/154) | Budgeted residency after ownership refactoring: asynchronous upload states/budgets, texture mip residency first, mesh LOD evaluation later. Preserve simple loading, ownership/native APIs and graceful missing assets. |
| P2 [#155](https://github.com/ponkoengine/ponko/issues/155) | Constrained named-stage custom passes after #147's resource model; safe hazards/lifetimes and fullscreen/compute examples without a mandatory plugin framework. |
| P2 [#161](https://github.com/ponkoengine/ponko/issues/161) | Independent asset/render-resource lifetime audit and load/unload/reload stress tests. Expose growing resource classes and disposal/cache rules. |
| P2 [#162](https://github.com/ponkoengine/ponko/issues/162) | Custom shading contract and compile-valid variant matrix covering depth/shadow/velocity, alpha, skinning, instancing and displaced bounds; coordinate with #152. |
| P2 [#163](https://github.com/ponkoengine/ponko/issues/163) | Evaluate opt-in quality autotuning after #151 and profiling; use actual GPU timings with hysteresis, user bounds and screenshot overrides. Never substitute CPU time for unavailable GPU data. |
| P3 [#164](https://github.com/ponkoengine/ponko/issues/164) | Isolated mesh-shader research versus indexed indirect/LOD paths; support matrix, fallback and maintenance costs decide adoption. Not a shipping prerequisite. |
| P3 [#165](https://github.com/ponkoengine/ponko/issues/165) | Ray-tracing/path-traced lighting research only after architecture, correctness, performance and streaming foundations; measure quality/cost and retain GI/SSR fallback. No production integration promise. |

## 5. Complete-game development workflows

Turn the capability inventory into a repeatable shipping path. This is an outcome track, not a new numbered backlog or a claim that every subsystem needs replacement. Select a small representative game and document gaps before creating additional issues; reuse existing issues where applicable.

Acceptance should cover scene transitions, input rebinding/gamepad use, readable UI and accessibility checks, audio, save migration/recovery, asset packaging, and a native/web release procedure. Add a networked/server case when that workflow is under evaluation. Validate imported animation, character/navigation interaction and large-world load/unload against actual game needs. Use headless assertions for simulation and captured, inspected images for visual behavior. #154/#161 address residency and ownership prerequisites for larger content; platform capability reporting comes from #151.

Advance this track through a playable vertical slice, a persisted/reloaded slice, then packaged target builds with recorded limitations. Do not block a small complete game on P3 graphics research or an elaborate editor.

## 6. AI-assisted development and automated validation

Existing MCP launch, reflection, stepping, screenshots, logs and performance reports support parts of the observe/correct loop. Make failures actionable and evidence reproducible before claiming increasingly autonomous workflows.

| Priority / issue | Outcome and dependency |
|---|---|
| P1 [#153](https://github.com/ponkoengine/ponko/issues/153) | Structured WGSL identity, original source locations, variant and reload result; retain last-known-good pipelines where feasible. Deliver in stages independently of the full #152 cache. |
| P2 [#145](https://github.com/ponkoengine/ponko/issues/145) | Structured access/dependency/query-match/dispatch diagnostics after scheduler and query-cache prototypes (#138/#139); show opaque systems and barriers with low disabled overhead. |
| P2 [#157](https://github.com/ponkoengine/ponko/issues/157) | Stable machine-readable extraction/prepare/encode, GPU pass, upload, culling and memory reports after baseline instrumentation. Unsupported timestamps are unavailable, never fabricated zeroes. |

A reference agent workflow should record build/features, launch/setup, deterministic inputs or state edits, sampled assertions, screenshots where relevant, logs and comparable profiles. Include a deliberate failure and successful diagnosis. Maintain existing MCP contracts under [STABILITY.md](STABILITY.md); additional discoverability must not require a visual editor.

## 7. Developer experience, portability and contributors

Keep one obvious common API, useful errors, current Rust/C++ examples and small independently verifiable contributions. A lightweight inspector, scene viewer and debugger are welcome; a large editor is not a prerequisite. Keep external asset tools such as Blender practical through verified import examples.

Maintainers coordinate guide coverage, native interfaces and platform/feature requirements, with scoped follow-ups where needed; Rust-only contributions are welcome. Test headless/fallback behavior where relevant. The packaging/reference-game track should expose missing deployment steps instead of treating a successful engine build as a shipped game. [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md) define the review and validation workflow; [STABILITY.md](STABILITY.md) governs compatibility. Mobile support and C++ browser workflows require separate evidence and scope decisions rather than an implied commitment.

## Backlog status and delivery gates

All 35 issues #132–#166 were open in the reviewed backlog snapshot. Their full descriptions informed this mapping. Source inspection found existing pieces—benchmark/profiling facilities, GPU image tests, static compute culling, reusable buffers, renderer modules and MCP tools—but did not establish that any issue's complete acceptance criteria were satisfied or superseded. Consequently none is marked complete here. Recheck issue state, merged changes and evidence when selecting work; issue prose can describe an older implementation snapshot.

The practical dependency spine is:

- ECS: #132/#133 → safe measured improvements; #137 plus #146's visibility design → #138; #140 → #148; #138/#139 → #145.
- Renderer: #134/#135 → #136 → #147/#152 and resource ownership for #154; #134/#140 → #148 → #150; #134 → #149 → #160 with #148; #147 → #155.
- Portability/observation: #151 plus trustworthy GPU profiling → #163; #134 instrumentation → #157; #153, #158 and #161 can make independent progress.
- Research: #164–#166 require representative evidence and explicit keep/reject decisions; they do not hold up shipping core workflows.

Before an item is called delivered, review its acceptance criteria against source and tests, preserve API/ABI/save/MCP compatibility, provide measurements for optimization claims and record runtime/platform gaps. Each implementation should explain its behavior and evidence; broader guide/native coverage can be maintainer-owned follow-ups rather than work required from every contributor. This roadmap creates no duplicate issues and changes no existing priority labels.
