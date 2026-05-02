# P2.5 — Rust Extraction of AXML Parser + axiom-l1-rs Integration

> AXML Lean theorems extracted to Rust. Replaces hand-written AXML parser. Translation validator green on 5,000-sample corpus. Performance within 15% of hand-written.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §6](../../../README.md#layer-1)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P2.5 |
| Owner(s) | G1 + G2 |
| Duration | Weeks 6–9 |
| Critical-path | yes |
| Hard prerequisites | P2.3 (AXML Lean) |

## 2. Goal & Scope

The Lean AXML parser extracted to Rust and integrated into `axiom-l1-rs`. Translation validator on Bench-10K. Throughput ≥ 600 APKs/sec/16-core for AXML decoding alone.

### In scope
- Extracted crate `axiom-l1-axml-verified`
- `axiom-l1-rs` switched to verified AXML by default; hand-written fallback feature-flagged
- Translation validator nightly on Bench-10K
- Performance regression gate

### Out of scope
- ARSC extraction (P2.6)
- Bundle-aware AXML lookups (P2.12)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P2.3** | AXML Lean modules + theorems |
| **P1.9** | Extraction pipeline + translation validator |
| **P1.4 / P2.2** | AXIOM-IR manifest dialect |

## 4. Required Tools, Libraries, and Languages

Inherited from P1.9 + P2.3. New: insta snapshot tests for AXML structural outputs.

| Tool | Version | Purpose |
|---|---|---|
| **Lean → Rust extractor** | from P1.9 | Production extractor |
| **Translation validator** | from P1.9 | Diffs Lean ↔ Rust outputs |
| **insta** | 1.40+ | Snapshot fixtures |
| **proptest** | 1.5+ | Property-based round-trip |
| **HACL\* SHA-256** | from P1.10 | For AXML chunk-digest verification (some adversarial cases require it) |

## 5. Third-Party Software, Services, Accounts & API Keys

Same toolchain as P1.9 + P1.10. **No new external dependencies.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ Lean toolchain, extractor, translation validator, AXML Lean modules

### Missing
- Nothing system-level; just add `insta = "1.40"` if not already present.

## 7. Features & Functions Delivered (Comprehensive)

### Extracted Rust API
- `pub fn parse_axml(bytes: &[u8]) -> Result<AxmlDocument, AxmlError>`
- `pub fn axml_to_manifest_ir(doc: &AxmlDocument) -> Result<ManifestIR, LoweringError>`
- `pub struct AxmlDocument { string_pool, resource_map, root_node, namespaces }`
- `pub struct AxmlError { kind: AxmlErrorKind, location: ByteRange }` — never panics
- All public APIs gated by phantom `Verified` marker (re-uses P1.8 type-state pattern)

### Integration into `axiom-l1-rs`
- `axiom-l1-rs` switches to verified AXML by default
- `cfg(feature = "legacy-axml")` retains hand-written fallback for rollback
- Streaming hooks emit `ParseEvent::AxmlStart / AxmlField / AxmlEnd` from the verified parser
- Merkle commit chain extends to AXML chunks (per-chunk BLAKE3)

### Translation validator
- `tools/translation-validator` runs on `Bench-10K` nightly
- Diffs every output: parsed `AxmlDocument` structure, AXML chunk hashes, derived `ManifestIR`
- Discrepancies blocks merge (CI gate)

### Performance instrumentation
- Pyroscope continuous profile captured for every CI run
- Criterion microbenchmarks per AXML structural component (string pool, resource map, tree)

### Documentation
- `docs/verified-axml.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Translation validator agreement on Bench-10K | 100 % | 100 % |
| Extracted AXML parser perf delta vs hand-Rust | within 15 % | within 5 % |
| AXML decode throughput single-core | ≥ 5K APKs/sec | ≥ 12K APKs/sec |
| Reproducibility: per-APK Merkle root bit-identical | 100 % | 100 % |
| Insta snapshots for 100 reference manifests | green | green |
| HACL\* SHA-256 on the verified path | yes | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   ├── axiom-l1-axml-verified/           # NEW — auto-generated
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── axiom-l1-rs/
│       ├── Cargo.toml                    # adds dep
│       └── src/
│           ├── parser/
│           │   └── axml.rs               # switches to verified path
│           └── lib.rs
├── tests/translation-validation/
│   └── axml-bench-10k.rs                 # NEW
└── docs/
    └── verified-axml.md                  # NEW
```

## 10. Standalone Output

```bash
nix develop
make extract-axml                  # Lean → Rust
buck2 test //tests/translation-validation:axml-bench-10k
# "10000/10000 AXML decodings axiom-l1-axml-verified ↔ Lean reference agree"
```

## 11. End-to-End Test

```bash
buck2 test //axiom-l1-rs:integration-axml-bench-10k
# Required:
#   - 100% verdict + structure agreement (HARD)
#   - throughput ≥ 5K APKs/sec/core (HARD)
#   - perf delta ≤ 15% vs hand-written (HARD)
#   - 100% Merkle root reproducibility
```

## 12. Exit Checklist

- [ ] AXML extracted Rust crate compiles
- [ ] `axiom-l1-rs` defaults to verified AXML
- [ ] Translation validator 100 % green on Bench-10K
- [ ] AXML throughput ≥ 5K APKs/sec/core (HARD)
- [ ] Perf delta vs hand-written ≤ 15 % (HARD)
- [ ] HACL\* SHA-256 on verified path
- [ ] Reproducibility 100 %
- [ ] Hand-written fallback flagged for removal in Phase 3
- [ ] `docs/verified-axml.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P2.10** | Verified AXML feeds Schrödinger semantics |
| **P2.11** | Bundle parser uses verified AXML |
| **P2.15** | AXML provenance fingerprint reads structural micro-features from verified parse |
| **P2.18** | E2E pipeline uses verified AXML |
