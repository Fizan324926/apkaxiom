# P4.16 — SLSA L4 Attestation + Reproducible-Build Verification

> Verify the APK matches its claimed build provenance. SLSA L4 chain. Source ↔ APK reproducibility verified end-to-end on F-Droid.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §13.7 (Supply chain)](../../../README.md#beyond-the-12) · [../../TECH_STACK.md §10](../../TECH_STACK.md#build)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.16 |
| Owner(s) | G12 |
| Duration | Weeks 6–18 |
| Critical-path | yes |
| Hard prerequisites | P4.1 (G12 onboarded) |

## 2. Goal & Scope

A complete SLSA L4 attestation + reproducible-build verifier. Given an APK + claimed build provenance, prove or disprove the APK was actually built from the claimed source. F-Droid is the authoritative reproducibility test corpus. Findings ship as `.axc` claims.

### In scope
- `crates/axiom-supply-chain` — SLSA + reproducibility crate
- SLSA L4 attestation parser
- Reproducible-build verifier (deterministic AXML re-encoder, DEX normalizer)
- `.axc` claim emitter for SLSA findings
- F-Droid reference corpus (≥ 1000 reproducible builds)
- Optional Sigstore (cosign) verification

### Out of scope
- Closed-source app reproduction (impossible by definition)
- Third-party signing-block schemes (Phase 2 covered)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P4.2** | `.axc` claim format |
| **P2.5/P2.6** | Verified AXML + ARSC for canonical re-encoding |
| **P2.8** | DEX normalizer |
| **P1.16** | Verified signing-block extraction |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **SLSA verifier** | latest from slsa-framework/slsa-verifier | Reference impl |
| **Sigstore (cosign)** | from P1.1 | Signing |
| **in-toto** Rust crate | latest | Attestation parser |
| **F-Droid reproducibility metadata** | live data feed | Authoritative reference |
| **diffoscope** | latest | Binary diff for non-reproducible cases |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **SLSA framework** | spec + tools | **Free** OSS | https://slsa.dev | Linux Foundation |
| **Sigstore (cosign)** | signing | **Free** OSS | already provisioned | |
| **F-Droid** | reproducible-build oracle | **Free** | https://f-droid.org/api/v1/index.json | Their build metadata is public |
| **diffoscope** | tool | **Free** OSS | https://diffoscope.org | Reproducible-builds.org |
| **in-toto** | crate / Go ref | **Free** OSS | https://in-toto.io | |

**No new API keys.** F-Droid provides metadata as JSON.

## 6. System Inventory — Have vs Need

### Already present
- ✅ Sigstore (P1.1)
- ✅ Rust + verified parsers

### Missing — must install
- ❌ **SLSA verifier** — `go install github.com/slsa-framework/slsa-verifier/v2/cli/slsa-verifier@latest`
- ❌ **diffoscope** — `apt-get install -y diffoscope`
- ❌ **in-toto** Rust crate — Cargo dep

```bash
go install github.com/slsa-framework/slsa-verifier/v2/cli/slsa-verifier@latest
sudo apt-get install -y diffoscope
```

## 7. Features & Functions Delivered (Comprehensive)

### Public Rust API
- `pub fn verify_slsa_l4(apk: &[u8], attestation: &SlsaAttestation) -> Result<SlsaResult, SlsaError>`
- `pub fn verify_reproducible_build(apk: &[u8], source: &SourceTree, build_recipe: &BuildRecipe) -> Result<ReproducibilityResult, ReproError>`
- `pub fn verify_full_chain(apk: &[u8], slsa: &SlsaAttestation, source: &SourceTree) -> Result<FullChainResult, ChainError>`
- `pub enum SlsaResult { Verified(Provenance), AttestationInvalid, BuilderUnknown, ... }`
- `pub enum ReproducibilityResult { Reproducible, NonReproducible(Vec<Difference>), MissingArtifact }`

### SLSA L4 attestation verification
- Parse in-toto attestation
- Verify Sigstore signature on the attestation
- Match builder identity (e.g., F-Droid GitLab CI)
- Match build-recipe digest
- Match source-tree commit SHA
- Match output-artifact digest

### Reproducible-build verifier
- Deterministic AXML re-encoder (using verified ARSC + AXML parsers from Phase 2)
- DEX normalizer (sort string-pool, normalize annotations, strip non-determinism)
- ZIP layout normalization
- Per-difference report when non-reproducible (calls `diffoscope`)
- Configurable strictness: byte-exact vs semantic-equivalence

### F-Droid integration
- Auto-fetch F-Droid index for reproducibility status
- Cross-check: F-Droid's "reproducible" flag vs our verdict
- Discrepancy = bug in our verifier (or in F-Droid's)

### `.axc` claim emission
- New claim type: `SLSA L4 Provenance`
- Carries: attestation digest, builder identity, source SHA, reproducibility verdict, per-difference list

### Documentation
- `docs/slsa-l4-verification.md`
- `docs/reproducible-builds-runbook.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| SLSA L4 verification operational | yes | yes |
| Reproducible-build round-trip on F-Droid sample | ≥ 1 sample | ≥ 100 samples |
| F-Droid agreement on "reproducible" flag | ≥ 95 % | ≥ 99 % |
| Verification per-APK p99 | ≤ 30 s | ≤ 8 s |
| Provenance metadata completeness in `.axc` | 100 % | 100 % |
| Sigstore signature verification | 100 % when present | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-supply-chain/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── slsa.rs
│           ├── reproducible.rs
│           ├── deterministic_axml.rs
│           ├── dex_normalize.rs
│           ├── zip_normalize.rs
│           ├── f_droid_oracle.rs
│           └── claim_emitter.rs
├── corpus/
│   └── f-droid-reproducible-100/        # 100+ F-Droid samples for regression
└── docs/
    ├── slsa-l4-verification.md
    └── reproducible-builds-runbook.md
```

## 10. Standalone Output

```bash
buck2 build //crates/axiom-supply-chain --release
buck2 run //tools/cli -- verify-slsa --apk app.apk --attestation app.intoto.jsonl
# ✓ SLSA L4 verified (builder: F-Droid GitLab CI; source: a1b2c3d...)
buck2 run //tools/cli -- verify-reproducibility --apk app.apk --source ./source-tree --recipe ./build-recipe.yaml
# ✓ Reproducible (or per-difference report)
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-supply-chain:f-droid-100
# - SLSA verification operational (HARD)
# - F-Droid agreement ≥ 95% (HARD)
# - Per-APK p99 ≤ 30 s (HARD)
```

## 12. Exit Checklist

- [ ] `axiom-supply-chain` crate compiles
- [ ] SLSA L4 attestation parser + verifier
- [ ] Reproducible-build verifier with deterministic re-encoders
- [ ] F-Droid agreement ≥ 95 % on 100-sample subset (HARD)
- [ ] Per-APK p99 ≤ 30 s (HARD)
- [ ] `.axc` claim emitter for SLSA / reproducibility
- [ ] Sigstore signature path 100 % (HARD)
- [ ] Documentation published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P4.18** | E2E measures SLSA + reproducibility |
| **P4.17** | Bug-bounty + supply-chain pilot |
| **External app stores** | First production SLSA L4 verifier for Android APKs |
