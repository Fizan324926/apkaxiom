#!/usr/bin/env python3
# Codegen the cross-implementation reference values into a Rust
# source file. Run two-step:
#
#   pip3 install --break-system-packages blake3
#   python3 scripts/gen-cross-impl-rs.py
#
# Step 1 produces `test-vectors/cross-impl-python-blake3.json`
# from python blake3 (BLAKE3-team C reference); step 2 codegens
# `crates/axiom-blake3-hacl/src/cross_impl.rs` from the JSON.
import json
import os

JSON_PATH = 'crates/axiom-blake3-hacl/test-vectors/cross-impl-python-blake3.json'
RS_PATH = 'crates/axiom-blake3-hacl/src/cross_impl.rs'

# Step 1: regenerate the JSON if blake3 is installed.
try:
    import blake3
    fixture_dir = 'crates/axiom-l1-rs/tests/fixtures'
    result = {
        'producer': 'python blake3 ' + blake3.__version__,
        'fixtures': {},
        'official_paint_vectors': {},
    }
    for name in sorted(os.listdir(fixture_dir)):
        if not name.endswith('.apk'):
            continue
        body = open(os.path.join(fixture_dir, name), 'rb').read()
        result['fixtures'][name] = {
            'len': len(body),
            'blake3': blake3.blake3(body).hexdigest(),
        }
    for L in [0, 1, 2, 3, 4, 5, 6, 7, 8, 63, 64, 65, 127, 128, 129,
              1023, 1024, 1025, 2048, 2049, 3072, 3073, 4096, 4097,
              5120, 5121, 6144, 6145, 7168, 7169, 8192, 8193,
              16384, 31744, 102400]:
        inp = bytes(i % 251 for i in range(L))
        result['official_paint_vectors'][str(L)] = blake3.blake3(inp).hexdigest()
    with open(JSON_PATH, 'w') as f:
        json.dump(result, f, indent=2, sort_keys=True)
        f.write('\n')
    print(f'regenerated {JSON_PATH} via {result["producer"]}')
except ImportError:
    print(f'blake3 python package not installed; reusing committed {JSON_PATH}')

# Step 2: codegen Rust.
v = json.load(open(JSON_PATH))
out = []
add = out.append
add('// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.')
add('//')
add('// AUTO-GENERATED from `test-vectors/cross-impl-python-blake3.json` by')
add('// `scripts/gen-cross-impl-rs.py`. DO NOT EDIT BY HAND.')
add('//')
add('#![allow(clippy::unreadable_literal)]')
add('')
add('// Reference values produced by the Python `blake3` package')
add(f'// ({v["producer"]}), which wraps the BLAKE3-team reference C library —')
add('// an independent implementation from the Rust `blake3` crate we use')
add('// in production. Asserting Rust-crate output equals these values is the')
add('// cross-implementation check (P1.10 §B item 9 in CHECKLIST).')
add('')
add('/// Cross-impl reference: BLAKE3 of the raw bytes of each F-Droid APK fixture.')
add("pub static FIXTURE_BLAKE3: &[(&str, usize, [u8; 32])] = &[")
for name, info in sorted(v['fixtures'].items()):
    raw = bytes.fromhex(info['blake3'])
    bs = ', '.join(f'0x{b:02x}' for b in raw)
    add(f'    ("{name}", {info["len"]}, [{bs}]),')
add('];')
add('')
add('/// Cross-impl reference: BLAKE3 of `paint_test_input(len)` for each')
add('/// of the 35 BLAKE3-official input lengths.')
add("pub static PAINT_VECTORS_BLAKE3: &[(usize, [u8; 32])] = &[")
for L in sorted((int(k) for k in v['official_paint_vectors']), key=int):
    raw = bytes.fromhex(v['official_paint_vectors'][str(L)])
    bs = ', '.join(f'0x{b:02x}' for b in raw)
    add(f'    ({L}, [{bs}]),')
add('];')
add('')
open(RS_PATH, 'w').write('\n'.join(out))
print(f'wrote {RS_PATH}')
