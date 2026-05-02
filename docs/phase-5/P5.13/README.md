# P5.13 — Dynamic Confirmation Bridge: UNKNOWN Refinement

> Wire Frida + eBPF traces into the static UNKNOWN flow: when L4 (Phase 3) or joint analyzer (P5.8) returns UNKNOWN, the bridge launches a sandboxed emulator session, runs trace scripts, and refines the abstraction.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.13 |
| Owner(s) | G5 + G10 |
| Duration | Weeks 12–18 |
| Critical-path | yes |
| Hard prerequisites | P5.11, P5.12 |

## 2. Goal & Scope

A pipeline that closes the static-dynamic loop:
1. L4 / joint analyzer emits UNKNOWN with abstraction marker
2. Bridge maps UNKNOWN → required Frida script set + eBPF programs
3. Emulator session spawned, target installed, scripts loaded
4. Drive APK with seed inputs (deeplinks, UI Monkey, fuzzed Intents)
5. Trace replayed back through static abstraction
6. Static UNKNOWN → ✓ / ✗ / UNKNOWN-with-evidence
7. Cert L6 carries the resolution evidence

### In scope
- UNKNOWN classifier → script-set mapper
- Emulator session lifecycle: acquire → install → drive → trace → release
- Trace replayer feeding back into L4
- UNKNOWN refinement rate ≥ 30 % HARD (≥ 60 % TARGET)
- Per-finding refinement p99 ≤ 300 s HARD (≤ 60 s TARGET)
- Driver: deeplink fuzz + Monkey + grammar-aware Intent fuzz
- Consent gating: dynamic only runs in *research* / pilot context, not on every certificate by default
- Cert evidence shape: signed trace digest + mapping to L4 abstraction

### Out of scope
- Frida / eBPF script content (P5.11 / P5.12)
- Emulator pool (P5.10)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P5.11** | Frida script library + auto-attach |
| **P5.12** | eBPF program library |
| **P5.10** | Emulator pool |
| **P5.8** | Joint analyzer producing typed UNKNOWNs |
| **P3.11** | UNKNOWN handling + abstraction-refinement loop |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Android UI Automator / Monkey** | matching SDK | Driver |
| **Drozer / MobSF (eval only)** | latest | Reference dynamic |
| **MLIR** | 19 | IR |
| **Rust** | 1.84+ | Implementation |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Android SDK Platform Tools** | tool | **Free** | https://developer.android.com/studio/releases/platform-tools | adb, monkey |
| **UI Automator / Espresso** | lib | **Free** | https://developer.android.com | |

**No new API keys.**

## 6. System Inventory — Have vs Need

All present.

## 7. Features & Functions Delivered (Comprehensive)

### Crate `axiom-dynamic-bridge`
- UNKNOWN classifier → script-set + eBPF-program-set mapper
- Emulator session lifecycle manager (acquire / install / drive / trace / release)
- Driver protocol:
  - Deeplink fuzz (parses manifest deeplink intents, generates inputs)
  - Monkey driver
  - Grammar-aware Intent fuzz (uses AXIOM-IR symbolic dialect to generate well-typed intents)
  - User-supplied seeds for known scenarios
- Trace ingester
- Trace replayer feeding L4 (refines abstraction with concrete data)
- Verdict computation: ✓ (refines UNKNOWN to SAT) / ✗ (refines to UNSAT) / UNKNOWN-with-evidence (still UNKNOWN, but with bounded evidence)

### UNKNOWN-evidence cert subtype
- New `.axc` cert subtype: dynamic-confirmation evidence
- Carries trace digest + script set used + emulator state digest
- Verifiable via `axiom-verify` against cert format

### Consent gating
- Dynamic confirmation is **opt-in** at the policy level
- Default: certs cite static-only proofs
- Pilot bug-bounty platform may opt-in for cross-language UNKNOWNs

### Performance
- Per-finding p99 ≤ 300 s HARD (≤ 60 s TARGET)
- Parallel sessions ≥ 8 on 16-core (HARD)

### Resolution rate
- UNKNOWN → ✓/✗ ≥ 30 % HARD (≥ 60 % TARGET)

### Reproducibility
- Same APK + same seed + same emulator snapshot → same verdict (modulo non-determinism, captured in evidence)

### Tools
- `axiom-dynamic-bridge-cli` — CLI
- `axiom-dynamic-bench` — perf

### Documentation
- `docs/dynamic-bridge.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| UNKNOWN refinement rate | ≥ 30 % | ≥ 60 % |
| Per-finding p99 | ≤ 300 s | ≤ 60 s |
| Parallel sessions on 16-core | ≥ 8 | ≥ 16 |
| Cert evidence verifiability | 100 % | 100 % |
| Consent-gating behavior tested | yes | yes |
| Reproducibility (modulo non-determinism) | tested | tested |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-dynamic-bridge/        # NEW
├── tools/
│   ├── axiom-dynamic-bridge-cli
│   └── axiom-dynamic-bench
├── circuits/                         # NEW evidence-circuit (cited from L6 cert)
│   └── dynamic-evidence/
└── docs/
    └── dynamic-bridge.md             # NEW
```

## 10. Standalone Output

The dynamic-confirmation bridge is a research-grade artifact citable in the Phase-5 paper.

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-dynamic-bridge:...
buck2 run //tools:axiom-dynamic-bench -- --corpus malware-1k --threads 16
# Expect: UNKNOWN refinement ≥ 30 %, p99 ≤ 300 s

# Single example
buck2 run //tools:axiom-dynamic-bridge-cli -- --apk path/to.apk --policy research
```

## 12. Exit Checklist

- [ ] UNKNOWN refinement ≥ 30 % (HARD)
- [ ] Per-finding p99 ≤ 300 s (HARD)
- [ ] Parallel sessions ≥ 8 on 16-core (HARD)
- [ ] Cert evidence verifiability 100 %
- [ ] Consent-gating respected; default static-only
- [ ] Reproducibility tested
- [ ] Cross-checked Frida vs eBPF traces (no contradictions)
- [ ] Documentation `docs/dynamic-bridge.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.18** | Dynamic bridge in E2E pipeline |
| **P5.19** | Dynamic-confirmation results in paper |
| **L6 cert** | New evidence subtype |
| **Pilot platform** | Optional opt-in dynamic for cross-language UNKNOWNs |
