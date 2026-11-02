# Crabby Engine

[![CI](https://github.com/bozoslav/crabbyengine/actions/workflows/ci.yml/badge.svg)](https://github.com/bozoslav/crabbyengine/actions/workflows/ci.yml)

A fast 2D engine in Rust for games with huge numbers of simulated things:
millions of entities on all CPU cores, a pausable fixed-rate simulation clock,
spatial queries, and headless runs for tests and servers.

**Documentation:** [the guide](https://bozoslav.github.io/crabbyengine/)
(searchable) and the [API reference](https://bozoslav.github.io/crabbyengine/api/crabbyengine/).

```sh
cargo run --release --example boids
HEADLESS=1 BOIDS=1000000 cargo run --release --example boids
```

Contributing (humans and agents): see [AGENTS.md](AGENTS.md).
