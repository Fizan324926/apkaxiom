# P6.15 — G14 Stabilization: SDK Polish + Production Verifier Service Deployment

> Polish axiom-py / axiom-go / axiom-ts SDKs to v1.0. Production-deploy `axiom-verify` as a service: public API, rate-limited, ≥ 99.99 % over 90 days. Bug-bounty pilot platform live in production.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.15 |
| Owner(s) | G14 |
| Duration | Weeks 1–18 |
| Critical-path | yes |
| Hard prerequisites | P6.1 |

## 2. Goal & Scope

The user-facing surface of v1.0: SDKs polished + published to package registries; `axiom-verify` deployed as a public service; bug-bounty pilot live.

### In scope
- SDK polish: API stabilization, examples, integration tests, package-registry publication
- `axiom-verify` production deployment: multi-region, autoscaled, rate-limited, Cloudflare-fronted
- Bug-bounty pilot platform: live in production, ingesting `.axc` certs in real triager flow
- Mobile builds: ARM64 + Wasm, both production-ready
- Documentation: per-SDK quickstart, API reference, examples

### Out of scope
- New SDKs (Java/Kotlin etc. deferred to v1.1)
- New verifier features

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P6.1** | Stabilization punch-list |
| **All Phase 4 G14 deliverables** | Continued |
| **P6.4** | Stable AXIOM-IR-v1.0 spec |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Cloudflare** | service | Front + DDoS |
| **k8s + Karpenter** | (existing) | Compute |
| **PyO3 / uniffi / cgo / wit-bindgen / wasm-bindgen** | (existing, pinned) | SDK gen |
| **PyPI / crates.io / npm / Go modules** | registries | Publication |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Cloudflare** | service | **Paid** Pro/Business plan ~$20–200/mo | https://www.cloudflare.com | DDoS + rate limit |
| **PyPI** | registry | **Free** | https://pypi.org | axiom-py |
| **npm** | registry | **Free** | https://www.npmjs.com | axiom-ts |
| **Go modules** (proxy.golang.org) | registry | **Free** | https://pkg.go.dev | axiom-go |
| **HackerOne / Bugcrowd partnership** | service | (existing) | already provisioned | Pilot |
| **Status page** (statuspage.io / Upptime) | service | **Free / Paid** | various | Public uptime |

**API keys required:** PyPI / npm / Go-proxy / Cloudflare API tokens.

## 6. System Inventory — Have vs Need

| Need | Status |
|---|---|
| Production verifier service infra | provision via Karpenter |
| Cloudflare account + WAF rules | configure |

## 7. Features & Functions Delivered (Comprehensive)

### SDK polish
- API stabilization: `#[stable]` everywhere, deprecations removed
- Examples per SDK: 5+ end-to-end examples each
- Integration test suite per SDK against same 1000-APK corpus
- Per-SDK quickstart docs
- Per-SDK API reference (auto-generated)

### SDK publication
- axiom-py → PyPI (`pip install axiom`)
- axiom-go → Go modules (`go get github.com/...`)
- axiom-ts → npm (`npm install @axiom/verify`)
- Cosign-signed releases

### Production verifier service
- Multi-region deployment (US-East primary, EU-West secondary)
- Karpenter auto-scaling
- Per-cert rate limit (1 cert/sec/IP free; bulk via API key)
- Cloudflare WAF + DDoS protection
- Public API: `POST /verify` (cert in, verdict out)
- Per-API-key throughput tier
- Status page live

### Bug-bounty pilot
- Live ingestion of `.axc` certs
- Triager UI rendering `.axc` claims to human-readable findings
- Per-finding evidence chain
- Ingestion ≥ 5 K / hour TARGET (was ≥ 500 / hour HARD)

### Mobile builds
- ARM64 native build → Pixel-class p99 ≤ 200 ms
- Wasm build → Chromium 122+ p99 ≤ 300 ms
- Released alongside Phase-6 SDK release

### Documentation
- `docs/g14-stabilization.md`
- Per-SDK README + quickstart
- `docs/verifier-api.md` (REST API reference)

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| All 3 SDKs published | yes |
| Production verifier live ≥ 99.99 % over 90 days | yes (continuous; ramp-up in W18 → P6.20) |
| Verifier p99 ≤ 100 ms over 10K cert sample | yes |
| Verifier cluster ≥ 10K verifications / sec | yes |
| Bug-bounty pilot live in production | yes |
| Pilot ingestion ≥ 500 / hour | yes |
| Mobile builds p99 hit | yes |
| FFI overhead < 30 % per SDK | yes |
| Cosign-signed releases | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── sdk/
│   ├── axiom-py/                     # polished
│   ├── axiom-go/                     # polished
│   └── axiom-ts/                     # polished
├── crates/
│   └── axiom-verify/                 # production-ready
├── infra/
│   └── verifier-service/             # NEW: k8s + Karpenter + Cloudflare
├── docs/
│   ├── g14-stabilization.md          # NEW
│   ├── verifier-api.md               # NEW
│   └── sdk-quickstart-{py,go,ts}.md  # NEW
└── (PyPI / npm / Go releases + status page)
```

## 10. Standalone Output

Production verifier service + 3 published SDKs + bug-bounty pilot.

## 11. End-to-End Test

```bash
# Verifier production
curl -X POST https://verify.apkaxiom.org/verify -d @cert.axc
# Expect: 200 OK with verdict, p99 ≤ 100 ms

# SDKs
pip install axiom && python -c "from axiom import verify; verify('cert.axc')"
go get github.com/apkaxiom/go && go run examples/verify.go
npm install @axiom/verify && node examples/verify.js
```

## 12. Exit Checklist

- [ ] All 3 SDKs published to registries (HARD)
- [ ] Verifier production deployed (HARD)
- [ ] Verifier p99 ≤ 100 ms (HARD)
- [ ] Verifier ≥ 99.99 % availability (continuous; 90 days by P6.20)
- [ ] Bug-bounty pilot live (HARD)
- [ ] Mobile builds production-ready
- [ ] FFI overhead < 30 % per SDK
- [ ] Cosign-signed releases
- [ ] Status page live
- [ ] Documentation (per-SDK quickstart + API reference) published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.16** | Verifier service for 50K eval (sanity check) |
| **P6.17** | Production deploy explained to auditor |
| **P6.19** | Production deploy is the deliverable |
| **P6.20** | "axiom-verify production-deployed + SDKs published + pilot live" item ✅ for ship gate |
