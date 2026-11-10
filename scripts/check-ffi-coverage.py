#!/usr/bin/env python3
"""Fail when a C ABI export has no test and no reasoned allowlist entry.

An export (`extern "C" fn` in crates/ponko_ffi/src) counts as tested when its
`crabby_` or `ponko_` spelling appears, outside comments and string literals
(a name in a message or a path is not a call), in an FFI test
(crates/ponko_ffi/tests/*.rs) or a native suite (examples/cpp/*.c, *.cpp).
Doctests in src/ don't count: most only pass a null pointer.

The functions that cannot be tested on a headless machine (they need a real
window, a GPU frame or a Steam client) are listed with a reason in
scripts/ffi-coverage-allowlist.txt. The check fails when
  - an export is untested and not on the allowlist (the count went up),
  - an allowlist entry is now tested or no longer exists (shrink the list),
  - an allowlist entry has no reason or appears twice.

    python3 scripts/check-ffi-coverage.py          # check
    python3 scripts/check-ffi-coverage.py --list   # print the untested exports
"""
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parent.parent
ALLOWLIST = ROOT / 'scripts' / 'ffi-coverage-allowlist.txt'
EXPORT = re.compile(r'extern "C" fn ([a-z0-9_]+)')
# What is not code: Rust raw strings, ordinary strings (Rust and C), character
# literals (so '"' does not start a string), block comments and line comments.
# Strings and characters are blanked, comments dropped, so only calls remain.
NOT_CODE = re.compile(
    r"""(?P<raw>b?r(?P<hashes>\#*)".*?"(?P=hashes))"""
    r"""|(?P<char>'(?:\\.|[^'\\\n])')"""
    r"""|(?P<string>b?c?"(?:\\.|[^"\\])*")"""
    r"""|(?P<block>/\*.*?\*/)"""
    r"""|(?P<line>//[^\n]*)""",
    re.DOTALL,
)


def code_only(text):
    def blank(match):
        if match.lastgroup in ('block', 'line'):
            return ' '
        return '""'
    return NOT_CODE.sub(blank, text)


def exports():
    names = set()
    for source in sorted((ROOT / 'crates/ponko_ffi/src').rglob('*.rs')):
        names.update(EXPORT.findall(source.read_text()))
    return names


def test_text():
    files = sorted((ROOT / 'crates/ponko_ffi/tests').glob('*.rs'))
    files += sorted((ROOT / 'examples/cpp').glob('*.c')) + sorted((ROOT / 'examples/cpp').glob('*.cpp'))
    return '\n'.join(code_only(f.read_text()) for f in files)


def untested(names, text):
    result = set()
    for name in names:
        short = name.removeprefix('crabby_')
        if not re.search(rf'(^|[^a-z0-9_])({name}|ponko_{short})([^a-z0-9_]|$)', text):
            result.add(name)
    return result


def allowlist():
    entries, problems = {}, []
    for number, line in enumerate(ALLOWLIST.read_text().splitlines(), 1):
        if not line.strip() or line.lstrip().startswith('#'):
            continue
        name, _, reason = line.partition('#')
        name = name.strip()
        if not reason.strip():
            problems.append(f'{ALLOWLIST.name}:{number}: {name} has no reason after "#"')
        if name in entries:
            problems.append(f'{ALLOWLIST.name}:{number}: {name} is listed twice')
        entries[name] = reason.strip()
    return entries, problems


def main():
    names = exports()
    missing = untested(names, test_text())
    if '--list' in sys.argv:
        print('\n'.join(sorted(missing)))
        print(f'{len(missing)} of {len(names)} exports have no test', file=sys.stderr)
        return 0
    allowed, problems = allowlist()
    for name in sorted(missing - allowed.keys()):
        problems.append(f'{name}: no test calls it and it is not on the allowlist. Test it '
                        f'(crates/ponko_ffi/tests/) or, if it needs a window or Steam, add it to '
                        f'scripts/{ALLOWLIST.name} with the reason.')
    for name in sorted(allowed.keys() - names):
        problems.append(f'{name}: on the allowlist but not exported any more; remove the entry.')
    for name in sorted((allowed.keys() & names) - missing):
        problems.append(f'{name}: has a test now; remove it from the allowlist.')
    if problems:
        print('\n'.join(problems))
        return 1
    print(f'{len(names) - len(missing)} of {len(names)} C exports are tested; '
          f'{len(missing)} are on the allowlist.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
