#!/usr/bin/env python3
"""Say which release the guide documents, and publish a guide per release.

The guide on the site follows `main`. A crates.io release can lag behind it,
so every page opens with a note naming the version it documents and the
dependency line that compiles its examples, and the Pages workflow publishes
a copy of the guide for each release tag next to the `main` one:

    /ponko/            the guide for main
    /ponko/v0.0.2/     the guide as it was at tag v0.0.2

Subcommands:

    guide-versions.py preprocess   mdBook preprocessor (see book/book.toml):
                                   puts the note under every chapter's title
    guide-versions.py releases     the release tags to publish, newest first
    guide-versions.py build --out DIR
                                   build the guide for main into DIR and the
                                   guide of every release tag into DIR/<tag>/
                                   (needs `mdbook` and git history with tags).
                                   A tag whose guide fails to build is left
                                   out, with a warning, and no page links to it
    guide-versions.py selftest     check the decisions above with made-up input

The note reads its context from the environment, so a plain `mdbook build book`
works anywhere and says "main" without links:

    PONKO_GUIDE_VERSION    "main" (default) or a tag such as "v0.0.2"
    PONKO_GUIDE_RELEASES   release tags with a published guide, newest first,
                           separated by spaces (`build` sets it); without it
                           the note has no links
"""
import io
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCRIPT = Path(__file__).resolve()
REPO_URL = "https://github.com/ponkoengine/ponko"
# Stable release tags only: rc and dry-run tags have no guide of their own.
RELEASE_TAG = re.compile(r"^v(\d+)\.(\d+)\.(\d+)$")
PREPROCESSOR = "guide-version"
# A tag's guide that takes longer than this to build is given up on.
BUILD_TIMEOUT = 900


def tag_key(tag):
    return tuple(int(part) for part in RELEASE_TAG.match(tag).groups())


def sort_release_tags(tags):
    """The stable `vX.Y.Z` tags among `tags`, newest first."""
    return sorted((t for t in tags if RELEASE_TAG.match(t)), key=tag_key, reverse=True)


def is_ponko_package(cargo_toml):
    """True for the root Cargo.toml of Ponko (the tags from before the rename are not)."""
    return re.search(r'^name\s*=\s*"ponko"\s*$', cargo_toml, re.M) is not None


def git(repo, *args, check=True, binary=False):
    result = subprocess.run(["git", "-C", str(repo), *args], capture_output=True, check=check)
    return result.stdout if binary else result.stdout.decode()


def release_tags(repo=ROOT):
    """Release tags of Ponko (not of its pre-rename name) that have a guide, newest first."""
    tags = sort_release_tags(git(repo, "tag", "--list", "v*").split())
    found = []
    for tag in tags:
        shown = subprocess.run(
            ["git", "-C", str(repo), "show", f"{tag}:Cargo.toml"], capture_output=True
        )
        has_book = subprocess.run(
            ["git", "-C", str(repo), "cat-file", "-e", f"{tag}:book/book.toml"], capture_output=True
        )
        if shown.returncode == 0 and has_book.returncode == 0 and is_ponko_package(shown.stdout.decode()):
            found.append(tag)
    return found


# --- the note -------------------------------------------------------------


def banner(version, releases, to_root):
    """The note for a chapter. `to_root` is the relative path to the guide's root."""
    if version == "main":
        text = (
            "**Which version?** This guide follows the `main` branch, so it can "
            "describe APIs that the latest crates.io release does not have yet. "
            f"Its examples compile against "
            f'`ponko = {{ git = "{REPO_URL}" }}`.'
        )
        if releases:
            latest = releases[0]
            text += (
                f" For the released crate, read the [guide for {latest[1:]}]"
                f'({to_root}{latest}/) (`ponko = "{latest[1:]}"`).'
            )
        return text
    number = version[1:]
    text = (
        f"**Which version?** This is the guide for Ponko {number} (tag `{version}`). "
        f'Its examples compile against `ponko = "{number}"`.'
    )
    newer = [t for t in releases if t != version and tag_key(t) > tag_key(version)]
    if newer:
        text += f" A newer release exists: [guide for {newer[0][1:]}]({to_root}../{newer[0]}/)."
    text += f" The [guide for `main`]({to_root}../) can describe newer APIs."
    return text


def add_banner(content, note):
    """`content` with the note, as a quote, right under the first `# ` title."""
    quote = "\n".join(f"> {line}" if line else ">" for line in note.split("\n"))
    lines = content.split("\n")
    in_fence = False
    for i, line in enumerate(lines):
        if line.lstrip().startswith(("```", "~~~")):
            in_fence = not in_fence
        elif not in_fence and line.startswith("# "):
            return "\n".join(lines[: i + 1] + ["", quote] + lines[i + 1 :])
    return quote + "\n\n" + content


def transform(book, version, releases):
    """Add the note to every chapter with a file, in place; returns the book."""
    items = book.get("items", book.get("sections", []))

    def walk(entries):
        for entry in entries:
            chapter = entry.get("Chapter") if isinstance(entry, dict) else None
            if chapter is None:
                continue
            if chapter.get("path"):
                depth = Path(chapter["path"]).parent.parts
                note = banner(version, releases, "../" * len(depth))
                chapter["content"] = add_banner(chapter["content"], note)
            walk(chapter.get("sub_items", []))

    walk(items)
    return book


def env_context():
    version = os.environ.get("PONKO_GUIDE_VERSION", "main") or "main"
    releases = os.environ.get("PONKO_GUIDE_RELEASES", "").split()
    return version, releases


def preprocess():
    if len(sys.argv) > 2 and sys.argv[2] == "supports":
        return  # every renderer is fine (exit code 0)
    # mdBook speaks UTF-8 JSON; the locale's encoding (cp1252 on Windows) would corrupt it.
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    _context, book = json.load(sys.stdin)
    version, releases = env_context()
    json.dump(transform(book, version, releases), sys.stdout, ensure_ascii=False)


# --- the versioned copies -------------------------------------------------


def site_url_of(book_toml):
    found = re.search(r'^site-url\s*=\s*"([^"]*)"', book_toml, re.M)
    return found.group(1).rstrip("/") if found else ""


def patch_book_toml(text, tag, site_url):
    """A tag's book.toml with its place on the site and the current preprocessor.

    `site_url` is main's (the tag may predate a rename of the site). The tag's
    sources are old, so the note comes from this script, wherever the tag's own
    book.toml (if it has one) says otherwise.
    """
    text = re.sub(r'^site-url\s*=.*$', f'site-url = "{site_url}/{tag}/"', text, flags=re.M)
    text = re.sub(
        rf"^\[preprocessor\.{PREPROCESSOR}\]\n(?:(?!\[).*\n?)*", "", text, flags=re.M
    )
    command = json.dumps(f'python3 {shlex.quote(SCRIPT.as_posix())} preprocess')
    return text.rstrip("\n") + f"\n\n[preprocessor.{PREPROCESSOR}]\ncommand = {command}\n"


def build_one(repo, tag, dest, releases):
    """Build the guide of `tag` into `dest`; raises if anything goes wrong."""
    with tempfile.TemporaryDirectory(prefix="ponko-guide-") as tmp:
        archive = git(repo, "archive", tag, binary=True)
        with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
            tar.extractall(tmp, filter="data")
        toml = Path(tmp) / "book" / "book.toml"
        site_url = site_url_of((ROOT / "book" / "book.toml").read_text(encoding="utf-8"))
        toml.write_text(patch_book_toml(toml.read_text(encoding="utf-8"), tag, site_url), encoding="utf-8")
        env = dict(os.environ, PONKO_GUIDE_VERSION=tag, PONKO_GUIDE_RELEASES=" ".join(releases))
        subprocess.run(
            ["mdbook", "build", str(Path(tmp) / "book"), "--dest-dir", str(dest)],
            check=True,
            env=env,
            timeout=BUILD_TIMEOUT,
        )


def settle(candidates, try_build):
    """The tags whose guide builds, each built knowing the final list of tags.

    `try_build(tag, releases)` builds one guide and returns whether it worked.
    A guide links to the others, so when some fail, the survivors are built
    again with links to the survivors only; this repeats until nothing fails.
    """
    tags = list(candidates)
    while True:
        built = [tag for tag in tags if try_build(tag, tags)]
        if built == tags:
            return tags
        tags = built


def build(out, repo=ROOT):
    """The guide for main in `out` and one per release tag in `out/<tag>/`.

    A release's guide must not block publishing main's: the tags are built
    first, into a staging folder, and any failure (a bad archive, an mdBook
    that no longer reads the old sources, a hang) only drops that tag, with a
    warning. Main is built last, linking only to the guides that exist, and
    only then are the finished guides moved in, so the site never holds a
    half-written one.
    """
    out = Path(out).resolve()
    with tempfile.TemporaryDirectory(prefix="ponko-guides-") as stage:
        stage = Path(stage)

        def try_build(tag, releases):
            dest = stage / tag
            shutil.rmtree(dest, ignore_errors=True)
            try:
                build_one(repo, tag, dest, releases)
            except Exception as error:  # noqa: BLE001 - nothing here may stop main
                shutil.rmtree(dest, ignore_errors=True)
                print(f"::warning::The guide for {tag} was not built and is left out: {error!r}")
                return False
            return True

        tags = settle(release_tags(repo), try_build)
        env = dict(os.environ, PONKO_GUIDE_VERSION="main", PONKO_GUIDE_RELEASES=" ".join(tags))
        subprocess.run(
            ["mdbook", "build", str(ROOT / "book"), "--dest-dir", str(out)], check=True, env=env
        )
        for tag in tags:
            shutil.move(str(stage / tag), str(out / tag))
            print(f"Guide for {tag} is in {out / tag}")
    return 0


# --- self test ------------------------------------------------------------


def selftest():
    assert sort_release_tags(
        ["v0.0.1", "v0.0.10", "v0.0.2", "v1.0.0-rc.1", "v0.0.0-test-desktop-dry-run", "x", "v0.1.0"]
    ) == ["v0.1.0", "v0.0.10", "v0.0.2", "v0.0.1"], "tags sort numerically, stable ones only"
    assert is_ponko_package('[package]\nname = "ponko"\nversion = "0.0.1"\n')
    assert not is_ponko_package('[package]\nname = "oldname"\n')
    assert not is_ponko_package('[package]\nname = "ponko_ecs"\n')

    main = banner("main", [], "")
    assert "follows the `main` branch" in main and 'git = "' in main
    assert "](" not in main, "no links without a release list"
    linked = banner("main", ["v0.0.2", "v0.0.1"], "../")
    assert "(../v0.0.2/)" in linked and 'ponko = "0.0.2"' in linked
    old = banner("v0.0.1", ["v0.0.2", "v0.0.1"], "")
    assert "Ponko 0.0.1" in old and 'ponko = "0.0.1"' in old
    assert "(../v0.0.2/)" in old and "(../)" in old, "an old guide links to newer ones and main"
    latest = banner("v0.0.2", ["v0.0.2", "v0.0.1"], "")
    assert "newer release" not in latest, "the latest release has nothing newer"

    page = "# Title\n\nText\n\n```sh\n# not a title\n```\n"
    assert add_banner(page, "note").startswith("# Title\n\n> note\n\nText")
    fenced = "```sh\n# not a title\n```\n\n# Real\n"
    assert add_banner(fenced, "note").endswith("# Real\n\n> note\n")
    assert add_banner("no title", "note").startswith("> note\n\nno title")

    book = {
        "items": [
            {"Chapter": {"path": "a.md", "content": "# A\n", "sub_items": [
                {"Chapter": {"path": "sub/b.md", "content": "# B\n", "sub_items": []}},
                {"Chapter": {"path": None, "content": "# Draft\n", "sub_items": []}},
            ]}},
            "Separator",
        ]
    }
    transform(book, "main", ["v0.0.2"])
    top = book["items"][0]["Chapter"]
    assert "(v0.0.2/)" in top["content"], top["content"]
    assert "(../v0.0.2/)" in top["sub_items"][0]["Chapter"]["content"], "links climb out of subfolders"
    assert top["sub_items"][1]["Chapter"]["content"] == "# Draft\n", "draft chapters are left alone"

    assert site_url_of('[output.html]\nsite-url = "/ponko/"\n') == "/ponko"
    toml = '[book]\ntitle = "Old"\n\n[output.html]\nsite-url = "/oldname/"\n'
    patched = patch_book_toml(toml, "v0.0.2", "/ponko")
    assert 'site-url = "/ponko/v0.0.2/"' in patched
    assert f"[preprocessor.{PREPROCESSOR}]" in patched and "guide-versions.py preprocess" in patched
    assert patch_book_toml(patched, "v0.0.2", "/ponko").count(f"[preprocessor.{PREPROCESSOR}]") == 1, (
        "patching twice does not duplicate the preprocessor"
    )
    global SCRIPT
    real = SCRIPT
    SCRIPT = Path("/a folder/with spaces/guide-versions.py")
    try:
        quoted = patch_book_toml(toml, "v0.0.2", "/ponko")
    finally:
        SCRIPT = real
    command = tomllib.loads(quoted)["preprocessor"][PREPROCESSOR]["command"]
    assert shlex.split(command) == [
        "python3", "/a folder/with spaces/guide-versions.py", "preprocess"
    ], "the script path survives spaces"

    calls = []

    def fake(tag, releases):
        calls.append((tag, tuple(releases)))
        return tag != "v0.0.2"

    assert settle(["v0.0.3", "v0.0.2", "v0.0.1"], fake) == ["v0.0.3", "v0.0.1"], (
        "a tag that fails to build is dropped"
    )
    assert calls[-1] == ("v0.0.1", ("v0.0.3", "v0.0.1")), "the survivors are rebuilt without the failed one"
    assert settle([], fake) == [] and settle(["v0.0.1"], lambda t, r: False) == []
    print("guide-versions.py: self-test passed")


def main():
    command = sys.argv[1] if len(sys.argv) > 1 else ""
    if command == "preprocess":
        return preprocess()
    if command == "releases":
        print(" ".join(release_tags()))
        return 0
    if command == "build":
        if "--out" not in sys.argv:
            sys.exit("usage: guide-versions.py build --out DIR")
        return build(sys.argv[sys.argv.index("--out") + 1])
    if command == "selftest":
        return selftest()
    sys.exit(__doc__)


if __name__ == "__main__":
    sys.exit(main())
