# 3D navigation meshes

Enable `navmesh` for pure Rust Recast baking and Detour pathfinding. It implies
`3d`, but needs no window, GPU, physics, Bevy or async runtime. Servers can build
with `--no-default-features --features navmesh`. The existing 2D `NavGrid` and
`FlowField` are independent.

Gather static geometry as world-space `NavSource` triangle soups, or use
`NavSource::from_mesh` with a `MeshData` and `Transform3D`. Include solid walls,
ceilings and platforms, not only floors. Floors face up with counter-clockwise
winding. Sources are copied: later mesh edits require another bake.

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/navmesh3d.rs:bake}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:nav_bake}}
```

</div>
</div>

Settings use world units, and `max_slope` uses degrees. Recast voxelizes triangles,
filters low ceilings and ledges, erodes for the agent radius, partitions regions,
and builds tiled polygons with detail heights. Radius and height round up to
cells; climb rounds down. Smaller cells improve accuracy and cost more baking
work and memory. Bake uses bounded backend budgets and returns `NavBakeError`
on invalid inputs or resource exhaustion. A scene with no walkable surface may
produce an empty mesh.

Sample explicitly when accepting a distant click or spawn position. Sampling
uses Euclidean distance, including vertical distance. `find_path` accepts endpoints
close to the surface (two cells horizontally and climb plus two cell heights
vertically) and returns string-pulled world-space corners. Disconnected or truncated
routes return `None`, never a partial success. Queries currently cap the search and
corridor at 8192 nodes/polygons; an oversized route can therefore return `None`.
Agent route preparation also caps surface samples at one million; an oversized
route leaves the agent stopped.

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/navmesh3d.rs:query}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:nav_query}}
```

</div>
</div>

Register `step_navmesh3d` with `Game::sim_system` (C++: `game::navmesh3d()`), like an exposed physics step.
There is no automatic navigation update. Insert a `NavMesh3D` resource, then spawn
entities with `Transform3D` and `NavAgent3D`. This works in `run_headless` and obeys
the simulation clock. Put destination/obstacle systems before the navigation step.

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/navmesh3d.rs:agents}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:nav_agents}}
{{#include ../../examples/cpp/guide.cpp:nav_agent_spawn}}
```

</div>
</div>

The follower accelerates up to its speed and follows a surface-sampled path.
Its transform represents its feet. It has no gravity, physical collision response,
turning inertia. Crowd avoidance is opt-in (see below). Do not also attach a physics body or character
controller that writes the same transform. Use a separate presentation offset if
your model origin is above its feet. Bake for the largest agent radius and height;
components with radius larger than the bake radius stop.

`NavObstacle3D` is a world-space axis-aligned box component. The sim system gathers
all boxes each tick. Changed, moved or removed boxes invalidate cached paths, as
does replacing the mesh resource, changing a destination or teleporting an agent.
Unreachable agents wait until one of these changes. An agent trapped inside a
newly blocked polygon stops; it is not automatically pushed out.

Dynamic boxes **block entire overlapping polygon bounds**, expanded by agent radius
and vertical clearance. This is conservative: a small obstacle may close a much
larger region, especially with large tiles. Tiles are baked, but local rebaking and
precise carving are not exposed yet. Changes scan all polygons; no spatial obstacle
index is maintained. Use geometry in the bake for permanent walls and holes.
For manual queries without the sim system, call `set_obstacles`; when registered,
the system owns the obstacle list and replaces it from ECS components.

With avoidance disabled, no path queries or heap allocations occur in steady agent movement; replans build
paths and sample their heights. Baking and obstacle scans are synchronous, so
schedule large bakes outside latency-sensitive ticks. Ordered source geometry,
settings and updates are deterministic; the backend enables `libm` for consistent
math. This does not establish cross-platform determinism for the rest of a game.

The backend is `navmesh-rs`, a Rust port of Recast/Detour. It avoids a native C++
build and keeps generation and 3D queries together. Landmass would still require
a baker, Polyanya requires extra 3D surface handling, and oxidized_navigation is
a Bevy plugin. The C API and C++ wrapper expose baking, queries, agents, avoidance and obstacles.


## Crowd avoidance

Set `NavAgent3D::avoidance` to opt into the backend's DetourCrowd manager.
The default is false so existing path following and arrival behaviour stay intact.
Only opted-in agents participate, including agents whose destination is `None`.
Do not mix opted-out followers into a crowd and expect them to collide.

<div class="lang-pair">
<div>

```rust
# use crabbyengine::*;
let mut agent = NavAgent3D::default();
agent.avoidance = true;
agent.avoidance_weight = 2.0;
agent.neighbour_distance = 3.6;
agent.destination = Some(Vec3::X * 5.0);
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:nav_avoidance}}
```

</div>
</div>

Crowd agents retain radius, maximum speed and acceleration. The nonnegative
`avoidance_weight` controls separation steering, rather than right of way.
`neighbour_distance` must be positive; choose it to cover approaching traffic.
The C equivalent is `crabby_ctx_set_nav_avoidance(ctx, entity, enabled, weight,
neighbour_distance)`. Invalid C parameters return an error; invalid Rust agent
parameters stop that agent and remove it from the crowd. Crowd radii must be
positive and no larger than the bake radius.

The backend uses a proximity grid, keeps the six nearest neighbours and samples
velocities against their predicted motion and local navmesh walls, with a 2.5 s
time horizon. Nearest-neighbour and sampling fixes are enabled. An internal 30%
radius margin reduces penetration from its iterative overlap correction. Movement
uses connected surface corridors, including height, rather than projecting onto a
globally nearest floor. Ticks are subdivided toward 60 Hz, up to 64 substeps.

This is approximate local avoidance, not a collision solver or a guarantee of
progress in arbitrarily dense traffic. Large time steps, excessive speeds, initial
overlaps, narrow passages and opposing bottlenecks can cause penetration or
stalling. Stopped agents can be displaced by overlap correction. Several agents
sharing one destination settle in a region around it: nonzero discs cannot all
arrive at one exact point. Give agents distinct reachable targets when precise
arrival is required. Moving boxes still use conservative polygon blocking and
invalidate crowd corridors; there is no predictive box velocity or tile-cache
carving.

Slots and corridors persist across ticks. Capacity grows in powers of two up to
16383 agents; excess agents wait. Mesh or obstacle changes rebuild the crowd,
and teleports recreate the affected slot. The backend currently allocates query
scratch during updates; avoid assuming allocation-free crowd simulation. The
original follower retains its allocation-free steady path. Baking, full-route
validation on a new target, and crowd updates are synchronous and headless.
