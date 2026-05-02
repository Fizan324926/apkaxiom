# P3.16 — Bisim Witness Emission + Equivalence Certificate Format

> The witness output of the bisim engine — formalized, signed, archived, replayable. Equivalence certificates carry both the abstract relation and the SMT discharges proving each step.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §10.2](../../../README.md#layer-5)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.16 |
| Owner(s) | G6 + G7 (preview) |
| Duration | Weeks 14–17 |
| Critical-path | yes |
| Hard prerequisites | P3.15 (bisim engine), P3.12 (DRAT cert format pattern) |

## 2. Goal & Scope

The equivalence-certificate format. Wraps a bisim witness (relation + per-transition SMT discharges) into a content-addressed, Ed25519-signed envelope. Independently checkable by an external verifier with sub-second latency.

### In scope
- `crates/axiom-l5-equiv-cert` — cert format + emitter + verifier
- Wire-format spec (`.equiv` file)
- Reference verifier binary (`equiv-verify`)
- Persistent archive in fjall LSM
- Per-cert provenance metadata (which BehaviorSets, which BSH-256s, which Lean theorems)
- Integration with P3.15 witness emission

### Out of scope
- zk-SNARK envelope (Phase 4)
- Layer 5 unified surface (P3.17)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.15** | Bisim witnesses |
| **P3.12** | DRAT cert pattern (we mirror its envelope structure) |
| **P3.14** | BSH-256 commits |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **HACL\* BLAKE3 + Ed25519** | from P1.10/P1.16 | Content addressing + signing |
| **rkyv** | latest | Zero-copy archived envelope |
| **Cap'n Proto** | from P1.4 | Wire-format definition |
| **fjall** | latest | Persistent archive |
| **DRAT-trim** | from P3.3 | Sub-cert verification |

## 5. Third-Party Software, Services, Accounts & API Keys

**No new third-party services.** Reuses verified-crypto stack.

## 6. System Inventory — Have vs Need

Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### Equivalence cert format (`docs/equiv-cert-format.md`)
- Wire-format spec — Cap'n Proto schema
- Frozen, versioned, backwards-compatible commitment
- Versioned: `equiv-cert-v1`

### Cert envelope content
```
EquivalenceCert {
  version: "v1",
  apk_a_id: ApkId, apk_a_bsh: Bsh256, apk_a_merkle_root: Hash,
  apk_b_id: ApkId, apk_b_bsh: Bsh256, apk_b_merkle_root: Hash,
  k_step_bound: u32,
  abstraction_relation: Vec<(AbsState, AbsState)>,
  matched_transitions: Vec<MatchedTransition>,
  smt_discharges: Vec<DratCert>,        // per-transition discharges
  provenance: { bisim_engine_version, lean_theorem_id, abstract_domain_set },
  android_versions_compatible: Vec<AndroidVersion>,
  blake3_content_digest: Hash,
  ed25519_signature: Sig,
}
```

### Public Rust API
- `pub fn emit(witness: BisimWitness, signing_key: &Ed25519Key) -> EquivalenceCert`
- `pub fn verify(cert: &EquivalenceCert, public_key: &Ed25519PublicKey) -> Result<(), VerifyError>`
- `pub fn archive(cert: &EquivalenceCert) -> Digest`

### Reference verifier (`equiv-verify`)
- Standalone binary
- Takes cert + APK-A + APK-B
- Re-extracts BSH-256 + ICC graph; verifies relation; checks each DRAT discharge
- Returns ✅ / ❌ with reason

### Sub-cert verification
- Each DRAT discharge in the cert independently re-verified via DRAT-trim
- 1% random sample on every CI run

### Persistent archive
- fjall LSM keyed by `(apk_a_id, apk_b_id)` pair
- Indexed for "have we seen this equivalence before?" queries

### Documentation
- `docs/equiv-cert-format.md` (frozen)
- `docs/equiv-cert-verification-runbook.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Cert emit overhead vs raw bisim | ≤ 30 % | ≤ 10 % |
| Cert size median | ≤ 200 KB | ≤ 80 KB |
| Cert size p99 | ≤ 2 MB | ≤ 500 KB |
| `equiv-verify` p99 latency on small certs | ≤ 200 ms | ≤ 50 ms |
| Sub-cert (DRAT) verification 1 % sample on CI | yes | yes |
| 0 failed sub-cert verifications | yes | yes |
| Cap'n Proto schema frozen | yes | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-l5-equiv-cert/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── envelope.rs
│           ├── emit.rs
│           ├── verify.rs
│           └── archive.rs
├── tools/
│   └── equiv-verify/                      # standalone verifier binary
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/main.rs
├── schema/
│   └── equiv_cert_v1.capnp                # NEW
├── findings/equiv-cert-archive/           # NEW
└── docs/
    ├── equiv-cert-format.md               # NEW
    └── equiv-cert-verification-runbook.md # NEW
```

## 10. Standalone Output

```bash
buck2 build //crates/axiom-l5-equiv-cert //tools/equiv-verify --release
# Standalone verifier
./equiv-verify --cert sample.equiv --apk-a a.apk --apk-b b.apk
# Returns ✅ or ❌
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-l5-equiv-cert:full
# - Cert emit overhead ≤ 30% (HARD)
# - Cert size p99 ≤ 2 MB (HARD)
# - Verifier p99 ≤ 200 ms (HARD)
# - 0 failed sub-cert verifications (HARD)
# - Cap'n Proto schema validates (HARD)
```

## 12. Exit Checklist

- [ ] Equiv cert format frozen (Cap'n Proto schema versioned)
- [ ] Cert emit + verify operational
- [ ] Reference verifier binary `equiv-verify` ships
- [ ] Cert size median ≤ 200 KB (HARD)
- [ ] `equiv-verify` p99 ≤ 200 ms (HARD)
- [ ] Sub-cert (DRAT) verification on 1 % CI sample (HARD)
- [ ] 0 failed verifications (HARD)
- [ ] Persistent archive in fjall LSM
- [ ] Documentation published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P3.17** | L5 unified surface emits equiv certs |
| **P3.18** | E2E measures cert KPIs |
| **Phase 4 / G7** | Equiv certs ship in `.axc` envelope |
| **External verifiers / bug-bounty** | `equiv-verify` is publicly runnable |
