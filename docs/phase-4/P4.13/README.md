# P4.13 — SDK: `axiom-py` (PyO3 + uniffi)

> Pythonic API over `axiom-verify`. PyO3 + uniffi from a single Rust source. ≥ 50 verifications/sec/core. PyPI-distributed wheels for x86_64 + ARM64 Linux/macOS/Windows.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §15 (SDK)](../../../README.md#sdk)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.13 |
| Owner(s) | G14 |
| Duration | Weeks 14–18 |
| Critical-path | yes |
| Hard prerequisites | P4.11 (verifier core) |

## 2. Goal & Scope

A Python SDK over `axiom-verify`. Generated from a single Rust source via PyO3 (or uniffi for richer language coverage). Ships wheels to PyPI for x86_64 + ARM64 Linux/macOS/Windows. ≥ 50 verifications/sec/core. FFI overhead < 30%.

### In scope
- `sdk/axiom-py` — Python package
- PyO3 bindings (or uniffi-generated)
- PyPI distribution (pip-installable)
- Type stubs (`.pyi`) for IDE
- Async API for high-throughput uses
- Comprehensive test suite

### Out of scope
- Pure-Python alternative (defeat the SDK purpose)
- Other SDKs (P4.14 axiom-go, P4.15 axiom-ts)

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P4.11** | Verifier core (the thing we wrap) |
| **P4.1** | uniffi installed |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **PyO3** | 0.22+ | Rust ↔ Python |
| **maturin** | 1.6+ | Build wheels |
| **uniffi** | 0.27+ | Single-source binding generator (alt path) |
| **Python 3.10–3.13** | latest | Target |
| **mypy / pyright** | latest | Type-checking the .pyi stubs |
| **pytest + asyncio** | latest | Test suite |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **PyO3 / maturin** | crates | **Free** OSS | crates.io | |
| **uniffi** | crate | **Free** OSS (Mozilla) | https://github.com/mozilla/uniffi-rs | |
| **PyPI** | package registry | **Free** | https://pypi.org | Sign up; account required |
| **TestPyPI** | staging | **Free** | https://test.pypi.org | For pre-release |
| **GitHub Actions for wheels** | CI | **Free** for public, **paid** for private | already provisioned | cibuildwheel + maturin-action |

**Account-level requirement:** PyPI publishing account for the `apkaxiom` user/org (free, but takes a day to set up org + verify maintainers).

## 6. System Inventory — Have vs Need

### Already present
- ✅ Rust + Python 3.12 (HAVE) + uniffi (P4.1)

### Missing — must install
- ❌ **maturin** — `cargo install maturin`
- ❌ **PyO3** — Cargo dep
- ❌ **cibuildwheel** — `pip install cibuildwheel`

```bash
cargo install maturin
pip install cibuildwheel
```

## 7. Features & Functions Delivered (Comprehensive)

### Python API (`axiom_py` package)
```python
from axiom_py import verify, Cert

cert = Cert.from_file("report.axc")
result = verify(cert, apk_bytes=open("app.apk", "rb").read())
print(result.ok, result.claims, result.audit_log)
```

- `axiom_py.verify(cert, apk_bytes=None) -> VerifyResult`
- `axiom_py.Cert` — wrapper around `.axc`
- `axiom_py.VerifyResult` — `.ok`, `.claims`, `.error`, `.audit_log`
- `axiom_py.Claim` — typed per-claim wrapper
- Async API: `await axiom_py.verify_async(cert)`

### Type stubs
- `axiom_py.pyi` ships
- Strict mypy / pyright pass
- IDE autocomplete works in VSCode + PyCharm

### Wheels
- x86_64 Linux (manylinux2014, manylinux_2_28)
- aarch64 Linux (manylinux_2_28)
- x86_64 macOS (macosx_11)
- ARM64 macOS (macosx_14)
- x86_64 Windows
- Reproducible builds — wheel contents byte-identical for same source SHA

### Distribution
- PyPI primary (`pip install axiom-py`)
- TestPyPI for pre-release
- GitHub releases mirror

### Test suite
- ≥ 100 unit tests
- ≥ 20 integration tests against real `.axc` certs
- Cross-platform CI (`cibuildwheel`)

### Documentation
- `docs/sdk-python.md`
- Auto-generated API docs (Sphinx)
- README on PyPI

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Single-core verifications/sec via `axiom_py` | ≥ 50 | ≥ 150 |
| FFI overhead vs native Rust | < 30 % | < 10 % |
| Wheels available for x86_64 + ARM64 Linux/macOS/Windows | ≥ 5 platforms | 6 |
| PyPI publishing pipeline | live | live |
| Type-stub strict mypy pass | yes | yes |
| Test suite pass | 100 % | 100 % |
| Reproducible wheels | bit-identical for same source | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── sdk/
│   └── axiom-py/
│       ├── Cargo.toml
│       ├── pyproject.toml                # maturin config
│       ├── BUCK
│       ├── src/
│       │   └── lib.rs                    # PyO3 bindings
│       ├── python/
│       │   └── axiom_py/
│       │       ├── __init__.py
│       │       └── axiom_py.pyi
│       └── tests/
│           ├── test_unit.py
│           └── test_integration.py
└── docs/
    └── sdk-python.md
```

## 10. Standalone Output

```bash
cd sdk/axiom-py && maturin develop
python -c "from axiom_py import verify, Cert; cert = Cert.from_file('sample.axc'); print(verify(cert).ok)"
# True
```

## 11. End-to-End Test

```bash
buck2 test //sdk/axiom-py:full
# - Throughput ≥ 50/sec/core (HARD)
# - FFI overhead < 30% (HARD)
# - Wheels build for ≥ 5 platforms (HARD)
# - mypy strict pass (HARD)
```

## 12. Exit Checklist

- [ ] `axiom-py` package compiles
- [ ] PyO3 bindings (or uniffi-generated) operational
- [ ] Throughput ≥ 50/sec/core (HARD)
- [ ] FFI overhead < 30 % (HARD)
- [ ] Wheels for ≥ 5 platforms (HARD)
- [ ] PyPI publishing pipeline live (HARD)
- [ ] Type stubs + mypy strict pass (HARD)
- [ ] ≥ 100 unit + 20 integration tests
- [ ] Reproducible wheels (bit-identical) (HARD)
- [ ] `docs/sdk-python.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P4.17** | Bug-bounty integrations with Python tooling |
| **P4.18** | E2E measures axiom-py throughput |
| **External Python users** | First Pythonic APK-cert verifier |
