# APKAXIOM

> **A proof-stack analysis platform for Android packages.**
> Every finding is backed by a machine-checkable certificate. Not heuristics. Not similarity scores. Proofs.

[![Status](https://img.shields.io/badge/status-private--research-red)]()
[![Stage](https://img.shields.io/badge/stage-architecture--draft-blue)]()
[![Target](https://img.shields.io/badge/target-USENIX%20%7C%20S%26P%20%7C%20NDSS-purple)]()

## Documents in this repository

| Document | Purpose |
|---|---|
| [README.md](./README.md) (this file) | Architecture, the 7-layer proof stack, the 14 engineering groups, comparison matrix, caveats |
| [ROADMAP.md](./ROADMAP.md) | 3-year executable plan to v1.0 — phases, hiring sequence, deliverables per group, risk register |
| [PHASE_GATES.md](./PHASE_GATES.md) | Comprehensive performance / scalability / real-time KPI checklist per phase. Hard gates that block phase advancement |

---

## TL;DR

APKAXIOM is the first APK analysis system designed end-to-end around **provable correctness** rather than heuristic approximation. Where current tools (Androguard, apkInspector, Apktool, JADX, MobSF, Quark) ask *"does this look like malware?"*, APKAXIOM asks *"can I produce a checkable proof that this APK has property P on Android version V?"*

The result is a system whose findings are **cryptographically certifiable** — every reported bug ships with a machine-verifiable certificate that an independent verifier can check in milliseconds. That eliminates the false-positive problem at the architectural level, not at the tuning-threshold level.

This is what we mean by changing bug-bounty: **a finding is either accompanied by a proof certificate, or it isn't a finding.**

---

## Table of Contents

1. [The Problem with Every Existing APK Tool](#the-problem)
2. [Design Principles](#design-principles)
3. [The Proof Stack: Seven Layers](#the-proof-stack)
4. [Architecture Diagram](#architecture-diagram)
5. [Layer 0 — Streaming ZIP Spine](#layer-0)
6. [Layer 1 — Version-Stratified Verified Parsers](#layer-1)
7. [Layer 2 — Bundle / Split-APK Resolver](#layer-2)
8. [Layer 3 — Structural Forensics](#layer-3)
9. [Layer 4 — Symbolic Manifest & Intent Resolver](#layer-4)
10. [Layer 5 — Behavior Surface Hash & Bisimulation](#layer-5)
11. [Layer 6 — Proof-Carrying Certificates](#layer-6)
12. [Continuous — Differential Fuzzing Plant](#continuous)
13. [Beyond the 12: Additional State-of-the-Art Components](#beyond-the-12)
14. [Why Zero False Positives — The Proof Chain](#why-zero-fp)
15. [Data Flow Diagram](#data-flow)
16. [Comparison Matrix](#comparison)
17. [Research Roadmap & Publication Targets](#roadmap)
18. [State-of-the-Art Techniques Catalogue](#sota-catalogue)
19. [Honest Caveats](#caveats)
20. [Contributing](#contributing)
21. [Major Feature Areas & Engineering Team Structure](#team-structure)
22. [apk-info as the Engineering Beachhead](#apkinfo-integration)

---

<a id="the-problem"></a>
## 1. The Problem with Every Existing APK Tool

Every APK analyzer in production today (open-source or commercial) is structurally a **heuristic stack**:

```
[Bytes]  →  [Approximate parser]  →  [Approximate model]  →  [Score]
```

At each arrow, information is lost or guessed. False positives are the *necessary consequence* of this architecture, because nothing in the chain is sound. You can tune the score threshold, but you can't make the underlying reasoning correct.

Concrete failures of the heuristic stack in 2025–2026:

| Failure mode | Real-world example | Tool affected |
|---|---|---|
| Parser confusion | BadPack family — APK installs on Android, crashes Androguard | All static analyzers |
| Bundle-era blindness | Malware in dynamic-feature module never seen by analyzer | Every tool except Play Protect |
| Repackaging false-negatives | Same malware family, layout obfuscated → fuzzy hash misses | ssdeep, TLSH, Dexofuzzy |
| Intent hijack false-positives | "App could hijack X" reports that depend on impossible app sets | FlowDroid, IC3, COVERT |
| Version-blind analysis | Analyzed as Android 11; targets Android 14 | Every tool |
| Native code dark-matter | `.so` libraries entirely ignored by static analysis | Every Java-only tool |

APKAXIOM rejects the heuristic stack. It replaces it with a **proof stack**:

```
[Bytes]  →  [Verified parser]  →  [Formal model]  →  [Certificate]
```

Every arrow is sound. Every output carries an attached proof.

---

<a id="design-principles"></a>
## 2. Design Principles

These are non-negotiable and shape every component below.

1. **Proof or it didn't happen.** Every output of every layer is either a typed artifact with a verifiable provenance, or it is rejected.
2. **No global approximations.** Where over-approximation is unavoidable (e.g., infinite state spaces), it is *locally explicit* and propagated as a typed `Approx<T>` carrying its abstraction domain.
3. **Versioned everything.** Android API level, AOSP source hash, parser version, and proof system version are part of every output.
4. **Cryptographic provenance throughout.** Inputs, intermediate artifacts, and outputs are content-addressed. Re-running on the same APK yields bit-identical certificates.
5. **Open IR.** A single, formally-specified intermediate representation (AXIOM-IR) is the universal currency between layers. Other tools should be able to consume it.
6. **Adversary-aware.** The system is designed assuming the input was crafted by a state-level adversary actively trying to confuse the analyzer.
7. **Reproducibility is mandatory.** Every certificate is reproducible bit-for-bit on any conforming verifier. No reproducibility → no merge.

---

<a id="the-proof-stack"></a>
## 3. The Proof Stack: Seven Layers

```
┌───────────────────────────────────────────────────────────────────────┐
│                       APKAXIOM PROOF STACK                            │
│                                                                       │
│   Layer 6  │  Proof-Carrying Certificates (zk-SNARK / STARK)          │
│            │  ──────────────────────────────────────────              │
│   Layer 5  │  Behavior Surface Hash + Bounded Bisimulation            │
│            │  ──────────────────────────────────────────              │
│   Layer 4  │  Symbolic Manifest & Intent Resolver (SMT-backed)        │
│            │  ──────────────────────────────────────────              │
│   Layer 3  │  Structural Forensics: Shadow Stack · AXML Provenance ·  │
│            │                       Negative-Space Statistics          │
│            │  ──────────────────────────────────────────              │
│   Layer 2  │  Bundle / Split-APK Resolver (Schrödinger APK)           │
│            │  ──────────────────────────────────────────              │
│   Layer 1  │  Version-Stratified Verified Parsers (Android 8–15)      │
│            │  Mechanized in Lean 4. Extracted to Rust.                │
│            │  ──────────────────────────────────────────              │
│   Layer 0  │  Streaming ZIP Spine + Merkle Commitment Tree            │
│                                                                       │
│   Cross-cutting:                                                      │
│   • Differential Fuzzing Plant (continuous oracle)                    │
│   • AXIOM-IR (typed intermediate representation)                      │
│   • Native Code Subsystem (DEX + ARM64 ELF lifters)                   │
│   • Dynamic Confirmation Bridge (Frida / eBPF)                        │
└───────────────────────────────────────────────────────────────────────┘
```

Each layer **strictly refines** the output of the layer below. A layer cannot claim more certainty than its input provides. This is enforced by the type system.

---

<a id="architecture-diagram"></a>
## 4. Architecture Diagram

```
                              ┌──────────────┐
                              │     APK      │
                              │  (or AAB,    │
                              │   bundle,    │
                              │   split set) │
                              └──────┬───────┘
                                     │
                                     ▼
        ┌──────────────────────────────────────────────────────┐
        │ L0: Streaming ZIP Spine                              │
        │  - Reads bytes incrementally                         │
        │  - Emits Merkle commitment per chunk                 │
        │  - Outputs: CommitChain<ZipEntry>                    │
        └────────────────────────┬─────────────────────────────┘
                                 │  CommitChain
                                 ▼
        ┌──────────────────────────────────────────────────────┐
        │ L1: Version-Stratified Verified Parser Bank          │
        │  ┌────────┬────────┬────────┬────────┬────────┐      │
        │  │ A8     │ A11    │ A12    │ A13    │ A14    │ ...  │
        │  │ Lean→Rs│ Lean→Rs│ Lean→Rs│ Lean→Rs│ Lean→Rs│      │
        │  └────┬───┴────┬───┴────┬───┴────┬───┴────┬───┘      │
        │       │        │        │        │        │           │
        │  Outputs: VersionedParse<Android v> with Lean proof  │
        └────────────────────────┬─────────────────────────────┘
                                 │  Vec<VersionedParse>
                                 ▼
        ┌──────────────────────────────────────────────────────┐
        │ L2: Bundle Resolver (Schrödinger APK)                │
        │  - Composes base + split + dynamic-feature           │
        │  - Resolves ABI / density / language splits          │
        │  - Outputs: BehaviorSet (full configuration space)   │
        └────────────────────────┬─────────────────────────────┘
                                 │  BehaviorSet
                                 ▼
        ┌──────────────────────────────────────────────────────┐
        │ L3: Structural Forensics                             │
        │  ┌──────────────┐ ┌──────────────┐ ┌─────────────┐   │
        │  │ Shadow Stack │ │ AXML Compiler│ │ Negative-   │   │
        │  │  (deletion   │ │  Fingerprint │ │   Space     │   │
        │  │  detection)  │ │              │ │  Anomaly    │   │
        │  └──────┬───────┘ └──────┬───────┘ └──────┬──────┘   │
        │         └─────────┬──────┴────────────────┘           │
        │           Forensic Findings (typed, located)          │
        └────────────────────────┬─────────────────────────────┘
                                 │
                                 ▼
        ┌──────────────────────────────────────────────────────┐
        │ L4: Symbolic Manifest & Intent Resolver              │
        │  - Lifts manifest into AXIOM-IR                      │
        │  - Z3 / cvc5 backed                                  │
        │  - Models PackageManager state symbolically          │
        │  - Outputs: ReachabilityProofs over IntentSpace      │
        └────────────────────────┬─────────────────────────────┘
                                 │
                                 ▼
        ┌──────────────────────────────────────────────────────┐
        │ L5: Equivalence & Fingerprinting                     │
        │  ┌─────────────────┐    ┌────────────────────────┐   │
        │  │ Behavior Surface│ →  │ Bounded Bisimulation   │   │
        │  │ Hash (BSH-256)  │    │ (k-step, abstract dom.)│   │
        │  │ obfuscation-inv.│    │ proves equivalence     │   │
        │  └─────────────────┘    └────────────────────────┘   │
        │  Outputs: FingerprintCertificate, EquivalenceProof   │
        └────────────────────────┬─────────────────────────────┘
                                 │
                                 ▼
        ┌──────────────────────────────────────────────────────┐
        │ L6: Proof-Carrying Certificate Emitter               │
        │  - Compiles all layer-outputs into a single proof    │
        │  - zk-SNARK over claimed properties (Halo2/Plonk)    │
        │  - Signed, version-pinned, timestamped               │
        │  - Output: APKAXIOM Certificate (.axc file)          │
        └────────────────────────┬─────────────────────────────┘
                                 │
                                 ▼
                     ┌────────────────────────┐
                     │  axc certificate file  │
                     │  + human-readable      │
                     │    finding bundle      │
                     └────────────────────────┘

       ╔══════════════════════════════════════════════════════╗
       ║  Continuous side-channel: Differential Fuzzer        ║
       ║  ───────────────────────────────────────────         ║
       ║  Generates structurally-valid edge-case APKs.        ║
       ║  Runs through Layer-1 parser bank in parallel.       ║
       ║  Disagreement → either AOSP CVE or proof-model bug.  ║
       ║  Either way: logged, classified, fed back.           ║
       ╚══════════════════════════════════════════════════════╝
```

---

<a id="layer-0"></a>
## 5. Layer 0 — Streaming ZIP Spine

**Purpose.** Parse the ZIP container incrementally and emit a Merkle commitment chain so downstream layers can reference any byte range cryptographically.

**Why incremental matters.** APKs are gigabytes in the bundle era. Wire-level inspection (proxies, app-store ingest, IDS) cannot afford "load entire file then start." Layer 0 emits structured commitments after the first kilobytes.

**Innovations beyond existing tools.**
- **Out-of-order ZIP handling.** ZIP's central directory is at the *end*, but we use a forward-streaming local-header parser plus a deferred reconciliation pass. No existing APK tool does this correctly.
- **Merkle Patricia Trie over entry layout.** Each entry's `(offset, size, crc32, name)` tuple goes into a trie whose root is the commitment. Tampering with any byte invalidates the root deterministically.
- **Anti-collision-attack hardening.** Uses BLAKE3 with personalization strings per chunk type to defeat ZIP-collision attacks (cf. Shattered, but for ZIP semantics).

**Output.** `CommitChain<ZipEntry>` — a chained sequence of typed commitments, each provably bound to a byte range of the input.

---

<a id="layer-1"></a>
## 6. Layer 1 — Version-Stratified Verified Parsers

**Purpose.** Parse the APK *as Android itself would parse it*, **separately per Android version**.

This is the load-bearing innovation of the system.

### Why per-version

Android's `libziparchive` and `PackageParser` differ across versions in subtle, exploitable ways. An APK that targets Android 14 may be malformed on Android 11 — current tools pick one version (or none) and report a single answer. Attackers exploit this as evasion.

### How

For each supported Android API level (8 through 15+), we mechanize the relevant parsing logic in **Lean 4** as a dependently-typed function:

```lean
def parseApk (v : AndroidVersion) (bytes : ByteArray) :
  Except ParseError (ParsedApk v) := ...

theorem parseApk_sound (v : AndroidVersion) (bs : ByteArray) :
  parseApk v bs = .ok p →
  AndroidVM.installable v bs ↔ AndroidVM.installAs v bs = some p
```

The Lean source is then **extracted to Rust** via a custom extraction pipeline (analogous to CompCert's OCaml extraction). The Rust executable carries a hash of the Lean proof object; tampering with the Rust without re-running extraction is detectable.

### Why this is hard but tractable

The full PackageParser is 15,000+ lines of Java. We don't formalize all of it. We formalize the **trust core**: ZIP layout, APK signing block parsing, manifest binary-XML decoding, and the resource ID resolution path. That's ~3,000 lines of equivalent Lean — large but bounded.

### Innovations

- **Per-version extraction pipeline.** Each Lean module compiles to a separate Rust crate with version pinning.
- **AOSP archaeology automation.** A custom tool (`aosp-diff`) tracks libziparchive commits and flags semantically-relevant changes for re-formalization.
- **Soundness-preserving optimizations.** Performance optimizations in the Rust output are verified by translation validation — every optimization preserves the Lean theorem.

---

<a id="layer-2"></a>
## 7. Layer 2 — Bundle / Split-APK Resolver (Schrödinger APK)

**Purpose.** Resolve the *full configuration space* of a modern Android App Bundle into a single `BehaviorSet` — the set of all programs the device might actually run.

### The problem nobody else solves

In 2026, ~80% of Play Store apps are App Bundles. A device installs:
- The base APK
- One ABI split (arm64-v8a, armeabi-v7a, x86_64)
- One density split (mdpi, hdpi, xxhdpi, ...)
- One language split per installed locale
- Zero to N dynamic feature modules (installed at runtime)
- Zero to N asset packs

Every static analysis tool implicitly assumes "APK = single file." This is wrong for the dominant deployment model. Malware in a dynamic feature module is **invisible** to current tools.

### How we solve it

A formal model of the bundle composition operator `⊕`:

```
BehaviorSet = ⋃ over (abi, density, lang, modules ∈ feasible_modules):
                base ⊕ split[abi] ⊕ split[density] ⊕ split[lang] ⊕ ⊕(modules)
```

We compute the **union of behaviors** across all feasible configurations, with each contribution tagged by the configuration that produces it. Downstream layers analyze the BehaviorSet, not a single APK.

### Innovations

- **Formal bundle composition semantics.** Published as part of the system spec (paper target: NDSS).
- **Configuration-tagged findings.** Every downstream finding states "occurs in configuration C with API ≥ V."
- **On-demand module enumeration.** Even modules not bundled with the install are fetched from the developer's distribution endpoint and analyzed (with consent gating).

---

<a id="layer-3"></a>
## 8. Layer 3 — Structural Forensics

Three independent passes operate on the BehaviorSet. None require a malware corpus or ML model — all are pure structural analysis.

### 8.1 Shadow Stack — Forensic Deletion Detection

Treats the APK as a forensic artifact. When an attacker repackages a benign app, they leave traces:
- Gaps in ZIP entry offsets
- Orphaned string-pool references
- Unreferenced resource IDs in non-contiguous ranges
- Dangling DEX type indices
- Stale timestamps inconsistent with ZIP central directory order

Each anomaly is a typed finding with a probability bound derived from the structural priors. The output is: *"the APK was probably originally signed by X; class `com.malicious.Loader` was injected at offset Y; resource `R.id.0x7f0099aa` is orphaned."*

### 8.2 AXML Compiler Provenance Fingerprint

`aapt`, `aapt2`, `apktool`, `axmlpp`, and several Chinese toolchains each produce subtly different binary AXML — string pool ordering, attribute sort order, chunk padding. We build a reference corpus by compiling identical manifests with each known toolchain, then learn structural signatures.

A manifest's `META-INF` may claim Android Studio + aapt2. If the AXML structure says apktool, we have **proof of repackaging from a single sample**, no original needed.

### 8.3 Negative-Space Resource Anomaly

Statistical analysis of the resource table treated as a distributional object. An English-only app with one Russian string, a benign UI app with a single resource ID floating in empty space, a single drawable in `drawable-anydpi` while everything else is `drawable-mdpi` — these are detectable as outliers without prior malware knowledge.

The novelty: we treat resource tables the way steganalysis treats images. There is no academic literature on this and no production tool that does it.

---

<a id="layer-4"></a>
## 9. Layer 4 — Symbolic Manifest & Intent Resolver

**Purpose.** Given the BehaviorSet, compute *exactly* which intents resolve to which components, under which device states, against which other installed apps.

### Why current tools fail

Intent hijacking has 12+ years of literature (IC3, COVERT, Epicc, IntentScope, ...). Every tool over-approximates and produces false positives at the rate of 30–80%. The reason is universal: they cannot model the full state of `PackageManager`.

### Our approach

We lift the manifest, intent filters, signatures, and exported components into **AXIOM-IR**. We model `PackageManager` symbolically, with state variables for:
- Installed package set
- Per-package signature
- Per-component enabled/disabled state
- User profile state
- Default-app preferences
- Per-intent priority

We hand it to **cvc5** and ask: "Does there exist a feasible device state such that intent `I` resolves to component `C`?"

The output is either:
- A reachability proof (concrete device state + install order witnessing the resolution), or
- An unreachability proof (UNSAT certificate), or
- An explicit "abstraction limit reached" flag — never a silent false positive.

### Innovations

- **First sound-and-complete intent resolver** for a useful fragment of Android's intent system.
- **CHC-based modeling.** Constrained Horn Clauses encode the recursive resolution algorithm; off-the-shelf CHC solvers (Spacer, Eldarica) finish the job.
- **Composable across installed app sets.** Given a *device snapshot* (set of installed APKs), reasons about cross-app vulnerability.

---

<a id="layer-5"></a>
## 10. Layer 5 — Behavior Surface Hash & Bounded Bisimulation

Two complementary primitives.

### 10.1 Behavior Surface Hash (BSH-256)

A canonical 256-bit hash computed from the **behavior surface** of the BehaviorSet:
- Sorted permission set
- Sorted intent filter set
- Sorted exported component set
- Canonicalized dangerous-API call set extracted from DEX
- Network destination set extracted from manifest + string pool

Two repackaged versions of the same malware get the same BSH-256 even after total layout obfuscation, because none of the inputs depend on byte layout.

We pair it with **locality-sensitive hashing** (MinHash + LSH) over the API call multi-set, enabling sub-linear similarity search across millions of APKs.

The novelty isn't fuzzy hashing — it's the **standardization** of a canonical behavior surface. When other tools cite `apkaxiom-bsh:abc123...`, we've defined a lingua franca.

### 10.2 Bounded Bisimulation Equivalence

For when "similar" isn't enough and you need **proof of equivalence**.

Borrowing from process calculus: two APKs are *behaviorally equivalent up to k steps* if their inter-component communication graphs and API call traces are bisimilar within k transitions, modulo a quotient by API renaming (handles obfuscated method names).

Computed via:
- Abstract domain construction (numeric, string, type)
- Refinement-type-style relation between abstract states
- SMT-discharged proof obligations at each transition

Output: an `EquivalenceProof` artifact, or an explicit divergence witness.

This is the first principled answer to "is this repackaged malware?" — others say *probably*; we say *here is the proof*.

---

<a id="layer-6"></a>
## 11. Layer 6 — Proof-Carrying Certificates

The final output of an APKAXIOM run is a **signed certificate file** (`.axc`) carrying machine-checkable proofs of every claimed property.

### Format

```
APKAXIOM Certificate v1
─────────────────────────
input_digest:        blake3:7a8f...
android_versions:    [8, 9, 10, 11, 12, 13, 14, 15]
parser_extraction:   lean4:0.4.0/aosp:android-15.0.0_r12
analysis_timestamp:  2026-05-02T14:33:17Z
signing_key:         ed25519:apkaxiom-instance-prod-7

claims:
  - kind: parser_consistency
    proof: lean4-proof-blob-base64...
    statement: ∀ v ∈ versions. parseApk v input = ok p_v
  - kind: intent_unreachability
    proof: cvc5-unsat-cert-base64...
    statement: ¬∃ device_state. resolves(state, intent="android.intent.action.SEND",
                                          component="com.victim.WhatsApp.ShareActivity")
  - kind: behavior_equivalence
    proof: bisim-witness-base64...
    statement: BSH(input) ≡ BSH(known_malware:Cerberus.v3.4)
  - kind: privacy_invariant
    proof: zk-snark-halo2-proof-base64...
    statement: ∀ exec_path. ¬touches(READ_CONTACTS) ∨ touches(NETWORK)

signature: ed25519:...
```

### Why this changes bug bounty

Every existing bug-bounty submission is, fundamentally, a story. The hunter says *"here is what I think happens."* The triager has to reproduce, debate, and judge.

An APKAXIOM submission is **not a story**. It's a certificate. The triager runs `axiom-verify report.axc` and gets either ✅ or ❌ in milliseconds. The proof is independent of the analyzer that produced it.

This is what "100% confirmed" actually means — not a marketing claim, but a verifier program that returns ✅ deterministically. (See [§19 Honest Caveats](#caveats) for what this does and does not guarantee.)

### Proof systems used

- **Lean 4** for parser theorems (extraction-based)
- **cvc5 / Z3** for symbolic-execution UNSAT certificates (DRAT-style)
- **Halo2 / Plonk** zk-SNARKs for privacy properties (because "this APK never reads contacts" is a universal claim that requires sound abstraction)
- **STARK** (FRI-based) as a post-quantum alternative when needed

---

<a id="continuous"></a>
## 12. Continuous — Differential Fuzzing Plant

Runs continuously, off the critical path of analysis.

### What it does

1. A grammar-aware fuzzer (custom `apk-grammar.lark`) generates structurally-valid edge-case APKs.
2. Each candidate is fed through every Layer 1 parser variant.
3. If two parsers (or any parser vs. real AOSP) disagree on installability or on any manifest field — the input is logged.

### Why this is unique

This is **the Android equivalent of OSS-Fuzz** combined with **differential testing** (cf. *Csmith*, *YarpGen*, *EMI*). Currently nothing like this exists for Android's installation pipeline.

Every disagreement is one of:
- A CVE in AOSP (we file).
- A bug in our formal model (we fix and re-extract).
- A genuine ambiguity in the spec (we publish).

The plant is the **continuous oracle** that keeps the proof stack honest. Without it, our formal models would silently drift from AOSP. With it, drift is detected automatically.

### Engineering

- 8+ AOSP versions cross-compiled into uniform libFuzzer harnesses (the unglamorous 60% of the work).
- Coverage-guided + grammar-aware (best of AFL++ and structure-aware fuzzing).
- Findings classified automatically into the three categories above using cluster analysis.

---

<a id="beyond-the-12"></a>
## 13. Beyond the 12: Additional State-of-the-Art Components

You asked us to dig further. These were not in the original 12 ideas. They are additions worth integrating.

### 13.1 Native Code Subsystem (DEX + ARM64 ELF lifters)

Most APK tools ignore native code (`lib/*.so`). We don't. A native subsystem lifts:
- DEX bytecode → AXIOM-IR (typed SSA)
- ARM64/ARMv7 ELF → AXIOM-IR via a custom lifter on top of LLVM MLIR

Once unified in AXIOM-IR, all upper layers (intent resolution, behavior surface, equivalence) operate on Java-and-native code uniformly. This is novel — no current APK tool reasons jointly over Java and native.

### 13.2 Dynamic Confirmation Bridge

For findings where the static layers report "abstraction limit reached" (Layer 4 returns inconclusive), we drop into a **Frida + eBPF dynamic confirmation harness**. The APK runs in a sandboxed Android emulator; the dynamic trace is ingested as evidence and refines the static abstraction.

This is concolic execution adapted to the Android domain — dynamic where static gives up, static everywhere else.

### 13.3 ML Model Integrity

Modern APKs ship TensorFlow Lite models. APKAXIOM verifies model integrity via a **structural model hash** (independent of weight quantization noise) and detects backdoor patterns using techniques from *Neural Cleanse* and *STRIP*.

### 13.4 Privacy Invariants as zk-SNARK statements

For statements like *"this APK provably does not transmit IMEI to any non-allow-listed network destination"*, we compile the property to a Halo2 circuit and prove it once at certificate-emission time. App stores can verify in milliseconds — no re-analysis needed.

### 13.5 Cross-APK Vulnerability Discovery

Given a *device snapshot* (set of installed APKs as a single input), Layer 4's symbolic resolver is invoked across the snapshot to find cross-app intent confusions, content-provider exposures, and permission-aggregation attacks.

### 13.6 Time-Travel Analysis

Given an APK and an Android version *not yet released*, predict its behavior using the current AOSP master branch. This is forward-looking: vulnerabilities introduced by future Android changes become detectable before the changes ship.

### 13.7 Supply-Chain Attestation (SLSA L4)

Verifies the APK against its claimed build provenance using SLSA Level 4 attestations. Combined with reproducible-build verification: given the source tree and the APK, prove or disprove they correspond.

### 13.8 Adversarial Robustness Scoring

For APKs with embedded ML, run adversarial-attack frameworks (cleverhans, foolbox, custom variants) and attach a robustness score to the certificate. Important for safety-critical apps.

### 13.9 AXIOM-IR

The **single typed intermediate representation** linking every layer. Designed in the MLIR tradition — multiple dialects (manifest, DEX, native, resource), a single typesystem, mechanical lowerings. AXIOM-IR is the artifact other researchers will build on for a decade.

---

<a id="why-zero-fp"></a>
## 14. Why Zero False Positives — The Proof Chain

The argument, made precisely.

A false positive in current tools comes from one of these sources:

| Source | Killed in APKAXIOM by |
|---|---|
| Heuristic parsing approximation | Layer 1: Lean-verified parsers |
| Drift between analyzer and actual Android | Continuous: differential fuzzer detects drift |
| Bundle-era blindness | Layer 2: behavior set covers full configuration space |
| Layout-dependent fingerprint breaks under obfuscation | Layer 5: behavior-surface hash is layout-independent |
| Over-approximated intent resolution | Layer 4: SMT-backed reachability proofs or explicit UNKNOWN |
| Similarity confused with equivalence | Layer 5.2: bisimulation produces proofs |
| Wrong-version assumption | Layer 1: stratified per Android version |
| Native-code dark matter | §13.1: native lifter to AXIOM-IR |

**The chain.** A reported finding is the conjunction:

```
verified_parse(v, bytes) ∧ behavior_set(parses) ∧
  forensic_finding(behavior_set) ∧ symbolic_witness(behavior_set, claim) ∧
    equivalence_proof(behavior_set, ref) ∧ certificate(...)
```

Every clause is sound. The conjunction is sound. **An adversary who wants to produce a false positive in our output would have to falsify a Lean theorem, an SMT UNSAT certificate, or a zk-SNARK** — all of which require breaking the underlying cryptographic or proof-theoretic assumptions.

That is what we mean by "100% confirmed bugs." Not literal infallibility (see caveats §19), but: every finding is reducible to a small, independently checkable proof artifact.

---

<a id="data-flow"></a>
## 15. Data Flow Diagram

```
APK bytes ─► L0 ─► CommitChain
                     │
                     ▼
              ┌──────┴──────┐
              │   L1 (×N)   │   N parsers run in parallel
              └──────┬──────┘
                     │
              Vec<VersionedParse>
                     │
                     ▼
                    L2 ─► BehaviorSet ◄───── (split APKs, dynamic features)
                     │
        ┌────────────┼────────────────┐
        ▼            ▼                ▼
       L3.1         L3.2             L3.3
    (Shadow)     (Provenance)    (Negative-Space)
        │            │                │
        └────────────┼────────────────┘
                     │
               ForensicFindings
                     │
                     ▼
                    L4 ─► ReachabilityProofs
                     │
        ┌────────────┴────────────┐
        ▼                         ▼
       L5.1                      L5.2
     (BSH-256)             (Bisimulation)
        │                         │
        └────────────┬────────────┘
                     │
               EquivalenceArtifacts
                     │
                     ▼
                    L6 ─► .axc certificate
                     │
                     ▼
              axiom-verify ─► ✅ / ❌

         (continuous, async)
         ┌─────────────────────┐
         │  Differential Fuzz  │
         │  Plant              │
         │  ──────────────     │
         │  Generates inputs   │
         │  Feeds L1 in parallel│
         │  Logs disagreements │
         │  Files CVEs         │
         └─────────────────────┘
```

---

<a id="comparison"></a>
## 16. Comparison Matrix

| Capability | Androguard | apkInspector | Apktool | MobSF | APKiD | Quark | apk-info | **APKAXIOM** |
|---|---|---|---|---|---|---|---|---|
| Static parsing | ✓ | ✓ | ✓ | ✓ | – | ✓ | ✓ | ✓ |
| **Verified parsers** | – | – | – | – | – | – | – | **✓ (Lean 4)** |
| **Per-Android-version stratification** | – | – | – | – | – | – | – | **✓** |
| Bundle / split APK | partial | – | – | partial | – | – | partial | **✓** |
| Dynamic feature modules | – | – | – | – | – | – | – | **✓** |
| Native code (.so) analysis | – | – | – | partial | partial | – | – | **✓ (lifted to IR)** |
| Symbolic intent resolution | – | – | – | – | – | – | – | **✓ (SMT-backed)** |
| Bisimulation equivalence | – | – | – | – | – | – | – | **✓** |
| Obfuscation-invariant fingerprint | – | – | – | – | – | partial | – | **✓ (BSH-256)** |
| Repackaging detection from single sample | – | – | – | – | – | – | – | **✓ (AXML provenance)** |
| Differential fuzzing oracle | – | – | – | – | – | – | – | **✓** |
| Cryptographic proof certificates | – | – | – | – | – | – | – | **✓ (.axc)** |
| zk-SNARK privacy invariants | – | – | – | – | – | – | – | **✓** |
| Streaming wire-speed inspection | – | – | – | – | – | – | – | **✓** |
| Reproducible bit-identical output | – | – | – | – | – | – | partial | **✓** |

---

<a id="roadmap"></a>
## 17. Research Roadmap & Publication Targets

> **For the executable phase-by-phase plan** — with hard gates, hiring sequence, deliverables per group, decision points, risk register, and the 20-item v1.0 ship checklist — see **[ROADMAP.md](./ROADMAP.md)**.
>
> The roadmap below is the high-level summary. The full plan is the source of truth.

### Phase 1 (months 0–6) — Foundations
- Lean 4 mechanization of ZIP layer + APK Signing Block
- Rust extraction pipeline
- AXIOM-IR v0.1
- Differential fuzzing plant prototype
- **Paper target:** *"Verified Parsing for the Android Package Format"* — CAV / OOPSLA

### Phase 2 (months 6–12) — Bundle Era
- Schrödinger APK formalization
- Bundle resolver in Rust
- BSH-256 specification + reference implementation
- Layer 3 forensics (all three sub-passes)
- **Paper target:** *"Rethinking the Unit of Analysis for Android Security in the App Bundle Era"* — USENIX Security / NDSS

### Phase 3 (months 12–18) — Symbolic & Equivalence
- Symbolic manifest resolver with cvc5
- Bounded bisimulation
- Cross-APK vulnerability discovery
- **Paper target:** *"Sound and Complete Intent Resolution for Android"* — IEEE S&P / NDSS

### Phase 4 (months 18–24) — Certificates
- Halo2 circuits for privacy invariants
- `.axc` format spec + verifier reference implementation
- Bug-bounty platform integration prototype
- **Paper target:** *"Proof-Carrying APKs: A New Architecture for Mobile App Distribution"* — CCS / S&P

### Phase 5 (months 24+) — Native + Dynamic
- ARM64 ELF lifter
- Frida/eBPF dynamic confirmation bridge
- ML model integrity layer
- **Paper target:** *"Joint Static-Dynamic Analysis of Android Native Code"* — NDSS / RAID

---

<a id="sota-catalogue"></a>
## 18. State-of-the-Art Techniques Catalogue

A non-exhaustive list of the research directions woven into the system. Each entry is a technique we use somewhere, with the layer that uses it.

| Technique | Layer | Why it matters here |
|---|---|---|
| Dependent types (Lean 4) | L1 | Encoding parser correctness as a type |
| Translation validation | L1 | Verifying Rust ↔ Lean equivalence |
| Coverage-guided grammar fuzzing | Continuous | Generating valid but adversarial inputs |
| Differential testing | Continuous | Cross-version bug discovery |
| Constrained Horn Clauses | L4 | Encoding recursive resolution |
| Refinement types | L5 | Bisimulation relation specification |
| Abstract interpretation with widening | L5 | Bounded fixed-points for equivalence |
| Locality-sensitive hashing (MinHash) | L5 | Sub-linear similarity search |
| Merkle Patricia Tries | L0 | Cryptographic provenance of bytes |
| BLAKE3 with personalization | L0 | Anti-collision content addressing |
| zk-SNARKs (Halo2 / Plonk) | L6 | Privacy-property proofs |
| FRI-based STARKs | L6 | Post-quantum proof option |
| MLIR-style multi-level IR | All | AXIOM-IR design |
| Concolic execution (Frida-bridged) | §13.2 | Static-dynamic refinement |
| Steganalysis-style anomaly detection | L3.3 | Negative-space resource analysis |
| Process-calculus bisimulation | L5 | Equivalence proofs |
| eBPF tracing | §13.2 | Kernel-level dynamic ground truth |
| Property-based testing (Hypothesis-style) | Test infra | Specification-driven test generation |
| SLSA L4 build attestation | §13.7 | Supply-chain provenance |
| Neural Cleanse / STRIP | §13.3 | TFLite backdoor detection |
| Reproducible builds | §13.7 | Source ↔ APK correspondence |

---

<a id="caveats"></a>
## 19. Honest Caveats

The README would be dishonest without these. They are **not bugs** — they are the boundary of what proofs can claim.

1. **"100% confirmed" means proof-checkable, not infallible.**
   A finding ships with a checkable proof. The proof is sound *relative to the formal model*. If the formal model diverges from real AOSP, a proof-checked finding could still be wrong in the world. The differential fuzzer is what bounds this drift, but it does not eliminate it.

2. **Proof systems have soundness assumptions.**
   Lean 4's logical kernel is small and well-studied, but not zero-trust. zk-SNARK schemes assume cryptographic hardness (DLOG, q-SDH, etc.). If those break, our proofs no longer hold.

3. **Layer 4 may return UNKNOWN.**
   Some intent resolution problems are undecidable in the general case. We return an explicit UNKNOWN (with the abstraction domain that triggered it) rather than a silent over-approximation. UNKNOWN is not a finding.

4. **Layer 5 bisimulation is bounded (k-step).**
   For finite k, we can prove equivalence-up-to-k. Two APKs that agree for k steps but diverge at step k+1 will be flagged as equivalent. Tunable, but not infinite.

5. **AOSP archaeology is a permanent maintenance burden.**
   New Android versions ship every 6 months. Each one requires re-formalization of any changed parser logic. This is staffing, not engineering.

6. **Native code lifting is approximate at the binary level.**
   Disassembly disagreements between objdump, Capstone, and BAP exist for adversarial binaries. We document our lifter's assumptions, but we cannot claim soundness against a binary-confusion attacker.

7. **Symbolic execution scales to thousands, not millions, of installed apps.**
   Cross-APK device snapshots are tractable for typical user devices. Enterprise fleet analysis would need additional abstraction.

8. **The system is currently a research artifact, not a product.**
   Scaling to billions of APKs/year (Play Store ingest scale) requires additional engineering not in scope of the research roadmap.

These are the honest limits. Every one is documented per-finding so users can calibrate their trust.

---

<a id="contributing"></a>
## 20. Contributing

This is a private research repository at present. Internal contributors should:

- Open issues against specific layers using the `L0`/`L1`/.../`L6` labels.
- All proofs (Lean / Coq / SMT) must be reproducible via `make verify-all`.
- All Rust extraction outputs must be reproducible via `make extract-all` against a pinned Lean toolchain hash.
- No PR may merge without the differential fuzzer passing on the changed parser variant.
- Public release planned post-Phase-2.

---

<a id="team-structure"></a>
## 21. Major Feature Areas & Engineering Team Structure

The system decomposes into **14 major feature areas**. Each maps to a dedicated engineering group with a precise mission, ownership boundary, headcount estimate, skill profile, and phase-activation schedule. Total headcount for full v1.0: **~42–58 engineers across ~3 years**.

The numbering below is the canonical group identifier — use `G1`...`G14` in PRs, issues, and commit prefixes.

### G1 — Formal Methods Core
- **Mission.** Mechanize the parser semantics in Lean 4 and operate the proof-extraction pipeline.
- **Owns.** Layer 1 (theorems), all Lean source, the `lean→rust` extractor, AXIOM-IR type-system formalization.
- **Headcount.** 4–6 (the highest-skill team — formal-methods PhDs, ITP experience required).
- **Skills.** Lean 4 / Coq, dependent types, translation validation, AOSP source archaeology.
- **Phase activation.** Phase 1, day 1. Continuous through Phase 5.
- **Hard dependencies.** None (foundation team).
- **Downstream consumers.** G2, G7.
- **Connection to apk-info.** Reads apk-info Rust parsers as the *behavioral specification* to formalize.

### G2 — Parser Engineering & AOSP Archaeology
- **Mission.** Ship the Rust parser bank that L1 extracts to. Maintain version-stratification across Android 8–15+. Track AOSP commits for semantically-relevant changes.
- **Owns.** Layer 0 (streaming ZIP spine + Merkle chain), Layer 1 (Rust crates), the `aosp-diff` archaeology tool, BLAKE3 commit infrastructure.
- **Headcount.** 5–7.
- **Skills.** Senior Rust, ZIP / DEX / AXML / ARSC binary formats, AOSP source navigation, performance engineering.
- **Phase activation.** Phase 1, day 1. Heaviest staffing in Phase 1–2.
- **Hard dependencies.** G1 (for extraction targets).
- **This is where apk-info engineers and code transplant directly.** See §22.

### G3 — AXIOM-IR & Bundle Resolver
- **Mission.** Design and maintain the typed multi-dialect intermediate representation. Implement the Schrödinger APK semantics and bundle composition operator.
- **Owns.** Layer 2 (bundle resolver), AXIOM-IR core, dialect specs (manifest, DEX, native, resource), lowerings between dialects.
- **Headcount.** 4–5.
- **Skills.** Compiler infrastructure, MLIR or LLVM IR experience, Android App Bundle internals, type theory.
- **Phase activation.** Phase 1 (IR design), Phase 2 (bundle resolver implementation).
- **Hard dependencies.** G1 (type-system spec), G2 (parser outputs).
- **Downstream consumers.** G4, G5, G6, G9.

### G4 — Structural Forensics
- **Mission.** Implement the three independent forensic passes on the BehaviorSet. None require ML or malware corpora.
- **Owns.** Layer 3.1 (Shadow Stack), 3.2 (AXML Compiler Provenance Fingerprint), 3.3 (Negative-Space Resource Anomaly).
- **Headcount.** 3–4.
- **Skills.** Digital forensics mindset, statistical anomaly detection, deep familiarity with build toolchains (`aapt`, `aapt2`, `apktool`, etc.).
- **Phase activation.** Phase 2.
- **Hard dependencies.** G3 (BehaviorSet input).
- **Subgroup option.** 3.2 is small enough to be one engineer's responsibility; 3.1 and 3.3 are 1–2 engineers each.

### G5 — Symbolic Execution & Intent Resolver
- **Mission.** Implement Layer 4 — the SMT-backed manifest and intent resolver. Encode `PackageManager` semantics as CHCs.
- **Owns.** Layer 4, all SMT integrations (cvc5, Z3, Spacer, Eldarica), CHC encoders, abstraction-domain library.
- **Headcount.** 4–5.
- **Skills.** Symbolic execution (KLEE, angr-style), SMT modeling, program analysis. Strong PL/formal-methods background but more applied than G1.
- **Phase activation.** Phase 3.
- **Hard dependencies.** G3 (BehaviorSet, AXIOM-IR).
- **Subgroup option.** Cross-APK / device-snapshot analysis is a 1–2 person sub-team that can spin out in Phase 4.

### G6 — Equivalence & Fingerprinting
- **Mission.** Behavior Surface Hash (BSH-256) as a citable standard, plus bounded bisimulation equivalence proofs.
- **Owns.** Layer 5.1 (BSH-256 spec + reference impl), Layer 5.2 (bisimulation engine), MinHash/LSH similarity index.
- **Headcount.** 3.
- **Skills.** Process calculus or refinement types, abstract interpretation, hash-based similarity (LSH, ssdeep, TLSH lineage).
- **Phase activation.** Phase 3.
- **Hard dependencies.** G3, G5.

### G7 — Proof Systems & Cryptography
- **Mission.** Layer 6 — the certificate emitter and verifier. zk-SNARK and STARK circuits for privacy invariants.
- **Owns.** `.axc` certificate format, `axiom-verify` reference verifier, Halo2 / Plonk circuits, optional STARK pipeline, signing infrastructure.
- **Headcount.** 4–5 (cryptographers + circuit engineers).
- **Skills.** zk-SNARK construction (Halo2, Plonk, Groth16), constraint-system design, ed25519, post-quantum awareness.
- **Phase activation.** Phase 4 (heaviest).
- **Hard dependencies.** G1 (for proof-object formats), G5 (UNSAT certs to lift), G6 (equivalence witnesses).

### G8 — Differential Fuzzing Plant
- **Mission.** The continuous oracle that keeps the proof stack honest.
- **Owns.** Grammar-aware APK fuzzer, AOSP cross-compile harnesses (8+ versions), disagreement classifier, CVE filing pipeline.
- **Headcount.** 4.
- **Skills.** Fuzzing infrastructure (AFL++, libFuzzer, structure-aware fuzzing), cross-compilation, AOSP build systems, OSS-Fuzz operations.
- **Phase activation.** Phase 1, day 30 (concurrently with G1 / G2 — the fuzzer needs the L1 parsers as soon as a single one exists).
- **Hard dependencies.** G2 (parser variants).
- **This team consumes the existing apk-info fuzz harness as its v0 seed.**

### G9 — Native Code Subsystem
- **Mission.** Lift DEX bytecode and ARM64/ARMv7 ELF into AXIOM-IR so upper layers can reason jointly over Java and native code.
- **Owns.** DEX lifter, ARM64 ELF lifter (built on LLVM MLIR), calling-convention modeling, JNI boundary modeling.
- **Headcount.** 4.
- **Skills.** Binary analysis (BAP, angr, radare2, IDA), MLIR, AOT compilation, ARM64 ISA, Dalvik bytecode internals.
- **Phase activation.** Phase 5 (latest-starting major group).
- **Hard dependencies.** G3 (AXIOM-IR with native dialect).

### G10 — Dynamic Confirmation Bridge
- **Mission.** When the static layers return UNKNOWN, drop into Frida + eBPF dynamic execution and refine the abstraction.
- **Owns.** Sandboxed emulator pool, Frida script library, eBPF tracing programs, concolic-execution glue, abstraction-refinement feedback loop.
- **Headcount.** 3.
- **Skills.** Android internals (Frida, Xposed/LSPosed), eBPF, kernel tracing, virtualization, mobile sandboxing.
- **Phase activation.** Phase 5.
- **Hard dependencies.** G5 (UNKNOWN findings as input), G3 (IR).

### G11 — ML Model Security
- **Mission.** Verify embedded ML models (TFLite, ONNX) for integrity, backdoors, adversarial fragility.
- **Owns.** Structural model hash, Neural Cleanse / STRIP integration, adversarial-attack harness (cleverhans, foolbox).
- **Headcount.** 2–3.
- **Skills.** ML security research, adversarial robustness, TFLite/ONNX internals.
- **Phase activation.** Phase 5 (could be deferred to v1.1 if scope pressure).
- **Hard dependencies.** Loose. Self-contained pass over `assets/*.tflite`.

### G12 — Supply Chain & Reproducibility
- **Mission.** SLSA L4 attestation verification. Source ↔ APK reproducibility proofs.
- **Owns.** SLSA verifier, deterministic AXML re-encoder, DEX normalizer, build-attestation parser.
- **Headcount.** 2.
- **Skills.** Reproducible builds (F-Droid, Sigstore, in-toto), SLSA spec.
- **Phase activation.** Phase 4.
- **Hard dependencies.** G2 (deterministic parser inverses).

### G13 — Platform Infrastructure
- **Mission.** The CI, build, and reproducibility substrate that every other group depends on.
- **Owns.** Hermetic build environment (Nix or Bazel), reproducibility test harness, monitoring/observability, release engineering, packaging (axiom-cli, axiom-py, container images).
- **Headcount.** 3.
- **Skills.** SRE, build engineering, hermetic builds, release engineering.
- **Phase activation.** Phase 1, day 1 (foundation, like G1/G2).
- **Hard dependencies.** None (everything depends on G13).

### G14 — Verifier, SDKs & Developer Tooling
- **Mission.** The user-facing surface. `axiom-verify` (the reference verifier that bug-bounty triagers run), SDKs, IDE integrations, app-store ingest tooling.
- **Owns.** `axiom-verify` reference impl, language SDKs (Rust / Python / Go / TypeScript), VS Code / IntelliJ plugins, REST/gRPC services for app-store integration.
- **Headcount.** 3–4.
- **Skills.** Senior Rust/Python, API design, developer experience.
- **Phase activation.** Phase 4.
- **Hard dependencies.** G7 (`.axc` format), all upstream layers (for SDK surface).

### Summary

| Group | Area | Layer | Headcount | Phase start |
|---|---|---|---|---|
| G1 | Formal Methods Core | L1 (theorems) | 4–6 | 1 |
| G2 | Parser Engineering & AOSP Archaeology | L0, L1 (Rust) | 5–7 | 1 |
| G3 | AXIOM-IR & Bundle Resolver | L2, IR | 4–5 | 1 |
| G4 | Structural Forensics | L3 | 3–4 | 2 |
| G5 | Symbolic Execution & Intent Resolver | L4 | 4–5 | 3 |
| G6 | Equivalence & Fingerprinting | L5 | 3 | 3 |
| G7 | Proof Systems & Cryptography | L6 | 4–5 | 4 |
| G8 | Differential Fuzzing Plant | Continuous | 4 | 1 |
| G9 | Native Code Subsystem | §13.1 | 4 | 5 |
| G10 | Dynamic Confirmation Bridge | §13.2 | 3 | 5 |
| G11 | ML Model Security | §13.3, §13.8 | 2–3 | 5 |
| G12 | Supply Chain & Reproducibility | §13.7 | 2 | 4 |
| G13 | Platform Infrastructure | Substrate | 3 | 1 |
| G14 | Verifier, SDKs & Developer Tooling | User surface | 3–4 | 4 |
| **Total** | | | **~48 (range 42–58)** | |

### Phase-staggered staffing curve

```
Phase 1 (mo 0–6):    G1, G2, G3, G8, G13                         ~20 eng
Phase 2 (mo 6–12):   + G4                                        ~24 eng
Phase 3 (mo 12–18):  + G5, G6                                    ~32 eng
Phase 4 (mo 18–24):  + G7, G12, G14                              ~42 eng
Phase 5 (mo 24+):    + G9, G10, G11                              ~52 eng
```

This is the realistic engineer-allocation table. It's how Google or a serious security lab would actually staff this.

---

<a id="apkinfo-integration"></a>
## 22. apk-info as the Engineering Beachhead

`apk-info` (the existing Rust APK parser, ~102 stars) is **not displaced** by APKAXIOM — it is **absorbed and elevated**. This section makes the integration concrete so the apk-info maintainers and APKAXIOM G2 are aligned from day 1.

### Where apk-info fits today

apk-info, as it stands, is a **non-verified, single-Android-version, single-process Rust parser** with PyO3 bindings. Mapped onto the APKAXIOM proof stack:

| apk-info component | APKAXIOM home | Status after integration |
|---|---|---|
| Core ZIP entry parsing | Layer 0 | Becomes the *fast path* execution backend; G2 wraps it with streaming + Merkle commits |
| AXML binary parser | Layer 1 (Rust target) | Becomes the *extraction target* for the Lean AXML formalization. apk-info parser remains as the executable; Lean output validates against it via translation validation |
| ARSC parser | Layer 1 (Rust target) | Same as above |
| APK Signature Schemes v1 / v2 / v3 / v3.1 | Layer 1 (Rust target) | Highest priority for Phase 1 Lean mechanization |
| Third-party signing block formats (Stamp, Channel, Packer NG, Vasdolly) | Layer 1 extension dialect | Stays as it is initially; Lean mechanization deferred to Phase 2 |
| MainActivity resolution mirroring AOSP | Layer 4 (subset) | Becomes a *unit test* of the symbolic intent resolver — if G5's resolver disagrees with apk-info's MainActivity, one of them is wrong |
| Fuzzing harness | Continuous (G8) | Direct seed for the Differential Fuzzing Plant; G8 extends with grammar-awareness and cross-version harnesses |
| PyO3 bindings | G14 (axiom-py) | Reference architecture for APKAXIOM Python SDK |
| Malformed-APK resilience (BadPack handling) | Layer 1 (current) → tracked in Continuous | What apk-info handles today through code becomes what L1 handles by *theorem*. The fuzzer keeps regression coverage |

### Optimization paths (for current apk-info)

These are improvements that are *natural* to apk-info on its own, *and* preconditions for Phase-1 integration into APKAXIOM:

1. **Streaming entry point.** Currently apk-info loads the file. Add a streaming variant (`ApkParser::from_reader<R: Read>`) and emit a typed event stream. Required for L0 integration.
2. **Per-version dispatch trait.** Refactor parsing into `trait AndroidVersionParser { ... }` with one `impl` per supported API level. Today it's effectively a single parser claiming to "match Android"; this surfaces the cross-version differences that L1 will exploit.
3. **Type-state guards.** Replace runtime checks with phantom types (`Apk<Unverified>` vs. `Apk<SignatureVerified>` vs. `Apk<FullyParsed<v: AndroidVersion>>`). Compiles to zero overhead, prevents misuse, eases Lean translation validation.
4. **Merkle commitment hooks.** Surface byte-range commitments at every parse step. `parser.commit_chain()` returns the BLAKE3 chain. Required for L0.
5. **AXIOM-IR emitter.** Add an `apk_info::ir` module that emits the manifest dialect of AXIOM-IR. Required for L2/L3/L4 to consume.
6. **Deterministic AXML re-encoding.** Inverse of decode, byte-stable. Required for G12 (reproducibility).
7. **Performance sustained, not regressed.** All of the above must keep the existing 10x-Androguard speed advantage. Benchmarks become CI gates.

### Migration path

```
apk-info v0.x (today)
    │
    ▼
apk-info v1.0 — "APKAXIOM-compatible"
    │  • streaming reader
    │  • per-version trait
    │  • type-state
    │  • Merkle commits
    │  • AXIOM-IR emitter
    │
    ▼
apk-info v2.0 — "APKAXIOM-extracted"
    │  • Rust parsers replaced by Lean-extracted modules
    │  • original Rust kept as differential oracle
    │
    ▼
APKAXIOM Layer 1 (production)
    Verified Rust parsers
    apk-info v2.0 = the fast unverified tier, used in performance mode
    apk-info v0.x retired
```

### Naming after integration

- Public crate: continues as `apk-info` for ecosystem continuity and the existing 102-star momentum.
- Internal name within APKAXIOM: `axiom-l1-rs` (workspace alias).
- The 10x-perf and "parses malware Androguard can't" claims **stay** — they're now framed as "apk-info, the unverified-but-fast tier of APKAXIOM Layer 1."

### Why this is the right structure

apk-info has real-world traction (102 stars, 11 forks, Apache-2.0). Throwing it away to start over would be wasteful and would lose the user base. Wrapping it as the "fast unverified tier" of L1, while G1 produces the verified tier, gives users a smooth speed-vs-soundness dial and gives APKAXIOM credibility on day 1.

This is the same pattern as **CompCert vs. GCC** — CompCert ships verified for safety-critical, GCC stays for performance, both coexist, both cite each other. apk-info is the GCC tier; the Lean-extracted parsers are the CompCert tier.

---

## License

TBD. Likely AGPL for the open-source core + commercial license for app-store / bug-bounty integrations.

## Citation

If you cite APKAXIOM before any of the publications listed in §17 are out, please cite this repository directly with a permalink to the commit.

```bibtex
@misc{apkaxiom2026,
  title  = {APKAXIOM: A Proof-Stack Analysis Platform for Android Packages},
  year   = {2026},
  note   = {Private research artifact, GitHub: Fizan324926/apkaxiom}
}
```

---

*"A finding is either accompanied by a proof, or it isn't a finding."*
