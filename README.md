# Crabby Engine

[![CI](https://github.com/bozoslav/crabbyengine/actions/workflows/ci.yml/badge.svg)](https://github.com/bozoslav/crabbyengine/actions/workflows/ci.yml)

A fast 2D engine in Rust for games with huge numbers of simulated things:
millions of entities on all CPU cores, a pausable fixed-rate simulation clock,
spatial queries, flow-field pathfinding for crowds, and headless runs for tests and servers. A 2D camera, text
and simple UI come built in, and AI agents can drive running games through
an MCP server (inspect, step, screenshot). Games can be written in Rust or
C++ (a header-only C++17 wrapper over a C API) at close to the same speed.

**Documentation:** [the guide](https://bozoslav.github.io/crabbyengine/)
(searchable) and the [API reference](https://bozoslav.github.io/crabbyengine/api/crabbyengine/).

```sh
cargo run --release --example boids
HEADLESS=1 BOIDS=1000000 cargo run --release --example boids
cargo run --release --example ants
cargo run --release --example flow
cargo run --release --example battle     # the flagship demo: two armies, 20k units
HEADLESS=1 UNITS=1000000 cargo run --release --example battle
cargo run --release --example ui

# C++ (needs CMake): see the guide's "C and C++" chapter
cargo build -p crabby_ffi --release
cmake -S examples/cpp -B target/native -DCMAKE_BUILD_TYPE=Release && cmake --build target/native
./target/native/crabby_cpp_boids
./target/native/crabby_cpp_battle   # the battle demo in C++: same speed as Rust
```

Contributing (humans and agents): see [AGENTS.md](AGENTS.md).
