#!/usr/bin/env python3
"""Refuse a release whose tag, crate versions and CHANGELOG section disagree.

    release-check.py check TAG      TAG is vX.Y.Z or vX.Y.Z-rc.N; the release
                                    workflow runs this before it builds or
                                    publishes anything
    release-check.py ci TAG         fail unless CI succeeded on the tagged commit,
                                    on main or its release branch (asks the
                                    Actions API with GITHUB_TOKEN)
    release-check.py notes TAG FILE
                                    write that version's CHANGELOG section,
                                    the GitHub release's notes, to FILE
    release-check.py published      every version of our crates on crates.io
                                    has a git tag (a scheduled workflow runs it)
    release-check.py selftest       check the decisions with made-up input

`check` fails unless all of these hold:

- the tag is `vX.Y.Z`, or `vX.Y.Z-rc.N` (N from 1), a release candidate:
  crates.io and Cargo treat its version as a pre-release, so nobody gets it
  without writing it out, and the GitHub release is marked pre-release.
  Candidates are for minor releases (X.Y.0), not for patch releases;
- every publishable crate (no `publish = false`) is at the tag's version
  (X.Y.Z or X.Y.Z-rc.N), and so are the `version = "..."` requirements it
  puts on its sibling crates;
- CHANGELOG.md has a `## [X.Y.Z]` section with something in it, and still
  has its `## [Unreleased]` heading. A candidate's notes may instead be in
  `## [X.Y.Z-rc.N]` or, as long as the release is not named yet, in
  `## [Unreleased]` itself (the first of the three that exists is used);
- that section fits in a GitHub release body (at most 125,000 characters),
  since it becomes the release's notes;
- no changelog.d/ fragment is left (the release folds them in first);
- a release that lists `### Breaking changes` is one that may break (0.y.0,
  x.0.0: never a patch release) and has its migration chapter,
  book/src/migrating-to-X.Y.md (`scripts/changelog.py --version X.Y.Z`
  generates it with the fold), listed in book/src/SUMMARY.md and holding every
  breaking entry of the section. A candidate needs the same chapter, under
  the same name, as soon as its section lists a break (testers need it most);
- every publishable crate under crates/ carries copies of the root
  LICENSE-MIT, LICENSE-APACHE and NOTICE (crates.io gets each crate's own
  folder), identical to the root ones; `selftest` checks this too, in CI;
- with `--on-branch`: the tagged commit is an ancestor of origin/main or of
  origin/release-X.Y (the patch branch of the tag's minor: v0.1.1 on
  release-0.1, never on release-0.2), i.e. it was merged through a pull
  request (the workflow checks out the full history for this). Whether CI
  passed on it is not looked at here, but by `ci`:

`ci` fails unless a run of ci.yml on that exact commit, on `main` or on the
tag's own `release-X.Y`, finished successfully (a pull request's run is of its
head, not of the merge, so it does not count). A run still going gets its own
message ("wait, then re-run this job"), so does a failed or cancelled one and
a commit CI never ran on.

`published` asks the crates.io API (one read request per crate) and compares
with `git tag`; it needs no token.

See "Releasing" in CONTRIBUTING.md for the whole procedure.
"""
import json
import os
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

TAG = re.compile(r"^v(\d+\.\d+\.\d+(?:-rc\.[1-9]\d*)?)$")
TAG_HELP = "a release tag like v1.2.3 or v1.2.3-rc.1"
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


def is_candidate(version):
    """Whether `version` (X.Y.Z-rc.N) is a release candidate."""
    return "-" in version


def notes_section(text, version):
    """(heading, lines) of the CHANGELOG section that holds the notes of `version`, or None.

    A release has its own `## [version]` section. A candidate may also use
    the final version's section or `## [Unreleased]` (the changes are folded
    in but the release is not named yet); the first that exists wins.
    """
    names = [version]
    if is_candidate(version):
        names += [version.split("-")[0], "Unreleased"]
    for name in names:
        section = changelog_section(text, name)
        if section is not None:
            return name, section
    return None


def candidate_preamble(version):
    """The line a candidate's GitHub release starts with."""
    return (f"Release candidate of {version.split('-')[0]}, for testing. Cargo does not select it "
            f"unless the version is written out (`ponko = \"{version}\"`); the release follows "
            "unchanged if no regression turns up.\n\n")


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
        return [f"`{tag}` is not {TAG_HELP}"]
    version = found.group(1)
    out = []
    if is_candidate(version) and not version.split("-")[0].endswith(".0"):
        out.append(f"`{tag}`: release candidates are for minor releases (X.Y.0); "
                   "a patch release is released directly")
    names = {name for name, _, _ in crate_list}
    for name, crate_version, data in crate_list:
        if crate_version != version:
            out.append(f"{name} is at {crate_version}, not {version}")
        for dep, req in sibling_requirements(data, names):
            if req.lstrip("=^~ ") != version:
                out.append(f"{name} requires {dep} {req}, not {version}")
    chosen = notes_section(changelog_text, version)
    if chosen is None:
        out.append(f"CHANGELOG.md has no `## [{version}]` section"
                   + (" (nor `## [Unreleased]`)" if is_candidate(version) else ""))
        section = None
    else:
        heading, section = chosen
        if not any(l.strip() for l in section):
            out.append(f"CHANGELOG.md's `## [{heading}]` section is empty")
        elif len(release_notes(section)) > MAX_NOTES:
            out.append(
                f"CHANGELOG.md's `## [{heading}]` section has {len(release_notes(section)):,} characters; "
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


def release_branch(version):
    """The patch branch of the minor `version` belongs to: release-0.1 for 0.1.1."""
    major, minor, _ = changelog.version_parts(version)
    return f"release-{major}.{minor}"


def branch_problems(tag, repo=ROOT):
    """Problems with `tag` not being a commit merged to origin/main or its release branch.

    Both are fetched first. The release branch is the tag's own minor's
    (release-0.1 for v0.1.1, release-0.2 for v0.2.0-rc.2); a tag is never
    accepted on another minor's branch.
    """
    found = TAG.match(tag)
    if not found:
        return [f"`{tag}` is not {TAG_HELP}"]
    if git("fetch", "--quiet", "origin", "main", repo=repo).returncode != 0:
        return ["could not fetch origin/main to check the tag against it"]
    if git("rev-parse", "--verify", "--quiet", f"{tag}^{{commit}}", repo=repo).returncode != 0:
        return [f"{tag} does not exist in this checkout"]
    if git("merge-base", "--is-ancestor", f"{tag}^{{commit}}", "origin/main", repo=repo).returncode == 0:
        return []
    branch = release_branch(found.group(1))
    # An explicit refspec: a plain `fetch origin NAME` would not update origin/NAME everywhere.
    fetched = git("fetch", "--quiet", "origin", f"+refs/heads/{branch}:refs/remotes/origin/{branch}", repo=repo)
    if fetched.returncode != 0:
        return [f"{tag} is not on origin/main, and there is no origin/{branch} branch it could be on: "
                f"tag a commit that was merged to main (or create {branch} from the minor's release)"]
    if git("merge-base", "--is-ancestor", f"{tag}^{{commit}}", f"origin/{branch}", repo=repo).returncode != 0:
        return [f"{tag} is on neither origin/main nor origin/{branch}: tag a commit that was merged to one of them"]
    return []


def ci_problems(tag, sha, runs):
    """Why `tag`'s commit does not have a green CI run on main or its release branch, as messages.

    runs: the workflow runs of ci.yml for that commit, as the Actions API
    lists them (dicts with head_sha, head_branch, event, status, conclusion
    and html_url). Only a run of a push (or a manual run) on `main` or on
    the tag's own release-X.Y counts: a pull request's run is of the pull
    request's head, not of the merge. One successful run is enough; a run
    that is still going (or being re-run) is reported before a failed one,
    because it may still turn green.
    """
    found = TAG.match(tag)
    if not found:
        return [f"`{tag}` is not {TAG_HELP}"]
    branch = release_branch(found.group(1))
    counted = [r for r in runs
               if r.get("head_sha") == sha and r.get("event") != "pull_request"
               and r.get("head_branch") in ("main", branch)]
    if any(r.get("status") == "completed" and r.get("conclusion") == "success" for r in counted):
        return []
    short = sha[:10]
    pending = [r for r in counted if r.get("status") != "completed"]
    if pending:
        return [f"CI is still running on {short} ({pending[0].get('html_url', 'no link')}): "
                f"{tag} may only be released once it succeeded. Wait for it to finish, then "
                "re-run this job (Actions, the Release run, \"Re-run failed jobs\")."]
    if counted:
        shown = "; ".join(f"{r.get('head_branch')}: {r.get('conclusion')} ({r.get('html_url', 'no link')})"
                          for r in counted[:3])
        return [f"CI did not succeed on {short}, the commit of {tag} ({shown}). Fix it on main "
                "(or the release branch) through a pull request and tag the fixed commit; if the run "
                "only failed by chance or was cancelled, re-run it first (\"Re-run failed jobs\") and "
                "then re-run this job."]
    return [f"no CI run of {short}, the commit of {tag}, was found on main or {branch}: CI runs on "
            f"every push to them, so tag a commit that was merged to one of them (a pull request's "
            "own run does not count)"]


def fetch_ci_runs(repo, sha, token, api="https://api.github.com"):
    """The ci.yml runs of commit `sha` (needs `actions: read`); exits with a message if the API fails."""
    url = f"{api}/repos/{repo}/actions/workflows/ci.yml/runs?head_sha={sha}&per_page=100"
    request = urllib.request.Request(url, headers={
        "Authorization": f"Bearer {token}",
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "ponko-release-check",
    })
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return json.load(response)["workflow_runs"]
    except urllib.error.HTTPError as error:
        sys.exit(f"GitHub answered {error.code} when asking for the CI runs of {sha[:10]}; "
                 "the job needs `actions: read`. Try again later")
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError, KeyError) as error:
        sys.exit(f"could not ask GitHub for the CI runs of {sha[:10]}: {error}")


def ci(tag):
    """Fails unless CI succeeded on the tagged commit (GITHUB_TOKEN and GITHUB_REPOSITORY are read)."""
    if not TAG.match(tag):
        sys.exit(f"`{tag}` is not {TAG_HELP}")
    token, repo = os.environ.get("GITHUB_TOKEN"), os.environ.get("GITHUB_REPOSITORY")
    if not token or not repo:
        sys.exit("release-check.py ci needs GITHUB_TOKEN and GITHUB_REPOSITORY (the release workflow sets them)")
    sha = git("rev-parse", "--verify", "--quiet", f"{tag}^{{commit}}").stdout.strip()
    if not sha:
        sys.exit(f"{tag} does not exist in this checkout")
    found = ci_problems(tag, sha, fetch_ci_runs(repo, sha, token, os.environ.get("GITHUB_API_URL", "https://api.github.com")))
    if found:
        print(f"Not releasing {tag}:")
        for message in found:
            print(f"  - {message}")
        return 1
    print(f"{tag}: CI succeeded on {sha[:10]}.")
    return 0


def check(tag, on_branch):
    fragments = [p.name for p in (ROOT / "changelog.d").glob("*.md") if p.name != "README.md"]
    found = TAG.match(tag)
    book = None
    if found:
        chapter = ROOT / "book" / "src" / changelog.chapter_name(found.group(1))
        book = (chapter.read_text() if chapter.is_file() else None,
                (ROOT / "book" / "src" / "SUMMARY.md").read_text())
    found = problems(tag, crates(), (ROOT / "CHANGELOG.md").read_text(), fragments, book) + license_problems()
    if on_branch:
        found += branch_problems(tag)
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
        sys.exit(f"`{tag}` is not {TAG_HELP}")
    version = found.group(1)
    chosen = notes_section((ROOT / "CHANGELOG.md").read_text(), version)
    if chosen is None or not any(l.strip() for l in chosen[1]):
        sys.exit(f"CHANGELOG.md has no notes for {version}")
    text = (candidate_preamble(version) if is_candidate(version) else "") + release_notes(chosen[1])
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


def on_branch_selftest():
    """branch_problems against a throw-away origin and clone (deleted afterwards)."""
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
        run("tag", "v0.1.0", cwd=work)
        run("push", "origin", "main", cwd=work)
        # The patch branch of 0.1, cut from the release, with a fix of its own.
        run("checkout", "-b", "release-0.1", "v0.1.0", cwd=work)
        run("commit", "--allow-empty", "-m", "fix on the patch branch", cwd=work)
        run("tag", "v0.1.1", cwd=work)
        run("tag", "v0.2.0", cwd=work)  # a tag of another minor on this branch
        run("push", "origin", "release-0.1", cwd=work)
        # Main moves on to the next minor.
        run("checkout", "main", cwd=work)
        run("commit", "--allow-empty", "-m", "breaking change on main", cwd=work)
        run("tag", "v0.2.0-rc.1", cwd=work)
        run("push", "origin", "main", cwd=work)
        run("checkout", "-b", "feature", cwd=work)
        run("commit", "--allow-empty", "-m", "never merged", cwd=work)
        run("tag", "v0.1.2", cwd=work)
        run("tag", "v0.2.0-rc.2", cwd=work)

        assert branch_problems("v0.1.0", work) == [], "a tag on a merged commit passes"
        assert branch_problems("v0.2.0-rc.1", work) == [], "a candidate on main passes"
        assert branch_problems("v0.1.1", work) == [], "a patch release on its release branch passes"
        found = branch_problems("v0.1.2", work)
        assert found and "neither origin/main nor origin/release-0.1" in found[0], (
            "a tag on an unmerged branch is refused even where its release branch exists"
        )
        found = branch_problems("v0.2.0-rc.2", work)
        assert found and "no origin/release-0.2 branch" in found[0], (
            "an unmerged candidate is refused, and the message names the branch it could be on"
        )
        found = branch_problems("v0.2.0", work)
        assert found and "no origin/release-0.2 branch" in found[0], (
            "release-0.1 is not the branch for a 0.2 tag"
        )
        assert "does not exist" in branch_problems("v9.9.9", work)[0]
        assert "is not a release tag" in branch_problems("0.1.1", work)[0]
        run("remote", "set-url", "origin", str(tmp / "missing.git"), cwd=work)
        assert "could not fetch" in branch_problems("v0.1.0", work)[0], "no main to compare with is refused"


def ci_selftest():
    sha = "a" * 40

    def run(branch="main", status="completed", conclusion="success", event="push", head=sha):
        return {"head_sha": head, "head_branch": branch, "event": event, "status": status,
                "conclusion": conclusion, "html_url": f"https://example.com/{branch}/{status}"}

    assert ci_problems("v0.2.0", sha, [run()]) == [], "a green run on main passes"
    assert ci_problems("v0.2.0-rc.1", sha, [run()]) == [], "so does a candidate's"
    assert ci_problems("v0.1.1", sha, [run("release-0.1")]) == [], "a green run on the tag's release branch passes"
    assert ci_problems("v0.1.1", sha, [run("main", event="workflow_dispatch")]) == [], "a manual run on main counts"
    assert ci_problems("v0.2.0", sha, [run(conclusion="failure"), run()]) == [], "one green run is enough"
    assert ci_problems("v0.2.0", sha, [run(conclusion="failure")])[0].startswith("CI did not succeed on aaaaaaaaaa"), (
        "a failed run stops the release"
    )
    assert "re-run" in ci_problems("v0.2.0", sha, [run(conclusion="cancelled")])[0], "a cancelled run says how to go on"
    found = ci_problems("v0.2.0", sha, [run(status="in_progress", conclusion=None)])
    assert len(found) == 1 and found[0].startswith("CI is still running on aaaaaaaaaa") and "re-run this job" in found[0], (
        "a run still in progress is reported as such, not as a failure"
    )
    assert "still running" in ci_problems("v0.2.0", sha, [run(conclusion="failure"), run(status="queued", conclusion=None)])[0], (
        "a run in progress is named before an older failure of the same commit"
    )
    assert "no CI run" in ci_problems("v0.2.0", sha, [])[0], "a commit CI never ran on is refused"
    assert "no CI run" in ci_problems("v0.2.0", sha, [run(event="pull_request")])[0], (
        "a pull request's run is not the merge commit's CI"
    )
    assert "no CI run" in ci_problems("v0.2.0", sha, [run(head="b" * 40)])[0], "a run of another commit does not count"
    assert "no CI run" in ci_problems("v0.2.0", sha, [run("release-0.1")])[0], (
        "release-0.1 is not the branch for a 0.2 tag"
    )
    assert "no CI run" in ci_problems("v0.2.0", sha, [run("feature")])[0], "a run on another branch does not count"
    assert ci_problems("v0.2.0", sha, [run("feature", conclusion="failure"), run()]) == [], (
        "a failure on an unrelated branch does not matter"
    )
    assert ci_problems("0.2.0", sha, [run()]) == [f"`0.2.0` is not {TAG_HELP}"]


def selftest():
    def toml(name, version, deps=""):
        return tomllib.loads(f'[package]\nname = "{name}"\nversion = "{version}"\n[dependencies]\n{deps}')

    sibling = 'ponko_ecs = { path = "crates/ponko_ecs", version = "0.0.2" }\nserde = "1"\n'
    ok = [("ponko", "0.0.2", toml("ponko", "0.0.2", sibling)), ("ponko_ecs", "0.0.2", toml("ponko_ecs", "0.0.2"))]
    log = "## [Unreleased]\n\n### Added\n- next\n\n## [0.0.2] - 2026-12-01\n\n### Added\n- thing\n\n## [0.0.1] - x\n- old\n"

    assert problems("v0.0.2", ok, log, []) == [], "a consistent release passes"
    assert problems("v0.0.2-rc.1", ok, log, [])[0].startswith("`v0.0.2-rc.1`: release candidates are for minor"), (
        "a patch release has no candidates"
    )
    assert problems("0.0.2", ok, log, []) == [f"`0.0.2` is not {TAG_HELP}"]
    for odd in ("v0.0.2-beta.1", "v0.0.2-rc", "v0.0.2-rc.0", "v0.0.2-rc.01", "v0.0.2-rc.1.2", "v0.0.2-desktop-dry-run",
                "v0.0.2+build", "v0.0", "v1.2.3.4"):
        assert problems(odd, ok, log, []) == [f"`{odd}` is not {TAG_HELP}"], f"{odd} is not a release tag"
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

    # Release candidates: crates at X.Y.0-rc.N, notes from the first of the
    # candidate's, the final version's or the Unreleased section.
    def rc_crates(version):
        dep = f'ponko_ecs = {{ path = "crates/ponko_ecs", version = "{version}" }}\n'
        return [("ponko", version, toml("ponko", version, dep)), ("ponko_ecs", version, toml("ponko_ecs", version))]

    rc = rc_crates("0.3.0-rc.1")
    unreleased = "## [Unreleased]\n\n### Added\n- the candidate's news\n\n## [0.2.0] - x\n- old\n"
    assert problems("v0.3.0-rc.1", rc, unreleased, []) == [], "a candidate may keep its notes under Unreleased"
    assert problems("v0.3.0", rc_crates("0.3.0"), unreleased, []) != [], "the release itself needs its own section"
    assert notes_section(unreleased, "0.3.0-rc.1") == ("Unreleased", ["", "### Added", "- the candidate's news", ""])
    assert notes_section(unreleased, "0.3.0") is None
    named = unreleased.replace("## [Unreleased]\n", "## [Unreleased]\n\n## [0.3.0-rc.1] - x\n- named\n")
    assert notes_section(named, "0.3.0-rc.1")[0] == "0.3.0-rc.1", "its own section comes first"
    final = unreleased.replace("## [0.2.0]", "## [0.3.0] - x\n- final\n\n## [0.2.0]")
    assert notes_section(final, "0.3.0-rc.2")[0] == "0.3.0", "then the final version's"
    assert problems("v0.3.0-rc.1", rc, "## [Unreleased]\n\n## [0.2.0] - x\n- old\n", []) == [
        "CHANGELOG.md's `## [Unreleased]` section is empty"
    ], "a candidate with nothing to tell is refused"
    assert any("no `## [0.3.0-rc.1]` section (nor `## [Unreleased]`)" in p
               for p in problems("v0.3.0-rc.1", rc, "## [0.2.0] - x\n- old\n", [])), (
        "a candidate with no section to take notes from is refused"
    )
    assert any("ponko is at 0.3.0, not 0.3.0-rc.1" in p for p in problems("v0.3.0-rc.1", rc_crates("0.3.0"), unreleased, [])), (
        "the crates carry the candidate's version, or they would publish as the release"
    )
    assert any("requires ponko_ecs 0.3.0, not 0.3.0-rc.1" in p for p in problems(
        "v0.3.0-rc.1", [("ponko", "0.3.0-rc.1", toml("ponko", "0.3.0-rc.1", 'ponko_ecs = { path = "x", version = "0.3.0" }\n')),
                        ("ponko_ecs", "0.3.0-rc.1", toml("ponko_ecs", "0.3.0-rc.1"))], unreleased, [])), (
        "sibling requirements move to the candidate too"
    )
    assert any("1 fragment(s)" in p for p in problems("v0.3.0-rc.1", rc, unreleased, ["1-x.md"])), (
        "fragments are folded in before a candidate, the way they are before a release"
    )
    # Breaking changes: a candidate's notes under Unreleased need the chapter too.
    breaking_unreleased = unreleased.replace("### Added", "### Breaking changes\n- `A` is `B` now.\n  Instead: use `B`.\n\n### Added")
    assert any("migrating-to-0.3.md does not exist" in p
               for p in problems("v0.3.0-rc.1", rc, breaking_unreleased, [], (None, ""))), (
        "a candidate with breaking entries needs the chapter"
    )
    assert problems("v0.3.0-rc.1", rc, breaking_unreleased, [], (chapter, good_summary)) == []
    assert problems("v0.3.0-rc.2", rc_crates("0.3.0-rc.2"), breaking_unreleased, [], (chapter, good_summary)) == [], (
        "every candidate of a minor shares its chapter"
    )
    assert release_branch("0.1.1") == "release-0.1" and release_branch("0.2.0-rc.1") == "release-0.2"
    assert release_branch("1.4.2") == "release-1.4"
    assert candidate_preamble("0.3.0-rc.1").startswith("Release candidate of 0.3.0, for testing.")
    assert "ponko = \"0.3.0-rc.1\"" in candidate_preamble("0.3.0-rc.1")
    assert untagged({"ponko": ["0.0.1", "0.0.2"], "ponko_ecs": ["0.0.1"]}, {"v0.0.1"}) == [
        ("ponko", "0.0.2")
    ], "a published version needs its tag"
    assert untagged({"ponko": ["0.0.1"]}, {"v0.0.1", "v1.0.0"}) == []
    ci_selftest()
    on_branch_selftest()
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
        return check(sys.argv[2], "--on-branch" in sys.argv)
    if command == "ci" and len(sys.argv) > 2:
        return ci(sys.argv[2])
    if command == "notes" and len(sys.argv) > 3:
        return notes(sys.argv[2], sys.argv[3])
    if command == "published":
        return published()
    if command == "selftest":
        return selftest()
    sys.exit(__doc__)


if __name__ == "__main__":
    sys.exit(main())
