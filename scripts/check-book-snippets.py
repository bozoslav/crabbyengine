#!/usr/bin/env python3
"""Compile the guide's inline Rust snippets, so a rename can't silently break them.

Every ```rust block in book/src is extracted into one scratch crate (one module
per block, one file per chapter) and checked with a single `cargo check`.

How a block is compiled (the same idea as rustdoc, kept deliberately small):

- Lines starting with `# ` (or a lone `#`) are mdBook's hidden lines: the book
  hides them, the checker keeps them (minus the `# `). Use them for context the
  reader doesn't need: `# struct Health(f32);`, `# let steer = Vec2::ZERO;`.
  `##` at the start of a line stands for a literal `#`.
- `use ponko::prelude::*;` is in scope, and the body sits in
  `fn snippet(ctx: &mut Ctx, world: &mut World) -> Result<(), Box<dyn Error>>`,
  so snippets can use `ctx`, `world` and `?` without setup. A block that
  defines its own `fn main` is used as it is, without the wrapper.
- `no_run` and `should_panic` blocks are compiled too (nothing is ever run).
- A block that is only an `{{#include ...}}` is skipped: the included file
  is compiled by `cargo build --examples` / `cargo test`.
- `ignore` is the escape hatch (at most MAX_IGNORED blocks, so use it rarely):
  the block must say why in a comment line, `// ignore: <reason>` (put it on
  a hidden line, `# // ignore: needs a GPU`, to keep it out of the book).
- Warnings such as unused variables are allowed; deprecated items are not.
  Most deprecated root spellings are re-exports, which rustc does not warn
  about, so the checker also rejects `use ponko::*;` and any root path
  (`ponko::Name`, `use ponko::{Name, ..}`) to a name in the naming chapter's
  list of deprecated root spellings, in snippets and in inline code in the
  prose (except the naming chapter itself, which lists them).

Generated files keep the markdown's line numbers, so a compiler error at
`src/physics.rs:40` is `book/src/physics.md:40`.

Usage: scripts/check-book-snippets.py [--features a,b] [--no-compile] [--keep]
"""

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BOOK = ROOT / "book" / "src"
MAX_IGNORED = 3
# Everything except the mutually exclusive or tooling-only features.
DEFAULT_FEATURES = "3d,physics,physics3d,navmesh,steam"

FENCE = re.compile(r"^(\s*)(`{3,}|~{3,})\s*([^`]*)$")
IGNORE_REASON = re.compile(r"//\s*ignore:\s*\S")
LOCAL_ATTRS = ("no_run", "should_panic", "ignore", "compile_fail")


def extract(path):
    """Yield (open_line, close_line, attrs, code_lines) per Rust block (1-based lines)."""
    lines = path.read_text().split("\n")
    i = 0
    while i < len(lines):
        m = FENCE.match(lines[i])
        if not m:
            i += 1
            continue
        indent, fence, info = m.groups()
        close = re.compile(r"^\s*" + fence[0] + "{" + str(len(fence)) + r",}\s*$")
        j = i + 1
        while j < len(lines) and not close.match(lines[j]):
            j += 1
        if j >= len(lines):
            sys.exit(f"{path}:{i + 1}: unterminated code fence")
        attrs = [a for a in re.split(r"[,\s]+", info.strip()) if a]
        if attrs and attrs[0] == "rust":
            body = [l[len(indent):] if l.startswith(indent) else l.lstrip() for l in lines[i + 1 : j]]
            yield i + 1, j + 1, attrs[1:], body
        i = j + 1


def unhide(line):
    """mdBook hidden lines: `# code` -> `code`, `#` -> ``, `## x` -> `# x`."""
    if line == "#":
        return ""
    if line.startswith("# "):
        return line[2:]
    if line.startswith("##"):
        return line[1:]
    return line


NAMING = BOOK / "naming.md"
LIST_START = "They stay until the next\nmajor release:"
LIST_END = "What does warn"


def deprecated_root_names():
    """The deprecated root re-exports listed in the naming chapter (old spellings only)."""
    text = NAMING.read_text()
    if LIST_START not in text or LIST_END not in text:
        sys.exit(f"{NAMING}: the list of deprecated root spellings is missing (looked for {LIST_START!r})")
    section = text.split(LIST_START, 1)[1].split(LIST_END, 1)[0]
    names = set()
    for line in section.splitlines():
        m = re.match(r"^- `\w+`: (.*)$", line)
        if not m:
            continue
        # Drop the new spellings in parentheses: `Collider3D` (`d3::Collider`).
        names.update(re.findall(r"`(\w+)`", re.sub(r"\(`[^)]*`\)", "", m.group(1))))
    if len(names) < 50:
        sys.exit(f"{NAMING}: found only {len(names)} deprecated root spellings; has the list's format changed?")
    return names


ROOT_PATH = re.compile(r"(?<![\w:])(?:::)?ponko::(\w+|\*|\{)")


def root_path_names(code):
    """Top-level names a piece of code reaches through the crate root (`*` for the root glob)."""
    found = []
    for m in ROOT_PATH.finditer(code):
        head = m.group(1)
        if head != "{":
            found.append(head)
            continue
        depth, i, start = 1, m.end(), m.end()
        while i < len(code) and depth:
            depth += {"{": 1, "}": -1}.get(code[i], 0)
            i += 1
        inner, depth, part = code[start : i - 1], 0, ""
        parts = []
        for ch in inner + ",":
            if ch == "," and depth == 0:
                parts.append(part.strip())
                part = ""
                continue
            depth += {"{": 1, "}": -1}.get(ch, 0)
            part += ch
        found.extend(re.match(r"(\w+|\*)", p).group(1) for p in parts if re.match(r"(\w+|\*)", p))
    return found


def old_spellings(code, deprecated):
    """Deprecated root spellings (and the root glob) used by `code`."""
    bad = []
    for name in root_path_names(code):
        if name == "*":
            bad.append("use ponko::*; (use ponko::prelude::*; and module imports)")
        elif name in deprecated:
            bad.append(f"ponko::{name}")
    return bad


def check_prose(md, deprecated):
    """Inline code in the prose names no deprecated root spelling."""
    problems = []
    in_fence = None
    for n, line in enumerate(md.read_text().split("\n"), 1):
        m = FENCE.match(line)
        if m:
            in_fence = None if in_fence else m.group(2)[0]
            continue
        if in_fence:
            continue
        for span in re.findall(r"`([^`]+)`", line):
            for bad in old_spellings(span, deprecated):
                problems.append(f"book/src/{md.name}:{n}: `{span}` uses the deprecated root spelling {bad}")
    return problems


def generate(out_src, features):
    """Write src/<chapter>.rs for every chapter with blocks. Returns (checked, skipped, ignored, problems)."""
    modules = []
    checked = skipped = 0
    ignored = []
    problems = []
    deprecated = deprecated_root_names()
    for md in sorted(BOOK.glob("*.md")):
        if md != NAMING:
            problems.extend(check_prose(md, deprecated))
        out = []  # generated lines; len(out) is the line number reached
        for start, end, attrs, body in extract(md):
            rel = f"book/src/{md.name}:{start}"
            if "ignore" in attrs:
                ignored.append(rel)
                if not any(IGNORE_REASON.search(l) for l in body):
                    problems.append(f"{rel}: `ignore` block without a `// ignore: <reason>` comment")
                continue
            if "compile_fail" in attrs:
                problems.append(f"{rel}: `compile_fail` blocks aren't supported; remove the attribute or use `ignore`")
                continue
            if any(l.lstrip().startswith("{{#") for l in body):
                skipped += 1
                continue
            code = [unhide(l) for l in body]
            for bad in sorted(set(old_spellings("\n".join(code), deprecated))):
                problems.append(f"{rel}: uses the deprecated root spelling {bad}; see the naming chapter")
            has_main = any(re.search(r"\bfn\s+main\s*\(", l) for l in code)
            allow = "#![allow(warnings)] #![deny(deprecated)]"
            if has_main:
                header = f"mod b{start} {{ {allow} use ponko::prelude::*;"
                footer = "}"
            else:
                header = (
                    f"mod b{start} {{ {allow} use ponko::prelude::*; "
                    "fn snippet(ctx: &mut Ctx, world: &mut World) -> Result<(), Box<dyn std::error::Error>> {"
                )
                footer = ";Ok(()) } }"
            # The header sits on the opening fence's line, the footer on the closing one,
            # so every code line keeps its markdown line number.
            out.extend([""] * (start - 1 - len(out)))
            out.append(header)
            out.extend(code)
            out.append(footer)
            checked += 1
            modules.append((md.stem, md.name))
        if out:
            (out_src / f"{md.stem}.rs").write_text("\n".join(out) + "\n")
    if len(ignored) > MAX_IGNORED:
        problems.append(f"{len(ignored)} `ignore` blocks ({', '.join(ignored)}); at most {MAX_IGNORED} are allowed")
    stems = sorted({stem for stem, _ in modules})
    lib = ["// Generated by scripts/check-book-snippets.py; do not edit."]
    for stem in stems:
        ident = re.sub(r"\W", "_", stem)
        lib.append(f'#[path = "{stem}.rs"] mod c_{ident};')
    (out_src / "lib.rs").write_text("\n".join(lib) + "\n")
    return checked, skipped, ignored, problems


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--features", default=DEFAULT_FEATURES, help="ponko features to enable (default: %(default)s)")
    ap.add_argument("--no-compile", action="store_true", help="only extract and validate the blocks")
    ap.add_argument("--keep", action="store_true", help="keep the scratch crate's files (always kept under target/)")
    args = ap.parse_args()

    target = Path(os.environ.get("CARGO_TARGET_DIR") or ROOT / "target")
    crate = target / "book-snippets"
    src = crate / "src"
    src.mkdir(parents=True, exist_ok=True)
    for old in src.glob("*.rs"):
        old.unlink()

    checked, skipped, ignored, problems = generate(src, args.features)
    features = ", ".join(f'"{f}"' for f in args.features.split(",") if f)
    (crate / "Cargo.toml").write_text(
        f"""# Generated by scripts/check-book-snippets.py; do not edit.
[package]
name = "book_snippets"
version = "0.0.0"
edition = "2024"
publish = false

[lib]
path = "src/lib.rs"

[dependencies]
ponko = {{ path = "{ROOT.as_posix()}", features = [{features}] }}
ponko_unity = {{ path = "{ROOT.as_posix()}/crates/ponko_unity" }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"

[workspace]
"""
    )
    # The same dependency versions as the workspace, so the build cache is shared.
    lock = ROOT / "Cargo.lock"
    if lock.exists():
        (crate / "Cargo.lock").write_text(lock.read_text())

    print(f"guide snippets: {checked} to compile, {skipped} included from real files, {len(ignored)} ignored")
    for p in problems:
        print(f"error: {p}", file=sys.stderr)
    if problems:
        return 1
    if args.no_compile:
        return 0

    cmd = ["cargo", "check", "--manifest-path", str(crate / "Cargo.toml"), "--target-dir", str(target), "--color", "never"]
    result = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    if result.returncode == 0:
        sys.stderr.write(result.stderr.splitlines()[-1] + "\n" if result.stderr else "")
        print("all guide snippets compile")
        return 0
    # Point errors at the markdown: src/physics.rs:40 is book/src/physics.md:40.
    text = re.sub(r"(--> )(?:\S*book-snippets/)?src/([\w.-]+)\.rs:", r"\1book/src/\2.md:", result.stderr)
    sys.stderr.write(text)
    print("\nerror: some guide snippets don't compile (paths above are book/src/<chapter>.md:<line>)", file=sys.stderr)
    print("Fix the snippet, or add hidden context lines (`# let x = ...;`); see book/src/contributing.md.", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
