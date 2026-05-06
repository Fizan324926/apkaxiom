#!/usr/bin/env python3
# Codegen the blake3 official test vectors into a Rust source file.
# Run: python3 scripts/gen-blake3-vectors.py
# Input:  crates/axiom-blake3-hacl/test-vectors/blake3-1.5.5.json
# Output: crates/axiom-blake3-hacl/src/vectors.rs
#
# The JSON is the verbatim BLAKE3-team `test_vectors.json` from the
# `1.5.5` release tag. Codegen runs offline against the committed
# JSON; nothing fetches at build time.
import json, sys

SRC = 'crates/axiom-blake3-hacl/test-vectors/blake3-1.5.5.json'
DST = 'crates/axiom-blake3-hacl/src/vectors.rs'

v = json.load(open(SRC))
key = v['key']
context = v['context_string']
cases = v['cases']
assert len(key) == 32

out = []
add = out.append
add('// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.')
add('//')
add('// AUTO-GENERATED from `test-vectors/blake3-1.5.5.json` by')
add('// `scripts/gen-blake3-vectors.py`. DO NOT EDIT BY HAND.')
add('//')
add('// Source: https://github.com/BLAKE3-team/BLAKE3/blob/1.5.5/test_vectors/test_vectors.json')
add('//')
add('// 35 input lengths × {hash, keyed_hash, derive_key} = 105 vectors.')
add('// Each output is 131 bytes (1048-bit XOF); the standard 32-byte BLAKE3')
add('// digest is the first 32 bytes.')
add('')
add('#![allow(clippy::unreadable_literal)]')
add('')
def rust_string_literal(s: str) -> str:
    out = ['"']
    for c in s:
        if c == '\\' or c == '"':
            out.append('\\')
            out.append(c)
        elif c == '\n':
            out.append('\\n')
        elif c == '\r':
            out.append('\\r')
        elif c == '\t':
            out.append('\\t')
        elif ord(c) < 0x20 or ord(c) == 0x7f:
            out.append(f'\\u{{{ord(c):x}}}')
        else:
            out.append(c)
    out.append('"')
    return ''.join(out)

add(f'/// 32-byte BLAKE3 keyed-hash key, ASCII: {rust_string_literal(key)}.')
add(f'pub const VECTOR_KEY: [u8; 32] = *b{rust_string_literal(key)};')
add('')
add('/// `derive_key` context string for the official suite.')
add(f'pub const VECTOR_CONTEXT: &str = {rust_string_literal(context)};')
add('')
add('/// One BLAKE3 test-vector case.')
add('#[derive(Debug)]')
add('pub struct Vector {')
add("    /// Input is the byte sequence `[i % 251 for i in 0..input_len]`.")
add('    pub input_len: usize,')
add('    /// Expected XOF output (131 bytes) for `BLAKE3.hash(input)`.')
add("    pub hash_xof: &'static [u8],")
add('    /// Expected XOF output for `BLAKE3.keyed_hash(VECTOR_KEY, input)`.')
add("    pub keyed_xof: &'static [u8],")
add('    /// Expected XOF output for `BLAKE3.derive_key(VECTOR_CONTEXT, input)`.')
add("    pub derive_key_xof: &'static [u8],")
add('}')
add('')

def emit_bytes(name, hexstr):
    raw = bytes.fromhex(hexstr)
    chunks = [f'0x{raw[i]:02x}' for i in range(len(raw))]
    add(f'static {name}: [u8; {len(raw)}] = [')
    for i in range(0, len(chunks), 12):
        add('    ' + ', '.join(chunks[i:i+12]) + ',')
    add('];')

for idx, case in enumerate(cases):
    L = case['input_len']
    emit_bytes(f'H_{idx:02}_{L}_HASH', case['hash'])
    emit_bytes(f'H_{idx:02}_{L}_KEYED', case['keyed_hash'])
    emit_bytes(f'H_{idx:02}_{L}_DERIVE', case['derive_key'])
    add('')

add(f'/// All {len(cases)} BLAKE3 official test vectors.')
add('pub const VECTORS: &[Vector] = &[')
for idx, case in enumerate(cases):
    L = case['input_len']
    add('    Vector {')
    add(f'        input_len: {L},')
    add(f'        hash_xof: &H_{idx:02}_{L}_HASH,')
    add(f'        keyed_xof: &H_{idx:02}_{L}_KEYED,')
    add(f'        derive_key_xof: &H_{idx:02}_{L}_DERIVE,')
    add('    },')
add('];')
add('')

open(DST, 'w').write('\n'.join(out))
print(f'wrote {DST}, {len(cases)} vectors')
