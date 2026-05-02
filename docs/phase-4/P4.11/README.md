# P4.11 — `axiom-verify` Reference Verifier Core (Rust)

> The user-facing surface of APKAXIOM. p99 ≤ 100 ms over 10K certs. Cold start ≤ 500 ms. The binary every bug-bounty triager runs.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §11](../../../README.md#layer-6)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.11 |
| Owner(s) | G7 + G14 |
| Duration | Weeks 6–14 |
| Critical-path | yes |
| Hard prerequisites | P4.2 (.axc spec), P4.3 (zk pool — verify side) |

## 2. Goal & Scope

A production-grade Rust reference verifier. Reads a `.axc` file + (optionally) the original APK; produces a single ✅ / ❌ verdict in milliseconds. Verifies every claim type — Lean parser objects, DRAT certs, equivalence certs, Halo2/Stwo proofs, SLSA attestations.

### In scope
- `crates/axiom-verify-core` — the verifier library
- `tools/axiom-verify` — CLI binary
- Per-claim-type verification handler
- Cert chain validation (BLAKE3 content + Ed25519 signing)
- Performance: ≤ 100 ms p99 over 10K certs (HARD)
- Cold start ≤ 500 ms (HARD)
- Process-isolated zk-verify worker pool

### Out of scope
- Wasm + ARM64 builds (P4.12)
- SDKs (P4.13–P4.15)
- Bug-bounty pilot (P4.17)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P4.2** | `.axc` v1 spec |
| **P4.3** | zk pool verify path |
| **P3.12** | DRAT-trim integration |
| **P3.16** | equiv-verify integration |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Rust** | 1.95 | Core implementation |
| **Halo2 + Stwo verify** | from P4.3/P4.10 | zk verify path |
| **DRAT-trim** | from P3.3 | DRAT verify |
| **HACL\* BLAKE3 + Ed25519** | from P1.10 | Crypto |
| **Cap'n Proto runtime** | from P1.4 | `.axc` parsing |
| **Glommio** | from P1.7 | Async dispatch |
| **HDR Histogram** | from P1.18 | Latency capture |

## 5. Third-Party Software, Services, Accounts & API Keys

**No new third-party.** Reuses verified-crypto stack + zk pool.

## 6. System Inventory — Have vs Need

Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### Public Rust API
- `pub fn verify(cert: &AxcCertificate, apk_bytes: Option<&[u8]>) -> VerifyResult`
- `pub enum VerifyResult { Ok(VerifiedClaims), Failed(VerifyError) }`
- `pub struct VerifiedClaims { claims: Vec<VerifiedClaim>, audit_log: AuditLog }`
- `pub fn verify_streaming<R: Read>(reader: R) -> VerifyResult` — for streaming verification

### Per-claim-type handlers
- ParserConsistency: re-verify Lean proof object (lookup in PK archive)
- IntentUnreachability: DRAT-trim re-verify
- IntentReachability: rebuild abstract state from witness; replay; assert outcome matches
- BehaviorEquivalence: rebuild bisim relation; check SMT discharges
- PrivacyInvariant: zk pool verify (Halo2 or Stwo per cert)
- RepackagingDetection: re-run AXML provenance + shadow-stack from APK; assert findings
- AOSPDifferentialFinding: replay against pinned AOSP harness
- NetworkAllowlistCompliant: zk pool verify
- MlModelIntegrity: zk pool verify
- SLSA L4 Provenance: chain verification (P4.16)

### Cert chain validation
- BLAKE3 content-digest matches
- Ed25519 signature verifies against signing key
- Provenance metadata internally consistent
- Each claim's proof artifact is independently checkable

### Process isolation
- zk-verify worker process pool (cgroups + seccomp)
- Per-verify timeout (default 200 ms)
- Crash containment

### CLI binary `axiom-verify`
- `axiom-verify report.axc` → ✅ or ❌
- `axiom-verify report.axc --apk app.apk` → full re-verification
- `axiom-verify --batch <pattern>` → many certs in one process (warm cache)
- `axiom-verify --json` → machine-readable output
- `axiom-verify --explain` → human-readable claim listing

### Performance
- HDR Histogram on every verify
- Pyroscope continuous profiling
- Prometheus metrics: verifies/sec, latency distribution, error rates

### Cold-start optimization
- PK archive memory-mapped on startup (no synchronous load)
- HACL\* + Halo2 lazily initialized
- Target ≤ 500 ms first-cert latency (HARD)

### Documentation
- `docs/axiom-verify-cli.md` — user-facing
- `docs/axiom-verify-internals.md` — developer-facing

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| `axiom-verify` p50 | ≤ 30 ms | ≤ 15 ms |
| `axiom-verify` p95 | ≤ 80 ms | ≤ 40 ms |
| `axiom-verify` p99 over 10K cert sample | ≤ 100 ms | ≤ 50 ms |
| `axiom-verify` p99.9 | ≤ 500 ms | ≤ 200 ms |
| Cold-start latency | ≤ 500 ms | ≤ 150 ms |
| All claim types verifiable | yes | yes |
| Process-isolation crash containment | 100 % | 100 % |
| Verifier service throughput, single 16-core node | ≥ 3,000 verifications/sec | ≥ 10,000/sec |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-verify-core/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── handlers/                  # per-claim-type
│           │   ├── parser_consistency.rs
│           │   ├── intent_unreachability.rs
│           │   ├── intent_reachability.rs
│           │   ├── behavior_equivalence.rs
│           │   ├── privacy_invariant.rs
│           │   ├── repackaging.rs
│           │   ├── aosp_differential.rs
│           │   ├── network_allowlist.rs
│           │   ├── ml_model_integrity.rs
│           │   └── slsa.rs
│           ├── chain.rs                    # BLAKE3 + Ed25519 chain check
│           ├── pool.rs                     # process-isolated worker pool
│           └── error.rs
├── tools/
│   └── axiom-verify/                      # CLI
│       ├── Cargo.toml
│       └── src/main.rs
└── docs/
    ├── axiom-verify-cli.md                 # NEW
    └── axiom-verify-internals.md           # NEW
```

## 10. Standalone Output

```bash
buck2 build //tools/axiom-verify --release
./axiom-verify report.axc
# ✓ Verified — 12 claims, all valid, 47ms
buck2 run //bench:axiom-verify-throughput
# "Throughput: 4200 verifications/sec on 16-core; p99: 78ms"
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-verify-core:full
# - p99 ≤ 100 ms (HARD)
# - p99.9 ≤ 500 ms (HARD)
# - Cold start ≤ 500 ms (HARD)
# - Throughput ≥ 3K/sec on 16-core (HARD)
# - All claim types verifiable (HARD)
```

## 12. Exit Checklist

- [ ] All claim-type handlers operational
- [ ] Cert-chain validation enforced (HARD)
- [ ] p99 ≤ 100 ms over 10K cert sample (HARD)
- [ ] Cold start ≤ 500 ms (HARD)
- [ ] Throughput ≥ 3K/sec single 16-core node (HARD)
- [ ] Process isolation 100 % crash containment (HARD)
- [ ] CLI `axiom-verify` shipped with `--json --explain --batch` modes
- [ ] Documentation published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P4.12** | Wasm + ARM64 mobile builds |
| **P4.13/P4.14/P4.15** | SDK wraps the verifier |
| **P4.17** | Bug-bounty pilot uses CLI directly |
| **P4.18** | E2E measures verifier KPIs |
| **External users** | First production proof-carrying APK verifier |
