# P5.6 — JNI Bridge Modeling (Java ↔ Native Boundary)

> Model the Java Native Interface boundary in AXIOM-IR: argument / return marshaling, ref types, JNIEnv calls (FindClass, GetMethodID, CallXXXMethod, NewGlobalRef, …). ≥ 75 % common-pattern coverage on a 50-bridge survey.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.6 |
| Owner(s) | G9 + G5 |
| Duration | Weeks 6–14 |
| Critical-path | yes (joint analyzer needs it) |
| Hard prerequisites | P5.3, P5.4 |

## 2. Goal & Scope

A JNI boundary model in AXIOM-IR sufficient for the joint Java + native intent analyzer (P5.8) to follow control + data across the Java↔native boundary.

### In scope
- JNI boundary nodes (frozen in P5.2): `jni.call`, `jni.return`, `jni.global_ref`, `jni.local_ref`, `jni.detach_thread`
- JNIEnv* function-pointer table modeled as 232 named ops (every JNI function)
- Static + dynamic bridge registration handled (`RegisterNatives` + `JNI_OnLoad`)
- Dispatch resolution: name-mangling (Java_pkg_Class_method) + signature decoding
- Argument / return marshaling: jboolean / jbyte / jchar / jshort / jint / jlong / jfloat / jdouble / jobject / jstring / jarray
- Ref-type lifecycle (local → global → weak global)
- Exception handling (`Throw`, `ExceptionCheck`, `ExceptionClear`)
- ≥ 75 % HARD / ≥ 95 % TARGET coverage on 50-bridge survey
- Provenance flow: data marked at the boundary so L4 / L5 see it

### Out of scope
- Joint analyzer logic (P5.8)
- Native common-library catalog (P5.7)
- Lean theorems for JNI marshaling (P5.9)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P5.2** | JNI boundary nodes frozen |
| **P5.3** | DEX SSA available |
| **P5.4** | ARM64 lift available |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **JDK 17 + JNI headers** | from OpenJDK | JNI function table reference |
| **Android NDK** | r26+ | NDK JNI sample patterns |
| **MLIR** | 19 | IR construction |
| **Rust** | 1.84+ | Implementation |
| **demangler** (`itanium-demangle`, etc.) | latest | C++ symbol decoding |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Android NDK r26+** | toolchain | **Free** | https://developer.android.com/ndk | |
| **OpenJDK 17 JNI headers** | headers | **Free** | https://openjdk.org | |
| **JNI sample apps corpus** | corpus | **internal** | — | NDK-100 subset |

**No new API keys.**

## 6. System Inventory — Have vs Need

| Need | Status |
|---|---|
| JDK 17 with JNI headers | install via apt |
| NDK r26+ | install |

```bash
sudo apt-get install -y openjdk-17-jdk-headless
wget https://dl.google.com/android/repository/android-ndk-r26d-linux.zip
unzip -d third-party/ndk-r26d android-ndk-r26d-linux.zip
```

## 7. Features & Functions Delivered (Comprehensive)

### Crate `axiom-jni-bridge`
- JNI function-pointer table (232 functions) modeled as MLIR ops
- Static bridge resolver: parses `.dynsym` → demangles → matches to Java method declarations from DEX SSA
- Dynamic bridge resolver: traces `JNI_OnLoad` + `RegisterNatives` calls in the lifted ELF SSA
- Argument-marshaling rules per type
- Return-marshaling rules per type
- Ref-type lifecycle FSM (created → local → global → released)
- Exception-channel modeling
- Provenance: every value crossing the boundary tagged with `jni:source=java|native, sig=<...>`

### Cross-dialect call op
- `crossdial.invoke`(java→jni→native) modeled as a single op carrying both sides' SSA values

### Coverage instrumentation
- Coverage metric: % of `Java_*` symbols + `RegisterNatives` entries matched to a corresponding Java declaration
- Survey corpus: 50 hand-curated bridges from popular libs (Mapbox, Realm, libsodium, OpenSSL via JNI, Tencent MMKV, AdMob, etc.)

### UNKNOWN classification
- For unmodeled bridges: emit IR `jni.unknown` with the boundary description; downstream L4 treats as opaque

### Differential testing
- Compare resolved bridges against Frida runtime trace (when emulator available, P5.10) — used as cross-check, not gate

### Documentation
- `docs/jni-bridge.md` — model + coverage table + how to extend

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| 50-bridge survey common-pattern coverage | ≥ 75 % | ≥ 95 % |
| All 232 JNIEnv* functions modeled | yes | yes |
| Static bridge resolution accuracy | ≥ 95 % | ≥ 99 % |
| Dynamic bridge resolution (`RegisterNatives`) | ≥ 90 % | ≥ 98 % |
| Provenance flow at boundary tested | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-jni-bridge/
│       ├── src/
│       │   ├── jni_table.rs
│       │   ├── static_resolve.rs
│       │   ├── dynamic_resolve.rs
│       │   ├── marshalling.rs
│       │   └── ref_lifecycle.rs
│       └── tests/
├── corpora/
│   └── jni-50-survey/                # NEW
└── docs/
    └── jni-bridge.md                 # NEW
```

## 10. Standalone Output

A reusable JNI boundary model usable in any Android-binary analyzer.

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-jni-bridge:...

buck2 run //tools:axiom-jni-survey -- --corpus jni-50-survey
# Expect ≥ 75 % common-pattern coverage
```

## 12. Exit Checklist

- [ ] 50-bridge survey coverage ≥ 75 % (HARD)
- [ ] All 232 JNIEnv* functions modeled
- [ ] Static bridge resolution ≥ 95 %
- [ ] Dynamic bridge resolution ≥ 90 %
- [ ] Provenance flow tests 100 %
- [ ] UNKNOWN classification documented for unmodeled bridges
- [ ] Documentation `docs/jni-bridge.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.7** | Bridge model used to classify common-library bridges |
| **P5.8** | Joint analyzer can follow Java→native control + data |
| **P5.9** | JNI boundary model spec for Lean theorems |
