# Agent instructions

Rules for any coding agent (Claude Code, Codex, Cursor, ...) working in this repo.

## Git workflow (required)

1. **Never work directly on `main`.** Before changing anything, create a branch from an up-to-date `main`:
   - `git switch main && git pull && git switch -c <type>/<short-name>`
   - Branch names: `m<N>/<topic>` for roadmap milestone work (e.g. `m3/ecs-crate`), `fix/<topic>`, `chore/<topic>`, `docs/<topic>`.
   - One branch per task. Start unrelated work on its own branch (use `git worktree` if another branch is in progress).
2. **Commit and push along the way**, not only at the end. Make a commit at every meaningful, working step (it builds, and tests pass) and push it right away (`git push -u origin <branch>` the first time).
   - Commit messages: short imperative subject line, then a body listing what changed.
   - Never commit `docs/`, which holds local planning notes and is git-ignored. Never commit `.DS_Store`, build output or secrets.
3. **Everything reaches `main` through a pull request.** Never push to `main` and never merge locally into `main`.
   - **One PR per big chunk of work**, not per small feature or fix: keep committing and pushing on the branch, and open the PR when a whole part is done (for example a milestone's worth: reflection + control endpoint + MCP server). Don't stack many small PRs and wait on CI for each.
   - Open the PR with `gh pr create --base main`. The body says what changed and how it was verified (commands run, benchmark numbers).
   - Merge with `gh pr merge --squash --delete-branch`, then `git switch main && git pull`.
   - `main` is protected: merging needs the **CI passed** check (it requires every CI job) and keeps history linear. Wait for CI (`gh pr checks <n> --watch`), then merge. Never bypass protection, even as admin.
   - If CI fails, fix it on the branch and push again; don't merge red.
4. Don't rewrite published history (`push --force` on shared branches, rebasing `main`).

## Tests

Where tests live:

| What | Where | Example |
|---|---|---|
| Public API of a crate | `crates/<crate>/tests/<area>.rs`, one file per area | `crates/crabby_ecs/tests/queries.rs` |
| Shared test helpers | `tests/common/mod.rs` next to them | components used by several ECS test files |
| Private internals (clock math, schedule order) | `#[cfg(test)] mod tests` at the bottom of the file | `src/time.rs` |
| Usage examples in docs | doctests on public items: every public item gets one | `World::spawn` |
| Engine and game logic | headless runs, no window or GPU (`Game::run_headless`) | `tests/` in the root crate |
| Performance | `crates/crabby_bench` (`cargo run --release -p crabby_bench [-- filter]`) | `iter/crabby` |

Rules:
- New public behavior gets a test in the matching area file. A new area gets a new file **and** a line in the CI `test` matrix (`.github/workflows/ci.yml`), so it shows up as its own check.
- Test names say what must hold: `stale_handles_stay_dead`, not `test_entity_2`.
- Panics that are part of the API get `#[should_panic(expected = "...")]` tests, so the message stays helpful.
- Benchmarks are not tests: never assert on timings in tests, except for coarse bounds such as "≥ 6 ms for three 2 ms steps".
- Run one area locally with `cargo test -p crabby_ecs --test queries`.

## Before every commit

```sh
cargo build --workspace --examples
cargo test --workspace
cargo clippy --workspace --all-targets
```

If you touched unsafe code in `crates/crabby_ecs`, also run the ECS tests under Miri:

```sh
cd crates/crabby_ecs
# Our unsafe code, strict model, without rayon:
cargo +nightly miri test --tests --no-default-features
# The parallel path. rayon's dependency crossbeam-epoch trips Stacked Borrows
# (not our code), and its global thread pool outlives main:
MIRIFLAGS="-Zmiri-tree-borrows -Zmiri-ignore-leaks" cargo +nightly miri test --test parallel
```

## Project rules

- Performance claims need numbers: measure in release builds and put the results in the PR.
- No per-frame heap allocations in steady state on hot paths.
- Every `unsafe` block needs a test that exercises it.
- Public API: simple first. One obvious way to do each thing.
- Guide examples come in Rust and C++, side by side: wrap the pair in `<div class="lang-pair">` (see `book/theme/lang-pair.css`), with the C++ snippet pulled from `examples/cpp/guide.cpp` (compiled and run by the native CI job). If C++ can't do it yet, the chapter says so.
- New engine features get a C API (`crates/crabby_ffi`) and a C++ wrapper (`include/crabby.hpp`) in the same PR when they're part of writing games, unless that's much bigger than the feature; then say so in the PR.
- Docs ship with the feature: a PR that adds or changes how games are written updates the guide in `book/src/` in the same PR (check with `mdbook build book`). Code that the guide shows from examples is pulled in with `{{#include}}` anchors, so it can't go stale.
