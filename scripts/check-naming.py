#!/usr/bin/env python3
"""Check the reviewed legacy-name budget; --inventory refreshes it for review."""
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
INVENTORY = ROOT / 'naming-inventory.json'
LEGACY = re.compile('crabby', re.IGNORECASE)


def references():
    paths = subprocess.check_output(['git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard'], cwd=ROOT).decode().split('\0')
    result = {}
    for name in sorted(set(paths)):
        path = ROOT / name
        if not name or name == INVENTORY.name or not path.is_file():
            continue
        try:
            count = len(LEGACY.findall(path.read_text()))
        except UnicodeDecodeError:
            count = 0
        path_count = len(LEGACY.findall(name))
        if count or path_count:
            result[name] = {'occurrences': count, 'path_occurrences': path_count}
    return result


def classify(name):
    if name.startswith(('include/', 'crates/ponko_ffi/')):
        return 'public compatibility', 'Stable native ABI implementation, generated aliases or compatibility coverage.'
    if name.endswith('README.md') or name in ('AGENTS.md', 'CLAUDE.md', 'CHANGELOG.md', 'NOTICE', 'LICENSE-MIT'):
        return 'intentional history', 'Pre-existing project instructions, README text, release history or legal attribution.'
    if name.startswith(('tests/', 'assets/')) or '/tests/' in name:
        return 'public compatibility', 'Existing saved-format, native or environment compatibility fixtures; isolated test names.'
    if name.startswith('book/') or name in ('STABILITY.md', 'CONTRIBUTING.md', 'SECURITY.md'):
        return 'public migration', 'Explains retained native, saved-format, registry, embedding or historical contracts.'
    if name.startswith('scripts/') or name.endswith('CMakeLists.txt'):
        return 'public compatibility', 'Migration checks, alias generation or existing leak/build configuration fallback.'
    return 'public compatibility', 'Retained public format, ABI type, environment fallback, embedding or external output identifier.'


def check(current, baseline):
    errors = []
    for name, counts in current.items():
        allowed = baseline.get(name, {})
        for key, count in counts.items():
            if count > allowed.get(key, 0):
                errors.append(f'{name}: {key} {count} exceeds reviewed budget {allowed.get(key, 0)}')
    return errors


def main():
    current = references()
    if '--inventory' in sys.argv:
        entries = {}
        for name, counts in current.items():
            category, reason = classify(name)
            entries[name] = dict(counts, category=category, reason=reason)
        INVENTORY.write_text(json.dumps({'description': 'Reviewed legacy references; see book/src/naming.md. Decreases are allowed; increases require review.', 'files': entries}, indent=2) + '\n')
        return
    errors = check(current, json.loads(INVENTORY.read_text())['files'])
    if errors:
        sys.exit('\n'.join(errors))
    print(f'Naming inventory passed ({len(current)} legacy-bearing files).')


if __name__ == '__main__':
    main()
