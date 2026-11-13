#!/usr/bin/env python3
"""Check the published crates for semver breaks (the `Semver` job of ci.yml).

    semver-checks.py run        # what CI does; reads the environment below
    semver-checks.py selftest   # check the decisions with made-up input

`cargo semver-checks` compares the public API of each published crate with
the last release on crates.io. What a break means depends on the branch:

  release-*  A patch release must not break anything: every break fails.
  anything   Breaks since the last release pile up on `main` until the next
  else       minor release, so "there is a break" says nothing about this
             change. The crates.io comparison is therefore only the report.
             What gates is the break this change adds, found by comparing
             with the commit it is based on. It passes only with the
             `breaking` label and a changelog fragment that has a
             `### Breaking changes` section (CONTRIBUTING.md, "Semver checks").

Which packages and features are checked comes from Cargo: every package
that can be published (not `publish = false`), with the features it builds on
docs.rs (`[package.metadata.docs.rs]`), so a new crate is covered without
touching this file. Crates the tool can't read (proc-macro crates, binaries)
are listed in the job summary.

The environment (ci.yml sets it): EVENT (github.event_name), BASE_REF
(github.base_ref), REF_NAME (github.ref_name), BASE_SHA (the pull request's
base commit, or a push's `before`), PR (a pull request's number), SHA,
REPO and GH_TOKEN (for `gh`).
"""
from fnmatch import fnmatch
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
LABEL = 'breaking'
HEADING = 'Breaking changes'
RELEASE_BRANCHES = 'release-*'
LIB_KINDS = {'lib', 'rlib', 'dylib', 'cdylib', 'staticlib'}
# `cargo semver-checks` exits 100 when it found a break; any other failure
# is an error (it couldn't build, the crate isn't published, ...).
BREAK_EXIT = 100


# Decisions: plain functions of their input, checked by `selftest`.

def decide_mode(base_ref, ref_name):
    """'release' on a release-* branch (or a pull request into one), else 'main'."""
    return 'release' if fnmatch(base_ref or ref_name or '', RELEASE_BRANCHES) else 'main'


def feature_args(package):
    """The cargo-semver-checks flags for the features the package builds on docs.rs.

    Not the tool's default (every feature but `unstable*`): that would include
    `hotpatch` and `bench-internals`, which are not part of the API.
    """
    docs = ((package.get('metadata') or {}).get('docs') or {}).get('rs') or {}
    if docs.get('all-features'):
        return ['--all-features']
    args = ['--only-explicit-features' if docs.get('no-default-features') else '--default-features']
    if docs.get('features'):
        args += ['--features', ','.join(docs['features'])]
    return args


def skip_reason(package):
    """Why the tool can't read this package, or None."""
    kinds = {kind for target in package['targets'] for kind in target['kind']}
    if 'proc-macro' in kinds:
        return 'proc-macro crate (the tool reads no API from it)'
    if not kinds & LIB_KINDS:
        return 'no library target (binary only, no API)'
    return None


def verdict(mode, breaks, new_breaks, errors, labeled, fragment):
    """(ok, message lines) for the whole run.

    breaks: crates that break against the last release; new_breaks: of those,
    the ones this change breaks (None in release mode, where every break counts).
    """
    lines = []
    if errors:
        return False, [f'`cargo semver-checks` could not check {", ".join(errors)}: '
                       'a label cannot excuse that, fix the error above.']
    if not breaks:
        return True, ['No breaking change against the last release.']
    if mode == 'release':
        return False, [f'Breaking change in {", ".join(breaks)}: a patch release must not '
                       'break the API (STABILITY.md). Fix it on `main` first and carry over a '
                       'change that is compatible, or release it as the next minor.']
    if not new_breaks:
        return True, [f'{", ".join(breaks)} differ from the last release, but not because of '
                      'this change: earlier merged changes announced them. Nothing to do.']
    lines.append(f'This change breaks the API of {", ".join(new_breaks)}.')
    missing = []
    if not labeled:
        missing.append(f'the `{LABEL}` label on the pull request (add it, then re-run this job)')
    if not fragment:
        missing.append(f'a changelog fragment with a `### {HEADING}` section naming the replacement')
    if missing:
        return False, lines + ['Missing: ' + ' and '.join(missing) + '.',
                               'Or undo the break: CONTRIBUTING.md, "Semver checks".']
    return True, lines + [f'Allowed: `{LABEL}` label and a `### {HEADING}` fragment are there.']


# Side effects: cargo, git and gh.

def run(cmd, **kw):
    return subprocess.run(cmd, cwd=ROOT, text=True, **kw)


def packages():
    out = run(['cargo', 'metadata', '--no-deps', '--format-version', '1'],
              capture_output=True, check=True).stdout
    # `publish = false` shows up as `publish: []`.
    return sorted((p for p in json.loads(out)['packages'] if p['publish'] != []),
                  key=lambda p: p['name'])


def group(title):
    print(f'::group::{title}' if os.environ.get('GITHUB_ACTIONS') else f'--- {title}', flush=True)


def endgroup():
    if os.environ.get('GITHUB_ACTIONS'):
        print('::endgroup::', flush=True)


def check(package, release_type, baseline, label):
    """Run the tool on one package: 'ok', 'break', 'unpublished' or 'error'."""
    cmd = ['cargo', 'semver-checks', 'check-release', '-p', package['name'],
           *feature_args(package), '--release-type', release_type, *baseline]
    group(f'{label}: {package["name"]} ({" ".join(cmd[5:])})')
    done = run(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print(done.stdout, end='')
    endgroup()
    package['output'] = done.stdout
    if done.returncode == 0:
        return 'ok'
    if done.returncode == BREAK_EXIT:
        return 'break'
    if 'not found in registry' in done.stdout:
        print(f'::notice::{package["name"]} is not on crates.io yet: nothing to compare with.')
        return 'unpublished'
    return 'error'


def gh(*args):
    done = run(['gh', *args], capture_output=True)
    if done.returncode != 0:
        sys.exit(f'gh {" ".join(args)} failed: {done.stderr.strip()}')
    return done.stdout.strip()


def pull_request_for(env):
    """(number, base branch) of the pull request this run speaks for, or (None, None)."""
    event, repo = env['EVENT'], env.get('REPO', '')
    if event == 'pull_request':
        return env.get('PR') or None, env['BASE_REF']
    if event == 'push':
        # The merged pull request of a commit on main.
        number = gh('api', f'repos/{repo}/commits/{env["SHA"]}/pulls', '--jq', '.[0].number // empty')
        return (number or None), None
    # workflow_dispatch (the `full-ci` label): the branch's open pull request.
    found = gh('pr', 'list', '--repo', repo, '--head', env['REF_NAME'], '--state', 'open',
               '--json', 'number,baseRefName', '--jq', '.[0] // empty | "\\(.number) \\(.baseRefName)"')
    return tuple(found.split()) if found else (None, None)


def base_commit(env, target):
    """The commit the change is compared with: the one it is based on."""
    before = env.get('BASE_SHA', '')
    if env['EVENT'] == 'workflow_dispatch' or not before or set(before) == {'0'}:
        if env['EVENT'] == 'push':
            return run(['git', 'rev-parse', 'HEAD^'], capture_output=True, check=True).stdout.strip()
        branch = target or 'main'
        return run(['git', 'merge-base', f'origin/{branch}', 'HEAD'],
                   capture_output=True, check=True).stdout.strip()
    return before


def labeled(env, pr):
    if not pr:
        print('::notice::No pull request found: there is no label to read.')
        return False
    names = gh('pr', 'view', pr, '--repo', env['REPO'], '--json', 'labels', '--jq', '.labels[].name')
    return LABEL in names.split()


def breaking_fragment(base):
    """Whether the change adds or edits a fragment with a Breaking changes section."""
    changed = run(['git', 'diff', '--name-only', '--diff-filter=AM', base, 'HEAD', '--', 'changelog.d'],
                  capture_output=True, check=True).stdout.split()
    if not changed:
        return False
    found = run([sys.executable, 'scripts/changelog.py', '--breaking', *changed],
                capture_output=True, check=True).stdout.split()
    return bool(found)


def failures(package):
    """The lint sections of a failed run, for the job summary."""
    match = re.search(r'--- failure.*?(?=\n\s*Summary)', package.get('output', ''), re.S)
    return match.group(0).strip() if match else package.get('output', '').strip()[-2000:]


def summary(path, mode, checked, skipped, results, ok, lines):
    out = [f'## Semver checks ({mode} mode)', '', *lines, '',
           '| Crate | Against the last release | Added by this change |', '|---|---|---|']
    for p in checked:
        name = p['name']
        out.append(f'| `{name}` | {results[name].get("registry", "-")} | {results[name].get("new", "-")} |')
    for p in skipped:
        out.append(f'| `{p["name"]}` | not checked: {skip_reason(p)} | - |')
    for p in checked:
        for key in ('new', 'registry'):
            if results[p['name']].get(key) == 'break':
                out += ['', f'### `{p["name"]}`', '```', failures(p), '```']
                break
    text = '\n'.join(out) + '\n'
    if path:
        with open(path, 'a') as f:
            f.write(text)


def main_run():
    env = os.environ
    pr, target = pull_request_for(env)
    mode = decide_mode(target or env.get('BASE_REF'), env.get('REF_NAME'))
    pkgs = packages()
    checked = [p for p in pkgs if not skip_reason(p)]
    skipped = [p for p in pkgs if skip_reason(p)]
    for p in skipped:
        print(f'{p["name"]}: not checked, {skip_reason(p)}')
    results = {p['name']: {} for p in checked}

    # `minor` is what 0.1.0 -> 0.1.x is for a 0.x crate (cargo's rule, which
    # the tool follows): additions pass, breaks fail. `patch` would also fail
    # additions, which STABILITY.md allows in a patch release. A main that
    # still says 0.1.0 is compared as the same.
    for p in checked:
        results[p['name']]['registry'] = check(p, 'minor', [], 'last release')
    breaks = [n for n, r in results.items() if r['registry'] == 'break']
    errors = [n for n, r in results.items() if r['registry'] == 'error']

    new_breaks = None
    is_labeled = has_fragment = False
    if mode == 'main' and breaks and not errors:
        base = base_commit(env, target)
        print(f'Breaks since the last release: {", ".join(breaks)}. Comparing with {base[:10]} '
              'to see which this change adds.')
        for p in checked:
            if p['name'] in breaks:
                results[p['name']]['new'] = check(p, 'minor', ['--baseline-rev', base], 'before this change')
        errors += [n for n, r in results.items() if r.get('new') == 'error']
        new_breaks = [n for n, r in results.items() if r.get('new') == 'break']
        if new_breaks:
            is_labeled = labeled(env, pr)
            has_fragment = breaking_fragment(base)

    ok, lines = verdict(mode, breaks, new_breaks, errors, is_labeled, has_fragment)
    print()
    for line in lines:
        print(f'::{"notice" if ok else "error"}::{line}' if os.environ.get('GITHUB_ACTIONS') else line)
    summary(env.get('GITHUB_STEP_SUMMARY'), mode, checked, skipped, results, ok, lines)
    sys.exit(0 if ok else 1)


def selftest():
    assert decide_mode('release-0.1', 'merge') == 'release'
    assert decide_mode(None, 'release-0.2') == 'release'
    assert decide_mode('main', 'merge') == 'main'
    assert decide_mode(None, 'feature/release-notes') == 'main'

    assert feature_args({}) == ['--default-features']
    assert feature_args({'metadata': {'docs': {'rs': {'all-features': True}}}}) == ['--all-features']
    docs = {'metadata': {'docs': {'rs': {'features': ['a', 'b']}}}}
    assert feature_args(docs) == ['--default-features', '--features', 'a,b']
    bare = {'metadata': {'docs': {'rs': {'no-default-features': True}}}}
    assert feature_args(bare) == ['--only-explicit-features']

    assert skip_reason({'targets': [{'kind': ['lib']}]}) is None
    assert skip_reason({'targets': [{'kind': ['rlib', 'staticlib', 'cdylib']}]}) is None
    assert 'proc-macro' in skip_reason({'targets': [{'kind': ['proc-macro']}]})
    assert 'binary' in skip_reason({'targets': [{'kind': ['bin']}]})

    ok = lambda *a: verdict(*a)[0]
    # No break: always fine. A tool error is never excused.
    assert ok('main', [], None, [], False, False)
    assert ok('release', [], None, [], False, False)
    assert not ok('main', [], None, ['ponko'], True, True)
    # A release branch fails on any break, label and fragment or not.
    assert not ok('release', ['ponko'], None, [], True, True)
    # Main: breaks that earlier changes made are not this change's problem.
    assert ok('main', ['ponko'], [], [], False, False)
    # This change's break needs the label AND the fragment.
    assert not ok('main', ['ponko'], ['ponko'], [], False, False)
    assert not ok('main', ['ponko'], ['ponko'], [], True, False)
    assert not ok('main', ['ponko'], ['ponko'], [], False, True)
    assert ok('main', ['ponko'], ['ponko'], [], True, True)
    assert 'label' in ' '.join(verdict('main', ['ponko'], ['ponko'], [], False, True)[1])
    assert 'fragment' in ' '.join(verdict('main', ['ponko'], ['ponko'], [], True, False)[1])
    print('semver-checks.py: self-test passed')


if __name__ == '__main__':
    command = sys.argv[1] if len(sys.argv) > 1 else ''
    if command == 'run':
        main_run()
    elif command == 'selftest':
        selftest()
    else:
        sys.exit(__doc__)
