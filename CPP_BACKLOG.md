# C and C++ backlog

Rust features waiting for their C API (`crates/crabby_ffi`), C++ wrapper
(`include/crabby.hpp`), tests and guide snippets. See AGENTS.md, "Rust
first, C/C++ after". Each entry is a spec: follow it exactly, especially
the ABI notes, then delete the entry in the same PR.

The C ABI is version 2 (`crabby_abi_version()`): add functions and enum
values; never change an existing struct's layout or function's signature.

## Polygon colliders

The C API and C++ wrapper are done (`crabby_set_polygon_collider`,
`crabby_collider_points`, `CRABBY_SHAPE_POLYGON`;
`world.set_polygon_collider(e, {points...}, settings)`,
`world.collider_points(e)`). Still missing:

- C++ test in `examples/cpp/tests.cpp`: a wedge
  `{{-30, 15}, {30, 15}, {-30, -15}}` on a dynamic body falls onto a fixed
  rect and rests near y = 175 with rotation about 0 (as
  `a_falling_polygon_comes_to_rest_on_its_flat_side` in `tests/physics.rs`);
  `collider_points` returns the three corners; 2 or 9 points throw.
- Guide snippet: an anchor `physics_polygon` in `examples/cpp/guide.cpp`
  (a fixed ramp like the one in `examples/physics.rs`), shown next to the
  Rust ramp in `book/src/physics.md` ("Bodies and colliders").

ABI: `CrabbyCollider` is unchanged on purpose (it has no room for points
and its layout is frozen in ABI 2). Reading a polygon gives
`shape = CRABBY_SHAPE_POLYGON` and its bounding box in `size`; writing that
back with `crabby_set` is refused, so polygons are only set with
`crabby_set_polygon_collider`.
