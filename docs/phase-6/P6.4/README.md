# P6.4 — G3 Stabilization: AXIOM-IR Final-Dialect Freeze + v1.0 Spec Publication

> Promote AXIOM-IR-v0.4 to AXIOM-IR-v1.0. Freeze every dialect (manifest, resource, DEX, symbolic, native, JNI, etc.). Publish the consolidated RFC-style spec. No dialect changes accepted post-freeze without leadership ADR.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.4 |
| Owner(s) | G3 |
| Duration | Weeks 1–12 |
| Critical-path | yes |
| Hard prerequisites | P6.1 |

## 2. Goal & Scope

The IR's v1.0 release: every dialect spec polished, consolidated, frozen. Single RFC document `docs/AXIOM-IR-v1.0.md` published with SHA-256 + Ed25519 sig + Zenodo DOI.

### In scope
- AXIOM-IR-v1.0 spec consolidation
- Per-dialect freeze: manifest, resource, DEX-SSA, symbolic, ELF-native, JNI-boundary, evidence (dynamic-confirmation)
- ABI compatibility check across runs / arches
- Bytewise-identical IR output across runs / arches
- ADR-0044 — IR-v1.0 freeze
- Public spec publication (Zenodo DOI)

### Out of scope
- New dialects (deferred to v1.1)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P6.1** | Stabilization punch-list |
| **AXIOM-IR-v0.4** | Phase-5 baseline |
| **All Phase 1–5 dialect docs** | Consolidated into v1.0 |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **MLIR / TableGen / ODS** | (existing) | Dialect codegen |
| **mdBook** | latest | Spec publication |
| **Zenodo CLI** | latest | DOI |
| **Ed25519 signing key** | (existing) | Spec sig |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Zenodo** | service | **Free** | https://zenodo.org | DOI |

**API keys required:** Zenodo OAuth.

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### AXIOM-IR-v1.0 spec (`docs/AXIOM-IR-v1.0.md`)
- Single consolidated document
- Per-dialect: ops, types, attributes, traits, semantics, soundness theorem reference
- Backwards-compat note from v0.x
- Wire format (Cap'n Proto schema for IR serialization)
- Versioning policy

### Dialect freeze
- Each dialect's `.td` files frozen
- Generated bindings stable
- Cross-arch byte-identical IR output verified

### ADR-0044 — IR-v1.0 freeze
- Dialects covered
- Versioning policy
- Migration path to v1.1

### Spec publication
- mdBook output
- SHA-256 + Ed25519 sig
- Zenodo DOI
- arXiv-mirror

### Test extension
- IR-v1.0 conformance suite (sample IR + expected ops + types)

### Documentation
- Per-dialect quickstart sections within the v1.0 spec

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| AXIOM-IR-v1.0 spec frozen by W12 | yes |
| ≥ 4-week RFC-review window | yes |
| All dialect leads + G1 + G5 + G6 + G7 + G9 + G14 sign off | yes |
| Bytewise-identical IR across runs / arches | yes |
| ADR-0044 merged | yes |
| Zenodo DOI assigned | yes |
| `docs/AXIOM-IR-v1.0.md` published with sig | yes |
| Conformance suite ≥ 50 tests, all green | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── AXIOM-IR-v1.0.md               # NEW: v1.0 RFC
│   └── ADR-0044-ir-v1.0-freeze.md     # NEW
├── ir/
│   ├── dialects/                      # frozen
│   └── tests/conformance/             # NEW
└── (Zenodo + signature artifacts)
```

## 10. Standalone Output

The AXIOM-IR-v1.0 spec is a citable open-data artifact (DOI). Reusable beyond APKAXIOM as a sound, lossless IR for Android-binary analyzers.

## 11. End-to-End Test

```bash
mdbook build docs/
sha256sum docs/AXIOM-IR-v1.0.md
cosign sign-blob --yes docs/AXIOM-IR-v1.0.md > AXIOM-IR-v1.0.sig

buck2 test //ir/tests/conformance:...
```

## 12. Exit Checklist

- [ ] Spec frozen by W12 (HARD)
- [ ] ≥ 4-week RFC review
- [ ] ≥ 7 lead sign-offs
- [ ] Bytewise-identical IR cross-arch
- [ ] ADR-0044 merged
- [ ] Zenodo DOI assigned
- [ ] Spec published + signed
- [ ] Conformance suite ≥ 50 tests green

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.16** | Stable IR for 50K eval |
| **P6.17** | Spec presented to external auditor |
| **P6.18** | IR spec embedded in `docs/v1.0/` |
| **P6.20** | "AXIOM-IR all dialects frozen, documented, versioned" item ✅ for ship gate |
