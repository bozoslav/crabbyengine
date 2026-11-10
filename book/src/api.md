# API reference

Every public item has rustdoc with an example. Build the reference locally:

```sh
cargo doc --workspace --no-deps --open
```

The crates:

- [`ponko`](api/ponko/index.html): the engine. Start here;
  `use ponko::prelude::*;` brings in what a typical game needs; `ponko::d2`,
  `ponko::d3` and the service modules (`net`, `perf`, `render`, ...) hold the rest.
- [`ponko_ecs`](api/ponko_ecs/index.html): the ECS on its own, including
  the C-compatible path (`register_component`, `world.raw().for_each_chunk`) for other
  languages.

- [`ponko_ffi`](api/ponko_ffi/index.html): the C ABI and safety contracts.
  The [C and C++ guide](c-cpp.md) covers building, examples and the C++ wrapper.
