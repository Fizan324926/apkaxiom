# P6.13 — G12 Stabilization: SLSA Edge Cases + Reproducible-Build Coverage Expansion

> Drive SLSA L4 verification from F-Droid sample to broader corpus (Play-Store-style, vendor-signed APKs). Reproducible-build coverage expanded. Documentation for downstream consumers.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.13 |
| Owner(s) | G12 |
| Duration | Weeks 1–14 |
| Critical-path | yes |
| Hard prerequisites | P6.1 |

## 2. Goal & Scope

SLSA L4 verifier hardened beyond F-Droid edge case to handle Play-Store-style + vendor-signed APKs + complex provenance chains. Reproducible-build round-trip on broader corpus.

### In scope
- SLSA L4 verifier edge cases: Play-Store-style provenance (when available), vendor-signed APKs (Samsung / Xiaomi / etc.), multi-step provenance chains
- Reproducible-build coverage expanded to ≥ 500 APKs (was F-Droid sample only)
- in-toto attestation chain validation
- Sigstore + Rekor integration confirmed
- Cosign artifact signing for v1.0 release

### Out of scope
- New SLSA levels
- Non-Android provenance

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P6.1** | Stabilization punch-list |
| **All Phase 4 G12 deliverables** | Continued |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **SLSA verifier** | latest | Reference |
| **Sigstore / cosign / fulcio / rekor** | latest | Crypto path |
| **in-toto-attestations** | latest | Attestation chain |
| **F-Droid build server** | (existing) | Reference reproducibility |
| **Rust** | 1.84+ | Implementation |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Sigstore** | service | **Free** | https://www.sigstore.dev | |
| **Rekor (transparency log)** | service | **Free** | https://docs.sigstore.dev/logging/overview | |
| **F-Droid build server** | service | **Free** | https://f-droid.org | |
| **Reproducible Builds** | community | **Free** | https://reproducible-builds.org | Methodology reference |

**API keys required:** Sigstore-fulcio identity tokens (already provisioned).

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### SLSA L4 verifier edge-case hardening
- Play-Store-style attestations: when available
- Vendor-signed APK chains: Samsung / Xiaomi / Huawei / Oppo / Vivo
- Multi-step provenance: source → builder → distributor

### Reproducible-build coverage expansion
- Corpus expanded to ≥ 500 APKs across F-Droid + open-source App-Store curated set
- Per-APK round-trip: source → build → APK → SLSA cert → re-build → byte-identity check

### in-toto + Sigstore + Rekor
- in-toto attestation chain validation tested
- Sigstore identity-token verification + Rekor transparency-log lookup
- Cert flow: cosign sign with fulcio identity → Rekor entry → AXC cert links Rekor entry

### Cosign for v1.0 release
- v1.0 binaries (axiom-verify + SDKs) signed via cosign
- Release tag signed via cosign
- Public verifying key published

### Documentation
- `docs/g12-stabilization.md`
- `docs/slsa-verification.md` — usage + integration guide

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| SLSA L4 verifier per APK | ≤ 2 s |
| Reproducible-build verifier per APK | ≤ 30 s |
| Reproducible-build coverage corpus | ≥ 500 APKs |
| Round-trip byte-identity rate | ≥ 95 % |
| Sigstore + Rekor integration tested | yes |
| Cosign signing for v1.0 release ready | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-supply-chain/           # extended
├── corpora/
│   └── reproducible-builds-500/      # NEW
└── docs/
    ├── g12-stabilization.md          # NEW
    └── slsa-verification.md          # NEW
```

## 10. Standalone Output

SLSA L4 verifier + reproducible-build verifier reusable beyond APKAXIOM.

## 11. End-to-End Test

```bash
buck2 run //tools:axiom-supply-chain-cli -- --corpus reproducible-builds-500 --report round-trip
# Expect: ≥ 95 % round-trip byte-identity

cosign sign-blob --yes <axiom-verify-binary> > axiom-verify.sig
cosign verify-blob --signature axiom-verify.sig <axiom-verify-binary>
```

## 12. Exit Checklist

- [ ] SLSA L4 verifier ≤ 2 s per APK (HARD)
- [ ] Reproducible-build verifier ≤ 30 s per APK (HARD)
- [ ] Coverage corpus ≥ 500 (HARD)
- [ ] Round-trip byte-identity ≥ 95 %
- [ ] Sigstore + Rekor integration tested
- [ ] Cosign signing flow ready
- [ ] Documentation `docs/g12-stabilization.md` + `docs/slsa-verification.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.16** | SLSA verifier in 50K eval |
| **P6.17** | Supply-chain path explained to auditor |
| **P6.19** | Production deploy: SLSA verifier surfaced via API |
| **P6.20** | "SLSA L4 verification works end-to-end" item ✅ for ship gate |
