# 3D physics: bodies, terrain and a walking character

Crates that tumble down a hill, a first-person character that walks up
slopes and stairs, rays that find what the player is looking at: 3D
rigid-body physics on [rapier](https://rapier.rs), designed like the
[2D physics](physics.md). It's behind the `physics3d` feature (which turns
on `3d`):

```toml
crabbyengine = { version = "1", features = ["physics3d"] }
```

(The C library, `crabby_ffi`, has it on by default.)

Turn it on with gravity in m/s² (+Y is up):

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/physics3d.rs:setup}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:physics3d_setup}}
```

</div>
</div>

The physics step is a simulation system, placed where you call `physics3d`.
It runs 50 times a second unless you chose a rate with `fixed_hz`, and
between steps every body is **drawn blended** between its last two poses,
so motion stays smooth at any frame rate. Simulation systems always see the
stepped pose; to read a body's exact physics position from a frame system,
use `ctx.physics3d.position(e)`.

## Bodies, colliders and terrain

An entity becomes a body with a `Transform3D`, a `RigidBody3D` and a
`Collider3D` (in meters). Terrain gets a **heightfield** made from the same
heights, size and corner as its [terrain tile](3d.md), so what you walk on
is what you see:

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/physics3d.rs:world}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:physics3d_world}}
```

</div>
</div>

| Rust | C++ | Meaning |
|---|---|---|
| `RigidBody3D::dynamic()` | `crabby::dynamic_body3d()` | moved by gravity and collisions |
| `RigidBody3D::fixed()` | `crabby::fixed_body3d()` | never moves: ground, walls, buildings |
| `RigidBody3D::kinematic()` | `crabby::kinematic_body3d()` | moved only by you; pushes, isn't pushed |
| `Collider3D::sphere(r)` | `crabby::sphere_collider(r)` | |
| `Collider3D::cuboid(size)` | `crabby::box_collider(size)` | full width, height, depth |
| `Collider3D::capsule(h, r)` | `crabby::capsule_collider3d(h, r)` | upright; `h` between the centers of its ends |
| `Collider3D::mesh(m)` | `crabby::mesh_collider(m)` | a heightfield, convex hull or triangle mesh |
| `.density(d)`, `.friction(f)`, `.restitution(r)` | fields of the same name | kg/m³ (default 1000), 0 ice..1 rubber, bounciness |
| `.sensor()` | `.sensor = true` | reports overlaps, doesn't push |
| `.offset(v)`, `.groups(g)` | `.offset`, `.groups` | shape moved from the entity; who it touches |

Collision meshes are made once and shared by any number of colliders:

- `ctx.physics3d.heightfield(&heights, size)` (C++
  `ctx.heightfield_collider`): n × n heights, row after row along +Z, NaN
  for a hole you can fall through. Put it on a fixed body at the tile's
  corner.
- `ctx.physics3d.convex_hull(&points)`: the hull around a point cloud, for
  rocks and props that move.
- `ctx.physics3d.trimesh(&vertices, &triangles)`: any triangle mesh, for
  static buildings and level geometry (it has no inside, so give it to
  fixed bodies).
- `ctx.physics3d.free_mesh(m)` lets go of one; colliders still using it
  keep it until they're removed.

**`Transform3D` stays the source of truth**, as in 2D: after each step,
dynamic bodies write their pose into it; set a transform yourself and the
body teleports there. A `Velocity3D` component reads and sets a body's
speed (in C++ a plain struct and a query column). Physics ignores a
transform's scale: size the collider instead.

**Collision groups** decide who touches whom. A collider is in the groups
of its `memberships` bits and touches colliders whose memberships share a
bit with its `filter`; both sides must agree. `CollisionGroups::ALL` (the
default) touches everything:

```rust
const GROUND: u32 = 1;
const DEBRIS: u32 = 2;
// Debris lands on the ground but passes through other debris.
let debris = CollisionGroups::new(DEBRIS, GROUND);
```

Collisions send `CollisionStarted` and `CollisionStopped` events, the same
ones as in 2D (C++: `ctx.collisions_started()`).

## A walking character

`Character3D` is a kinematic capsule that walks: it slides along walls,
climbs slopes up to `max_slope`, steps up ledges up to `step_height`, stays
on the ground going downhill (`snap_distance`), pushes light dynamic bodies
(as if it weighed `push_mass`) and sets off sensors. Its `Transform3D`
position is at its **feet**. Set `velocity` (x and z to walk, y to jump)
and read `grounded`; the step adds gravity while it's in the air:

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/physics3d.rs:walk}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:physics3d_walk}}
```

</div>
</div>

| Field | Default | |
|---|---|---|
| `height`, `radius` | 1.8, 0.3 | feet to head; the capsule's width |
| `max_slope` | 45° | steeper ground is a wall |
| `step_height` | 0.35 m | 0: no stepping |
| `snap_distance` | 0.3 m | how far down it follows the ground |
| `push_mass` | 80 kg | 0: doesn't push |
| `gravity_scale` | 1 | 0: floats |
| `groups` | all | what it walks on and bumps into |

For mouse look, lock the cursor and turn the head by the raw mouse motion
([input](input.md#mouse-look)) in a frame system:

```rust
{{#include ../../examples/physics3d.rs:look}}
```

## Rays, shape casts and overlaps

`ctx.physics3d` answers questions about the world as of the last step:

<div class="lang-pair">
<div>

```rust
{{#include ../../examples/physics3d.rs:queries}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:physics3d_queries}}
```

</div>
</div>

- `cast_ray(origin, direction, max_distance, filter)`: the closest hit,
  with the entity, point, surface normal and distance.
- `cast_ray_all(.., &mut out)`: every hit, nearest first.
- `cast_sphere` and `cast_capsule`: sweep a shape (a wheel, a character)
  and get the first thing it would touch.
- `overlap_sphere`, `overlap_box` and `overlap_capsule`: every entity
  inside, into `out`.
- `apply_impulse(e, impulse)` and `apply_impulse_at(e, impulse, point)`:
  a push in kg × m/s (multiply by `mass(e)` for a speed change); at a point
  it also spins the body.

Every query takes a `QueryFilter3D` (C++ `crabby::filter3d`):
`.exclude(e)` looks through one entity (the one asking), `.groups(g)` sees
what a collider in those groups would touch, and `.with_sensors()` sees
sensors too (they're skipped by default). Lists go into a `Vec` you keep,
so queries don't allocate.

New bodies join at the next step, so a query in the tick you spawned them
won't see them yet.

## Cost

Bodies cost only where they exist, and steady-state steps add no
allocations of their own on top of rapier's. Compared with calling rapier
directly, the engine's syncing adds 15–20%
(`cargo run --release -p crabby_bench -- physics3d`, M4 Max):

| Boxes falling into a pit, 250 ticks | Crabby | rapier alone |
|---|---|---|
| 1,000 | 28.5 ms | 23.3 ms |
| 10,000 | 414 ms | 361 ms |

Characters cost more than bodies: each one sweeps its capsule against the
world several times a step (walking, stepping, snapping to the ground).
On heightfield terrain that's about 50 µs per character per step (100
characters: 5 ms a step), on flat boxes about 16 µs: fine for a player and
a few dozen others, not for crowds (move those yourself, with a ray down
to find the ground). Level geometry as a triangle mesh is somewhat cheaper
than a heightfield.

Try it: `cargo run --release --example physics3d --features physics3d`.
