#!/usr/bin/env python3
"""Generate the deprecated compatibility layer from primary Ponko declarations.

Regenerate include/ponko.h with cbindgen first. --check never edits files.
"""
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
NOTICE = 'Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10.'

def legacy(name):
    return name.replace('ponko_', 'crabby_', 1).replace('Ponko', 'Crabby', 1).replace('PONKO_', 'CRABBY_', 1)

def split_args(args):
    # Function-pointer parameters can contain commas, as can array types.
    depth = 0
    start = 0
    for i, char in enumerate(args):
        if char in '([<':
            depth += 1
        elif char in ')]>':
            if char != '>' or (i and args[i - 1] != '-'):
                depth -= 1
        elif char == ',' and depth == 0:
            if args[start:i].strip():
                yield args[start:i].strip()
            start = i + 1
    if args[start:].strip():
        yield args[start:].strip()

def declarations(sources):
    """Read the deliberately restricted, non-generic FFI declaration grammar.

    Fail closed on representation/field forms we cannot bridge. Rust compilation
    and generated size/alignment/offset assertions independently check the result.
    """
    result = {}
    pattern = r'((?:#\[[^\]]*\]\s*)*)pub (struct|enum|type) (Ponko\w+)'
    for source in sources:
        for match in re.finditer(pattern, source):
            attrs, kind, name = match.groups()
            start = match.end()
            if kind == 'type':
                end = source.index(';', start) + 1
            else:
                rest = source[start:].lstrip()
                if rest.startswith('{'):
                    opening = source.index('{', start)
                    depth = 1
                    end = opening + 1
                    while depth:
                        if source[end] == '{': depth += 1
                        elif source[end] == '}': depth -= 1
                        end += 1
                else:
                    end = source.index(';', start) + 1
            body = source[start:end]
            # Comments can mention unrelated types, so exclude them from the graph.
            body = re.sub(r'//[^\n]*', '', body)
            result[name] = (attrs.strip(), kind, body)
    return result


def rename_types(text):
    return re.sub(r'\b(?:Ponko|PONKO_)\w+', lambda m: legacy(m.group()), text)


def bridge(expression, ty, distinct):
    if not any(n in distinct for n in re.findall(r'\bPonko\w+', ty)):
        return expression
    if expression.startswith('unsafe '):
        expression = f'({expression})'
    if ty.strip().startswith(('*mut ', '*const ')):
        return f'{expression}.cast()'
    if ty.strip().startswith('['):
        inner = ty.strip()[1:].split(';', 1)[0]
        return f'{expression}.map(|v| {bridge("v", inner, distinct)})'
    if ty.strip() in distinct:
        return f'{expression}.into()'
    raise SystemExit(f'Unsupported legacy ABI conversion: {ty}; extend the generator explicitly')


def compatibility_types(decls, sources, attr):
    # Enum variants must remain real variants: type aliases plus associated
    # constants break explicit/wildcard imports from the old enum path.
    distinct = {n for n, (_, kind, _) in decls.items() if kind == 'enum'}
    while True:
        expanded = distinct | {n for n, (_, _, body) in decls.items()
                               if distinct.intersection(re.findall(r'\bPonko\w+', body))}
        if expanded == distinct: break
        distinct = expanded
    rust = []
    all_source = '\n'.join(sources)
    for name, (attrs, kind, body) in decls.items():
        old = legacy(name)
        if name not in distinct:
            rust += [attr, f'pub use crate::{name} as {old};']
            continue
        if kind not in ('enum', 'struct') or not any(r in attrs for r in ('#[repr(C)]', '#[repr(u32)]')):
            raise SystemExit(f'Unsupported distinct legacy type {name}: require a plain repr(C) enum/struct')
        rust += [attr, attrs, f'pub {kind} {old}{rename_types(body)}']
        fields = re.findall(r'pub (\w+): ([^,\n]+)', body) if kind == 'struct' else []
        variants = re.findall(r'^\s*(PONKO_\w+)\s*(?:=|,)', body, re.M) if kind == 'enum' else []
        if kind == 'struct':
            if not fields or len(fields) != len(list(split_args(body.strip()[1:-1]))):
                raise SystemExit(f'Unsupported fields in {name}; only public named fields may be bridged')
        for source_type, target_type, reverse in [(f'crate::{name}', old, False), (old, f'crate::{name}', True)]:
            rust += [f'impl From<{source_type}> for {target_type} {{', f'fn from(value: {source_type}) -> Self {{']
            if kind == 'enum':
                rust += ['match value {']
                rust += [f'{source_type}::{legacy(v) if reverse else v} => Self::{v if reverse else legacy(v)},' for v in variants]
                rust += ['}']
            else:
                rust += ['Self {']
                rust += [f'{field}: {bridge("value." + field, ty, distinct)},' for field, ty in fields]
                rust += ['}']
            rust += ['}', '}']
        if kind == 'struct' and re.search(r'impl Default for ' + name + r'\b', all_source):
            rust += [f'impl Default for {old} {{ fn default() -> Self {{ crate::{name}::default().into() }} }}']
        rust += ['const _: () = {', f'assert!(std::mem::size_of::<{old}>() == std::mem::size_of::<crate::{name}>());', f'assert!(std::mem::align_of::<{old}>() == std::mem::align_of::<crate::{name}>());']
        rust += [f'assert!(std::mem::offset_of!({old}, {field}) == std::mem::offset_of!(crate::{name}, {field}));' for field, _ in fields]
        rust += [f'assert!({old}::{legacy(v)} as u32 == crate::{name}::{v} as u32);' for v in variants]
        rust += ['};']
        for variant in variants:
            rust += [attr, f'pub use {old}::{legacy(variant)};']
    return rust, distinct

def outputs():
    header = (ROOT / 'include/ponko.h').read_text()
    if re.search('crabby', header, re.I):
        raise SystemExit('Primary header contains legacy names; regenerate it from primary declarations.')
    names = sorted(set(re.findall(r'\b(?:ponko_[A-Za-z0-9_]+|Ponko[A-Za-z0-9_]+|PONKO_[A-Za-z0-9_]+)\b', header)))
    c = ['/* Generated by scripts/generate-ponko-header.py; do not edit. */', f'/* {NOTICE} */', '#ifndef CRABBY_H', '#define CRABBY_H', '#include "ponko.h"', '']
    c += [f'#define {legacy(n)} {n}' for n in names if n != 'PONKO_H']
    c += ['', '#endif', '']
    rust = ['//! Generated deprecated ABI/source compatibility; do not edit.', f'//! {NOTICE}', '#![allow(deprecated, missing_docs)]', 'use std::ffi::{c_char, c_void};', '']
    attr = f'#[deprecated(note = "{NOTICE}")]'
    sources = [p.read_text() for p in sorted((ROOT / 'crates/ponko_ffi/src').glob('*.rs')) if p.name != 'legacy.rs']
    types, distinct = compatibility_types(declarations(sources), sources, attr)
    rust += types
    for source in sources:
        for name in re.findall(r'pub const (PONKO_\w+)', source):
            rust += [attr, f'pub use crate::{name} as {legacy(name)};']
        for match in re.finditer(r'pub (unsafe )?extern "C" fn (ponko_\w+)\s*\(', source):
            unsafe, name = match.groups()
            start = match.end()
            depth = 1
            end = start
            while depth:
                if source[end] == '(':
                    depth += 1
                elif source[end] == ')':
                    depth -= 1
                end += 1
            args = source[start:end - 1]
            ret_end = source.index('{', end)
            result = source[end:ret_end].strip()
            params = ', '.join(bridge(a.split(':', 1)[0].strip(), a.split(':', 1)[1].strip(), distinct) for a in split_args(args))
            call = f'crate::{name}({params})'
            if unsafe:
                call = 'unsafe { ' + call + ' }'
            if result:
                call = bridge(call, result.removeprefix('->').strip(), distinct)
            if unsafe:
                rust += ['/// # Safety', f'/// The caller must uphold the same preconditions as [`crate::{name}`].']
            rust += [attr, '#[unsafe(no_mangle)]', f'pub {unsafe or ""}extern "C" fn {legacy(name)}({rename_types(args)}) {rename_types(result)} {{']
            if unsafe:
                rust += ['    // SAFETY: ABI-2 forwarding preserves the primary function\'s exact', '    // pointer, lifetime, ownership and callback preconditions. Distinct value', '    // types have identical repr(C) layouts and valid discriminants, checked above.']
            rust += [f'    {call}', '}', '']
    # Match cargo fmt so --check works after normal formatter runs.
    formatted = subprocess.run(['rustfmt', '--edition', '2024', '--config-path', str(ROOT / 'rustfmt.toml')], input='\n'.join(rust), text=True, check=True, capture_output=True).stdout
    return {ROOT / 'include/crabby.h': '\n'.join(c), ROOT / 'crates/ponko_ffi/src/legacy.rs': formatted}

def main():
    for path, output in outputs().items():
        if '--check' in sys.argv:
            if not path.exists() or path.read_text() != output:
                raise SystemExit(f'{path.relative_to(ROOT)} is stale; run scripts/generate-ponko-header.py')
        else:
            path.write_text(output)

if __name__ == '__main__':
    main()
