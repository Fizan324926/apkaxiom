# P1.13 — Differential Fuzzing Plant: Cuttlefish A14 Harness via Nyx

> First real fuzzing campaign. Cuttlefish A14 image fuzzed via Nyx snapshot-based hypervisor fuzzing. Disagreements logged from week 1.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../README.md §12 (Continuous fuzzing)](../../README.md#continuous) · [../../TECH_STACK.md §9 (Fuzzing)](../../TECH_STACK.md#fuzzing)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P1.13 |
| Owner(s) | G8 (Differential Fuzzing Plant) |
| Duration | Weeks 4–12 (long ramp; runs continuous after launch) |
| Critical-path | no — runs in parallel with critical path |
| Hard prerequisites | P1.1 (Bazel sub-workspace, hermetic build) |

## 2. Goal & Scope

A Cuttlefish A14 image is wrapped as a Nyx fuzzing harness. The fuzzer mutates ZIP/APK byte structures and observes whether the A14 install pipeline accepts or rejects the input. Disagreements with our `axiom-l0` parser are logged and classified.

This is the **single most expensive infrastructure item in Phase 1** because it requires KVM, hardware virtualization, and disk for AOSP source + emulator images.

### In scope
- Cuttlefish A14 hermetic image (built via Bazel sub-workspace)
- Nyx snapshot + harness wrapper
- APK grammar (initial) for Nautilus-style mutation guidance
- Disagreement classifier — manual taxonomy at first, automated by P1.14
- Fuzzing dashboard (Grafana, fed by Prometheus)
- 24/7 operation with ≥99% uptime

### Out of scope
- A8 + A11 harnesses (P1.14)
- Automated classifier (P1.14)
- Nautilus grammar-aware mutation in production (P1.14)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P1.1** | Bazel sub-workspace; CI infrastructure |
| **P1.5** | AOSP partial sync at A14 |
| **P1.7** | Streaming `axiom-l0` parser (the diff target) |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Cuttlefish** | latest from AOSP | Headless Android emulator |
| **Nyx** | latest research drop from `nyx-fuzz/nyx` | Snapshot-based hypervisor fuzzing |
| **KVM** | kernel feature | Hardware virtualization (REQUIRED) |
| **QEMU** | 8.x+ | VM substrate Nyx layers on |
| **libvirt** | 10.x | VM management (optional but recommended) |
| **AFL++** | 4.x (from P1.6) | Mutation engine inside Nyx |
| **Nautilus** | research code | Grammar-aware fuzzing (basic in P1.13, advanced in P1.14) |
| **Centipede** | from Google | Distributed fuzz orchestration (used at P1.14 scale) |
| **Rust** | 1.95 | Disagreement-classifier driver |
| **rkyv** | 0.7+ | Persistent fuzz corpus + finding store |
| **fjall** | 0.5+ | LSM tree for finding archive |
| **OpenTelemetry** | 0.100+ | Tracing per fuzz iteration |
| **Prometheus + Grafana** | latest | Dashboards |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Cuttlefish** | Android emulator | **Free** OSS (Apache 2.0) | https://source.android.com/docs/devices/cuttlefish | Google AOSP project |
| **Nyx** | snapshot fuzzer | **Free** OSS | https://github.com/nyx-fuzz/nyx | Bochum / TUDA research; Apache 2.0 |
| **AFL++** | fuzzer | **Free** OSS (Apache 2.0) | https://github.com/AFLplusplus/AFLplusplus | From P1.6 |
| **Nautilus** | grammar fuzzer | **Free** OSS | https://github.com/nautilus-fuzz/nautilus | Bochum research |
| **Centipede** | distributed fuzzer | **Free** OSS | https://github.com/google/centipede | Google |
| **OSS-Fuzz** | Google fuzzing service | **Free** *only if open-source* | https://google.github.io/oss-fuzz/ | Available later if APKAXIOM goes OSS pre-v1.0 |
| **Hetzner / OVH dedicated server with KVM** | fuzzing host | **Paid** (~ €100–300 / month per node) | https://www.hetzner.com/dedicated-rootserver/ | **REQUIRED** — KVM-enabled hardware. Need ≥ 4 nodes for P1.14, 1 minimum here |
| **GitHub Issues / private security advisories** | CVE coordination | **Free** | (within our org) | Used to draft CVE filings |
| **MITRE CVE Numbering Authority (CNA)** | CVE allocation | **Free** | https://cveform.mitre.org | Apply for CNA status (long-lead) or use a partner CNA |
| **Google Android Security team** | private disclosure | **Free** coordinated disclosure | https://www.google.com/about/appsecurity/android-rewards/ | Up to $1M reward for severe Android findings |

**Hardware requirement:** This sub-phase is the first that **cannot run on the development host** because the host has no `/dev/kvm` (verified at M0 inventory). A dedicated machine (Hetzner AX102 with VT-x/AMD-V) is required by Week 4.

**Account-level decisions:**
- CNA status (long lead — start application Week 1) or partner CNA (Google Android Security accepts CVEs through their pipeline).
- AndroZoo access (already in P1.3) supplies seed corpus.

## 6. System Inventory — Have vs Need

### On development host
- ✅ AFL++ (after P1.6 install)
- ✅ Java 21 / javac 17
- ❌ **/dev/kvm** — ABSENT — must run on KVM-enabled hardware
- ❌ **QEMU system emulators** — `sudo apt-get install -y qemu-system-x86 qemu-system-arm`
- ❌ **libvirt + virsh** — `sudo apt-get install -y libvirt-daemon-system libvirt-clients virtinst`

### On dedicated KVM host (procured for this sub-phase)
- KVM-enabled CPU (Intel VT-x or AMD-V) with virtualization in BIOS
- Linux 6.x with kvm + kvm_intel/amd modules loaded
- ≥ 64 GB RAM (Cuttlefish + Nyx are memory-hungry)
- ≥ 1 TB NVMe (AOSP source + emulator images + fuzz corpus)
- Ubuntu Server 24.04 or equivalent

### Install commands

```bash
# On the dedicated KVM host:

# 1) QEMU + libvirt
sudo apt-get install -y qemu-kvm qemu-system-x86 qemu-system-arm \
  libvirt-daemon-system libvirt-clients virtinst bridge-utils

# 2) Verify KVM
ls /dev/kvm  # must exist
sudo kvm-ok  # "KVM acceleration can be used"

# 3) Cuttlefish prerequisites (Ubuntu)
git clone https://github.com/google/android-cuttlefish
cd android-cuttlefish
tools/buildutils/build_packages.sh
sudo dpkg -i ../cuttlefish-base_*.deb ../cuttlefish-user_*.deb
sudo usermod -aG kvm,cvdnetwork,render $USER

# 4) Pull A14 Cuttlefish system image (signed-in builds.cocoon.gov-compatible)
mkdir -p /opt/cuttlefish-images/A14 && cd /opt/cuttlefish-images/A14
# Download official AOSP A14 cuttlefish CVD
wget https://ci.android.com/builds/submitted/<build-id>/aosp_cf_x86_64_phone-userdebug/latest/aosp_cf_x86_64_phone-img-<build-id>.zip
unzip aosp_cf_x86_64_phone-img-<build-id>.zip

# 5) Nyx
git clone https://github.com/nyx-fuzz/nyx
cd nyx && ./setup.sh
# Builds Nyx KVM extension + harness loader

# 6) Centipede (for orchestration at scale)
git clone https://github.com/google/centipede && cd centipede
bazel build :all
```

## 7. Working Directory & Files Produced

```
apkaxiom/
├── fuzz/
│   ├── BUCK
│   ├── grammars/
│   │   └── apk-v1.lark                  # NEW — basic APK grammar
│   ├── corpus/
│   │   ├── seed/                         # NEW — seeds from Bench-1K + adversarial
│   │   └── persistent/                   # NEW — discovered inputs (rkyv archive)
│   ├── findings/
│   │   ├── archive.fjall                 # NEW — LSM of all disagreements
│   │   └── rcas/                         # NEW — root-cause analyses
│   ├── harness/
│   │   ├── Cargo.toml
│   │   ├── BUCK
│   │   └── src/
│   │       ├── main.rs                   # Nyx harness driver
│   │       ├── cuttlefish.rs              # Cuttlefish lifecycle
│   │       └── differ.rs                  # axiom-l0 vs Cuttlefish diff
│   └── dashboards/
│       └── grafana-fuzzing.json
├── external/aosp/
│   └── cuttlefish-A14/                    # vendored, gitignored
└── docs/
    └── differential-fuzzer.md             # NEW
```

## 8. Standalone Output

```bash
# On the KVM host:
buck2 build //fuzz/harness:fuzz-driver
buck2 run //fuzz/harness:fuzz-driver -- --android-version A14 --duration continuous
# Logs disagreements to fuzz/findings/archive.fjall
# Dashboard: http://kvm-host:3000/d/fuzzing
```

## 9. End-to-End Test

7-day continuous run:
- Fuzzer running 24/7 with ≥ 99% uptime (HARD).
- ≥ 5 distinct (replay-verified) disagreements logged in 7 days.
- Each disagreement reproducible byte-for-byte.

```bash
buck2 test //fuzz/harness:smoke-7d
# Validates the 7-day soak and the disagreement count
```

## 10. Exit Checklist

- [ ] KVM-enabled hardware procured and provisioned
- [ ] Cuttlefish A14 image hermetically built
- [ ] Nyx wrapper operational
- [ ] APK grammar drafted and seed corpus loaded
- [ ] Fuzzer runs 24/7 with ≥ 99% uptime over 7 days
- [ ] ≥ 5 disagreements logged with replay-verified reproducer (HARD)
- [ ] Findings archive uses fjall LSM tree, persistent across restarts
- [ ] Grafana dashboard live, paged on regression
- [ ] CNA / coordinated-disclosure path documented
- [ ] `docs/differential-fuzzer.md` published

## 11. Hand-Off

| Consumed by | What they need |
|---|---|
| **P1.14** | Single-version harness extends to A8 + A11; classifier automation |
| **P1.18** | Disagreement count is a Phase-1 KPI |
| **P1.20** | Fuzzer must be running at gate review with ≥ 10 disagreements/week classified |
| **Phase 2 / G8** | Scaling to 5 AOSP versions; Centipede orchestration |
