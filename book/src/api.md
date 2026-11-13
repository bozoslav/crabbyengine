# API reference

Every public item is documented in rustdoc, most with an example. The
reference for the 0.1.0 release is on docs.rs (linked below); items behind a
cargo feature say which one ("Available on crate feature `3d` only"). The
reference for `main` builds locally:

```sh
cargo doc --workspace --no-deps --open
```

The crates, version 0.1.0:

- [`ponko`](https://docs.rs/ponko/0.1.0/ponko/): the engine. Start here;
  `use ponko::prelude::*;` brings in what a typical game needs; `ponko::d2`,
  `ponko::d3` and the service modules (`net`, `perf`, `render`, ...) hold the rest.
- [`ponko_ecs`](https://docs.rs/ponko_ecs/0.1.0/ponko_ecs/): the ECS on its own, including
  the C-compatible path (`world.raw().register_component`, `world.raw().for_each_chunk`) for other
  languages.
- [`ponko_macros`](https://docs.rs/ponko_macros/0.1.0/ponko_macros/): `#[derive(Component)]`,
  used through `ponko` or `ponko_ecs`.
- [`ponko_ffi`](https://docs.rs/ponko_ffi/0.1.0/ponko_ffi/): the C ABI and safety contracts.
  The [C and C++ guide](c-cpp.md) covers building, examples and the C++ wrapper.
- [`ponko_mcp`](https://docs.rs/ponko_mcp/0.1.0/ponko_mcp/): the MCP server for AI agents
  (the `ponko-mcp` binary); see [AI agents](agents.md).
- [`ponko_unity`](https://docs.rs/ponko_unity/0.1.0/ponko_unity/): reads Unity projects saved
  as text, for importing; see [Importing from Unity](unity.md).

For the newest release, replace `0.1.0` in a link with `latest`.

The C++ reference is `include/ponko.hpp` itself: every public type and
function has a `///` comment saying what it does, its units, who owns what and
what it throws (Doxygen and editors read them). The C functions are documented
with their Rust definitions in `ponko_ffi`, on docs.rs.
