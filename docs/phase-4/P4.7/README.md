# P4.7 — Privacy Invariant 3: Location-Without-Network Halo2 Circuit

> *"This APK never accesses location without prior network connectivity."* Catches stalkerware-class behaviors. Compliance-grade for healthcare and finance apps.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §11](../../../README.md#layer-6)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.7 |
| Owner(s) | G7 |
| Duration | Weeks 8–13 |
| Critical-path | yes |
| Hard prerequisites | P4.4, P4.5 |

## 2. Goal & Scope

A Halo2 circuit proving location-API access is causally-after a network connectivity check. Catches stalkerware patterns that read location and silently exfiltrate. Useful for healthcare apps (HIPAA), finance apps, and ride-share/dating-app compliance.

### In scope
- `theorems/Apkaxiom/PrivacyInvariants/LocationGated.lean`
- Halo2 circuit `crates/axiom-circuit-location-gated`
- Witness extractor: extract all location-API call sites + flow analysis from network-API check sites
- End-to-end demo

### Out of scope
- Other invariants (P4.8–P4.9)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P4.4** | Lean → Halo2 pipeline |
| **P3.5** | Intent-resolution semantics (used to identify call paths) |
| **P3.10** | Type abstraction domains (for control-flow ordering) |

## 4. Required Tools, Libraries, and Languages

Same as P4.5 / P4.6.

| Tool | Version | Purpose |
|---|---|---|
| **Halo2 / Poseidon2** | from P4.3/P4.5 | Circuit |
| **petgraph** | latest | Control-flow graph |

## 5. Third-Party Software, Services, Accounts & API Keys

**No new third-party.**

## 6. System Inventory — Have vs Need

Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### Lean theorem
```lean
theorem location_gated_by_network (apk : APK) :
  ∀ (path : ExecutionPath apk),
    path.calls.location_call →
    ∃ (earlier_call : path.calls), earlier_call.is_network_check ∧
      path.causal_order earlier_call call
```

### Witness extractor
- Identify all location-API call sites: `LocationManager.requestLocationUpdates`, `FusedLocationProviderClient.*`, etc. across A8–A15 + Google Play Services
- Identify all network-check call sites: `ConnectivityManager.getActiveNetworkInfo`, `NetworkCallback.*`, etc.
- Build per-component control-flow graph
- Per-location call: prove a network-check appears earlier on every reaching path

### Halo2 circuit
- Public input: Merkle root of (location_call_set, network_check_set)
- Private witness: per-location-call, the proof of "network-check happened first" — a path through the CFG with both call IDs
- Constraints: CFG topological ordering + Merkle membership of both calls
- Custom gates: `cfg_path_walk`, `causal_order`
- Circuit size: target ≤ 2^17 rows

### Limitation handling
- Apps with no network checks but always-on location: produce UNKNOWN with `Reason::NoNetworkChecksFound`
- This is an explicit-incompleteness signal, not a silent over-approximation

### Soundness chain
- Apply P4.4's trust-bridge theorem

### Documentation
- `docs/circuit-location-gated.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Lean theorem mechanized | yes | yes |
| Circuit operational | yes | yes |
| Prove p99 | ≤ 5 s | ≤ 1.5 s |
| Verify p99 | ≤ 20 ms | ≤ 5 ms |
| Demo on healthcare-app subset (manual curation) | ≥ 80 % provable | ≥ 95 % |
| Cert size | ≤ 60 KB | ≤ 25 KB |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── theorems/Apkaxiom/PrivacyInvariants/
│   └── LocationGated.lean
├── crates/
│   └── axiom-circuit-location-gated/
└── docs/
    └── circuit-location-gated.md
```

## 10. Standalone Output

```bash
buck2 run //tools/cli -- prove-location-gated --apk app.apk
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-circuit-location-gated:demo
# - Demo provable ≥ 80% (HARD)
# - Prove p99 ≤ 5 s (HARD)
# - Verify p99 ≤ 20 ms (HARD)
```

## 12. Exit Checklist

- [ ] Lean theorem mechanized
- [ ] Circuit compiles
- [ ] Soundness chained
- [ ] Prove p99 ≤ 5 s (HARD)
- [ ] Verify p99 ≤ 20 ms (HARD)
- [ ] Demo ≥ 80 % provable (HARD)
- [ ] UNKNOWN with explicit Reason::NoNetworkChecksFound
- [ ] Documentation published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P4.11** | Verifier handles claim type |
| **P4.17** | Bug-bounty + healthcare pilot |
