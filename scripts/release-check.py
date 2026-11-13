#!/usr/bin/env python3
"""Refuse a release whose tag, crate versions and CHANGELOG section disagree.

    release-check.py check vX.Y.Z   the release workflow runs this before it
                                    builds or publishes anything
    release-check.py notes vX.Y.Z FILE
                                    write that version's CHANGELOG section,
                                    the GitHub release's notes, to FILE
    release-check.py published      every version of our crates on crates.io
                                    has a git tag (a scheduled workflow runs it)
    release-check.py selftest       check the decisions with made-up input

`check` fails unless all of these hold:

- the tag is `vX.Y.Z` (or `vX.Y.Z-suffix`, a pre-release);
- every publishable crate (no `publish = false`) is at X.Y.Z, and so are the
  `version = "..."` requirements it puts on its sibling crates;
- CHANGELOG.md has a `## [X.Y.Z]` section with something in it, and still
  has its `## [Unreleased]` heading;
- that section fits in a GitHub release body (at most 125,000 characters),
  since it becomes the release's notes;
- no changelog.d/ fragment is left (the release folds them in first);
- a release that lists `### Breaking changes` is one that may break (0.y.0,
  x.0.0: never a patch release) and has its migration chapter,
  book/src/migrating-to-X.Y.md (`scripts/changelog.py --version X.Y.Z`
  generates it with the fold), listed in book/src/SUMMARY.md and holding every
  breaking entry of the section;
- every publishable crate under crates/ carries copies of the root
  LICENSE-MIT, LICENSE-APACHE and NOTICE (crates.io gets each crate's own
  folder), identical to the root ones; `selftest` checks this too, in CI;
- with `--on-main`: the tagged commit is an ancestor of origin/main, i.e. it
  was merged through a pull request (the workflow checks out the full history
  for this). Whether CI passed on it is not looked at here.

`published` asks the crates.io API (one read request per crate) and compares
with `git tag`; it needs no token.

See "Releasing" in CONTRIBUTING.md for the whole procedure.
"""
import json
import re
import subprocess
import sys
import tempfile
import tomllib
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))
import changelog  # noqa: E402  (the version rules and the migration chapter's name)

TAG = re.compile(r"^v(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)$")
# GitHub refuses a release whose body is longer than this many characters.
MAX_NOTES = 125_000


def crates(root=ROOT):
    """(name, version, manifest dict) of every publishable crate of the workspace."""
    found = []
    for manifest in [root / "Cargo.toml", *sorted(root.glob("crates/*/Cargo.toml"))]:
        data = tomllib.loads(manifest.read_text())
        package = data.get("package")
        if package is None or package.get("publish") is False:
            continue
        found.append((package["name"], package["version"], data))
    return found


LICENSE_FILES = ("LICENSE-MIT", "LICENSE-APACHE", "NOTICE")


def license_problems(root=ROOT):
    """Publishable sub-crates whose license files are missing or differ from the root's."""
    out = []
    for manifest in sorted(root.glob("crates/*/Cargo.toml")):
        package = tomllib.loads(manifest.read_text()).get("package")
        if package is None or package.get("publish") is False:
            continue
        for name in LICENSE_FILES:
            copy = manifest.parent / name
            if not copy.is_file():
                out.append(f"{package['name']} has no {name} (copy the root one into {manifest.parent.name}/)")
            elif copy.read_bytes() != (root / name).read_bytes():
                out.append(f"{manifest.parent.name}/{name} differs from the root {name}: copy it again")
    return out


def sibling_requirements(data, names):
    """(dependency, version requirement) of every dependency on a sibling crate."""
    out = []
    for table in ("dependencies", "dev-dependencies", "build-dependencies"):
        for name, spec in data.get(table, {}).items():
            if name in names and isinstance(spec, dict) and "version" in spec:
                out.append((name, spec["version"]))
    return out


def changelog_section(text, version):
    """The lines of `## [version]` in a changelog, or None if there is no section."""
    lines = text.splitlines()
    start = next((i for i, l in enumerate(lines) if l.startswith(f"## [{version}]")), None)
    if start is None:
        return None
    end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith("## [")), len(lines))
    return [l for l in lines[start + 1 : end] if not re.match(r"^\[[^\]]*\]: ", l)]


def release_notes(section):
    """A changelog section's lines as the text of a GitHub release body."""
    return "\n".join(section).strip() + "\n"


def breaking_headlines(section):
    """The bullets of the `### Breaking changes` subsection of a changelog section."""
    found, inside = [], False
    for line in section:
        if line.startswith("### "):
            inside = line[4:].strip() == changelog.BREAKING
        elif inside and line.startswith("- "):
            found.append(line[2:].strip())
    return found


def migration_problems(version, section, chapter, summary):
    """What is wrong with the migration chapter of `version`, as messages.

    section: the lines of its changelog section; chapter: the text of
    book/src/migrating-to-X.Y.md or None if there is none; summary: the text
    of book/src/SUMMARY.md.
    """
    headlines = breaking_headlines(section)
    if not headlines:
        return []
    if not changelog.may_break(version):
        return [f"CHANGELOG.md's `## [{version}]` section lists breaking changes, but {version} "
                "is a patch release and must not break the API (STABILITY.md): release it as the "
                "next minor"]
    name = changelog.chapter_name(version)
    if chapter is None:
        return [f"{version} has breaking changes but book/src/{name} does not exist: run "
                f"`python3 scripts/changelog.py --version {version}` with the fragments in place, "
                "which writes it from the same entries"]
    out = []
    if f"({name})" not in summary:
        out.append(f"book/src/SUMMARY.md has no line for {name}")
    missing = [h for h in headlines if h.rstrip(".") not in chapter]
    if missing:
        shown = "; ".join(f"`{h[:50]}`" for h in missing[:3]) + (" ..." if len(missing) > 3 else "")
        out.append(f"book/src/{name} lacks {len(missing)} breaking change(s) of the CHANGELOG: {shown}")
    return out


def problems(tag, crate_list, changelog_text, fragments, book=None):
    """What is wrong with releasing `tag`, as a list of messages (empty: fine).

    book: (the migration chapter's text or None, SUMMARY.md's text), or None
    to skip the migration check.
    """
    found = TAG.match(tag)
    if not found:
        return [f"`{tag}` is not a release tag like v1.2.3"]
    version = found.group(1)
    out = []
    names = {name for name, _, _ in crate_list}
    for name, crate_version, data in crate_list:
        if crate_version != version:
            out.append(f"{name} is at {crate_version}, not {version}")
        for dep, req in sibling_requirements(data, names):
            if req.lstrip("=^~ ") != version:
                out.append(f"{name} requires {dep} {req}, not {version}")
    section = changelog_section(changelog_text, version)
    if section is None:
        out.append(f"CHANGELOG.md has no `## [{version}]` section")
    elif not any(l.strip() for l in section):
        out.append(f"CHANGELOG.md's `## [{version}]` section is empty")
    elif len(release_notes(section)) > MAX_NOTES:
        out.append(
            f"CHANGELOG.md's `## [{version}]` section has {len(release_notes(section)):,} characters; "
            f"a GitHub release body holds at most {MAX_NOTES:,}. Shorten it (summarize, and link the "
            "full lists in the guide)"
        )
    if section and book is not None:
        out += migration_problems(version, section, *book)
    if not any(l.startswith("## [Unreleased]") for l in changelog_text.splitlines()):
        out.append("CHANGELOG.md lost its `## [Unreleased]` heading")
    if fragments:
        shown = ", ".join(sorted(fragments)[:3]) + (" ..." if len(fragments) > 3 else "")
        out.append(
            f"changelog.d/ still has {len(fragments)} fragment(s) (run scripts/changelog.py "
            f"first): {shown}"
        )
    return out


def untagged(published, tags):
    """The (crate, version) pairs on crates.io whose `vVERSION` tag does not exist."""
    return sorted((name, v) for name, versions in published.items() for v in versions if f"v{v}" not in tags)


def git(*args, repo=ROOT):
    return subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True)


def main_problems(tag, repo=ROOT):
    """Problems with `tag` not being a commit merged to origin/main (fetched first)."""
    if git("fetch", "--quiet", "origin", "main", repo=repo).returncode != 0:
        return ["could not fetch origin/main to check the tag against it"]
    if git("rev-parse", "--verify", "--quiet", f"{tag}^{{commit}}", repo=repo).returncode != 0:
        return [f"{tag} does not exist in this checkout"]
    if git("merge-base", "--is-ancestor", f"{tag}^{{commit}}", "origin/main", repo=repo).returncode != 0:
        return [f"{tag} is not on origin/main: tag a commit that was merged to main"]
    return []


def check(tag, on_main):
    fragments = [p.name for p in (ROOT / "changelog.d").glob("*.md") if p.name != "README.md"]
    found = TAG.match(tag)
    book = None
    if found:
        chapter = ROOT / "book" / "src" / changelog.chapter_name(found.group(1))
        book = (chapter.read_text() if chapter.is_file() else None,
                (ROOT / "book" / "src" / "SUMMARY.md").read_text())
    found = problems(tag, crates(), (ROOT / "CHANGELOG.md").read_text(), fragments, book) + license_problems()
    if on_main:
        found += main_problems(tag)
    if found:
        print(f"Not releasing {tag}:")
        for message in found:
            print(f"  - {message}")
        return 1
    print(f"{tag}: crate versions, CHANGELOG section and fragments agree.")
    return 0


def notes(tag, out):
    """Writes the release notes of `tag` to `out` (the same checks as `check`)."""
    found = TAG.match(tag)
    if not found:
        sys.exit(f"`{tag}` is not a release tag like v1.2.3")
    version = found.group(1)
    section = changelog_section((ROOT / "CHANGELOG.md").read_text(), version)
    if section is None or not any(l.strip() for l in section):
        sys.exit(f"CHANGELOG.md has no notes for {version}")
    text = release_notes(section)
    if len(text) > MAX_NOTES:
        sys.exit(f"the notes for {version} have {len(text):,} characters; GitHub allows {MAX_NOTES:,}")
    Path(out).write_text(text, encoding="utf-8")
    print(f"wrote {out} ({len(text):,} characters)")
    return 0


def fetch_published(names):
    out = {}
    for name in names:
        request = urllib.request.Request(
            f"https://crates.io/api/v1/crates/{name}", headers={"User-Agent": "ponko-release-check"}
        )
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                data = json.load(response)
        except urllib.error.HTTPError as error:
            if error.code == 404:  # never published
                continue
            sys.exit(f"crates.io answered {error.code} for {name}; try again later")
        except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as error:
            sys.exit(f"could not ask crates.io about {name}: {error}")
        out[name] = [v["num"] for v in data["versions"]]
    return out


def published():
    published_versions = fetch_published([name for name, _, _ in crates()])
    tags = set(git("tag", "--list", "v*").stdout.split())
    missing = untagged(published_versions, tags)
    for name, version in missing:
        print(f"{name} {version} is on crates.io but v{version} is not a git tag")
    if missing:
        print("Tag the commit it was published from (see \"Releasing\" in CONTRIBUTING.md).")
        return 1
    print("Every published version has a git tag.")
    return 0


def on_main_selftest():
    """main_problems against a throw-away origin and clone (deleted afterwards)."""
    with tempfile.TemporaryDirectory(prefix="release-check-") as tmp:
        tmp = Path(tmp)

        def run(*args, cwd):
            subprocess.run(
                ["git", "-c", "user.name=t", "-c", "user.email=t@example.com",
                 "-c", "commit.gpgsign=false", "-c", "tag.gpgsign=false", *args],
                cwd=cwd, check=True, capture_output=True,
            )

        run("init", "--bare", "--initial-branch=main", "origin.git", cwd=tmp)
        work = tmp / "work"
        run("clone", "origin.git", "work", cwd=tmp)
        run("checkout", "-b", "main", cwd=work)
        run("commit", "--allow-empty", "-m", "merged", cwd=work)
        run("tag", "v0.0.1", cwd=work)
        run("push", "origin", "main", cwd=work)
        run("checkout", "-b", "feature", cwd=work)
        run("commit", "--allow-empty", "-m", "never merged", cwd=work)
        run("tag", "v0.0.2", cwd=work)

        assert main_problems("v0.0.1", work) == [], "a tag on a merged commit passes"
        found = main_problems("v0.0.2", work)
        assert found and "is not on origin/main" in found[0], "a tag on an unmerged branch is refused"
        assert "does not exist" in main_problems("v9.9.9", work)[0]
        run("remote", "set-url", "origin", str(tmp / "missing.git"), cwd=work)
        assert "could not fetch" in main_problems("v0.0.1", work)[0], "no main to compare with is refused"


def selftest():
    def toml(name, version, deps=""):
        return tomllib.loads(f'[package]\nname = "{name}"\nversion = "{version}"\n[dependencies]\n{deps}')

    sibling = 'ponko_ecs = { path = "crates/ponko_ecs", version = "0.0.2" }\nserde = "1"\n'
    ok = [("ponko", "0.0.2", toml("ponko", "0.0.2", sibling)), ("ponko_ecs", "0.0.2", toml("ponko_ecs", "0.0.2"))]
    log = "## [Unreleased]\n\n### Added\n- next\n\n## [0.0.2] - 2026-12-01\n\n### Added\n- thing\n\n## [0.0.1] - x\n- old\n"

    assert problems("v0.0.2", ok, log, []) == [], "a consistent release passes"
    assert problems("v0.0.2-rc.1", ok, log, [])[0].startswith("ponko is at 0.0.2, not 0.0.2-rc.1")
    assert problems("0.0.2", ok, log, []) == ["`0.0.2` is not a release tag like v1.2.3"]
    assert any("ponko is at 0.0.2, not 0.0.3" in p for p in problems("v0.0.3", ok, log, [])), (
        "a tag ahead of the crate versions is refused"
    )
    assert any("has no `## [0.0.3]` section" in p for p in problems("v0.0.3", ok, log, []))
    lagging = [ok[0], ("ponko_ecs", "0.0.1", toml("ponko_ecs", "0.0.1"))]
    assert any("ponko_ecs is at 0.0.1" in p for p in problems("v0.0.2", lagging, log, []))
    stale = [("ponko", "0.0.2", toml("ponko", "0.0.2", sibling.replace("0.0.2", "0.0.1"))), ok[1]]
    assert any("requires ponko_ecs 0.0.1" in p for p in problems("v0.0.2", stale, log, [])), (
        "a sibling requirement must move with the version"
    )
    empty = "## [Unreleased]\n\n## [0.0.2] - x\n\n[0.0.2]: https://example.com\n"
    assert any("section is empty" in p for p in problems("v0.0.2", ok, empty, [])), (
        "link lines are not release notes"
    )
    assert any("lost its" in p for p in problems("v0.0.2", ok, "## [0.0.2]\n- x\n", []))
    assert any("1 fragment(s)" in p for p in problems("v0.0.2", ok, log, ["12-x.md"]))
    assert changelog_section(log, "0.0.1") == ["- old"] or changelog_section(log, "0.0.1")[-1] == "- old"
    huge = log.replace("- thing\n", "- thing\n" + "- x\n" * (MAX_NOTES // 4 + 1))
    assert any("a GitHub release body holds at most" in p for p in problems("v0.0.2", ok, huge, [])), (
        "notes longer than a GitHub release body are refused before anything is built"
    )
    assert release_notes(changelog_section(log, "0.0.2")) == "### Added\n- thing\n"

    # A release with breaking changes needs its migration chapter.
    breaking_log = log.replace(
        "### Added\n- thing\n",
        "### Breaking changes\n\nSee the guide.\n\n- `A` is `B` now.\n  Instead: use `B`.\n- `C` is gone.\n\n### Added\n- thing\n",
    )
    section = changelog_section(breaking_log, "0.0.2")
    assert breaking_headlines(section) == ["`A` is `B` now.", "`C` is gone."]
    assert breaking_headlines(changelog_section(log, "0.0.2")) == []
    chapter = "# Migrating\n## `A` is `B` now\n## `C` is gone\n"
    summary = "- [Migrating](migrating-to-0.0.md)\n"
    assert migration_problems("0.0.2", section, chapter, summary) != [], "0.0.2 is a patch release"
    assert "patch release" in migration_problems("0.0.2", section, chapter, summary)[0]
    minor = section
    assert migration_problems("0.3.0", minor, None, "")[0].endswith("which writes it from the same entries"), (
        "a minor release with breaks and no chapter is refused"
    )
    assert "migrating-to-0.3.md does not exist" in migration_problems("0.3.0", minor, None, "")[0]
    good_summary = "- [Migrating from 0.2 to 0.3](migrating-to-0.3.md)\n"
    assert migration_problems("0.3.0", minor, chapter, good_summary) == []
    assert migration_problems("0.3.0-rc.1", minor, chapter, good_summary) == [], "a candidate needs it too, same chapter"
    assert any("no line for migrating-to-0.3.md" in p for p in migration_problems("0.3.0", minor, chapter, ""))
    assert any("lacks 1 breaking change" in p and "`C` is gone" in p
               for p in migration_problems("0.3.0", minor, chapter.replace("## `C` is gone\n", ""), good_summary))
    assert migration_problems("0.3.0", changelog_section(log, "0.0.2"), None, "") == [], "no breaks, no chapter needed"
    # Through problems(): the chapter is looked at only when a book is given.
    three = [("ponko", "0.3.0", toml("ponko", "0.3.0"))]
    log3 = breaking_log.replace("[0.0.2]", "[0.3.0]")
    assert problems("v0.3.0", three, log3, []) == [], "skipped without a book"
    assert any("does not exist" in p for p in problems("v0.3.0", three, log3, [], (None, "")))
    assert problems("v0.3.0", three, log3, [], (chapter, good_summary)) == []
    assert untagged({"ponko": ["0.0.1", "0.0.2"], "ponko_ecs": ["0.0.1"]}, {"v0.0.1"}) == [
        ("ponko", "0.0.2")
    ], "a published version needs its tag"
    assert untagged({"ponko": ["0.0.1"]}, {"v0.0.1", "v1.0.0"}) == []
    on_main_selftest()
    with tempfile.TemporaryDirectory(prefix="release-check-") as tmp:
        tmp = Path(tmp)
        (tmp / "crates" / "a").mkdir(parents=True)
        (tmp / "crates" / "a" / "Cargo.toml").write_text('[package]\nname = "a"\nversion = "0.0.2"\n')
        for name in LICENSE_FILES:
            (tmp / name).write_text(name)
            (tmp / "crates" / "a" / name).write_text(name)
        assert license_problems(tmp) == [], "identical copies pass"
        (tmp / "crates" / "a" / "NOTICE").write_text("old")
        assert "a/NOTICE differs" in license_problems(tmp)[0], "a stale copy is refused"
        (tmp / "crates" / "a" / "NOTICE").unlink()
        assert "a has no NOTICE" in license_problems(tmp)[0], "a missing copy is refused"
    real = license_problems()
    assert real == [], f"the sub-crates' license files: {real}"
    names = {n for n, _, _ in crates()}
    assert "ponko" in names and "ponko_ecs" in names, "reads the real workspace"
    print("release-check.py: self-test passed")


def main():
    command = sys.argv[1] if len(sys.argv) > 1 else ""
    if command == "check" and len(sys.argv) > 2:
        return check(sys.argv[2], "--on-main" in sys.argv)
    if command == "notes" and len(sys.argv) > 3:
        return notes(sys.argv[2], sys.argv[3])
    if command == "published":
        return published()
    if command == "selftest":
        return selftest()
    sys.exit(__doc__)


if __name__ == "__main__":
    sys.exit(main())
