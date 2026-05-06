# P1.15 — Closure Checklist

**Status:** ✅ closed (AXIOM-IR-v0.1 emitter: AXML + ARSC chunked parsers, glue layer, round-trip gate 100/100, IR overhead 5.9 %, determinism confirmed) on 2026-05-06.

**Spec gates** (P1.15 §10):

| Gate | Result |
|---|---|
| AXML chunked-binary parser + emitter (HARD) | ✅ `crates/axiom-l1-rs/src/ir/axml.rs` — `parse` + `emit` + `round_trip`; 5 unit tests (parse, round-trip, rejects-non-axml, rejects-truncated, rejects-chunk-overflow) all green |
| ARSC chunked-binary parser + emitter (HARD) | ✅ `crates/axiom-l1-rs/src/ir/arsc.rs` — same chunk-walk pattern; `ArscDoc` preserves trailer bytes for alignment padding; 3 unit tests all green |
| `ManifestIr` + `ResourceIr` glue layer | ✅ `crates/axiom-l1-rs/src/ir/emit.rs` — `emit_manifest`, `reencode_manifest`, `emit_resources`, `reencode_resources`; 1 unit test |
| Round-trip gate ≥ 95 % byte-identical on F-Droid corpus (HARD) | ✅ **100/100 APKs pass** both AXML and ARSC channels; `p115-roundtrip --corpus fuzz/corpus/real-apks`; elapsed < 0.1 s |
| IR emission overhead ≤ 15 % throughput hit (HARD) | ✅ **+5.9 % overhead** (arm A base=3 002 ns, arm B base+IR=3 179 ns); `p115-ir-overhead-bench`; gate 15 % |
| IR output deterministic — same input → same bytes | ✅ 3 determinism tests: `axml_determinism_synthetic`, `arsc_determinism_synthetic`, `emit_manifest_glue_determinism`; parse is structurally `PartialEq` + `Clone`; emit is pure |

---

## §A. Architecture

### Round-trip strategy: raw bytes alongside parsed form

Every `Chunk` carries both `type_id` and `raw: Vec<u8>` (the full
chunk header + payload). Re-emission writes the raw bytes back
unchanged. This guarantees byte-identical round-trip by construction
without recomputing string-pool offsets, attribute table layouts, or
chunk padding — all of which vary between aapt / aapt2 emitters.

AXML outer wrapper: `RES_XML` (0x0003), 8-byte header. Inner chunks
cover all known types (string pool 0x0001, resource map 0x0180,
namespace start/end 0x0100/0x0101, element start/end 0x0102/0x0103,
CDATA 0x0104) plus any unknown type (preserved opaque).

ARSC outer wrapper: `RES_TABLE` (0x0002), 12-byte header (adds
`package_count`). Inner chunks include the global string pool
(0x0001) + per-package chunks (0x0200). `ArscDoc` captures a
`trailer: Vec<u8>` for the aapt2 alignment bytes that may appear
after the last inner chunk.

### `Apk<Unverified>` accessor surface extension

Added `manifest_bytes() -> Option<&[u8]>` and
`resources_bytes() -> Option<&[u8]>` to `Apk<Unverified>`. The
captured bodies are already in memory after `from_reader`; these
accessors expose them without requiring the caller to advance
through signature verification. Used by `p115-roundtrip` and the
IR overhead bench to feed real APK bytes into the IR layer without
signature-chain ceremony.

### Overhead bench methodology

The bench fixture is a 4-entry stored ZIP carrying minimal valid
AXML (36 bytes, 1 string-pool chunk) and minimal ARSC (40 bytes,
1 string-pool chunk). Arm A parses with `Apk::from_reader` only;
arm B adds `emit_manifest + reencode_manifest + emit_resources +
reencode_resources`. Five runs × 100 000 iterations each; warm-up
of 1 000 iters discarded. Result: 5.9 % overhead vs 15 % gate.

---

## §C. Operator one-shots (no code change required)

| # | Item | Blocker |
|---|---|---|
| §C-1 | Run `p115-roundtrip` against AndroZoo Bench-1K (1 000 real APKs, academic license) | AndroZoo credentials |
| §C-2 | Run `p115-roundtrip` against production APKs with non-standard AXML padding to confirm trailer handling | Live device / proprietary corpus |
