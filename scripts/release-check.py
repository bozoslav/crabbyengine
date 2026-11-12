#!/usr/bin/env python3
"""Refuse a release whose tag, crate versions and CHANGELOG section disagree.

    release-check.py check vX.Y.Z   the release workflow runs this before it
                                    publishes anything
    release-check.py published      every version of our crates on crates.io
                                    has a git tag (a scheduled workflow runs it)
    release-check.py selftest       check the decisions with made-up input

`check` fails unless all of these hold:

- the tag is `vX.Y.Z` (or `vX.Y.Z-suffix`, a pre-release);
- every publishable crate (no `publish = false`) is at X.Y.Z, and so are the
  `version = "..."` requirements it puts on its sibling crates;
- CHANGELOG.md has a `## [X.Y.Z]` section with something in it, and still
  has its `## [Unreleased]` heading;
- no changelog.d/ fragment is left (the release folds them in first);
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
TAG = re.compile(r"^v(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)$")


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


def problems(tag, crate_list, changelog, fragments):
    """What is wrong with releasing `tag`, as a list of messages (empty: fine)."""
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
    section = changelog_section(changelog, version)
    if section is None:
        out.append(f"CHANGELOG.md has no `## [{version}]` section")
    elif not any(l.strip() for l in section):
        out.append(f"CHANGELOG.md's `## [{version}]` section is empty")
    if not any(l.startswith("## [Unreleased]") for l in changelog.splitlines()):
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
    found = problems(tag, crates(), (ROOT / "CHANGELOG.md").read_text(), fragments)
    if on_main:
        found += main_problems(tag)
    if found:
        print(f"Not releasing {tag}:")
        for message in found:
            print(f"  - {message}")
        return 1
    print(f"{tag}: crate versions, CHANGELOG section and fragments agree.")
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

    assert untagged({"ponko": ["0.0.1", "0.0.2"], "ponko_ecs": ["0.0.1"]}, {"v0.0.1"}) == [
        ("ponko", "0.0.2")
    ], "a published version needs its tag"
    assert untagged({"ponko": ["0.0.1"]}, {"v0.0.1", "v1.0.0"}) == []
    on_main_selftest()
    names = {n for n, _, _ in crates()}
    assert "ponko" in names and "ponko_ecs" in names, "reads the real workspace"
    print("release-check.py: self-test passed")


def main():
    command = sys.argv[1] if len(sys.argv) > 1 else ""
    if command == "check" and len(sys.argv) > 2:
        return check(sys.argv[2], "--on-main" in sys.argv)
    if command == "published":
        return published()
    if command == "selftest":
        return selftest()
    sys.exit(__doc__)


if __name__ == "__main__":
    sys.exit(main())
