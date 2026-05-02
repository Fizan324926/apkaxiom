# P3.13 — Behavior Surface Hash (BSH-256) RFC Freeze

> The 256-bit obfuscation-invariant fingerprint specification. Frozen as an RFC. Defines the lingua franca for cross-tool similarity references.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §10.1 (BSH-256)](../../../README.md#layer-5)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.13 |
| Owner(s) | G6 |
| Duration | Weeks 4–12 |
| Critical-path | yes |
| Hard prerequisites | P3.1 (G6 onboarded) |

## 2. Goal & Scope

The Behavior Surface Hash (BSH-256) specification — frozen as a public RFC. Defines exactly what bytes go into the hash, the canonicalization rules, the collision-resistance argument, and the versioning policy. Reference Rust implementation lands in P3.14.

### In scope
- `docs/BSH-256-RFC.md` (≥ 50 pages)
- Canonical input definition (sorted permissions + intent filters + exported components + dangerous-API call set + network destinations)
- Hash construction (BLAKE3-keyed, with personalization)
- Collision-resistance analysis
- Versioning policy (post-freeze change-control)
- External reviewer feedback (Lean community, cryptographers)
- ADR-0019 — BSH-256 freeze

### Out of scope
- Implementation (P3.14)
- LSH index (P3.14)
- Bisimulation (P3.15)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.1** | G6 onboarded |
| **P2.9** | AXIOM-IR-v0.2 frozen (manifest + DEX + resource dialects) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **HACL\* BLAKE3** | from P1.10 | Hash primitive |
| **Lean 4** | for canonicalization theorems | Optional formal grounding |
| **PlantUML / Mermaid** | latest | Diagrams |
| **Markdown linting** | latest | RFC consistency |
| **External cryptographers (1–2 reviewers)** | — | Pre-freeze review |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **BLAKE3 reference team test vectors** | reference | **Free** | https://github.com/BLAKE3-team/BLAKE3 | Used to verify our personalization layer doesn't break BLAKE3 |
| **External cryptographic-review service** | service | **Paid** $5–25 K (Trail of Bits / NCC small engagement) | varies | Optional pre-RFC freeze review |
| **arXiv** | preprint | **Free** | https://arxiv.org | Eventual paper deposit (Phase 4) |
| **Zenodo** | DOI | **Free** | https://zenodo.org | RFC artifact DOI |

**Optional paid item:** external cryptographic-review service to validate the collision-resistance argument before freeze. Strongly recommended at nation-grade.

## 6. System Inventory — Have vs Need

### Already present
- ✅ HACL\* BLAKE3 (P1.10)
- ✅ Lean / Lake (optional formalization)

### Missing
- Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### BSH-256 RFC (`docs/BSH-256-RFC.md`)

#### Inputs (the behavior surface)
1. **Sorted permission set** — every `<uses-permission>` declaration, sorted lexicographically, BCP-47-canonicalized
2. **Sorted intent filter set** — every component's intent filters, sorted by `(action, category, data-scheme, mimetype, priority)` tuple
3. **Sorted exported component set** — `<activity android:exported="true">`, etc.
4. **Sorted dangerous-API call set** — extracted from DEX (Phase-2 opcode-subset coverage); list of `(class, method, signature)` tuples for known dangerous APIs (cryptography, networking, file-system, location, contacts, etc.)
5. **Sorted network destinations** — extracted from manifest + DEX string-pool: hostnames, IPs, IP ranges
6. **Sorted asset-pack delivery descriptors** — for bundle-era completeness
7. **Sorted dynamic-feature module names + delivery types**

#### Canonicalization rules
- All inputs sorted in stable lexicographic order
- All names normalized to fully-qualified Java naming
- All IPs normalized to CIDR notation when ranges
- All locales BCP-47-normalized
- DEX string-pool deduplicated before extraction
- All inputs encoded in UTF-8 with explicit length prefix

#### Hash construction
- `BSH-256 := BLAKE3("apkaxiom-bsh-256-v1" || sha256(canonical_input))`
- `BLAKE3` invoked with personalization string `"apkaxiom-bsh-256"`
- 256-bit output

#### Collision-resistance argument
- Reduction: BSH-256 collision implies BLAKE3 collision (cryptographic strength)
- Adversary model: state-level threat with full APK control
- Discussion of false-collision via canonicalization edge cases (UTF-8 normalization, locale aliases)
- Per-input independent contribution (no cross-input cancellation)

#### Versioning policy
- BSH-256-v1: this freeze
- Future versions: explicit version-namespacing in personalization string
- Migration guide between versions
- Backwards-compatibility commitments

#### Stability across obfuscators
- Section: ProGuard, R8, DexGuard transformations don't affect BSH-256
- Argument: layout/method-name changes don't change behavior surface
- Empirical baseline (target ≥90% stability on Repack-2K, measured in P3.14)

### External cryptographic review
- Engagement: 2-week NCC Group / Trail of Bits review (paid, $5–25K)
- Findings + responses documented in `docs/BSH-256-external-review.md`

### Lean grounding (optional)
- `theorems/Apkaxiom/Bsh.lean` — canonicalization theorems (deduplication, sort-stability)
- Lean theorem: `bsh_canonicalization_idempotent`

### Decision log
- ADR-0019 — BSH-256 freeze
- ADR-0020 — Personalization string + version-namespacing policy

### Documentation
- `docs/BSH-256-RFC.md` (frozen)
- `docs/BSH-256-rationale.md` (why these inputs, why this hash)
- `docs/BSH-256-external-review.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| BSH-256-RFC ≥ 50 pages | yes | ≥ 80 pages |
| All 7 input categories defined and canonicalized | yes | yes |
| Collision-resistance argument documented and reviewed | yes | yes |
| External cryptographer review completed | yes | yes |
| Reviewer sign-off (G1, G6, G7 leads + 2 external) | yes | yes |
| RFC frozen ≥ 4 weeks before P3.18 | yes | yes |
| ADRs 0019 + 0020 merged | 2 | 2 |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── BSH-256-RFC.md                    # NEW — FROZEN
│   ├── BSH-256-rationale.md              # NEW
│   ├── BSH-256-external-review.md        # NEW
│   ├── ADR-0019-bsh-256-freeze.md        # NEW
│   └── ADR-0020-bsh-versioning-policy.md # NEW
├── theorems/Apkaxiom/Bsh.lean            # NEW (optional formalization)
└── diagrams/
    └── bsh-input-flow.mmd
```

## 10. Standalone Output

The frozen RFC + cryptographic-review report. Citable, auditable, reproducible by any third party.

## 11. End-to-End Test

```bash
# Verification
test -f docs/BSH-256-RFC.md
grep -q "FROZEN ON" docs/BSH-256-RFC.md
test -f docs/BSH-256-external-review.md
grep -c "^✅ approved by" docs/sign-offs/P3.13.md  # ≥ 5 (3 internal + 2 external)
```

## 12. Exit Checklist

- [ ] BSH-256-RFC ≥ 50 pages (HARD)
- [ ] All 7 input categories defined + canonicalization rules (HARD)
- [ ] Collision-resistance argument documented
- [ ] External cryptographic review completed and findings addressed (HARD)
- [ ] Reviewer sign-offs from G1, G6, G7 + 2 external (HARD)
- [ ] RFC frozen ≥ 4 weeks before P3.18 (HARD)
- [ ] ADRs 0019, 0020 merged
- [ ] Optional Lean canonicalization theorems land

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P3.14** | RFC as input to BSH-256 Rust implementation |
| **P3.15** | BSH-256 used as similarity oracle by bisimulation engine |
| **P3.17** | Layer 5 unifies BSH + bisim + LSH |
| **External community** | First citable BSH-256 standard (lingua franca) |
