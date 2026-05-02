# P6.19 — Production Deployment of `axiom-verify` Service + Open-Data Paper

> Production deployment of `axiom-verify` as a public service. ≥ 99.99 % availability over 90 days. Public API, rate-limited, Cloudflare-fronted. Open-data paper finalized.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P6.19 |
| Owner(s) | G14 + G13 |
| Duration | Weeks 18–24 |
| Critical-path | yes |
| Hard prerequisites | P6.16, P6.17 |

## 2. Goal & Scope

The verifier-as-a-service is the public-facing v1.0 deliverable. Multi-region, autoscaling, rate-limited, Cloudflare-fronted. 90-day availability window opens at W18 and must be ≥ 99.99 % at v1.0 ship.

### In scope
- Multi-region deployment (US-East primary, EU-West secondary, optional APAC tertiary)
- Autoscaling via Karpenter
- Rate limit: 1 cert/sec/IP free; bulk via API key (tiered 100/1K/10K cert/sec)
- Cloudflare WAF + DDoS protection
- Public API (REST + gRPC)
- Status page live (statuspage.io or self-hosted Upptime)
- p99 ≤ 100 ms continuous regression test
- 90-day availability window opens
- 90-day SLA dashboard
- Open-data paper finalized: *"The APKAXIOM Corpus: Proof-Stack Evaluation on 50K Android Packages"*
- Press kit + launch communications

### Out of scope
- New verifier features
- New API methods (deferred to v1.1)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P6.16** | 50K eval results + paper draft |
| **P6.17** | Audit close-out (no critical findings) |
| **P6.15** | Verifier service stabilized + Cloudflare configured |
| **P6.18** | API reference docs |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **k8s + Karpenter** | (existing) | Compute |
| **Cloudflare** | (existing) | Front + DDoS |
| **statuspage.io / Upptime** | service | Public uptime |
| **Sentry** | (existing) | Error tracking |
| **Pyroscope / Prometheus / Grafana** | (existing) | Observability |
| **gRPC** | latest | Bonus protocol |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Cloudflare Pro / Business** | service | **Paid** $20–200/mo | (existing) | DDoS + rate limit |
| **AWS / GCP multi-region** | service | **Paid** | (existing) | |
| **statuspage.io** | service | **Paid** $29–99/mo | https://statuspage.io | Public status |
| **Upptime** (self-hosted alt) | tool | **Free** OSS | https://upptime.js.org | |
| **Twilio / PagerDuty** | service | **Paid** | various | On-call |

**API keys required:** Cloudflare token (existing), statuspage / PagerDuty integration tokens.

## 6. System Inventory — Have vs Need

All present from P6.1 + P6.15.

## 7. Features & Functions Delivered (Comprehensive)

### Production deployment
- Multi-region: US-East (primary), EU-West (secondary), optional APAC (tertiary)
- Karpenter auto-scaling 0 → cluster-max
- Per-region health-check + failover ≤ 5 min
- Per-cert rate limit: 1 cert/sec/IP free; bulk via API key tiers
- Cloudflare WAF + DDoS + rate limit at edge

### Public API
- REST: `POST /verify` (cert in, verdict out), `GET /healthz`, `GET /metrics`
- gRPC: bonus, same surface
- API key issuance: self-serve via web portal + Stripe billing for paid tiers (bonus; v1.1 if not ready)
- API reference docs (from P6.18)

### Status page
- Live at status.apkaxiom.org
- Auto-updated from Prometheus health probes
- Per-region status

### 90-day availability window
- W18 → P6.20 = 90 days
- Continuous monitoring; any down-time event tracked
- v1.0 ship gate: ≥ 99.99 % over the window

### Per-PR regression
- p99 ≤ 100 ms continuous; per-PR check; CI gate

### Open-data paper finalized
- Polished + submitted (USENIX Security 2028 / VLDB / open-data track)
- arXiv pre-print updated

### Press kit + launch communications
- v1.0 announcement draft (reviewed in P6.20)
- Per-channel: blog post, Twitter/X thread, HN, Reddit r/programming + r/netsec, academic mailing lists, security newsletters

### On-call rotation
- 24/7 rotation across G14 + G13 SREs
- PagerDuty integration
- On-call runbooks for top 10 incident types

### Documentation
- `docs/g14-production.md` (production runbook)
- `docs/v1.0/api/verifier-rest.md` finalized

## 8. KPIs (this sub-phase)

| KPI | HARD |
|---|---|
| Multi-region deployment (≥ 2 regions) | yes |
| Karpenter auto-scaling operational | yes |
| Public API live (REST) | yes |
| Status page live | yes |
| 90-day availability window opens | yes (continuous) |
| 90-day availability ≥ 99.99 % at P6.20 | yes |
| p99 ≤ 100 ms continuous (90-day window) | yes |
| Cloudflare WAF + rate limit operational | yes |
| On-call rotation operational | yes |
| Open-data paper submitted | yes |
| Press kit drafted | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── infra/
│   └── verifier-service/             # multi-region
├── papers/
│   └── eval-50k-open-data/           # finalized + submitted
├── docs/
│   └── g14-production.md             # NEW
├── press-kit/                        # NEW
│   ├── announcement.md
│   ├── blog-post.md
│   └── social.md
└── (Status page + Cloudflare WAF rules + on-call rotation tracker)
```

## 10. Standalone Output

Production verifier service + open-data paper.

## 11. End-to-End Test

```bash
# Production verifier
curl -X POST https://verify.apkaxiom.org/verify -d @cert.axc
# Expect: 200 OK with verdict, p99 ≤ 100 ms

# Multi-region failover
curl -X POST https://verify-eu.apkaxiom.org/verify -d @cert.axc
# Expect: 200 OK

# Status page
curl -s https://status.apkaxiom.org/api/v2/summary.json | jq '.status.indicator'
# Expect: "none" (= operational)
```

## 12. Exit Checklist

- [ ] Multi-region deployed (HARD)
- [ ] Public API live (HARD)
- [ ] Status page live (HARD)
- [ ] 90-day availability window opens (continuous; ≥ 99.99 % at ship)
- [ ] p99 ≤ 100 ms continuous (HARD)
- [ ] Cloudflare WAF + rate limit operational
- [ ] On-call rotation operational
- [ ] Open-data paper submitted (HARD)
- [ ] Press kit drafted
- [ ] Documentation `docs/g14-production.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P6.20** | Production deploy + 90-day availability + open-data paper for ship gate |
| **External community** | Production verifier + paper |
