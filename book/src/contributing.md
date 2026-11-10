# Contributing

Start with [`CONTRIBUTING.md`](https://github.com/ponkoengine/ponko/blob/main/CONTRIBUTING.md).
The detailed maintainer/agent integration checklist is in
[`AGENTS.md`](https://github.com/ponkoengine/ponko/blob/main/AGENTS.md).
In short:

- Work on a branch, open a pull request to `main`, wait for CI.
- Add relevant tests for changed behavior and helpful nearby API comments.
  You do not need a doctest for every item or unrelated local toolchains.
- Performance claims need release-build numbers in the PR.
- Rust-only changes are welcome; guide chapters and C/C++ bindings are not
  required from every contributor. Maintainers coordinate that coverage and
  own follow-ups. Preserve existing compatibility and describe behavior and tests.

## Testing the C API

Every function the C library exports (`extern "C" fn` in `crates/ponko_ffi/src`)
needs a test that calls it, in `crates/ponko_ffi/tests/` or a native suite in
`examples/cpp/`: a doctest that passes a null pointer proves argument checking,
not that the function works. CI runs `python3 scripts/check-ffi-coverage.py`,
which fails for an export no test calls. The only exceptions are listed, each
with the reason it can't run on a machine without a window or a Steam client,
in `scripts/ffi-coverage-allowlist.txt`. Test the function instead of adding to
that list; the check also fails when a listed function gets a test, so the
list only shrinks.

## Editing this book

The book is plain Markdown in `book/src/`, built with
[mdBook](https://rust-lang.github.io/mdBook/):

```sh
cargo install mdbook --locked
mdbook serve book --open
```

Every page has an edit button (top right) that opens the file on GitHub.

### Rust snippets compile

CI compiles every ` ```rust ` block in the book (`scripts/check-book-snippets.py`,
one `cargo check` for all of them), so a rename can't silently break the guide.
Run it before pushing a chapter change:

```sh
python3 scripts/check-book-snippets.py
```

A block is wrapped in `fn snippet(ctx: &mut Ctx, world: &mut World) -> Result<..>`
with `use ponko::prelude::*;` in scope, so it can use `ctx`, `world` and `?` directly.
Anything else it needs goes on hidden lines, which mdBook doesn't show and the
checker keeps (`# ` prefix, as in rustdoc):

````md
```rust
# struct Health(f32);
# let enemy = ctx.world.spawn((Health(10.0),));
ctx.world.despawn(enemy);
```
````

- `no_run` and `should_panic` blocks are compiled too; nothing is ever run.
- A block that is only an `\{{#include}}` of an example file is skipped:
  that file is compiled by `cargo build --examples` and `cargo test`. Prefer an
  include for anything longer than a few lines.
- ` ```rust,ignore ` is a last resort (at most three in the whole book) and
  needs a reason in a comment, ideally hidden: `# // ignore: needs a GPU`.
- The check enables the `3d`, `physics`, `physics3d`, `navmesh` and `steam`
  features, so snippets for those compile without extra setup.

## License

Ponko is dual licensed under
[MIT](https://github.com/ponkoengine/ponko/blob/main/LICENSE-MIT) or
[Apache-2.0](https://github.com/ponkoengine/ponko/blob/main/LICENSE-APACHE),
at your option. Contributions are accepted under the same terms.
