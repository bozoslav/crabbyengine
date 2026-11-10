# Contributing to Ponko

Thanks for your interest! Bug reports, benchmarks on your hardware, docs
fixes, examples and code are all welcome.

## Start with what you know

**Rust-only pull requests are welcome.** You do not need to know C or C++,
write a guide chapter, or work on unrelated subsystems to contribute a Rust
fix or feature. Small fixes, tests, examples and documentation are useful
contributions on their own. No design proposal is needed for an isolated fix.

Keep your change focused, explain the behavior in the PR, and run the checks
relevant to it. Tests for changed behavior matter; a full integration checklist
is not an entry requirement. If a check needs tools or hardware you do not
have, say so and ask for help.

Maintainers own the broader integration work: guide updates, native bindings,
paired examples and the full CI matrix. They can help in the PR or record
separate follow-ups without assigning that work to you. Compatibility of
existing APIs and bindings still needs to be preserved.

[AGENTS.md](AGENTS.md) contains the detailed workflow and maintainer/agent
integration checklist; you only need the parts relevant to your contribution.

## Questions, bugs and ideas

- **Bugs:** open an [issue](https://github.com/ponkoengine/ponko/issues/new/choose)
  with the smallest code that shows the problem, your OS and GPU, and what
  you expected.
- **Ideas and features:** open an issue first, so we can agree on the
  shape before you write the code. The engine aims for one obvious way to
  do each thing, so a new API gets discussed before it's built.
- **Security issues:** don't open a public issue; see [SECURITY.md](SECURITY.md).

## Finding a manageable contribution

You do not need to understand the whole engine to fix an isolated bug, improve an
example or clarify a guide page. Start with one reproducible behavior and its tests.
Browse the [open issues](https://github.com/ponkoengine/ponko/issues), check for an
existing assignee or active PR, and comment with the slice you want to tackle.

The ECS/rendering backlog (#132–#166) uses `help wanted`, `priority: P0` through
`priority: P3`, subsystem labels such as `area: ecs`, `area: rendering`,
`area: shaders`, `area: assets`, and `type:` labels. P0 establishes foundations;
P1 addresses high-impact limitations, P2 is useful lower-urgency work, and P3 is research. Priority is not difficulty:
this backlog has no difficulty labels. Read acceptance criteria and dependencies,
and ask for a bounded first step if the scope is unclear.

For example, [#133](https://github.com/ponkoengine/ponko/issues/133) covers ECS
correctness tests and [#135](https://github.com/ponkoengine/ponko/issues/135)
covers visual regressions; a focused reproducer or test case can contribute toward
either without claiming the entire issue. Performance work starts with the baselines
in [#132](https://github.com/ponkoengine/ponko/issues/132) and
[#134](https://github.com/ponkoengine/ponko/issues/134).
[ROADMAP.md](ROADMAP.md) explains the sequence; verify live issue status before starting.

## Setting up

You need stable Rust (`rustup update stable`). On Linux, also install the
ALSA and udev headers for audio and gamepads
(`sudo apt-get install libasound2-dev libudev-dev`). For the C++
side, you need CMake and a C++17 compiler.

```sh
git clone https://github.com/ponkoengine/ponko && cd ponko
cargo test --workspace
cargo run --release --example battle
```

## Repository layout

Cargo packages use `ponko` / `ponko_*`; legacy native ABI names,
C/C++ headers and runtime environment variable names remain supported. New code
uses `include/ponko.h`, `include/ponko.hpp`, and `PONKO_*` settings. See
[the naming migration](book/src/naming.md) for precedence and compatibility.

| Path | What's there |
|---|---|
| `src/` | The engine (`ponko` crate) |
| `crates/ponko_ecs` | The `ponko_ecs` ECS, usable on its own |
| `crates/ponko_ffi` | The C API; `include/crabby.h` is generated from it |
| `include/ponko.hpp` | The header-only C++ wrapper |
| `crates/ponko_mcp` | The MCP server for AI agents |
| `crates/ponko_bench` | Benchmarks (`cargo run --release -p ponko_bench [-- filter]`) |
| `examples/`, `examples/cpp/` | Rust and C++ examples |
| `tests/`, `crates/*/tests/` | Tests, one file per area |
| `book/` | The guide ([mdBook](https://rust-lang.github.io/mdBook/)) |

## Making a change

1. Fork the repository and create a branch (`fix/<topic>`, `docs/<topic>`, ...).
2. Make one focused change. Add a regression test for a bug fix or a test of
   new behavior where practical; explain any testing gap in the PR.
3. Add or adjust nearby API comments when needed to explain the behavior.
   Broader guide chapters, C/C++ bindings and paired examples are welcome but
   **not required from every contributor**. Mention follow-up needs you notice;
   maintainers will help identify and own the rest.
4. If users will notice the change, add a changelog fragment,
   `changelog.d/<pr-or-topic>.md`, instead of editing `CHANGELOG.md`
   directly (see [changelog.d/README.md](changelog.d/README.md)).
5. Run relevant checks, then open a PR to `main` describing the change and
   what you checked. It is fine to open a draft early for feedback.

### Checks: start with your change

For a Rust change, format it and run the affected package or test, for example:

```sh
cargo fmt --all --check
cargo test -p ponko_ecs --test queries  # an ECS query change
cargo test -p ponko --test headless    # a headless simulation change
```

For guide changes, run `mdbook build book` if mdBook is available. You do not
need a C++ toolchain for an unrelated Rust change. Report checks you could not
run; maintainers and CI cover the broader matrix before merging.

Maintainers/agents use the full integration commands in
[AGENTS.md](AGENTS.md#integration-checks), including workspace builds, tests,
Clippy, relevant native checks and Miri for affected unsafe ECS code. Required
CI must pass before merge; focused local checks do not waive compatibility,
correctness, privacy or leak checks. The exact platform/feature matrix is in
[ci.yml](.github/workflows/ci.yml): a pull request runs quick Linux checks
(no tests: run them locally), and the tests and the full macOS/Windows/feature
matrix run on `main` after the change
lands (maintainers can add the `full-ci` label to run it on the PR first).

### Performance changes

Claims need numbers: measure in release builds before and after, and put
both in the pull request. Avoid per-frame heap allocations on hot paths.

## Architecture proposals

Read [PROJECT_VISION.md](PROJECT_VISION.md), [ARCHITECTURE.md](ARCHITECTURE.md),
[ROADMAP.md](ROADMAP.md) and [STABILITY.md](STABILITY.md) for the affected work.
Discuss major changes in an issue before implementing them. Describe the problem,
a small proposed interface, ownership and dependency boundaries, alternatives,
compatibility/platform effects and validation plan. Explain why a new abstraction,
crate or dependency is better than a smaller internal change.

The aim is modular internals with an integrated public experience. Keep ordinary
`Game`/`Ctx` workflows usable, preserve optional features and headless operation,
and avoid unrelated rewrites. The proposed parallel system scheduler remains
optional; it needs sound access boundaries and explicit ordering/determinism rules,
never concurrent unrestricted mutable access to `Ctx` or `World`.

## Reproductions and GPU evidence

A useful bug report includes a minimal runnable example or failing test, commands,
engine revision, Rust version, feature flags, expected and actual results, and logs.
For nondeterministic failures, include the seed or input sequence when available.
For rendering, add OS, GPU/driver/backend, resolution and quality settings, plus
screenshots when safe to share. Never attach private project assets or secrets.

For GPU changes, run relevant offscreen/visual tests and inspect comparable
before/after images. Exercise capability fallbacks and resize/history behavior
when affected. Record skipped tests and unsupported adapters; a headless logic
test does not establish rendering correctness. If hardware is unavailable, say
which validation is missing in the PR so reviewers can help.

The offscreen GPU suites live in `src/render/*_render_test.rs`. Their shared
conventions (getting the adapter with a logged skip reason, tolerance-based image
comparison and semantic pixel checks, reference images with metadata under
`tests/fixtures/render/`) are documented in `src/render/render_test_support.rs`.
Run them with `cargo test -p ponko --lib --features 3d render_test -- --nocapture`
and report which ran and which logged `render test SKIPPED`. Reference images are
only rewritten with `PONKO_UPDATE_REFERENCES=1` (refused in CI); a regenerated
reference needs the before/after images in the PR.

Performance PRs should record the revision, commands, toolchain, features, hardware,
workload, warm-up and comparable before/after results. Include CPU and GPU times,
frame-time percentiles, memory and allocations where relevant, and identify metrics
that could not be collected. Verify correctness independently; never generalize
one benchmark into an overall engine ranking.

## Using coding agents

Coding agents are welcome under the same contribution rules. You remain responsible
for understanding and reviewing their changes, verifying APIs and dependencies,
running the required checks, and keeping the change scoped. Use existing MCP tools
for runtime inspection, screenshots, stepping or profiling when helpful; retain
reproducible commands/tests and explain what the evidence establishes. Do not treat
agent narration as proof. Report incomplete work, test skips and platform limitations,
and distinguish implemented behavior from proposals. Keep private code and assets
out of public submissions and external services.

## License

By contributing, you agree that your contributions are dual licensed under
[MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE), like the rest of the
project, without any additional terms or conditions.
