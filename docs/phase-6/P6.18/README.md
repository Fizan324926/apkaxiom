# P6.18 — Documentation Completeness

> Spec docs for `.axc`, AXIOM-IR (all dialects), every L0–L6 layer's correctness theorem, every group's design rationale, migration guide for downstream consumers. All published under `docs/v1.0/`.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.18 |
| Owner(s) | All groups + tech-writing lead |
| Duration | Weeks 12–22 |
| Critical-path | yes |
| Hard prerequisites | P6.2 .. P6.15 |

## 2. Goal & Scope

A complete, citable, reproducible v1.0 documentation set under `docs/v1.0/`. Every spec, theorem index, design rationale, and consumer-facing guide is published with SHA-256 + Ed25519 sig + Zenodo DOI.

### In scope
- `.axc` format spec v1.0 (RFC-style)
- AXIOM-IR-v1.0 spec (all dialects, consolidated from P6.4)
- L0–L6 correctness-theorem index (linked to Lean theorems)
- Per-group design rationale (one document per G1..G14)
- Consumer migration guide (apk-info v0.x → v2.0)
- Production-verifier API reference
- Per-SDK quickstart
- Threat model document (consolidated from P6.17 onboarding pack)
- Reproducibility evidence index
- v1.0 release notes (drafted; finalized in P6.20)

### Out of scope
- New tutorials beyond quickstart (deferred to community contribs)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P6.2 .. P6.15** | All stabilization deliverables |
| **P6.16** | Eval-50K paper |
| **P6.17** | Audit onboarding pack |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **mdBook** | latest | Spec publishing |
| **rustdoc / pydoc / godoc / typedoc** | latest | API references |
| **Zenodo CLI** | latest | DOI |
| **LaTeX** | (existing) | Cited papers |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **mdBook** | tool | **Free** OSS | https://rust-lang.github.io/mdBook | |
| **Zenodo** | service | **Free** | https://zenodo.org | DOI |
| **Read the Docs / GitHub Pages** | service | **Free** | various | Hosting |

**API keys required:** Zenodo + RTD (if used) auth.

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### `docs/v1.0/` directory structure
```
docs/v1.0/
├── axc-format/                       # .axc v1.0 spec (RFC-style)
├── axiom-ir/                         # AXIOM-IR v1.0 (all dialects)
├── theorems/                         # L0–L6 correctness-theorem index
├── design-rationale/
│   ├── g1-lean-and-extraction.md
│   ├── g2-rust-parsers.md
│   ├── g3-ir.md
│   ├── g4-forensics.md
│   ├── g5-symbolic.md
│   ├── g6-equivalence.md
│   ├── g7-proof-systems.md
│   ├── g8-fuzzing.md
│   ├── g9-native.md
│   ├── g10-dynamic.md
│   ├── g11-ml.md
│   ├── g12-supply-chain.md
│   ├── g13-infra.md
│   └── g14-tooling.md
├── api/
│   ├── verifier-rest.md
│   ├── axiom-py.md
│   ├── axiom-go.md
│   └── axiom-ts.md
├── quickstart/
│   ├── verifier.md
│   ├── axiom-py.md
│   ├── axiom-go.md
│   └── axiom-ts.md
├── consumer-migration.md
├── threat-model.md
├── reproducibility-evidence.md
└── release-notes.md
```

### Per-document
- SHA-256 + Ed25519 sig at end
- Zenodo DOI at top

### mdBook output
- Single static site
- Published to GitHub Pages + own domain `docs.apkaxiom.org`

### API references
- Auto-generated from rustdoc / pydoc / godoc / typedoc
- Linked from `docs/v1.0/api/`

### Theorem index
- Per theorem: name, statement (informal English), Lean filename, proof-object hash, scope
- Cross-link to relevant cert subtype + design rationale

### Reproducibility evidence index
- Per phase release: SHA-256, builder, builder version, build flags, repro instructions
- Link to `audit/proof-object-log.jsonl`

### Documentation sustainability
- Per-document owner + review cadence
- Quarterly review reminder

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| `docs/v1.0/axc-format/` complete + signed | yes |
| `docs/v1.0/axiom-ir/` complete + signed | yes |
| `docs/v1.0/theorems/` complete | yes |
| All 14 group-rationale documents complete | yes |
| API references complete (verifier + 3 SDKs) | yes |
| Quickstarts complete (verifier + 3 SDKs) | yes |
| Migration guide complete | yes |
| Threat model complete | yes |
| Reproducibility-evidence index complete | yes |
| mdBook static site live at docs.apkaxiom.org | yes |
| Per-document SHA-256 + Ed25519 sig | 100 % |
| Zenodo DOIs assigned | per-document |

## 9. Working Directory & Files Produced

```
apkaxiom/
└── docs/
    └── v1.0/                         # NEW: complete v1.0 doc set
```

## 10. Standalone Output

A complete, signed, DOI-tracked documentation set published as the canonical v1.0 specification.

## 11. End-to-End Test

```bash
mdbook build docs/v1.0
# Expect: clean build

# Per-document signature
for f in docs/v1.0/**/*.md; do
  cosign verify-blob --signature ${f}.sig $f
done

# Zenodo DOIs
zenodo-cli list --collection axiom-v1
```

## 12. Exit Checklist

- [ ] All 11 categories under `docs/v1.0/` complete (HARD)
- [ ] All 14 group-rationale documents complete (HARD)
- [ ] All API references complete (HARD)
- [ ] All quickstarts complete (HARD)
- [ ] mdBook static site live (HARD)
- [ ] Per-document SHA-256 + sig 100 %
- [ ] Per-document Zenodo DOI
- [ ] Quarterly review cadence assigned

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.19** | Docs deployed alongside production verifier |
| **P6.20** | "Documentation: spec docs complete for all formats and theorems" item ✅ for ship gate |
| **External community** | Citable v1.0 doc set |
