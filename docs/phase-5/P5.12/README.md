# P5.12 — eBPF Program Library for Kernel-Level Tracing

> Build an eBPF program library producing kernel-level traces that complement Frida's user-space hooks. Catches syscalls Frida misses (anti-Frida apps, native packers).

**Parent plan:** [../README.md](../README.md)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P5.12 |
| Owner(s) | G10 |
| Duration | Weeks 6–14 |
| Critical-path | yes (dynamic confirmation needs both Frida + eBPF) |
| Hard prerequisites | P5.10 |

## 2. Goal & Scope

eBPF programs running on the emulator-host kernel that observe the target Android process below user-space, immune to user-space anti-tamper. Cross-checks Frida; surfaces syscalls in heavily-packed apps that Frida cannot enter.

### In scope
- eBPF programs covering:
  - `syscall:enter/*` and `syscall:exit/*` filtered to target pid
  - `tcp_connect`, `tcp_sendmsg`, `tcp_recvmsg`
  - `udp_send_skb`, `udp_recv_skb`
  - `inet_sock_set_state`
  - `vfs_read`, `vfs_write`, `do_filp_open`, `do_unlinkat`
  - `mmap_pgoff`, `mprotect`, `munmap`
  - `ptrace_attach`, `ptrace_detach` (anti-Frida detection)
  - `binder_ioctl` (binder transactions, parsed for Intent dispatches)
- BTF / CO-RE for portability across kernel versions
- Per-pod isolation (one map per emulator pod)
- Load latency ≤ 200 ms (HARD), ≤ 30 ms (TARGET)
- Trace event schema unified with Frida (P5.11)

### Out of scope
- User-space hooks (P5.11)
- Dynamic-bridge logic (P5.13)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P5.10** | Emulator-pool host kernels with BTF enabled |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **libbpf** | latest | eBPF runtime |
| **bpftrace** | latest | High-level tracing |
| **CO-RE (BTF)** | kernel 5.4+ | Portable eBPF |
| **clang / LLVM** | matching | eBPF compiler |
| **libbpf-rs** | latest | Rust bindings |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **libbpf** | lib | **Free** OSS | https://github.com/libbpf/libbpf | Kernel project |
| **bpftrace** | tool | **Free** OSS | https://github.com/iovisor/bpftrace | |
| **libbpf-rs** | lib | **Free** OSS | https://github.com/libbpf/libbpf-rs | |

**No new API keys.**

## 6. System Inventory — Have vs Need

| Need | Status |
|---|---|
| BTF in emulator-host kernel | enable in K8s node config |
| libbpf + libbpf-rs | install via apt + cargo |

```bash
sudo apt-get install -y libbpf-dev clang
cargo install libbpf-cargo
```

## 7. Features & Functions Delivered (Comprehensive)

### eBPF program suite (`ebpf-progs/`)
- `syscall-trace.bpf.c` — generic syscall enter/exit, pid-filtered
- `net-trace.bpf.c` — tcp_connect / tcp_sendmsg / tcp_recvmsg / udp / inet_sock_set_state
- `file-trace.bpf.c` — vfs_read / vfs_write / do_filp_open / do_unlinkat
- `mem-trace.bpf.c` — mmap / mprotect / munmap (catches packer unpack)
- `ptrace-detect.bpf.c` — ptrace_attach / ptrace_detach (anti-Frida flag)
- `binder-trace.bpf.c` — binder_ioctl with Intent parser

### CO-RE / BTF
- All programs CO-RE-clean across kernel 5.4–6.10
- BTF enabled in pool-host config

### Per-pod isolation
- One eBPF map ringbuf per emulator pod
- Per-pod pid filter

### Load latency
- ≤ 200 ms HARD (≤ 30 ms TARGET)
- Pre-compile + verify cached, skip-load on attach

### Trace schema
- Unified protobuf with Frida (`protos/frida-trace.proto` extended for kernel events)

### Tools
- `axiom-ebpf-load` — load programs onto host, attach to pid
- `axiom-ebpf-replay` — replay captured eBPF trace

### Reproducibility
- Same input → same trace, modulo timestamps

### Documentation
- `docs/ebpf-library.md` — programs + extension guide

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Load latency | ≤ 200 ms | ≤ 30 ms |
| CO-RE portability across kernel 5.4–6.10 | yes | yes |
| Per-pod isolation tested | 100 % | 100 % |
| Trace schema unified with Frida | yes | yes |
| Trace event throughput | ≥ 100 K events/s | ≥ 500 K events/s |
| Anti-Frida detection (via ptrace events) | tested | tested |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── ebpf-progs/                       # NEW: BPF source
│   ├── syscall-trace.bpf.c
│   ├── net-trace.bpf.c
│   ├── file-trace.bpf.c
│   ├── mem-trace.bpf.c
│   ├── ptrace-detect.bpf.c
│   └── binder-trace.bpf.c
├── crates/
│   └── axiom-ebpf-load/              # NEW: Rust loader
├── tools/
│   ├── axiom-ebpf-load
│   └── axiom-ebpf-replay
└── docs/
    └── ebpf-library.md               # NEW
```

## 10. Standalone Output

eBPF programs + Rust loader, reusable beyond APKAXIOM.

## 11. End-to-End Test

```bash
buck2 build //ebpf-progs:...
buck2 build //crates/axiom-ebpf-load:...

buck2 run //tools:axiom-ebpf-load -- --pid <emu-pid> --progs all
# Expect: load ≤ 200 ms

# Anti-Frida flag
buck2 run //tools:axiom-ebpf-load -- --watch ptrace
```

## 12. Exit Checklist

- [ ] Load latency ≤ 200 ms (HARD)
- [ ] CO-RE portability 5.4–6.10 verified
- [ ] Per-pod isolation 100 %
- [ ] Trace schema unified with Frida
- [ ] Trace throughput ≥ 100 K events/s
- [ ] Anti-Frida detection tested
- [ ] Documentation `docs/ebpf-library.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P5.13** | eBPF traces feeding dynamic confirmation |
| **P5.18** | eBPF library in E2E pipeline |
| **Production** | Optional: surface eBPF in production canary (consent-gated) |
