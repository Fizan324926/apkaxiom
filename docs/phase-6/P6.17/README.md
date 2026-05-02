# P6.17 — External Security Audit (Trail of Bits / NCC / Aleph)

> ~10-week external security audit by Trail of Bits, NCC Group, Aleph Research, or Atredis. Soundness review, cryptographic review, supply-chain review, side-channel review, operational review. No critical findings open at v1.0 ship.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.17 |
| Owner(s) | G7 + G13 + leadership |
| Duration | Weeks 6–22 (≈ 10-week engagement + remediation buffer) |
| Critical-path | yes (gates v1.0 ship) |
| Hard prerequisites | P6.1 (engagement signed) |

## 2. Goal & Scope

A formal external audit. Scope: full proof stack + cryptographic path + supply-chain + verifier service + reproducibility. Auditor sandbox provisioned in P6.1. Critical findings (if any) remediated and re-audited before ship.

### In scope
- Soundness review: every theorem statement, every layer's correctness claim
- Cryptographic review: HACL\* invariants, Halo2 / Plonky3 / Binius / Stwo circuit correctness, signing keys, trusted setup
- Supply-chain review: Buck2 + Bazel + Nix flake hermeticity, SLSA L4, in-toto, Sigstore
- Side-channel review: timing, cache, power (where applicable)
- Operational review: production verifier service, secrets handling, key custody, on-call procedures
- Documentation review: spec docs, API references, threat model
- Remediation if any critical finding
- Re-audit if remediation needed

### Out of scope
- Pen-test of in-the-wild apps (out of scope for APKAXIOM auditor)
- Audit of downstream consumer apps

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P6.1** | Auditor engagement signed + sandbox provisioned |
| **P6.2 .. P6.15** | All stabilization deliverables |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Read-only repo mirror** | live | Auditor sandbox |
| **Eval cluster (read-only access)** | live | Auditor reproduces results |
| **Slack / encrypted email** | (existing) | Auditor communication |
| **NDA signing portal** | varies | Per auditor |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Trail of Bits** | service | **Paid** $250–500K | https://www.trailofbits.com | Reference firm |
| **NCC Group** | service | **Paid** $250–500K | https://www.nccgroup.com | Alt |
| **Aleph Research** | service | **Paid** $200–400K | https://alephsecurity.com | Alt |
| **Atredis Partners** | service | **Paid** $200–400K | https://www.atredis.com | Alt |

**API keys required:** auditor sandbox creds, ProtonMail / encrypted-email auth.

## 6. System Inventory — Have vs Need

All present from P6.1.

## 7. Features & Functions Delivered (Comprehensive)

### Auditor onboarding pack
- Threat model document
- Architecture overview
- Per-layer correctness theorem index
- Cryptographic invariants list
- Supply-chain attestation chain
- Production deployment topology
- Key custody policy
- Incident response runbook
- Reproducibility evidence

### Audit engagement
- Weekly check-in with audit lead
- Auditor questions tracked in private issue queue
- ≤ 48 h response SLA on auditor questions

### Findings tracker
- Per finding: severity (critical / high / medium / low / info)
- Per finding: response (fix / accept / decline-with-rationale)
- Critical findings → fix + re-audit before ship

### Remediation budget
- ≥ 4-week remediation buffer (W18–W22)

### Final report
- Auditor delivers final report by W22
- Public summary published with v1.0 release (full report under NDA at auditor's discretion)

### Audit close-out
- All critical findings resolved or accepted-with-rationale (the latter requires leadership ADR)
- Audit close-out letter signed
- Audit-firm name + engagement scope cited in v1.0 release notes

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| Engagement signed by W2 | yes |
| Auditor onboarding pack delivered by W4 | yes |
| Weekly check-ins green | yes |
| Auditor question SLA ≤ 48 h | yes |
| Final report delivered by W22 | yes |
| 0 critical findings open at v1.0 ship | yes |
| All high findings either fixed or accepted with leadership ADR | yes |
| Public audit summary published | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── threat-model.md               # NEW (or refreshed)
│   ├── crypto-invariants.md          # NEW
│   ├── audit-onboarding.md           # NEW
│   └── audit-summary-public.md       # NEW (post-engagement)
├── audit/
│   ├── findings.jsonl                # NEW (private)
│   ├── responses.jsonl               # NEW (private)
│   └── close-out-letter.pdf          # NEW
└── (Auditor private deliverables stored encrypted)
```

## 10. Standalone Output

External validation = the v1.0 endorsement that distinguishes APKAXIOM from research code.

## 11. End-to-End Test

The audit-close-out letter itself. Plus:

```bash
# Public audit summary
test -f docs/audit-summary-public.md
grep -c "no critical open" docs/audit-summary-public.md  # ≥ 1
```

## 12. Exit Checklist

- [ ] Engagement signed (HARD)
- [ ] Onboarding pack delivered
- [ ] Weekly check-ins green
- [ ] Final report delivered
- [ ] 0 critical findings open at v1.0 (HARD)
- [ ] All high findings fixed or accepted
- [ ] Public audit summary published
- [ ] Audit-firm + scope cited in v1.0 release notes
- [ ] Audit close-out letter signed

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.19** | Audit close-out letter referenced in production launch |
| **P6.20** | "External audit completed, no critical findings open" item ✅ for ship gate |
| **External community** | Public audit summary in v1.0 release |
