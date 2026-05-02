# P4.17 — Bug-Bounty Pilot Platform Integration

> Live in production with a partner platform. Triagers run `axiom-verify` and get ✅/❌ in ≤ 2 s. The first bug-bounty pipeline that ingests proof-carrying findings.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §11](../../../README.md#layer-6)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.17 |
| Owner(s) | G14 + bug-bounty platform partner |
| Duration | Weeks 16–22 |
| Critical-path | yes |
| Hard prerequisites | P4.11 (verifier core), P4.13 (axiom-py for back-end), P4.5–P4.9 (real claims to ingest) |

## 2. Goal & Scope

A working pilot integration with a bug-bounty platform. Triagers receive `.axc` certs, click "Verify", and see ✅ / ❌ + claim-level breakdown in ≤ 2 s. Goal: ≥ 500 `.axc` files / hour ingestion rate.

### In scope
- Partnership agreement with HackerOne or Bugcrowd or Open Bug Bounty
- Backend service: REST/gRPC API on top of `axiom-verify-core`
- Triager UI integration (front-end widget or platform-side render)
- `.axc` upload + storage
- Per-finding render: human-readable claim breakdown
- 1000+ verifications during pilot to gather feedback

### Out of scope
- Multiple platform partnerships (Phase 5+)
- Custom analyzer integrations (researchers can submit `.axc` independently)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P4.11** | Verifier core |
| **P4.13** | axiom-py (back-end services) |
| **P4.5–P4.9** | Real claims to ingest |
| **P4.16** | SLSA / reproducibility claims |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **axiom-verify-core** | from P4.11 | Verification |
| **axiom-py** | from P4.13 | Back-end SDK |
| **gRPC** + **prost** | latest | Protocol |
| **PostgreSQL** | 16+ | Cert + finding archive |
| **MinIO** (from earlier) | latest | Object store for `.axc` blobs |
| **Cloudflare Workers** *(optional)* | latest | Edge verifier |
| **React + Tailwind** *(if we own the UI)* | latest | Triager-facing widget |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **HackerOne** | platform partner | **Paid** for clients; **revenue share** for partners | https://www.hackerone.com | Conversation initiated in P4.1 |
| **Bugcrowd** | platform partner | **Paid** for clients; **revenue share** | https://www.bugcrowd.com | Alt partner |
| **Open Bug Bounty** | platform partner | **Free** | https://www.openbugbounty.org | Free option |
| **PostgreSQL** | DB | **Free** OSS | already common | Self-host |
| **gRPC / prost** | RPC | **Free** OSS | crates.io | |
| **Auth0 / Clerk / GitHub Auth** | auth | Free tier; **paid** | varies | For triager auth |
| **Stripe** *(if revenue model in pilot)* | payments | **Paid** ~ 2.9 % + $0.30 | https://stripe.com | Probably out of scope for Phase 4 pilot |

**API keys (likely):**
- HackerOne / Bugcrowd partnership API token (issued by partner)
- PostgreSQL connection string (managed via env)
- Auth provider OAuth credentials

## 6. System Inventory — Have vs Need

### Already present
- ✅ axiom-verify + axiom-py + Phase-3 stack

### Missing — must install
- ❌ **PostgreSQL 16+** — Docker Compose
- ❌ **gRPC tooling** — Cargo deps + protoc

```bash
docker run -d --name pg16 -p 5432:5432 -e POSTGRES_PASSWORD=$(openssl rand -base64 32) postgres:16
sudo apt-get install -y protobuf-compiler
```

## 7. Features & Functions Delivered (Comprehensive)

### Backend service (`crates/axiom-bb-pilot`)
- gRPC service exposing `VerifyCert(stream Cert) returns (stream VerifyResult)`
- REST gateway for compatibility
- PostgreSQL persistence: cert archive, verification logs, audit trail
- Process-isolated verification workers
- Auth via OAuth (GitHub / Google / SSO)
- Rate limiting per auth principal

### Triager-facing UI
- Either:
  1. Submit React widget for embedding into HackerOne / Bugcrowd UI, OR
  2. Implement standalone webapp consuming partner APIs

### Per-finding render
- Human-readable claim breakdown
- Per-claim ✅ / ❌ with reasoning
- Witness inspection (where applicable)
- "Re-verify with apk" button

### Ingestion pipeline
- File upload via REST/gRPC
- BLAKE3 deduplication
- Per-cert audit trail
- Cert archive in MinIO + Lance for analytics

### Performance
- p50 verify ≤ 200 ms (cold) / ≤ 50 ms (warm)
- Throughput ≥ 500 `.axc` / hour ingestion (HARD)
- Triager render ≤ 2 s p99 (HARD)

### Pilot operations
- ≥ 1000 verifications during 4-week pilot
- Triager feedback collection
- Bug filing for any cert that triggers a researcher dispute
- Weekly metrics review with partner

### Documentation
- `docs/bb-pilot-integration.md` — partner-facing
- `docs/bb-pilot-runbook.md` — operations

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Pilot platform live in production | yes | yes |
| `.axc` ingestion rate | ≥ 500 / hour | ≥ 5,000 / hour |
| Triager-facing render p99 | ≤ 2 s | ≤ 300 ms |
| Cert→human-readable pipeline p99 | ≤ 5 s | ≤ 1 s |
| Verifications during pilot | ≥ 1,000 | ≥ 10,000 |
| Triager dispute rate (false ✅) | 0 | 0 |
| Triager dispute rate (false ❌) | < 1 % | 0 |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-bb-pilot/
│       ├── Cargo.toml
│       ├── BUCK
│       ├── proto/
│       │   └── pilot.proto
│       └── src/
│           ├── main.rs
│           ├── grpc.rs
│           ├── rest.rs
│           ├── db.rs
│           └── audit.rs
├── ui/
│   └── bb-pilot-widget/                  # React widget
│       ├── package.json
│       └── src/
└── docs/
    ├── bb-pilot-integration.md
    └── bb-pilot-runbook.md
```

## 10. Standalone Output

```bash
buck2 build //crates/axiom-bb-pilot --release
docker compose up -d
# Pilot service live at https://pilot.axiom.example.com
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-bb-pilot:full
# - Live pilot reachable (HARD)
# - Ingestion ≥ 500/hour (HARD)
# - Render ≤ 2 s p99 (HARD)
# - 0 false-✅ over pilot (HARD)
```

## 12. Exit Checklist

- [ ] Partnership agreement signed
- [ ] Backend service live in production (HARD)
- [ ] gRPC + REST API operational
- [ ] Triager UI integrated
- [ ] Ingestion ≥ 500 `.axc` / hour (HARD)
- [ ] Triager render ≤ 2 s p99 (HARD)
- [ ] ≥ 1000 verifications during pilot (HARD)
- [ ] 0 false-✅ over pilot (HARD)
- [ ] Triager-feedback collection in place
- [ ] Documentation published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P4.18** | E2E measures pilot KPIs |
| **P4.19** | Pilot results in paper |
| **Phase 5+** | Multi-partner expansion + additional analyzer integrations |
| **External users** | First production proof-carrying bug-bounty pipeline |
