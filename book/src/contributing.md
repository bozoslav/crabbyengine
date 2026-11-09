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

## Editing this book

The book is plain Markdown in `book/src/`, built with
[mdBook](https://rust-lang.github.io/mdBook/):

```sh
cargo install mdbook --locked
mdbook serve book --open
```

Every page has an edit button (top right) that opens the file on GitHub.

## License

Ponko is dual licensed under
[MIT](https://github.com/ponkoengine/ponko/blob/main/LICENSE-MIT) or
[Apache-2.0](https://github.com/ponkoengine/ponko/blob/main/LICENSE-APACHE),
at your option. Contributions are accepted under the same terms.
