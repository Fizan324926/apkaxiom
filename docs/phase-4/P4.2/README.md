# P4.2 — `.axc` Certificate Format RFC v1

> The wire-format spec that defines APKAXIOM's output. Cap'n Proto schema, Ed25519-signed, content-addressed. Frozen for v1.0+ backwards compatibility.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §11 (Layer 6)](../../../README.md#layer-6)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.2 |
| Owner(s) | G7 |
| Duration | Weeks 1–4 |
| Critical-path | **yes** |
| Hard prerequisites | P4.1 |

## 2. Goal & Scope

`.axc` (Apkaxiom Certificate) format v1 — the wire-format spec. RFC-style, ≥ 60 pages, frozen for the rest of Phase 4 and Phase 5+ backwards-compatibility commitment. Carries all proof artifacts: parser-consistency (Lean), reachability witnesses, UNSAT certs (DRAT), equivalence certs (bisim), privacy-invariant proofs (zk-SNARK), provenance metadata. Cap'n Proto schema for wire format; reference Rust types compile.

### In scope
- `docs/AXC-v1-RFC.md` (≥ 60 pages)
- Cap'n Proto schema `schema/axc_v1.capnp`
- Reference Rust types `crates/axiom-axc-format`
- Versioning policy + extension hooks
- ADR-0022 — `.axc` v1 freeze
- Pre-freeze external review

### Out of scope
- Verifier implementation (P4.11)
- Per-claim circuit implementations (P4.5–P4.9)
- SDK consumption (P4.13–P4.15)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P4.1** | G7 onboarded |
| **P3.12** | DRAT cert format (we wrap) |
| **P3.16** | Equiv cert format (we wrap) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Cap'n Proto** | 1.x | Wire format |
| **Rust** | 1.95 | Reference types |
| **Lean 4** | pinned | Optional formalization of canonicalization |
| **HACL\* BLAKE3 + Ed25519** | from P1.10/P1.16 | Content-addressing + signing |
| **DRAT-trim, equiv-verify** | from Phase 3 | For embedded sub-cert verification |
| **PlantUML / Mermaid** | latest | Format-flow diagrams |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Cap'n Proto** | wire-format library | **Free** OSS (MIT) | https://capnproto.org | |
| **External cryptographic-review service** | service | **Paid** $10–50K (Trail of Bits / NCC small engagement) | varies | For pre-freeze review of cert-signing chain |
| **arXiv** | preprint | **Free** | already endorsed | Eventual `.axc` paper |
| **Zenodo** | DOI | **Free** | https://zenodo.org | RFC artifact DOI |

## 6. System Inventory — Have vs Need

### Already present
- ✅ Cap'n Proto compiler (P1.4)
- ✅ HACL\* + Rust + Lean

### Missing
- Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### `.axc` v1 RFC (`docs/AXC-v1-RFC.md`)
- ≥ 60 pages
- Wire format (Cap'n Proto)
- Cryptographic chain: BLAKE3 content addressing → Ed25519 signing
- Versioning policy: `axc-v1.0`, `axc-v1.1`, … (semantic-version backwards-compatibility commitments)
- Profile system: `core` profile (mandatory fields), `extended` (optional richer findings)

### Cert envelope content (excerpt)
```capnp
struct AxcCertificate {
  version @0 :Text;            # "v1.0"
  inputDigest @1 :Hash;        # BLAKE3 of original APK bytes
  androidVersions @2 :List(AndroidVersion);
  parserExtraction @3 :Text;   # "lean4:0.4.0/aosp:android-15.0.0_r12"
  analysisTimestamp @4 :Time;
  signingKey @5 :Ed25519PublicKey;

  claims @6 :List(Claim);
  signature @7 :Ed25519Signature;
}

struct Claim {
  kind @0 :ClaimKind;          # ParserConsistency | IntentUnreachability | BehaviorEquivalence | PrivacyInvariant | RepackagingDetection | ...
  proof @1 :ProofBlob;          # DRAT | LRAT | Equiv | Halo2 | Stwo | LeanObject
  statement @2 :Text;          # human-readable predicate
  metadata @3 :Metadata;        # provenance: encoder version, Lean theorem ID, etc.
}
```

### Claim kinds (≥ 10)
1. **ParserConsistency** — Lean proof object proving parser was sound on this APK
2. **IntentUnreachability** — DRAT cert proving no resolution
3. **IntentReachability** — replayable witness proving resolution path
4. **BehaviorEquivalence** — bisim cert proving equivalence to known APK
5. **PrivacyInvariant** — zk-SNARK proof of e.g. "never reads contacts"
6. **RepackagingDetection** — AXML provenance + shadow-stack findings
7. **AOSPDifferentialFinding** — cross-version disagreement with reproducer
8. **NetworkAllowlistCompliant** — zk-SNARK proof
9. **MlModelIntegrity** — zk-SNARK proof
10. **SLSA L4 Provenance** — verified attestation chain

### Provenance metadata (mandatory per claim)
- Encoder ID + version
- Lean theorem reference (where applicable)
- Solver version + flags
- Proving key digest
- Wall-clock duration
- Memory peak

### Signing chain
- Per-claim BLAKE3 content digest
- Whole-cert BLAKE3 root
- Ed25519 signature on root
- Optional multi-signature for high-stakes deployments
- Key rotation policy documented

### Versioning policy
- v1.x within v1 line: backwards-compat additions only
- v2: breaking changes; new format
- Migration tooling commitment

### Reference Rust types
- `crates/axiom-axc-format` — compiles
- 100 hand-written cert samples round-trip via Cap'n Proto + serde

### Pre-freeze external review
- 2-week NCC Group / Trail of Bits / similar engagement
- Findings + responses documented
- Cryptographic chain validated by external reviewer

### Lean grounding (optional)
- `theorems/Apkaxiom/AxcCanonical.lean` — canonicalization theorems

### Decision logs
- ADR-0022 — `.axc` v1 freeze
- ADR-0023 — Signing-key rotation policy
- ADR-0024 — Profile system (core vs extended)

### Diagrams
- Cert lifecycle (mermaid)
- Cert verification flow (mermaid)
- Signing-chain dependency graph (graphviz)

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| `.axc` RFC ≥ 60 pages | yes | ≥ 100 pages |
| Cap'n Proto schema validates | yes | yes |
| Reference Rust types compile + round-trip 100 samples | yes | yes |
| External cryptographic review completed | yes | yes |
| Reviewer sign-offs (G1, G5, G6, G7, G14 + 2 external) | yes | yes |
| RFC frozen ≥ 4 weeks before P4.18 | yes | yes |
| ADRs 0022 + 0023 + 0024 merged | 3 | 3 |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── AXC-v1-RFC.md                    # NEW — FROZEN
│   ├── AXC-v1-rationale.md              # NEW
│   ├── AXC-v1-external-review.md        # NEW
│   ├── ADR-0022-axc-v1-freeze.md
│   ├── ADR-0023-signing-key-rotation.md
│   └── ADR-0024-axc-profile-system.md
├── schema/
│   └── axc_v1.capnp                     # NEW — wire format
├── crates/
│   └── axiom-axc-format/                # NEW
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── envelope.rs
│           ├── claim.rs
│           ├── proof_blob.rs
│           └── signing.rs
├── theorems/Apkaxiom/AxcCanonical.lean  # NEW (optional)
└── diagrams/
    ├── axc-lifecycle.mmd
    ├── axc-verification.mmd
    └── signing-chain.dot
```

## 10. Standalone Output

A frozen wire-format RFC + reference Rust types + Cap'n Proto schema + external cryptographic-review report.

## 11. End-to-End Test

```bash
buck2 build //crates/axiom-axc-format
buck2 test //crates/axiom-axc-format:roundtrip-100
test "$(grep -c '^✅ approved by' docs/sign-offs/P4.2.md)" -ge 7  # 5 internal + 2 external
test -f docs/AXC-v1-RFC.md
grep -q "FROZEN ON" docs/AXC-v1-RFC.md
```

## 12. Exit Checklist

- [ ] `.axc` v1 RFC ≥ 60 pages (HARD)
- [ ] Cap'n Proto schema frozen (HARD)
- [ ] Reference Rust types compile + 100-sample round-trip (HARD)
- [ ] External cryptographic review completed and findings addressed (HARD)
- [ ] Sign-offs from G1, G5, G6, G7, G14 + 2 external (HARD)
- [ ] Spec frozen ≥ 4 weeks before P4.18 (HARD)
- [ ] ADRs 0022, 0023, 0024 merged
- [ ] Optional Lean canonicalization theorems land
- [ ] Diagrams rendered

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P4.11** | `.axc` format spec → axiom-verify implementation |
| **P4.13/P4.14/P4.15** | SDK reads + writes `.axc` |
| **P4.4–P4.10** | Privacy-invariant + STARK proofs all wrap into this format |
| **P4.16** | SLSA proofs ship as `.axc` claims |
| **P4.17** | Bug-bounty pilot consumes `.axc` |
| **External community** | First citable mobile-app-security cert format |
