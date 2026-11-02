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
   - Open the PR with `gh pr create --base main`. The body says what changed and how it was verified (commands run, benchmark numbers).
   - Merge with `gh pr merge --squash --delete-branch`, then `git switch main && git pull`.
   - Only merge when the build, tests and clippy pass.
4. Don't rewrite published history (`push --force` on shared branches, rebasing `main`).

## Before every commit

```sh
cargo build --workspace --examples
cargo test --workspace
cargo clippy --workspace --all-targets
```

If you touched unsafe code in `crates/crabby_ecs`, also run the ECS tests under Miri:

```sh
cd crates/crabby_ecs && cargo +nightly miri test
```

## Project rules

- Performance claims need numbers: measure in release builds and put the results in the PR.
- No per-frame heap allocations in steady state on hot paths.
- Every `unsafe` block needs a test that exercises it.
- Public API: simple first. One obvious way to do each thing.
