# Contributing to Crabby Engine

Thanks for your interest! Bug reports, benchmarks on your hardware, docs
fixes, examples and code are all welcome.

The full rules for humans and coding agents are in [AGENTS.md](AGENTS.md).
This page is the short version.

## Questions, bugs and ideas

- **Bugs:** open an [issue](https://github.com/bozoslav/crabbyengine/issues/new/choose)
  with the smallest code that shows the problem, your OS and GPU, and what
  you expected.
- **Ideas and features:** open an issue first, so we can agree on the
  shape before you write the code. The engine aims for one obvious way to
  do each thing, so a new API gets discussed before it's built.
- **Security issues:** don't open a public issue; see [SECURITY.md](SECURITY.md).

## Setting up

You need stable Rust (`rustup update stable`). On Linux, also install the
ALSA and udev headers for audio and gamepads
(`sudo apt-get install libasound2-dev libudev-dev`). For the C++
side, you need CMake and a C++17 compiler.

```sh
git clone https://github.com/bozoslav/crabbyengine && cd crabbyengine
cargo test --workspace
cargo run --release --example battle
```

## Repository layout

| Path | What's there |
|---|---|
| `src/` | The engine (`crabbyengine` crate) |
| `crates/crabby_ecs` | The ECS, usable on its own |
| `crates/crabby_ffi` | The C API; `include/crabby.h` is generated from it |
| `include/crabby.hpp` | The header-only C++ wrapper |
| `crates/crabby_mcp` | The MCP server for AI agents |
| `crates/crabby_bench` | Benchmarks (`cargo run --release -p crabby_bench [-- filter]`) |
| `examples/`, `examples/cpp/` | Rust and C++ examples |
| `tests/`, `crates/*/tests/` | Tests, one file per area |
| `book/` | The guide ([mdBook](https://rust-lang.github.io/mdBook/)) |

## Making a change

1. Fork the repository and create a branch (`fix/<topic>`, `docs/<topic>`, ...).
2. Make the change, with tests:
   - New public behavior gets a test in the matching area file.
   - Every public item gets a doctest.
   - Bug fixes get a test that fails without the fix.
3. If it changes how games are written, update the guide in `book/src/` in
   the same pull request. Guide examples come in Rust and C++.
4. If it's an engine feature games use, add it to the C API and the C++
   wrapper too, or say in the pull request why not.
5. Run the checks below, then open a pull request to `main`. Describe what
   changed and how you checked it.

### Checks

```sh
cargo fmt --all --check
cargo build --workspace --examples
cargo test --workspace
cargo clippy --workspace --all-targets
mdbook build book            # if you touched the guide
```

If you changed unsafe code in `crates/crabby_ecs`, also run the Miri
commands in [AGENTS.md](AGENTS.md#before-every-commit). CI runs all of this
on Windows, macOS and Linux.

### Performance changes

Claims need numbers: measure in release builds before and after, and put
both in the pull request. Avoid per-frame heap allocations on hot paths.

## License

By contributing, you agree that your contributions are dual licensed under
[MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE), like the rest of the
project, without any additional terms or conditions.
