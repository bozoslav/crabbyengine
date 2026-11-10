#!/usr/bin/env python3
"""Check every primary/legacy dynamic export and cross-spelling ownership.

Usage: python3 scripts/test-native-symbols.py path/to/library [--primary-only]
The historical ABI-2 baseline client separately checks unchanged C layouts.
"""
import ctypes
from pathlib import Path
import re
import sys

root = Path(__file__).resolve().parent.parent
library = ctypes.CDLL(str(Path(sys.argv[1]).resolve()))
primary_only = '--primary-only' in sys.argv
header = (root / 'include/ponko.h').read_text()
names = sorted(set(re.findall(r'\b(ponko_\w+)\s*\(', header)))
assert names
for name in names:
    getattr(library, name)
    old = name.replace('ponko_', 'crabby_', 1)
    if primary_only:
        try:
            getattr(library, old)
        except AttributeError:
            pass
        else:
            raise AssertionError(f'Optional legacy export is still present: {old}')
    else:
        getattr(library, old)
for prefix in ['ponko_'] if primary_only else ['ponko_', 'crabby_']:
    version = getattr(library, prefix + 'abi_version')
    version.restype = ctypes.c_uint32
    assert version() == 2
    create = getattr(library, prefix + 'world_new')
    create.restype = ctypes.c_void_p
    world = create()
    assert world
    other = 'ponko_' if primary_only or prefix == 'crabby_' else 'crabby_'
    free = getattr(library, other + 'world_free')
    free.argtypes = [ctypes.c_void_p]
    free.restype = ctypes.c_uint32
    assert free(world) == 0
print(f'{len(names)} primary dynamic exports verified' + ('.' if primary_only else ', plus every legacy forwarding export.'))
