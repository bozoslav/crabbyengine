# API reference

Every public item has rustdoc with an example. The reference of each release
is on docs.rs (linked below); build the one for `main` locally:

```sh
cargo doc --workspace --no-deps --open
```

The crates:

- [`ponko`](https://docs.rs/ponko/latest/ponko/): the engine. Start here;
  `use ponko::prelude::*;` brings in what a typical game needs; `ponko::d2`,
  `ponko::d3` and the service modules (`net`, `perf`, `render`, ...) hold the rest.
- [`ponko_ecs`](https://docs.rs/ponko_ecs/latest/ponko_ecs/): the ECS on its own, including
  the C-compatible path (`world.raw().register_component`, `world.raw().for_each_chunk`) for other
  languages.

- [`ponko_ffi`](https://docs.rs/ponko_ffi/latest/ponko_ffi/): the C ABI and safety contracts.
  The [C and C++ guide](c-cpp.md) covers building, examples and the C++ wrapper.
