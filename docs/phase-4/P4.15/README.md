# P4.15 — SDK: `axiom-ts` (Wasm + wit-bindgen)

> TypeScript SDK over `axiom-verify`'s Wasm build. ≥ 20 verifications/sec/core in V8. Distributed via npm.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §15 (SDK)](../../../README.md#sdk)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.15 |
| Owner(s) | G14 |
| Duration | Weeks 14–18 |
| Critical-path | yes |
| Hard prerequisites | P4.12 (Wasm build) |

## 2. Goal & Scope

A TypeScript SDK wrapping `axiom-verify`'s Wasm build. Idiomatic ESM + CommonJS package. Works in Node.js 20+, Deno, browsers (Chromium 122+, Firefox 124+, Safari 17+), and edge runtimes (Cloudflare Workers, Vercel Edge). ≥ 20 verifications/sec/core in V8 (Wasm CPU baseline; faster with WebGPU when available).

### In scope
- `sdk/axiom-ts` — TypeScript package
- wit-bindgen for Component Model
- Distribution: npm + Deno modules
- Type definitions (.d.ts)
- ESM + CJS dual exports
- Comprehensive test suite (Vitest)

### Out of scope
- Other SDKs

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P4.12** | Wasm build of axiom-verify |
| **P4.1** | wit-bindgen installed |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **TypeScript** | 5.5+ | Target |
| **Vitest** | latest | Test framework |
| **tsup** / **rollup** | latest | Bundling |
| **wit-bindgen** (for TypeScript) | latest | Component-Model TS bindings |
| **wasm-bindgen** (for Web target) | from P4.12 | Glue |
| **Node.js 20+** | latest | Runtime |
| **Deno** | latest | Alt runtime |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **npm** | package registry | **Free** | https://www.npmjs.com | Account required |
| **JSR** (Deno's registry) | package registry | **Free** | https://jsr.io | Account required (Deno's modern alternative to npm for TS) |
| **Vitest / TypeScript / tsup** | dev tools | **Free** OSS | crates.io / npm | |
| **Cloudflare Workers / Vercel Edge** | runtime | Free tier; **paid** | already discussed in P4.12 | |

**Account-level requirement:** npm + JSR publishing accounts.

## 6. System Inventory — Have vs Need

### Already present
- ✅ Node.js 20.20.2 (HAVE)
- ✅ npm 10.8.2 (HAVE)
- ✅ wit-bindgen (P4.1)

### Missing — must install
- ❌ **TypeScript 5.5+** — `npm i -g typescript`
- ❌ **Vitest** — npm dev dep
- ❌ **tsup** — npm dev dep
- ❌ **Deno** — `curl -fsSL https://deno.land/install.sh | sh`

```bash
npm i -g typescript@5.5 tsup
curl -fsSL https://deno.land/install.sh | sh
```

## 7. Features & Functions Delivered (Comprehensive)

### TypeScript API
```typescript
import { verify, Cert } from "@apkaxiom/axiom-verify";

const cert = await Cert.fromFile("report.axc");
const result = await verify(cert, apkBytes);
console.log(result.ok, result.claims, result.auditLog);
```

- `verify(cert: Cert, apkBytes?: Uint8Array): Promise<VerifyResult>`
- `Cert.fromFile(path: string): Promise<Cert>` (Node.js)
- `Cert.fromBytes(bytes: Uint8Array): Cert`
- `class VerifyResult { ok: boolean; claims: Claim[]; auditLog: AuditLog }`
- Streaming: `verifyStream(cert: ReadableStream): AsyncIterable<VerifyEvent>`

### Browser-friendly
- ESM-first
- Tree-shakeable
- Works as ES module from CDN
- Pre-bundled .wasm fetched lazily

### Edge-runtime support
- Cloudflare Workers tested
- Vercel Edge tested
- Deno deploy tested
- Node.js 20+ tested

### Type definitions
- `.d.ts` ships
- Strict TypeScript pass
- IDE autocomplete works

### Distribution
- npm: `@apkaxiom/axiom-verify`
- JSR: `@apkaxiom/axiom-verify` (Deno's registry)
- Sub-resource integrity hashes for CDN use

### Test suite
- Vitest
- ≥ 100 unit tests + browser-DOM tests + Node.js tests + Deno tests
- Cross-runtime parity verified

### Documentation
- `docs/sdk-typescript.md`
- npm README with usage examples
- TypeDoc auto-generated

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Single-core verifications/sec in V8 (Node.js) | ≥ 20 | ≥ 80 |
| Wasm load + first verify (cold) in Chromium 122+ | ≤ 500 ms | ≤ 200 ms |
| Bundle size delivered | ≤ 5 MB compressed | ≤ 2 MB |
| Cross-runtime parity (Node + Deno + Chromium + Firefox + Safari) | 100 % verdicts identical | 100 % |
| TypeScript strict pass | yes | yes |
| npm + JSR publishing live | yes | yes |
| Reproducible bundles | bit-identical | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── sdk/
│   └── axiom-ts/
│       ├── package.json
│       ├── tsconfig.json
│       ├── BUCK
│       ├── src/
│       │   ├── index.ts
│       │   ├── verify.ts
│       │   ├── cert.ts
│       │   └── wasm-loader.ts
│       ├── tests/
│       │   ├── unit.test.ts
│       │   ├── browser.test.ts
│       │   ├── deno.test.ts
│       │   └── node.test.ts
│       └── dist/                          # bundled output
└── docs/
    └── sdk-typescript.md
```

## 10. Standalone Output

```bash
cd sdk/axiom-ts && npm install && npm run build
node -e "import('./dist/index.mjs').then(({verify, Cert}) => { Cert.fromFile('sample.axc').then(c => verify(c).then(r => console.log(r.ok))) })"
# true
```

## 11. End-to-End Test

```bash
buck2 test //sdk/axiom-ts:full
# - V8 throughput ≥ 20/sec/core (HARD)
# - Cross-runtime parity 100% (HARD)
# - Bundle ≤ 5 MB (HARD)
# - TypeScript strict pass (HARD)
```

## 12. Exit Checklist

- [ ] `axiom-ts` package compiles
- [ ] Wasm build embedded + lazy-loaded
- [ ] Throughput ≥ 20/sec/core in V8 (HARD)
- [ ] Bundle size ≤ 5 MB compressed (HARD)
- [ ] Cross-runtime parity (Node + Deno + 3 browsers) 100 % (HARD)
- [ ] TypeScript strict pass (HARD)
- [ ] npm + JSR publishing live (HARD)
- [ ] ≥ 100 unit + browser + Node + Deno tests
- [ ] Reproducible bundles (HARD)
- [ ] `docs/sdk-typescript.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P4.17** | Bug-bounty pilot may use TS SDK in front-end |
| **P4.18** | E2E measures axiom-ts throughput |
| **External JS/TS users** | First TypeScript verifier for proof-carrying APKs |
