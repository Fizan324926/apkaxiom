# P3.9 — Cross-APK Device-Snapshot Prototype

> Reason over *sets* of installed APKs. First sound-and-complete cross-app intent-confusion analyzer. ≥ 1 zero-day intent-hijack found from cross-APK queries.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §13.5 (Cross-APK)](../../../README.md#beyond-the-12)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P3.9 |
| Owner(s) | G5 (sub-team — 2 engineers) |
| Duration | Weeks 12–17 |
| Critical-path | no, but feeds Phase-3 paper |
| Hard prerequisites | P3.8 (single-APK L4) |

## 2. Goal & Scope

Extend Layer 4 to reason over a *set* of installed APKs (a "device snapshot"). Discover intent-hijack, content-provider exposure, and permission-aggregation attacks across an installed app set. Goal: ≥ 1 zero-day intent-hijack discovered via cross-APK analysis.

### In scope
- `crates/axiom-l4-snapshot` — cross-APK extension
- Snapshot-budget abstraction — bounded reasoning over fleet snapshots
- Pre-computed snapshot indices for common app combinations
- Snapshot-fuzzing harness — randomized installed-app-set generation
- Consent-gating: user (or device-fleet operator) must explicitly approve cross-APK analysis

### Out of scope
- Full enterprise-fleet analysis (deferred past v1.0)
- Symbolic+native joint analysis (Phase 5)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P3.8** | Single-APK L4 base |
| **P3.7** | CHC encoder extends naturally to snapshots |
| **P2.12** | BehaviorSet for each installed APK in the snapshot |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Spacer / Eldarica** | already integrated | CHC over enlarged snapshots |
| **Soufflé** | optional, from P3.7 | Datalog for some snapshot-fuzzing patterns |
| **Hypothesis** (Python) | latest | Property-based snapshot generation |
| **rkyv / fjall** | latest | Snapshot archive |
| **Glommio** | from P1.7 | Async dispatch |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **AndroZoo** (for realistic snapshots) | corpus | **Free academic** | already provisioned | Sample real-world app combinations |
| **CICAndMal2017** | labeled corpus | **Free research** | UNB Canada | Adversarial snapshots |
| **Coordinated-disclosure CNA partner** | CVE filing | **Free** | continuation from P1.13 | For zero-day intent-hijack findings |
| **Google Android Security Rewards** | bug bounty | **Free** | https://www.google.com/about/appsecurity/android-rewards/ | Up to $1M for severe findings |

**No new API keys.**

## 6. System Inventory — Have vs Need

### Already present
- ✅ All from P3.8 + P3.7 + P3.6
- ✅ Hypothesis

### Missing
- Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### Public Rust API
- `pub fn resolve_snapshot(snapshot: &DeviceSnapshot, query: &SnapshotQuery) -> SnapshotOutcome`
- `pub struct DeviceSnapshot { installed_apps: Vec<(ApkId, BehaviorSet, SignatureSet)>, user_id: UserId, api_level: AndroidVersion, default_apps: Map<...>, permissions: Map<...> }`
- `pub enum SnapshotQuery { CanIntercept(intent, victim_apk, victim_component), CanReadProvider(provider_uri, attacker_apk), CanAggregatePerm(perm_group, attacker_apk), Custom(SmtAssertion) }`
- `pub enum SnapshotOutcome { AttackPathFound(Vec<AttackStep>), NoAttackProof(UnsatCert), Unknown(AbsDomain, Reason) }`

### Snapshot construction
- `pub fn from_real_device(apks: Vec<&[u8]>) -> DeviceSnapshot` — real-world ingest
- `pub fn from_fuzzer(seed: u64, fleet_size: usize) -> DeviceSnapshot` — randomized for fuzz
- `pub fn from_androzoo_sample(stratum: &Stratum) -> DeviceSnapshot` — realistic combinations

### Snapshot-budget abstraction
- Tunable per-query: max_apps_in_snapshot, max_permission_combinations, timeout
- For very-large snapshots: abstraction levels (only-public-components, only-exported, full)
- UNKNOWN with `AbsDomain::SnapshotBudgetExhausted` when budget hit

### Pre-computed indices
- Common app combinations indexed for fast snapshot-fuzzing turn-around
- DiskANN-backed index keyed by app-set fingerprint (preview of P3.14)

### Consent-gating
- Cross-APK analysis requires explicit operator approval (config flag)
- Audit log of approved snapshots
- No analysis of fetched-from-Internet APK sets without consent

### Attack-path output
- Each step: which APK acts, which component runs, which intent is dispatched, what permissions are aggregated
- Replayable: given the snapshot, an external verifier can rebuild and observe

### Documentation
- `docs/l4-cross-apk-snapshot.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Snapshot resolution throughput (10-app snapshots) | ≥ 5 snapshots/sec/core | ≥ 20 snapshots/sec/core |
| Snapshot p99 latency (10-app) | ≤ 5 s | ≤ 1 s |
| Cross-APK UNKNOWN rate on benign 1K snapshots | ≤ 35 % | ≤ 15 % |
| Zero-day intent-hijack discovered | ≥ 1 reproducible | ≥ 5 |
| Reproduction across A11..A15 verified | yes | yes |
| Consent-gating in place | yes | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-l4-snapshot/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/
│           ├── lib.rs
│           ├── snapshot.rs
│           ├── query.rs
│           ├── budget.rs
│           ├── index.rs                   # pre-computed snapshot index
│           └── consent.rs
├── corpus/snapshots/
│   ├── benign-1k/                          # 1K realistic snapshots
│   └── adversarial/                        # crafted attack scenarios
├── findings/
│   └── snapshot-attack-paths/              # NEW
└── docs/
    └── l4-cross-apk-snapshot.md            # NEW
```

## 10. Standalone Output

```bash
nix develop
buck2 build //crates/axiom-l4-snapshot --release
buck2 run //tools/cli -- snapshot-resolve --snapshot path/to/snapshot.json --query "CanIntercept ACTION_SEND com.victim.WhatsApp.ShareActivity"
# Outputs AttackPathFound or NoAttackProof
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-l4-snapshot:full-eval
# - Snapshot resolution throughput ≥ 5 q/sec/core (HARD)
# - p99 ≤ 5 s (HARD)
# - UNKNOWN ≤ 35% on benign-1K (HARD)
# - ≥ 1 zero-day reproduced (HARD)
```

## 12. Exit Checklist

- [ ] Cross-APK extension lands
- [ ] Throughput ≥ 5 q/sec/core (HARD)
- [ ] p99 ≤ 5 s (HARD)
- [ ] UNKNOWN ≤ 35 % on benign-1K (HARD)
- [ ] ≥ 1 zero-day intent-hijack reproducible (HARD)
- [ ] Consent-gating active and audited
- [ ] CVE-filing pipeline tested with the discovered zero-day
- [ ] `docs/l4-cross-apk-snapshot.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P3.18** | Cross-APK results part of E2E |
| **P3.19** | The discovered zero-day is a paper highlight |
| **Phase 4 / G7** | Snapshot-attack-path findings ship in `.axc` |
