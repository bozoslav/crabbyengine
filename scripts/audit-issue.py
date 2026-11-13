#!/usr/bin/env python3
"""Open, update or close the issue for the weekly dependency audit.

    audit-issue.py sync OUTPUT_FILE FAILED   # what deny.yml does
    audit-issue.py selftest                  # check the logic with made-up input

OUTPUT_FILE holds what `cargo deny check advisories` printed; FAILED is
`true` when it found something. With advisories, the one open issue titled
TITLE is created or, when the set of advisories changed, updated (body and a
comment). Without, an open issue is commented on and closed. Reads GH_TOKEN
(the workflow's own token, `issues: write`) and GITHUB_REPOSITORY, and runs
`gh`. Nothing from the audit output is ever interpolated into a command or a
shell: it goes into the issue as a quoted code block.
"""
import json
import os
import re
import subprocess
import sys
import tempfile

TITLE = 'Dependency audit: security advisories found by the weekly run'
LABELS = ('dependencies', 'area: build')
ID = re.compile(r'RUSTSEC-\d{4}-\d{4}|GHSA-[0-9a-z]{4}-[0-9a-z]{4}-[0-9a-z]{4}')
ANSI = re.compile(r'\x1b\[[0-9;]*m')
# GitHub rejects bodies over 65536 characters.
LIMIT = 60000


def advisory_ids(text):
    """The advisory ids the audit reported, sorted, each once."""
    return sorted(set(ID.findall(text)))


def clean(text):
    """The output without colour codes, and with nothing that could end the code fence."""
    return ANSI.sub('', text).replace('```', "'''")


def body(text, run_url):
    ids = advisory_ids(text)
    text = clean(text)
    if len(text) > LIMIT:
        text = text[:LIMIT] + '\n... (cut; see the run)'
    listing = ''.join(f'- [{i}](https://rustsec.org/advisories/{i})\n' if i.startswith('RUSTSEC') else f'- {i}\n' for i in ids)
    if not listing:
        listing = '- (no advisory id in the output; see the run)\n'
    return (
        'The weekly `cargo deny check advisories` run on `main` reported:\n\n'
        f'{listing}\n'
        f'Run: {run_url}\n\n'
        'What to do: update the dependency if a fixed version exists, otherwise '
        'add an `ignore` entry with a `reason` to `deny.toml` '
        '(CONTRIBUTING.md, "Dependency audit"). This issue is updated by the '
        'weekly run and closed when the audit is clean.\n\n'
        f'<details><summary>cargo-deny output</summary>\n\n```\n{text}\n```\n\n</details>\n'
    )


def gh(*args, stdin=None):
    return subprocess.run(['gh', *args], check=True, capture_output=True, text=True, input=stdin).stdout


def open_issue(repo):
    found = json.loads(gh('issue', 'list', '-R', repo, '--state', 'open', '--limit', '100', '--json', 'number,title,body'))
    for issue in found:
        if issue['title'] == TITLE:
            return issue
    return None


def sync(path, failed, repo, run_url):
    text = open(path, encoding='utf-8', errors='replace').read()
    issue = open_issue(repo)
    with tempfile.NamedTemporaryFile('w', suffix='.md', delete=False, encoding='utf-8') as f:
        if failed:
            f.write(body(text, run_url))
        else:
            f.write('The weekly audit found no advisories: closing. A new one reopens a new issue.')
        note = f.name
    if failed and issue is None:
        args = ['issue', 'create', '-R', repo, '--title', TITLE, '--body-file', note]
        for label in LABELS:
            args += ['--label', label]
        print(gh(*args).strip())
    elif failed:
        old = advisory_ids(issue['body'] or '')
        new = advisory_ids(text)
        gh('issue', 'edit', str(issue['number']), '-R', repo, '--body-file', note)
        if old != new:
            added = ', '.join(sorted(set(new) - set(old))) or 'none'
            gh('issue', 'comment', str(issue['number']), '-R', repo, '--body', f'The set of advisories changed (new: {added}). Run: {run_url}')
        print(f'updated #{issue["number"]}')
    elif issue is not None:
        gh('issue', 'close', str(issue['number']), '-R', repo, '--comment', open(note, encoding='utf-8').read())
        print(f'closed #{issue["number"]}')
    else:
        print('no advisories and no open issue')
    os.unlink(note)


def selftest():
    sample = (
        '\x1b[31merror\x1b[0m[unmaintained]: x\n ID: RUSTSEC-2025-0141\n ID: RUSTSEC-2025-0141\n'
        ' ID: RUSTSEC-2024-0436\n```\n'
    )
    assert advisory_ids(sample) == ['RUSTSEC-2024-0436', 'RUSTSEC-2025-0141'], advisory_ids(sample)
    assert advisory_ids('no ids here') == []
    out = body(sample, 'https://example.test/run/1')
    assert out.count('```') == 2, 'the output must not end its own code fence'
    assert '\x1b' not in out
    assert '- [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436)' in out
    assert len(body('x' * 200000, 'u')) < 65536, 'the body must fit the issue limit'
    assert 'no advisory id' in body('failed for another reason', 'u')
    print('  ok: advisory ids, fence, limit')


if __name__ == '__main__':
    if sys.argv[1:] == ['selftest']:
        selftest()
    elif len(sys.argv) == 4 and sys.argv[1] == 'sync':
        server = os.environ.get('GITHUB_SERVER_URL', 'https://github.com')
        repo = os.environ['GITHUB_REPOSITORY']
        run_url = f'{server}/{repo}/actions/runs/{os.environ.get("GITHUB_RUN_ID", "")}'
        sync(sys.argv[2], sys.argv[3] == 'true', repo, run_url)
    else:
        sys.exit(__doc__)
