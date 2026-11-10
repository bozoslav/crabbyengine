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
    if name.endswith('README.md') or name in ('AGENTS.md', 'CLAUDE.md', 'CHANGELOG.md', 'NOTICE', 'LICENSE-MIT') or name.endswith('CONTENT-NOTICES.txt'):
        return 'intentional history', 'Preserved project instructions, README text, release history or legal attribution.'
    if '/fixtures/' in name:
        return 'intentional history', 'Immutable historical ABI or saved-format baseline and its provenance/generator.'
    if name.startswith(('include/crabby', 'crates/ponko_ffi/src/legacy')):
        return 'deprecated compatibility', 'Optional legacy native exports and source shims; removal requires a major release no earlier than 2027-11-10.'
    if name.startswith('book/') or name == 'STABILITY.md':
        return 'deprecated compatibility', 'Migration instructions and deprecation schedule for legacy names.'
    if name.startswith('src/') or name == 'crates/ponko_mcp/src/lib.rs':
        return 'deprecated compatibility', 'Legacy input fallback, protocol interoperability alias or deprecated Rust alias and its regression coverage.'
    if name.startswith(('tests/', 'crates/ponko_ffi/tests/', 'crates/ponko_mcp/tests/')) or name.startswith('examples/cpp/') and not name.endswith('CMakeLists.txt'):
        return 'deprecated compatibility', 'Tests old native symbols/headers or old persisted/environment names during the deprecation cycle.'
    return 'deprecated compatibility', 'Compatibility-layer generation/checks, legacy build fallbacks or preserved leak-check configuration.'


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
