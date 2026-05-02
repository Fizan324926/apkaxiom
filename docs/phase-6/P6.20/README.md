# P6.20 — v1.0 Ship-Gate Review + Tag + Release Announcement

> The v1.0 ship gate. Walk every line of the 20-item checklist (PHASE_GATES.md §10 + ROADMAP §15) against the live dashboard. If every item is ✅ for ≥ 90 consecutive days, tag `v1.0.0`, sign via cosign, announce. If any one is ❌, slip. Do not ship a degraded v1.0.

**Parent plan:** [../README.md](../README.md) · **PHASE_GATES.md §10:** [../../PHASE_GATES.md#phase-6](../../PHASE_GATES.md#phase-6) · **ROADMAP.md DoD:** [../ROADMAP.md#dod](../ROADMAP.md#dod)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.20 |
| Owner(s) | Project leadership + all G1–G14 leads |
| Duration | Weeks 24–26 |
| Critical-path | yes — final ship gate |
| Hard prerequisites | P6.16, P6.17, P6.18, P6.19 |

## 2. Goal & Scope

The non-negotiable v1.0 ship gate. Live dashboard walkthrough. Every box ✅ for ≥ 90 consecutive days. Tag + signed release + announcement. Or: slip the date.

### In scope
- v1.0 ship-gate review meeting (recorded + minuted)
- Live dashboard walkthrough — every PHASE_GATES.md §10 item verified
- Audit close-out letter signed off (P6.17)
- Sign-off from all G1–G14 leads + leadership
- v1.0 tag created via cosign signed
- Release announcement
- Post-launch monitoring rotation 24/7
- Phase 6 retrospective merged
- v1.1 roadmap kickoff ADR

### Out of scope
- v1.1 implementation
- Major new features

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **All P6.x** | Their exit checklists must be ✅ for ≥ 90 consecutive days |
| **P6.16** | 50K eval results published |
| **P6.17** | Audit close-out, no critical findings |
| **P6.18** | Documentation complete |
| **P6.19** | Production deploy + 90-day availability |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **gh CLI** | latest | GitHub release |
| **cosign** | (existing) | Tag signing |
| **rekor** | (existing) | Transparency log |
| **All P6 dashboards** | live | Verification |

## 5. Third-Party Software, Services, Accounts & API Keys

All existing.

**No new API keys** beyond those needed for press distribution (HN/Twitter/X/Reddit auth — handled by leadership).

## 6. System Inventory — Have vs Need

Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### v1.0 ship-gate review meeting
- Live walkthrough: every PHASE_GATES.md §10 line + ROADMAP §15 line verified against dashboards
- Recorded for archival
- Per item: ✅ / ❌ / re-check-needed
- Single ❌ → slip (not partial ship)

### v1.0 tag
- `git tag -s v1.0.0 -m "APKAXIOM v1.0"` (or unsigned + cosign artifact sig if no GPG)
- `cosign sign-blob` of git-rev SHA → publish to Rekor
- Release-notes generation (auto-pulled from per-phase release notes)

### Release announcement
- Blog post on apkaxiom.org
- HN submission
- Twitter/X thread
- Reddit r/programming + r/netsec
- Academic mailing lists
- Security newsletter outreach
- Press kit distributed

### Post-launch monitoring
- 24/7 on-call rotation continuing from P6.19
- Daily review of verifier metrics for first 30 days post-launch
- Bug-bounty pilot live monitoring

### Phase 6 retrospective
- What worked / didn't
- Process learnings for v1.1+
- Recognition for contributors

### v1.1 roadmap kickoff ADR
- Carry-forward debt from Phase 6 → v1.1
- Major features deferred from v1.0 (A16 formalization, ONNX scanning, additional native ISAs, JVM bytecode, etc.)
- v1.1 timeline (target: 6–12 months post v1.0)
- Phase numbering for v1.1 cycle

### Sign-offs
- All 14 group leads
- Leadership
- External-audit firm sign-off (from P6.17)
- DPO sign-off on dataset release

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| All 20 v1.0 ship-gate items ✅ for ≥ 90 consecutive days | yes |
| External audit no-critical-open | yes |
| 90-day reproducibility-byte-identical CI green | yes |
| 90-day verifier availability ≥ 99.99 % | yes |
| 90-day soundness-zero-incident | yes |
| ≥ 3 papers accepted at top venues | yes |
| ≥ 10 CVEs filed | yes |
| Pilot platform live ingesting `.axc` | yes |
| All documentation published | yes |
| v1.0 tag signed via cosign | yes |
| Release announcement published | yes |
| Sign-off from all leads + leadership | yes |
| Phase 6 retrospective merged | yes |
| v1.1 roadmap ADR approved | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── docs/
│   ├── phase6-retrospective.md       # NEW
│   ├── ADR-v1.1-Roadmap-Kickoff.md   # NEW
│   └── v1.0-release-notes.md         # NEW (consolidated)
├── meetings/
│   └── 2026-MM-DD-v1.0-ship-gate-review.md
└── (Tag v1.0.0 + cosign sig + Rekor entry + GitHub release + blog + press)
```

## 10. Standalone Output

The v1.0 release itself — the entire 36-month plan deliverable.

## 11. End-to-End Test

The v1.0 ship-gate review meeting itself. Live dashboard walkthrough; every line verified.

```bash
# Tag + sign
git tag -a v1.0.0 -m "APKAXIOM v1.0 — Proof-Carrying APKs in Production"
git push origin v1.0.0

cosign sign-blob --yes \
  $(git rev-parse v1.0.0) > v1.0.0.sig
gh release create v1.0.0 \
  --title "APKAXIOM v1.0 — Proof-Carrying APKs in Production" \
  --notes-file docs/v1.0-release-notes.md \
  --target main
gh release upload v1.0.0 v1.0.0.sig

# Verify the signed release
cosign verify-blob --signature v1.0.0.sig $(git rev-parse v1.0.0)

# Production launch announcement
# (handled via leadership + press)
```

## 12. Exit Checklist (the v1.0 ship gate — 20 items, all hard)

```
[ ] axiom-verify p99 ≤ 100 ms over 10K certs (90 consecutive days green)
[ ] Service availability ≥ 99.99 % over 90 days
[ ] 50K APK eval completes ≤ 72 h on 100-core cluster
[ ] 90 consecutive days byte-identical CI
[ ] Three-arch (x86_64 + ARM64 + RISC-V) bit-identical certificates
[ ] Crash rate < 1 per 10M APKs
[ ] Soundness regression incidents = 0 over 90 days
[ ] MTBF ≥ 720 h in production
[ ] 5× burst tolerance verified
[ ] Streaming verification ≤ 50 ms after last byte
[ ] External audit closed, no critical findings
[ ] 50K APK eval published as paper + dataset
[ ] ≥ 3 papers accepted at top venues
[ ] ≥ 10 CVEs filed
[ ] Pilot bug-bounty platform live in production
[ ] All documentation published
[ ] All SDKs all archs pass integration suite
[ ] Wire-speed inspection ≥ 1 Gbps verified
[ ] Cross-time reproducibility verified (rebuild Phase-1 release on Phase-6 toolchain)
[ ] v1.0 tag signed + announcement published
```

If 19/20 are ✅ → **slip the release**. Do not ship a degraded v1.0.

If 20/20 are ✅ → **tag, sign, announce**. v1.0 ships.

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **External community** | Public v1.0 release: tag + binaries + SDKs + paper + dataset + audit summary |
| **v1.1 cycle** | Phase 6 retrospective + v1.1 ADR + carry-forward debt list |
| **Production** | 24/7 monitoring rotation continues |
| **Future audits** | Annual external audit cadence established post-v1.0 |

---

*"v1.0 ships when every theorem checks, every cert verifies, every CI gate is green for 90 days, and every external auditor has signed off. Not before."*
