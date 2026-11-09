### Changed
- CI compiles the guide's inline Rust snippets (`scripts/check-book-snippets.py`),
  so an API rename can't silently break the book. Snippets get their context from
  hidden `# ` lines; see "Rust snippets compile" in the contributing chapter.
