# Contributing

The rules for humans and coding agents are the same and live in
[`AGENTS.md`](https://github.com/bozoslav/crabbyengine/blob/main/AGENTS.md).
In short:

- Work on a branch, open a pull request to `main`, wait for CI.
- New public behavior gets a test in the matching area file, and every
  public item gets a doctest.
- Performance claims need release-build numbers in the PR.
- Features that change how games are written update this book in the same PR.

## Editing this book

The book is plain Markdown in `book/src/`, built with
[mdBook](https://rust-lang.github.io/mdBook/):

```sh
cargo install mdbook --locked
mdbook serve book --open
```

Every page has an edit button (top right) that opens the file on GitHub.
