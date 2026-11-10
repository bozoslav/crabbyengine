# Attachments: things that follow things

Health bars over units, turrets on tanks, selection rings, a carried flag:
entities that should stay at some place relative to another entity. Give
them an `Attached` component and add the built-in `systems::follow_attachments`
system after the systems that move things:

<div class="lang-pair">
<div>

```rust
{{#include ../../tests/attach.rs:attach}}
```

</div>
<div>

```cpp
{{#include ../../examples/cpp/guide.cpp:attach}}
```

</div>
</div>

Every tick, each attached entity's `Transform2D` gets:

- **position**: the followed entity's position plus `offset`, turned by its
  rotation and stretched by its scale (so flipping a sprite with a negative
  x scale mirrors what hangs off it);
- **rotation**: the followed entity's rotation plus `rotation`.

Its own **scale** is left alone, so a small icon stays small on a big unit.

| Rust | C++ | Meaning |
|---|---|---|
| `Attached::to(e)` | `ponko::attach_to(e)` | right on top of `e` |
| `.offset(v)` | `attach_to(e, v)` | `v` away, in `e`'s space |
| `.rotated(r)` | `attach_to(e, v, r)` | turned `r` radians more |

## Chains and cleanup

Attachments can be attached to attachments (an arm on a body, a hand on the
arm): each link is placed in its parent's space, from the root down, all in
one tick, whatever order the entities were spawned in.

When the entity at the top of a chain is despawned, everything attached to
it (directly or down the chain) is despawned in the same tick. A chain that
loops back on itself, or is more than 64 links deep, is treated as broken
and despawned too.

## Cost

Placing runs on all cores and allocates nothing in steady state: about
**0.7 ms per tick for 100,000 attachments** (release, M-series Mac,
`cargo run --release -p ponko_bench -- attach`). Entities without
`Attached` cost nothing.

Attachments are a sim system, so they're deterministic, saved with the world
(`Attached` is reflected) and visible to tools and agents like any other
component.
