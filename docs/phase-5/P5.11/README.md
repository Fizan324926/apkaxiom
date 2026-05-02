# P5.11 — Frida Script Library + Auto-Attach

> Curate a reusable Frida script library covering the JNI boundary, native intent dispatch, syscalls, OpenSSL/BoringSSL, TFLite runtime, and common anti-tamper bypasses. Auto-attach with low latency on emulator pool.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.11 |
| Owner(s) | G10 |
| Duration | Weeks 6–14 |
| Critical-path | yes (dynamic confirmation needs it) |
| Hard prerequisites | P5.10 |

## 2. Goal & Scope

A library of Frida scripts that, when activated on a target APK, produce structured trace events consumable by the dynamic-confirmation bridge.

### In scope
- Frida script library covering:
  - JNI boundary entry / exit (matching JNIEnv* table from P5.6)
  - Native intent dispatch (`startActivity` / `sendBroadcast` etc.)
  - Syscalls of interest (open, read, write, connect, sendto, recvfrom, ioctl)
  - OpenSSL / BoringSSL tracing (TLS handshake, cert pinning bypass detection)
  - TFLite runtime (model load, invoke, output)
  - Reflection probes (Java reflection API)
- Auto-attach mechanism: detect emulator → install + spawn → attach Frida
- Anti-tamper bypass profiles: DexProtector, Promon, AppGuard, BANK family — multi-strategy rotation
- Trace event schema: typed protobuf, suitable for replay
- Low attach latency: ≤ 2 s (HARD), ≤ 500 ms (TARGET)
- Minimal overhead: < 30 % runtime overhead per traced syscall

### Out of scope
- eBPF kernel-side tracing (P5.12)
- Dynamic-bridge logic (P5.13)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P5.10** | Emulator pool live with Frida-server in image |
| **P5.6** | JNI boundary model (used to design hooks) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Frida** | 16.x or later | Instrumentation runtime |
| **frida-rs** | latest | Rust client |
| **TypeScript** | 5+ | Frida script language |
| **protoc** | 25+ | Trace schema |
| **Cap'n Proto** | latest | Optional alt schema |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Frida** | tool | **Free** OSS | https://frida.re | |
| **frida-rs** | lib | **Free** OSS | https://github.com/frida/frida-rust | |
| **DexProtector test corpus** | corpus | (acquire / partner) | https://dexprotector.com | For bypass tuning |
| **Promon SHIELD test corpus** | corpus | (acquire / partner) | https://promon.co | |
| **AppGuard test corpus** | corpus | (acquire / partner) | https://lookout.com | |

**API keys required:** vendor partnerships (Promon, DexProtector, Lookout) for test access — handled via legal team, not in scope of engineering.

## 6. System Inventory — Have vs Need

All present from P5.1 + P5.10.

## 7. Features & Functions Delivered (Comprehensive)

### Frida script library (`frida-scripts/`)
- `jni-boundary.ts` — hooks JNIEnv* table, emits typed boundary events
- `intent-dispatch.ts` — `startActivity` / `sendBroadcast` / `bindService` / `IntentSender.sendIntent` (Java + native side via JNI back-call)
- `syscalls.ts` — open / read / write / connect / sendto / recvfrom / ioctl
- `openssl.ts` — TLS handshake, pinning bypass detection, cipher suite log
- `boringssl.ts` — same surface
- `tflite.ts` — model load, invoke, input/output tensor capture
- `reflection.ts` — `Method.invoke`, `Class.forName`, `Field.get/set`
- `permissions.ts` — runtime-permission grant detection
- `device-id.ts` — IMEI / serial / MAC / Settings.Secure access
- `package-manager.ts` — `getPackageInfo` / `queryIntentActivities` / `queryBroadcastReceivers`

### Auto-attach mechanism
- `axiom-frida-attach` Rust client
- Detect emulator → install APK → spawn → wait for boot-complete → spawn Frida → load script set
- Re-attach on lost connection
- Multi-pid support for processes with `:isolatedProcess`

### Anti-tamper bypass profiles
- DexProtector: bypass profile with multi-injection-strategy rotation
- Promon SHIELD: profile
- AppGuard: profile
- Detection-evasion is **only** active in the Phase-5 emulator-research context (consent-gated). Bypass code is gated behind `DYNAMIC_PROFILE=research` env to ensure it never enters production verifier paths.

### Trace event schema
- Typed protobuf (`protos/frida-trace.proto`)
- Each event: timestamp, pid, tid, hook id, args, return, stack
- Compressed to Brotli for storage

### Performance
- Attach latency p99 ≤ 2 s (HARD)
- Per-syscall hook overhead ≤ 30 %

### Reproducibility
- Same APK + same emulator state → same trace ordering modulo non-determinism
- Trace replayer exists for deterministic post-hoc analysis

### Tools
- `axiom-frida-attach` — CLI for manual + automated runs
- `axiom-frida-replay` — replay captured trace through static analyzer

### Documentation
- `docs/frida-library.md` — script catalog + extension guide

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Attach latency p99 | ≤ 2 s | ≤ 500 ms |
| Per-syscall hook overhead | < 30 % | < 10 % |
| Script library size | ≥ 10 scripts | ≥ 15 scripts |
| Anti-tamper bypass profiles | ≥ 3 | ≥ 5 |
| Trace event throughput | ≥ 50 K events/s | ≥ 200 K events/s |
| Auto-attach success rate (across emulator API levels) | ≥ 95 % | ≥ 99 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── frida-scripts/                   # NEW: TypeScript scripts
│   ├── jni-boundary.ts
│   ├── intent-dispatch.ts
│   ├── syscalls.ts
│   ├── openssl.ts
│   ├── boringssl.ts
│   ├── tflite.ts
│   ├── reflection.ts
│   ├── permissions.ts
│   ├── device-id.ts
│   └── package-manager.ts
├── crates/
│   └── axiom-frida-attach/          # NEW: Rust client
├── tools/
│   ├── axiom-frida-attach
│   └── axiom-frida-replay
├── protos/
│   └── frida-trace.proto            # NEW
└── docs/
    └── frida-library.md             # NEW
```

## 10. Standalone Output

The Frida script library + auto-attach is reusable beyond APKAXIOM in any Android dynamic-analysis tool.

## 11. End-to-End Test

```bash
buck2 build //frida-scripts:...
buck2 build //crates/axiom-frida-attach:...

buck2 run //tools:axiom-frida-attach -- --target <pkg> --scripts all
# Expect: attach ≤ 2 s, all 10 scripts loaded
```

## 12. Exit Checklist

- [ ] Attach latency p99 ≤ 2 s (HARD)
- [ ] Per-syscall overhead < 30 % (HARD)
- [ ] ≥ 10 scripts published (HARD)
- [ ] ≥ 3 anti-tamper bypass profiles (HARD)
- [ ] Trace throughput ≥ 50 K events/s (HARD)
- [ ] Auto-attach success ≥ 95 % across API levels
- [ ] Trace replayer functional + deterministic
- [ ] Documentation `docs/frida-library.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.13** | Frida traces feeding dynamic confirmation |
| **P5.18** | Frida library in E2E pipeline |
| **L4 / L5** | Trace replayer feeding refined static analysis |
