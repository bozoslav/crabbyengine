#!/usr/bin/env python3
"""Fold changelog.d/ fragments into CHANGELOG.md's [Unreleased] section.

    python3 scripts/changelog.py           # fold the fragments in and delete them
    python3 scripts/changelog.py --dry-run # print the new Unreleased section only
    python3 scripts/changelog.py --check   # validate the fragments (CI runs this)

A fragment is changelog.d/<pr-or-topic>.md holding one or more `### Heading`
sections with `- ` bullets (see changelog.d/README.md). Bullets are added at
the end of the Unreleased subsection with the same heading; a missing
subsection is created in Keep a Changelog order, with `Breaking changes`
first. Fragments fold in file-name
order.
"""
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parent.parent
FRAGMENTS = ROOT / 'changelog.d'
CHANGELOG = ROOT / 'CHANGELOG.md'
ORDER = ['Breaking changes', 'Added', 'Changed', 'Deprecated', 'Removed', 'Fixed', 'Security']
HEADING = re.compile(r'^### (.+?)\s*$')


def fragment_files():
    if not FRAGMENTS.is_dir():
        return []
    return sorted(p for p in FRAGMENTS.glob('*.md') if p.name != 'README.md')


def parse(path):
    """A fragment as [(heading, [lines])], or exits with a message."""
    sections = []
    for number, line in enumerate(path.read_text().splitlines(), 1):
        match = HEADING.match(line)
        if match:
            sections.append((match.group(1), []))
        elif not line.strip():
            continue
        elif not sections:
            sys.exit(f'{path.relative_to(ROOT)}:{number}: text before the first `### Heading`')
        elif line.startswith('#'):
            sys.exit(f'{path.relative_to(ROOT)}:{number}: use `### Heading` sections only')
        elif not sections[-1][1] and not line.startswith('- '):
            sys.exit(f'{path.relative_to(ROOT)}:{number}: a section starts with a `- ` bullet')
        else:
            sections[-1][1].append(line.rstrip())
    if not sections:
        sys.exit(f'{path.relative_to(ROOT)}: no `### Heading` section')
    for heading, lines in sections:
        if not lines:
            sys.exit(f'{path.relative_to(ROOT)}: section `{heading}` has no bullets')
    return sections


def unreleased_bounds(lines):
    start = next((i for i, l in enumerate(lines) if l.startswith('## [Unreleased]')), None)
    if start is None:
        sys.exit('CHANGELOG.md has no `## [Unreleased]` section')
    end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith('## ')), len(lines))
    return start, end


def fold(body, heading, bullets):
    """Add bullets to the subsection `heading` of body (the Unreleased lines)."""
    heads = [i for i, l in enumerate(body) if HEADING.match(l)]
    named = [i for i in heads if HEADING.match(body[i]).group(1) == heading]
    if named:
        start = named[0]
        end = next((i for i in heads if i > start), len(body))
        while end > start + 1 and not body[end - 1].strip():
            end -= 1
        body[end:end] = bullets
        return
    # New subsection: before the first standard heading that comes later in
    # ORDER, otherwise at the end.
    rank = ORDER.index(heading) if heading in ORDER else len(ORDER)
    at = len(body)
    for i in heads:
        name = HEADING.match(body[i]).group(1)
        if name in ORDER and ORDER.index(name) > rank:
            at = i
            break
    while at > 0 and not body[at - 1].strip():
        at -= 1
    body[at:at] = ['', f'### {heading}', *bullets]


def main():
    files = fragment_files()
    parsed = [(path, parse(path)) for path in files]
    if '--check' in sys.argv:
        print(f'Changelog fragments passed ({len(files)} pending).')
        return
    if not files:
        print('No changelog fragments to fold.')
        return
    lines = CHANGELOG.read_text().splitlines()
    start, end = unreleased_bounds(lines)
    body = lines[start + 1:end]
    for _, sections in parsed:
        for heading, bullets in sections:
            fold(body, heading, bullets)
    while body and not body[-1].strip():
        body.pop()
    body.append('')
    if '--dry-run' in sys.argv:
        print('\n'.join([lines[start], *body]))
        return
    lines[start + 1:end] = body
    CHANGELOG.write_text('\n'.join(lines) + '\n')
    for path in files:
        path.unlink()
    print(f'Folded {len(files)} fragment(s) into CHANGELOG.md.')


if __name__ == '__main__':
    main()
