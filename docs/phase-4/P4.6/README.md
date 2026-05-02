# P4.6 — Privacy Invariant 2: Network-Destination Allowlist Halo2 Circuit

> *"This APK provably never sends network traffic to a destination outside allowlist X."* The compliance-grade invariant for app stores and enterprise deployments.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §11](../../../README.md#layer-6)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.6 |
| Owner(s) | G7 |
| Duration | Weeks 7–12 |
| Critical-path | yes |
| Hard prerequisites | P4.4, P4.5 (template established) |

## 2. Goal & Scope

A Halo2 circuit proving the APK's network destinations are a subset of a configurable allowlist. The allowlist is a public input (the verifier sees it). Soundness: a Halo2 ✓ proves the APK *cannot* connect to anything off-list, on any execution path, across A8–A15.

### In scope
- `theorems/Apkaxiom/PrivacyInvariants/NetworkAllowlist.lean`
- Halo2 circuit `crates/axiom-circuit-network-allowlist`
- Witness extractor — extract all network destinations from manifest + DEX string-pool
- Public-input encoding for the allowlist (Merkle tree of allowed hostnames + IP ranges)
- End-to-end demo on F-Droid + AndroZoo benign subset

### Out of scope
- Other invariants (P4.7–P4.9)
- Dynamic confirmation (Phase 5)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P4.4** | Lean → Halo2 pipeline |
| **P4.5** | Template invariant |
| **P3.10** | String abstraction domains |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Halo2 / Poseidon2** | from P4.5 | Circuit framework |
| **publicsuffix list** | latest | Hostname normalization |
| **maxminddb / GeoIP2** | optional | Source-of-truth IP ranges |
| **Rust regex** | latest | Hostname pattern matching |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **publicsuffix list** | data | **Free** | https://publicsuffix.org | Mozilla maintained; canonical hostname normalization |
| **maxminddb / GeoLite2** | data | **Free** ASN/Country tier (account required) | https://www.maxmind.com/en/geolite2/signup | Optional for IP-range allowlists; needs sign-up |

**API key:** GeoLite2 (free tier; needs account creation). Used only for richer IP-range allowlist support.

## 6. System Inventory — Have vs Need

### Already present
- ✅ All from P4.5

### Missing — must install
- ❌ **publicsuffix list** — `cargo add publicsuffix`
- ❌ **maxminddb** — `cargo add maxminddb` + GeoLite2 mmdb file (optional)

## 7. Features & Functions Delivered (Comprehensive)

### Lean theorem
```lean
theorem network_dest_subset (apk : APK) (allowlist : Finset NetworkDest) :
  ∀ (path : ExecutionPath apk), path.network_calls.all
    (fun call ⇒ call.destination ∈ allowlist)
```

### Witness extractor
- Manifest scan: `<network-security-config>` references, hostnames in URLs
- DEX scan: string-pool entries matching hostname / IP regex
- Native-code coverage: TODO Phase 5 (G9 native subsystem)
- Per-A8–A15 deltas: differences in how networking APIs are exposed

### Halo2 circuit
- Public input: Merkle root of allowlist (Poseidon2-hashed hostnames + IP CIDRs)
- Private witness: each extracted network destination + Merkle inclusion proof
- Constraints: every destination ∈ allowlist (Merkle membership check)
- Custom gate: `hostname_canonicalize` (BCP 47-style normalization)
- Custom gate: `ip_in_cidr` (range membership)
- Circuit size: target ≤ 2^17 rows for typical APK + 100-entry allowlist

### Public-input format
- Merkle root of allowlist (Poseidon2)
- Allowlist size
- Encoding hint (hostname-vs-IP-vs-mixed)

### Soundness chain
- Apply P4.4's trust-bridge theorem
- Network-allowlist `.axc` claim → Lean theorem holds for the witness APK

### Demo
- F-Droid subset (1000 apps with stated allowlists in network-security-config)
- 95% provable target rate

### Documentation
- `docs/circuit-network-allowlist.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Lean theorem mechanized | yes | yes |
| Halo2 circuit operational | yes | yes |
| Prove p99 (H100, typical APK) | ≤ 5 s | ≤ 1.5 s |
| Verify p99 | ≤ 20 ms | ≤ 5 ms |
| F-Droid demo provable rate | ≥ 90 % | ≥ 99 % |
| Cert size for this claim | ≤ 50 KB | ≤ 20 KB |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── theorems/Apkaxiom/PrivacyInvariants/
│   └── NetworkAllowlist.lean
├── crates/
│   └── axiom-circuit-network-allowlist/
├── corpus/
│   └── publicsuffix-pinned/             # snapshot of publicsuffix list
└── docs/
    └── circuit-network-allowlist.md
```

## 10. Standalone Output

```bash
buck2 run //tools/cli -- prove-network-allowlist \
  --apk app.apk --allowlist allowlist.json
# Outputs .axc claim
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-circuit-network-allowlist:f-droid-demo
# - Provable rate ≥ 90% (HARD)
# - Prove p99 ≤ 5 s (HARD)
# - Verify p99 ≤ 20 ms (HARD)
```

## 12. Exit Checklist

- [ ] Lean theorem mechanized
- [ ] Circuit compiles + verifies
- [ ] Soundness chained
- [ ] Prove p99 ≤ 5 s (HARD)
- [ ] Verify p99 ≤ 20 ms (HARD)
- [ ] F-Droid demo ≥ 90 % provable (HARD)
- [ ] Cert ≤ 50 KB (HARD)
- [ ] Documentation published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P4.11** | Verifier handles this claim type |
| **P4.17** | Bug-bounty pilot can demo |
| **External (app stores, enterprise)** | First production network-allowlist zk-proof for Android |
