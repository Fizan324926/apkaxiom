# P5.7 — Native Common-Library Catalog (libc, OpenSSL, BoringSSL, NDK Patterns)

> Curate function-summary catalog for the most-used native dependencies in Android NDK code so the joint analyzer doesn't re-derive them. ≥ 30 NDK patterns + libc + OpenSSL + BoringSSL.

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.7 |
| Owner(s) | G9 |
| Duration | Weeks 4–14 |
| Critical-path | yes |
| Hard prerequisites | P5.4 |

## 2. Goal & Scope

A catalog of **function summaries** for native libraries that appear in nearly every Android app, so the joint analyzer can replace lifted bodies with verified summaries (much faster, much more precise).

### In scope
- libc (Bionic) — ≥ 200 functions: malloc/free, memcpy, memcmp, strcmp, snprintf, open/read/write/close, mmap/munmap, pthread_*, etc.
- OpenSSL — ≥ 80 functions: EVP_* family, SSL_*, RSA_*, EC_*, X509_*
- BoringSSL — same surface, separate fingerprints
- libsodium — ≥ 40 functions
- libcrypto common usages
- 30+ NDK pattern recipes: dlopen / dlsym, AAssetManager_*, ANativeWindow, EGL/GLES, MediaCodec, OpenSLES, AAudio, NDK NetworkSecurity API

### Out of scope
- Application-specific business logic (handled by joint analyzer P5.8)
- Lean theorems for summaries (P5.9 covers a subset)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P5.4** | ELF lifter mature enough to verify summaries |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Bionic libc source** | from AOSP | Reference |
| **OpenSSL** | 3.x stable | Reference |
| **BoringSSL** | latest from `boringssl` repo | Reference |
| **libsodium** | latest | Reference |
| **MLIR** | 19 | Summary IR encoding |
| **Rust** | 1.84+ | Implementation |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Bionic libc** | source | **Free** OSS (BSD-3) | AOSP | Vendored |
| **OpenSSL** | source | **Free** OSS (Apache 2.0) | https://www.openssl.org | |
| **BoringSSL** | source | **Free** OSS (BSD-3) | https://boringssl.googlesource.com | |
| **libsodium** | source | **Free** OSS (ISC) | https://libsodium.org | |

**No new API keys.**

## 6. System Inventory — Have vs Need

| Need | Status |
|---|---|
| Bionic / OpenSSL / BoringSSL / libsodium sources | clone + vendor |

## 7. Features & Functions Delivered (Comprehensive)

### Catalog crate `axiom-native-catalog`
- One folder per library: `libc/`, `openssl/`, `boringssl/`, `libsodium/`
- Per-function summary file with:
  - Name + arity + signature
  - Side effects (memory access, syscalls, errno)
  - Aliasing / pointer behavior
  - Cryptographic semantics (OpenSSL EVP_* etc.) — used by privacy-invariant scanner
  - Reference behavioral test
- Fingerprints of binaries: BLAKE3 hashes of canonical disassemblies
- Multi-version support: per-Android-version fingerprints (Bionic differs across A8…A15)

### NDK pattern recipes
- 30+ patterns documented: dlopen / dlsym, AAssetManager_*, ANativeWindow, EGL/GLES, MediaCodec, OpenSLES, AAudio, libusb, AHardwareBuffer, NetworkSecurityConfig native, JNI ART API, libjnigraphics, libcamera2ndk, libmediandk, etc.
- Each pattern: detection signature + summary IR + sample inputs

### Verification
- Each summary verified vs the lifter's body output on the corresponding upstream-library build
- Disagreements → either summary fix or lifter bug

### Tools
- `axiom-native-catalog-cli` — match a binary against catalog, emit replacements
- `axiom-native-catalog-add` — helper to add a new summary

### Per-version handling
- Catalog entries tagged with Android version range; lifter chooses the matching version

### Documentation
- `docs/native-catalog.md` — taxonomy + how to add a summary

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Bionic libc functions summarized | ≥ 200 | ≥ 300 |
| OpenSSL functions summarized | ≥ 80 | ≥ 150 |
| BoringSSL functions summarized | ≥ 80 | ≥ 150 |
| libsodium functions summarized | ≥ 40 | ≥ 80 |
| NDK pattern recipes documented | ≥ 30 | ≥ 60 |
| Summary-vs-lifter agreement rate | ≥ 95 % | ≥ 99 % |
| Catalog reproducibility (deterministic fingerprints) | 100 % | 100 % |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── crates/
│   └── axiom-native-catalog/
│       ├── src/
│       └── catalog/
│           ├── libc/
│           ├── openssl/
│           ├── boringssl/
│           ├── libsodium/
│           └── ndk-patterns/
├── tools/
│   ├── axiom-native-catalog-cli
│   └── axiom-native-catalog-add
└── docs/
    └── native-catalog.md             # NEW
```

## 10. Standalone Output

The catalog itself is a public artifact of independent value to any Android binary analyzer.

## 11. End-to-End Test

```bash
buck2 test //crates/axiom-native-catalog:...

buck2 run //tools:axiom-native-catalog-cli -- --match path/to/lib.so
# Expect: ≥ 90 % of recognized funcs replaced with summaries
```

## 12. Exit Checklist

- [ ] Bionic ≥ 200 funcs (HARD)
- [ ] OpenSSL ≥ 80 funcs (HARD)
- [ ] BoringSSL ≥ 80 funcs (HARD)
- [ ] libsodium ≥ 40 funcs (HARD)
- [ ] NDK patterns ≥ 30 (HARD)
- [ ] Summary-vs-lifter agreement ≥ 95 %
- [ ] Reproducibility 100 %
- [ ] Documentation `docs/native-catalog.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.6** | Catalog used to classify common JNI bridges |
| **P5.8** | Joint analyzer replaces summarized functions, doesn't re-derive |
| **L4 / L5** | OpenSSL summaries enable better intent / data-flow precision |
