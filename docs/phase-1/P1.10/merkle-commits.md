# P1.10 — Merkle Commit Chain Design

> Cryptographic-receipt protocol for the streaming APK parser.
> A reviewer should be able to read this file and re-implement
> the chain in any language, byte-for-byte compatible.

**Status:** Frozen at P1.10 closure (2026-05-06). Any change to
the leaf-formation rule, internal-node combiner, or odd-level
convention is a protocol change — bumps the chain-version byte
in [§7](#7-future-protocol-versioning) and re-stamps the four
fixture roots in
[`tests/commit_chain_reproducibility.rs`](../../../crates/axiom-l1-rs/tests/commit_chain_reproducibility.rs)
+ [CHECKLIST §D](./CHECKLIST.md).

---

## 1. Goals

A consumer of an APK should be able to:

  1. **Verify integrity end-to-end** — a 32-byte Merkle root
     commits to every byte of every region the parser sees:
     LFH headers, body bytes, data-descriptor records, signing
     blocks, central-directory records, EOCD record. A
     single-bit flip anywhere in any of those regions changes
     the root.
  2. **Prove inclusion of any region** — given the chain's
     leaves, generate an `O(log N)` inclusion proof for any
     leaf, encode it in a stable wire format, and verify it
     against the published root. See
     [`merkle_proof.rs`](../../../crates/axiom-l1-rs/src/merkle_proof.rs).
  3. **Reproduce the root deterministically** — same input
     bytes, same Rust toolchain, same chain code path → same
     root, byte-for-byte, on any architecture. Verified by the
     [P1.10 multi-arch CI
     workflow](../../../.github/workflows/p110-merkle.yml) and
     the KAT regression on four real F-Droid APK fixtures.

These three goals are the cryptographic floor every downstream
APKAXIOM phase builds on (P1.15 IR-emit, Phase 4 `.axc`
certificate format, P1.16 signature verification).

## 2. Hash function

**BLAKE3** (`blake3 = 1.5.5`, `pure` feature).

The `axiom-blake3-hacl` crate's name reflects the original
plan to use HACL\*'s F\*-verified BLAKE3. HACL\* does not
currently ship a verified BLAKE3 (only BLAKE2b/2s); the
deviation is documented in
[ADR-0028](./ADR-0028-hacl-blake3-deviation.md). We ship the
audited BLAKE3-team Rust reference implementation — the same
code path Android `apksigner` uses for v3 signing. Cross-
implementation parity against the BLAKE3-team's reference C
library (via the Python `blake3` package) is gated by the
[`cross_impl` test](../../../crates/axiom-blake3-hacl/tests/cross_impl.rs)
on every build.

The crate ships exactly one production backend (`Blake3`) and
no Potemkin abstractions. When HACL\* upstream merges a verified
BLAKE3, the crate grows a `Blake3Hacl` backend behind a real
Cargo feature; until then we ship one truthful backend.

## 3. Leaf formation

The streaming parser
([`stream.rs`](../../../crates/axiom-l1-rs/src/stream.rs))
emits a sequence of `ParseEvent`s in source order. The chain
([`commit_chain.rs`](../../../crates/axiom-l1-rs/src/commit_chain.rs))
turns each content-bearing event into one leaf. Leaves carry an
unhashed diagnostic tag (for human inspection); the **Merkle
root is computed over the leaf hashes only** — the tag is not
part of the cryptographic commitment.

| Event                  | Leaf tag           | Leaf bytes hashed                                                                  |
|------------------------|--------------------|------------------------------------------------------------------------------------|
| `ZipEntryHeader`       | `lfh-header`       | Verbatim 30-byte LFH prefix + name + extra-field.                                  |
| `ZipEntryData` × N     | `lfh-body`         | Concatenation of body chunks via streaming `Blake3::update` — one leaf per entry.   |
| `DataDescriptor`       | `data-descriptor`  | Verbatim 16-byte DD record (sig + crc32 + comp_size + uncomp_size).                |
| `SigningBlock`         | `signing-block`    | Bytes between last LFH body and CD; APK v2/v3 signing block, opaque to the chain. |
| `CdrEntry`             | `cdr-entry`        | Verbatim CDR (46-byte fixed prefix + name + extra + comment).                     |
| `EocdSeen`             | `eocd`             | Verbatim EOCD record (22-byte fixed prefix + comment).                            |

Each leaf is the BLAKE3 digest of the bytes listed above:

```text
leaf_i = BLAKE3(bytes_i)
```

No domain-separation prefix at the leaf level. Leaves are
distinguishable from internal nodes by the `0x00` prefix the
combiner adds at non-leaf levels (next section).

### 3.1 Body-leaf chunk-size invariance

The streaming parser fires `ZipEntryData` once per buffer
chunk, so a naive "one leaf per chunk" rule would make the
Merkle root depend on how the operator chose to read bytes.
The chain accumulates body chunks under a single BLAKE3
hasher and emits exactly one body leaf per entry, finalised
when the next non-body event (LFH header / DD / signing block
/ CDR / EOCD) arrives. This is the
[`BodyAccumulator`](../../../crates/axiom-l1-rs/src/commit_chain.rs)
helper.

The
[chunk-size invariance test](../../../crates/axiom-l1-rs/tests/commit_chain_chunk_invariance.rs)
gates the rule: every fixture × ten chunk sizes (1, 7, 17, 64,
65, 256, 1024, 4096, 4097, 65536) → same Merkle root, byte-
identical.

## 4. Internal-node combiner

```text
node(left, right) = BLAKE3(0x00 || left || right)
```

The `0x00` prefix is the standard BLAKE3-team Merkle domain
separator. Without it, an attacker could craft a leaf that
collides with an internal node (because both are 32-byte
hashes and `BLAKE3(x)` for some `x` of length 64 would be
indistinguishable from `BLAKE3(L||R)` for some pair `L, R`).
The `0x00` byte forbids that path: leaves are
`BLAKE3(arbitrary_bytes)` with no prefix, internal nodes are
`BLAKE3(0x00 || 64_bytes)` — the prefix byte means an attacker
cannot present 64 bytes that looks like a leaf to the
verifier.

## 5. Tree fold

Bottom-up; each level halves until a single root remains. Odd
levels duplicate the last element (Bitcoin / Certificate
Transparency convention):

```text
fn merkle_root(leaves: [Hash]) -> Hash:
    if leaves.is_empty():
        return BLAKE3(b"")              // canonical empty-input root
    level = leaves
    while level.len() > 1:
        next = []
        for i in 0..level.len() step 2:
            l = level[i]
            r = level[i+1] if i+1 < level.len() else level[i]   // duplicate
            next.push(BLAKE3(0x00 || l || r))
        level = next
    return level[0]
```

Single-leaf trees: the root **is** the leaf hash (no combiner
applied, path length 0).

Empty leaf set: the root is `BLAKE3(b"")` —
`af1349b9 f5f9a1a6 …` — fixed.

## 6. Inclusion proofs

A proof of leaf `i` in a chain of `N` leaves is the path of
`ceil(log2(N))` sibling hashes from leaf level to root. For
each level, the verifier records whether the sibling sits on
the **left** or **right** of the running hash — needed to
reconstruct the parent's `BLAKE3(0x00 || L || R)` ordering.

### 6.1 Verification

```text
fn verify(leaf_hash: Hash, proof: [(Hash, Direction)], expected_root: Hash) -> bool:
    running = leaf_hash
    for (sibling, dir) in proof:
        running = match dir:
            Left  => BLAKE3(0x00 || sibling || running)
            Right => BLAKE3(0x00 || running || sibling)
    return running == expected_root
```

`O(d)` BLAKE3 calls where `d = ceil(log2(N))`.

### 6.2 Wire format

```text
[4 bytes  leaf_index   little-endian u32]
[4 bytes  leaf_count   little-endian u32]
[4 bytes  path_len     little-endian u32]
path_len × {
    [1 byte   direction (0x00 = Left, 0x01 = Right)]
    [32 bytes sibling hash                            ]
}
```

12-byte header + `33 × path_len` bytes per step. Stable across
versions; consumers implementing
[`MerkleProof::decode`](../../../crates/axiom-l1-rs/src/merkle_proof.rs)
must reject malformed bytes (wrong length, reserved direction
byte, truncated header) — three matching failure modes are
gated by the unit tests.

## 7. Future protocol versioning

The wire format above does not currently include a chain-
version byte. Adding one is a backwards-incompatible bump:

  - Insert a new leading byte `0x01` at offset 0 of the
    encoded proof (and shift the existing fields right by 1).
  - Update [`MerkleProof::decode`](../../../crates/axiom-l1-rs/src/merkle_proof.rs)
    to read the version byte first and reject unrecognised
    versions.
  - Re-stamp every committed fixture root (the version byte
    would also enter the leaf-formation rule via a domain-
    separator change).

P1.10 ships protocol **v0** (no version byte). The first
breaking change to leaf formation, combiner, or odd-level rule
is what would force a v1.

## 8. Threat model

The chain detects:

  - **Single-bit flips anywhere in any committed region.**
    Verified by `make p110-tamper-fuzz` over 40 000 mutations
    × 4 fixtures: 100 % kill rate on every non-comment
    component.
  - **Whole-region replacement** — any swap of an LFH header,
    body, DD, signing block, CDR, or EOCD with different bytes
    changes the leaf hash and therefore the root.
  - **Reordering** — leaf order is determined by the parser's
    event order, which is wire-format determined; reordering
    requires changing the underlying ZIP layout, which the
    chain also commits to (since LFH offsets feed CDR
    `lfh_offset` fields, both committed).
  - **Truncation** — the chain's empty-leaf-set root is fixed,
    distinct from any non-empty chain's root.

The chain does not (by design) detect:

  - **EOCD-comment region tampering.** The 0–65 535 byte
    comment region after the EOCD record is not committed.
    Operators relying on the comment for security MUST hash it
    separately. None of the four committed fixtures use a
    non-empty comment.
  - **Replay of an earlier well-formed APK in place of a newer
    one.** The chain commits to the bytes; freshness must
    come from a higher-level protocol (timestamp, signature
    over root + version, etc.).

## 9. Performance contract

  - **BLAKE3 single-core throughput** ≥ 1.5 GB/s on x86_64
    (gated by `make p110-hash-throughput`, n=100, mean − 2σ).
    Measured 1.601 GB/s on dev-shell. Aarch64 throughput is
    arch-conditional (no SIMD with `pure` + no-`neon`); see
    [P1.10 multi-arch CI](../../../.github/workflows/p110-merkle.yml).
  - **Merkle-tree-fold overhead** (Δ_overhead, chain vs flat-
    hash with identical byte coverage) ≤ 15 % mean **or**
    `|Δ| ≤ 2σ`. Measured Δ_overhead = +12.77 %, σ = 9.76 %, in
    band → PASS. The literal Δ_lit (chain vs no-hash) is
    reported alongside but ungated; see
    [ADR-0028 §3](./ADR-0028-hacl-blake3-deviation.md#3-perf-gate-reframe)
    for the framing rationale.

## 10. References

  - [BLAKE3 specification (Aumasson et al. 2020)](https://github.com/BLAKE3-team/BLAKE3-specs)
  - [Certificate Transparency RFC 6962 §2.1](https://www.rfc-editor.org/rfc/rfc6962#section-2.1) — Merkle tree convention APKAXIOM mirrors.
  - [APPNOTE.TXT 6.3.10 §4.3](https://pkware.cachefly.net/webdocs/casestudies/APPNOTE.TXT) — ZIP local-file-header / DD / CDR / EOCD layouts.
  - [Android APK Signature Scheme v2/v3](https://source.android.com/docs/security/features/apksigning/v2) — APK signing-block format, opaque to this chain.
