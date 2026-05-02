# P4.8 — Privacy Invariant 4: Device-Identifier Read Forbidden Halo2 Circuit

> *"This APK provably never reads device identifiers (IMEI / Mac / SerialNumber / AAID without user consent)."* Compliance with Android privacy guidelines + GDPR.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §11](../../../README.md#layer-6)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.8 |
| Owner(s) | G7 |
| Duration | Weeks 9–14 |
| Critical-path | yes |
| Hard prerequisites | P4.4, P4.5 |

## 2. Goal & Scope

A Halo2 circuit proving the APK never reads device-identifier APIs. Compliance-grade for GDPR / Google Play Privacy Policy / Apple ATT-equivalent. Catches a wide class of advertising-SDK abuse.

### In scope
- `theorems/Apkaxiom/PrivacyInvariants/NoDeviceIdRead.lean`
- Halo2 circuit `crates/axiom-circuit-no-device-id`
- Witness extractor: comprehensive device-identifier API surface across A8–A15
- End-to-end demo on F-Droid (apps that should be clean)

### Out of scope
- Per-permission-grant gating (the invariant is "never reads"; we don't model "reads only after consent")

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P4.4** | Lean → Halo2 pipeline |
| **P4.5** | Template |
| **P3.10** | Abstract domains for over-approximation |

## 4. Required Tools, Libraries, and Languages

Same as P4.5.

## 5. Third-Party Software, Services, Accounts & API Keys

**No new third-party.**

## 6. System Inventory — Have vs Need

Nothing new system-level.

## 7. Features & Functions Delivered (Comprehensive)

### Lean theorem
```lean
inductive DeviceIdReadKind where
  | TelephonyImei
  | TelephonyImsi
  | NetworkInterfaceMac
  | SettingsSecure_AndroidId
  | BuildSerial
  | AdvertisingId

theorem no_device_id_read (apk : APK) :
  ∀ (path : ExecutionPath apk),
    path.calls.all (fun call ⇒ ¬ call.is_device_id_read)
```

### Comprehensive API surface (A8–A15)
- `TelephonyManager.getDeviceId()` (deprecated A10+, but apps still call)
- `TelephonyManager.getImei()`
- `TelephonyManager.getMeid()`
- `TelephonyManager.getSubscriberId()`
- `WifiInfo.getMacAddress()` (returns 02:00:00:00:00:00 since A6, but apps still call)
- `NetworkInterface.getHardwareAddress()`
- `Settings.Secure.getString(ANDROID_ID)`
- `Build.SERIAL` / `Build.getSerial()`
- `AdvertisingIdClient.getAdvertisingIdInfo()` (Google Play Services)
- Per-version deltas captured from P3.2 archaeology

### Witness extractor
- Reflection-aware: catches `Class.forName().getMethod()`-style invocations
- Native-code aware: TODO Phase 5
- AAR/JAR import-aware: catches when device-ID call is buried in a vendored library

### Halo2 circuit
- Public input: Merkle root of (api_surface)
- Private witness: per-call site, proof that the call is *not* in the surface
- Constraints: callee API ∉ Merkle-encoded surface
- Circuit size: target ≤ 2^16 rows

### Soundness chain
- Apply P4.4 trust-bridge theorem

### Documentation
- `docs/circuit-no-device-id.md`

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Lean theorem mechanized | yes | yes |
| Circuit operational | yes | yes |
| Comprehensive A8–A15 device-ID surface | 100 % covered | 100 % |
| Reflection-aware (catches Class.forName-style) | yes | yes |
| Prove p99 | ≤ 5 s | ≤ 1.5 s |
| Verify p99 | ≤ 20 ms | ≤ 5 ms |
| F-Droid demo provable rate | ≥ 80 % | ≥ 95 % |
| Cert size | ≤ 30 KB | ≤ 10 KB |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── theorems/Apkaxiom/PrivacyInvariants/
│   └── NoDeviceIdRead.lean
├── crates/
│   └── axiom-circuit-no-device-id/
└── docs/
    └── circuit-no-device-id.md
```

## 10. Standalone Output

```bash
buck2 run //tools/cli -- prove-no-device-id --apk app.apk
```

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-circuit-no-device-id:demo
# - Provable rate ≥ 80% on F-Droid (HARD)
# - Prove p99 ≤ 5 s (HARD)
# - Verify p99 ≤ 20 ms (HARD)
# - Reflection-aware test cases pass (HARD)
```

## 12. Exit Checklist

- [ ] Lean theorem mechanized
- [ ] Comprehensive A8–A15 device-ID surface
- [ ] Reflection awareness
- [ ] Prove p99 ≤ 5 s (HARD)
- [ ] Verify p99 ≤ 20 ms (HARD)
- [ ] F-Droid ≥ 80 % provable (HARD)
- [ ] Cert ≤ 30 KB (HARD)
- [ ] Documentation published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P4.11** | Verifier handles claim type |
| **P4.17** | Bug-bounty + privacy-compliance pilot |
| **External (regulators)** | First production GDPR-compliance zk-proof |
