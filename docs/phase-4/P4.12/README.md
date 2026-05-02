# P4.12 — `axiom-verify` Wasm + ARM64 Mobile Builds

> Verify in the browser, in a service worker, on a Pixel. Wasm p99 ≤ 300 ms. ARM64 mobile p99 ≤ 200 ms. Same cert, same verdict.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §11](../../../README.md#layer-6)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.12 |
| Owner(s) | G14 |
| Duration | Weeks 12–18 |
| Critical-path | yes |
| Hard prerequisites | P4.11 (verifier core) |

## 2. Goal & Scope

Two additional builds of `axiom-verify`:
1. **Wasm** for browsers + Node.js + Cloudflare Workers
2. **ARM64 mobile** for native Android / iOS deployment

Cross-platform verdict equivalence — same cert produces byte-identical verdict across all builds.

### In scope
- `tools/axiom-verify-wasm` — wasm-bindgen + wit-bindgen build
- `tools/axiom-verify-mobile` — ARM64 mobile-friendly build
- Wasm size optimization (target ≤ 5 MB compressed)
- Mobile cold-start ≤ 1 s
- Cross-platform parity test
- WebGPU acceleration for in-browser zk verify (where supported)

### Out of scope
- iOS-Swift wrapper (uniffi handles partly; Swift binding is a Phase-5 nicety)
- Native ARM64 ZK proving (only verify side here)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P4.11** | Verifier core |
| **P4.3** | zk pool — but only the verify path |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **wasm-bindgen** | latest | Rust → Wasm |
| **wit-bindgen** | latest | Component Model |
| **wasm-pack** | latest | Build pipeline |
| **wasmtime** | latest | Wasm runtime (CLI test) |
| **WebGPU / WGSL** | preview-stable | In-browser GPU acceleration |
| **icicle wasm port** | research | GPU verify in-browser |
| **Android NDK** | 26+ | ARM64 mobile build |
| **iOS toolchain** | latest | iOS / iPadOS build (optional) |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **wasm-bindgen / wit-bindgen / wasm-pack** | crates | **Free** OSS | crates.io | |
| **wasmtime** | runtime | **Free** OSS | https://wasmtime.dev | |
| **Cloudflare Workers** *(deployment target)* | runtime | Free tier 100K req/day; **paid** $5/mo | https://workers.cloudflare.com | Useful for app-store ingest at edge |
| **Firebase / Vercel / Netlify** *(hosting)* | service | Free tier; **paid** | various | For browser demo |
| **Apple Developer Program** *(if iOS)* | service | **Paid** $99/yr | https://developer.apple.com | iOS distribution |
| **Google Play Console** *(if Android distribution)* | service | **Paid** $25 one-time | https://play.google.com/console | Android distribution; not strictly needed for verifier |

**API keys (potential):** Cloudflare Workers API token if deploying as edge function. Free tier sufficient for Phase-4 pilot.

## 6. System Inventory — Have vs Need

### Already present
- ✅ Rust + Cargo + Buck2

### Missing — must install
- ❌ **wasm-pack** — `cargo install wasm-pack`
- ❌ **wasmtime** — `curl https://wasmtime.dev/install.sh -sSf | bash`
- ❌ **Android NDK** — Android SDK has it; install if not
- ❌ **WebGPU runtime test infrastructure** — Chromium 122+ headless

```bash
cargo install wasm-pack wit-bindgen-cli
curl https://wasmtime.dev/install.sh -sSf | bash

# Android NDK (via cmdline-tools sdkmanager)
sdkmanager "ndk;26.3.11579264"
```

## 7. Features & Functions Delivered (Comprehensive)

### Wasm build
- `axiom-verify.wasm` produced via wasm-pack
- Component-Model variant via wit-bindgen for richer host-language interop
- Bundle size ≤ 5 MB compressed (HARD)
- Cold-start in browser ≤ 500 ms

### Wasm GPU acceleration (where supported)
- WebGPU / WGSL kernels for zk-verify hot paths
- Falls back to Wasm CPU when WebGPU unavailable
- 3–10× speedup on supported browsers

### ARM64 mobile build
- Cross-compiled with NDK
- Static binary; no Java dependencies
- Cold-start ≤ 1 s on Pixel 8
- Verify p99 ≤ 200 ms (HARD)

### Cross-platform parity test
- Same `.axc` file → same verdict on:
  - x86_64 Linux native
  - x86_64 macOS native
  - x86_64 Windows native (best-effort)
  - ARM64 Linux native
  - ARM64 macOS (Apple Silicon) native
  - Wasm in Chromium 122+
  - Wasm in Firefox 124+
  - Wasm in Safari 17+
  - Wasm in Node.js 20+
  - ARM64 mobile (Android)
- 10K-cert benchmark on each
- Verdicts byte-identical (HARD)

### Distribution
- Wasm published to npm (`@apkaxiom/axiom-verify`)
- Mobile binary published to GitHub Releases
- Reproducible builds (BLAKE3 hashes pinned)

### Documentation
- `docs/axiom-verify-wasm.md` — browser usage, integration patterns
- `docs/axiom-verify-mobile.md` — Android / iOS deployment

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Wasm bundle size compressed | ≤ 5 MB | ≤ 2 MB |
| Wasm p99 latency in Chromium 122+ | ≤ 300 ms | ≤ 120 ms |
| Wasm cold-start in browser | ≤ 500 ms | ≤ 200 ms |
| ARM64 mobile p99 on Pixel 8 | ≤ 200 ms | ≤ 80 ms |
| ARM64 mobile cold-start | ≤ 1 s | ≤ 300 ms |
| Cross-platform verdict parity | 100 % byte-identical | 100 % |
| WebGPU acceleration speedup (when available) | ≥ 3× | ≥ 10× |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── tools/
│   ├── axiom-verify-wasm/
│   │   ├── Cargo.toml
│   │   ├── BUCK
│   │   ├── webgpu/                       # WGSL kernels
│   │   └── src/lib.rs
│   └── axiom-verify-mobile/
│       ├── Cargo.toml
│       ├── BUCK
│       └── src/main.rs
├── tests/
│   └── cross-platform-parity/             # 10K-cert eval per platform
└── docs/
    ├── axiom-verify-wasm.md
    └── axiom-verify-mobile.md
```

## 10. Standalone Output

```bash
buck2 build //tools/axiom-verify-wasm --release
ls -lh tools/axiom-verify-wasm/dist/axiom-verify.wasm.gz
# 1.8M
buck2 build //tools/axiom-verify-mobile --release --target=aarch64-linux-android
adb push axiom-verify-mobile /data/local/tmp/
adb shell /data/local/tmp/axiom-verify-mobile report.axc
```

## 11. End-to-End Test

```bash
buck2 test //tests/cross-platform-parity:10k-cert-bench
# - Wasm p99 ≤ 300 ms (HARD)
# - Mobile p99 ≤ 200 ms (HARD)
# - Verdict parity 100% (HARD)
# - Bundle size ≤ 5 MB (HARD)
```

## 12. Exit Checklist

- [ ] Wasm build via wasm-pack + wit-bindgen
- [ ] Bundle size compressed ≤ 5 MB (HARD)
- [ ] Wasm p99 in Chromium 122+ ≤ 300 ms (HARD)
- [ ] ARM64 mobile build via NDK
- [ ] Mobile p99 on Pixel 8 ≤ 200 ms (HARD)
- [ ] Cross-platform parity 100 % byte-identical verdicts (HARD)
- [ ] WebGPU optional acceleration (where supported)
- [ ] npm + GitHub Releases distribution
- [ ] `docs/axiom-verify-wasm.md` and `docs/axiom-verify-mobile.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P4.15** | axiom-ts SDK uses the Wasm build |
| **P4.17** | Bug-bounty pilot may run verifier in-browser |
| **External users** | First production verifier in browsers + on mobile |
