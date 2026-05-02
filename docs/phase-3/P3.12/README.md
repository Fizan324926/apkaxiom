# P3.12 — DRAT-Style UNSAT Certificate Emission from cvc5

> Every UNSAT outcome ships with a DRAT certificate. Independently checkable in milliseconds via DRAT-trim. The proof artifact downstream `.axc` certs build on.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §11 (L6 — preview)](../../../README.md#layer-6)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.12 |
| Owner(s) | G5 + G7 (preview) |
| Duration | Weeks 11–15 |
| Critical-path | yes |
| Hard prerequisites | P3.7 (CHC encoding), P3.8 (L4 emits UNSAT) |

## 2. Goal & Scope

The full pipeline for capturing, persisting, indexing, and verifying DRAT (or LRAT) UNSAT certificates from cvc5/Spacer. DRAT-trim independently verifies a 1%+ random sample on every CI run. Certs feed into Phase 4's `.axc` envelope.

### In scope
- DRAT/LRAT capture from cvc5 (`--produce-proofs --proof-format=drat`)
- DRAT capture from Z3/Spacer (where supported)
- LRAT linearization (some solvers emit DRAT; we lift to LRAT for stable linear-time checking)
- Persistent archive in `findings/drat-archive/` (fjall LSM)
- DRAT-trim verifier integration
- Sampling policy: 1 % random sample verified on every CI run
- Cert format spec: provenance metadata (which encoder, which Lean theorem authorizes)

### Out of scope
- zk-SNARK envelope wrapping (Phase 4 / G7)
- STARK alternative (Phase 6)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.7** | CHC encoder produces queries that yield UNSAT |
| **P3.8** | L4 emits UNSAT outcomes |
| **P3.6** | SMT bridge captures DRAT |
| **P3.3** | UNSAT-cert carrier types defined |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **DRAT-trim** | latest | Reference verifier |
| **LRAT-checker** | latest | Linearized variant |
| **cvc5** | 1.2+ | Source of DRAT |
| **Z3** | 4.13+ | Source of DRAT (when available) |
| **fjall** | 0.5+ | Persistent archive |
| **rkyv** | 0.7+ | Cert envelope serialization |
| **HACL\* BLAKE3 + Ed25519** | from P1.10/P1.16 | Content addressing + signing |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **DRAT-trim** | UNSAT-cert checker | **Free** OSS | already provisioned | |
| **LRAT-checker** | linearized variant | **Free** OSS | https://github.com/marijnheule/drat-trim | |
| **cvc5 / Z3** | solvers (as cert producers) | **Free** OSS | already provisioned | |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ DRAT-trim, cvc5, Z3, Spacer, fjall, rkyv, BLAKE3, Ed25519

### Missing
- Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### Capture pipeline
- cvc5: invoked with `--produce-proofs --proof-format=drat`
- Z3/Spacer: `--produce-proofs --proof-format=drat-incremental` (when supported)
- Captured DRAT trace persisted to `findings/drat-archive/<digest>.drat`
- Provenance metadata sidecar `.json` containing: encoder ID, Lean theorem reference, query hash, solver version, timeout, wall-time

### Linearization
- DRAT → LRAT conversion via DRAT-trim's `--lrat` mode
- LRAT preferred for downstream consumption (linear-time checking, easier zk-SNARK lift)

### Verification
- 1% random sample verified on every CI run via DRAT-trim
- Failed verification = P0 incident (cert is unsound)
- Sampling policy adjustable per-deployment; 1% baseline

### Persistent archive
- fjall LSM tree keyed by content-digest of the original query
- Cert envelope: `{ query_digest, lean_theorem_id, solver, drat_blob, lrat_blob_optional, signature_ed25519 }`
- BLAKE3-content-addressed, Ed25519-signed
- Retention policy: indefinite for v1.0 ship-gate certs; 30-day for transient CI certs

### Cert format spec
- Documented in `docs/drat-cert-format.md`
- Wire-format stable for Phase 4 `.axc` consumption
- Backwards-compatibility commitment

### Public Rust API
- `pub fn emit_cert(unsat_outcome: &UnsatOutcome) -> DratCert`
- `pub fn verify_cert(cert: &DratCert) -> Result<(), VerifyError>`
- `pub fn archive_cert(cert: &DratCert) -> Digest`

### Documentation
- `docs/drat-cert-format.md`
- `docs/cert-verification-runbook.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| DRAT capture rate on UNSAT outcomes | 100 % | 100 % |
| DRAT-trim sample verification on CI | ≥ 1 % | ≥ 5 % |
| Failed sample verifications (cert unsound) | 0 | 0 |
| LRAT-conversion success rate on captured certs | ≥ 99 % | 100 % |
| Cert envelope size median | ≤ 100 KB | ≤ 30 KB |
| Cert envelope size p99 | ≤ 1 MB | ≤ 200 KB |
| Persistent archive throughput | ≥ 1,000 certs/sec | ≥ 5,000 certs/sec |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-drat-cert/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── capture.rs
│           ├── linearize.rs              # DRAT → LRAT
│           ├── verify.rs                 # via DRAT-trim
│           ├── archive.rs                # fjall LSM
│           └── envelope.rs               # the cert format
├── findings/drat-archive/                 # extended
├── tests/drat-cert/
│   └── sample-verify-runbook.rs
└── docs/
    ├── drat-cert-format.md                # NEW
    └── cert-verification-runbook.md       # NEW
```

## 10. Standalone Output

```bash
buck2 build //crates/axiom-drat-cert --release
buck2 test //tests/drat-cert:full-eval
# "1 of 100 sampled certs failed verification — investigation required"  (must be 0!)
# Or "100/100 sampled certs verified"
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-drat-cert:full
# - 100% capture on UNSAT (HARD)
# - 0 failed sample verifications (HARD)
# - LRAT conversion ≥ 99% (HARD)
# - Archive throughput ≥ 1K certs/sec (HARD)
```

## 12. Exit Checklist

- [ ] DRAT capture pipeline operational
- [ ] LRAT linearization ≥ 99 % (HARD)
- [ ] DRAT-trim sample verification on every CI run (HARD)
- [ ] 0 failed sample verifications (HARD)
- [ ] Cert envelope spec frozen and documented
- [ ] Persistent archive in fjall, content-addressed
- [ ] Ed25519-signed envelopes
- [ ] `docs/drat-cert-format.md` published
- [ ] `docs/cert-verification-runbook.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **Phase 4 / G7** | DRAT/LRAT certs are the input to `.axc` envelope |
| **P3.18** | Cert capture rate measured in E2E |
| **Phase 6 audit** | DRAT certs as evidence of soundness |
