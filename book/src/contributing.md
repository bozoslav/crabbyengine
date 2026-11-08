# Contributing

Start with [`CONTRIBUTING.md`](https://github.com/bozoslav/ponko/blob/main/CONTRIBUTING.md).
The full rules, the same for humans and coding agents, are in
[`AGENTS.md`](https://github.com/bozoslav/ponko/blob/main/AGENTS.md).
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

## License

Crabby Engine is dual licensed under
[MIT](https://github.com/bozoslav/ponko/blob/main/LICENSE-MIT) or
[Apache-2.0](https://github.com/bozoslav/ponko/blob/main/LICENSE-APACHE),
at your option. Contributions are accepted under the same terms.
