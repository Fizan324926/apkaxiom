# P4.14 — SDK: `axiom-go` (cgo + uniffi)

> Idiomatic Go API over `axiom-verify`. ≥ 200 verifications/sec/core. Distributed via Go module proxy + Homebrew + apt-get.

**Parent plan:** [../README.md](../README.md) · **Architecture:** [../../../README.md §15 (SDK)](../../../README.md#sdk)

---

## 1. Identity

| Field | Value |
|---|---|
| Sub-phase ID | P4.14 |
| Owner(s) | G14 |
| Duration | Weeks 14–18 |
| Critical-path | yes |
| Hard prerequisites | P4.11 (verifier core) |

## 2. Goal & Scope

A Go SDK over `axiom-verify`. cgo for Go ↔ Rust FFI; uniffi for declarative cross-language source. ≥ 200 verifications/sec/core (Go's lower FFI overhead lets us be much faster than Python). Distributed via Go module proxy + Homebrew + apt-get.

### In scope
- `sdk/axiom-go` — Go package
- cgo bindings (or uniffi-generated)
- Go-idiomatic API (channels, contexts)
- Distribution: Go module proxy + Homebrew + apt-get
- Comprehensive test suite (Go test framework)

### Out of scope
- Pure-Go alternative
- Other SDKs

## 3. Hard Dependencies on Prior Sub-Phases

| Source | Artifact consumed |
|---|---|
| **P4.11** | Verifier core |
| **P4.1** | uniffi installed |

## 4. Required Tools, Libraries, and Languages

| Tool | Version | Purpose |
|---|---|---|
| **Go** | 1.23+ | Target |
| **cgo** | bundled | Go ↔ C FFI |
| **uniffi-bindgen-go** | latest | Go binding generator |
| **goreleaser** | latest | Multi-platform release |
| **Homebrew** | system | macOS distribution |
| **dh-make** / **fpm** | latest | apt-get / rpm packages |

## 5. Third-Party Software, Services, Accounts & API Keys

| Item | Type | Free / Paid | URL | Notes |
|---|---|---|---|---|
| **Go module proxy** | distribution | **Free** | https://proxy.golang.org | Standard |
| **Homebrew core / Homebrew tap** | distribution | **Free** | https://brew.sh | Free; core acceptance is competitive |
| **GitHub Releases** | binary distribution | **Free** | already provisioned | Pre-built binaries |
| **Linux package repos (PPA / Copr)** *(optional)* | distribution | **Free** | https://launchpad.net / https://copr.fedoraproject.org | For apt-get / dnf-installable packages |

**No new API keys.** Module proxy is automatic; PPA/Copr account creation needed.

## 6. System Inventory — Have vs Need

### Already present
- ✅ Rust + uniffi
- ❓ Go — `go version` failed earlier; let me reinstall

### Missing — must install
- ❌ **Go 1.23+** — `apt-get install -y golang-1.23` or download from go.dev
- ❌ **goreleaser** — `go install github.com/goreleaser/goreleaser/v2@latest`
- ❌ **uniffi-bindgen-go** — `cargo install uniffi-bindgen-go`

```bash
# Go
sudo add-apt-repository ppa:longsleep/golang-backports -y
sudo apt-get update && sudo apt-get install -y golang-1.23

# goreleaser
go install github.com/goreleaser/goreleaser/v2@latest

# uniffi-bindgen-go
cargo install uniffi-bindgen-go
```

## 7. Features & Functions Delivered (Comprehensive)

### Go API
```go
import "github.com/Fizan324926/apkaxiom/axiom-go"

cert, err := axiomgo.LoadCert("report.axc")
if err != nil { /* ... */ }
result, err := axiomgo.Verify(cert, apkBytes)
fmt.Println(result.OK, result.Claims, result.AuditLog)
```

- `axiomgo.LoadCert(path string) (*Cert, error)`
- `axiomgo.Verify(cert *Cert, apkBytes []byte) (*VerifyResult, error)`
- `axiomgo.VerifyAsync(ctx context.Context, cert *Cert) (<-chan *VerifyResult, error)` — channel-based async
- `axiomgo.VerifyBatch(certs []*Cert) []VerifyResult` — concurrent batch

### Idiomatic patterns
- `context.Context` for cancellation
- Channels for streaming results
- Errors via Go's idiomatic `error` interface (not panics)
- Native `[]byte` rather than wrapped types

### Distribution
- Go module: `github.com/Fizan324926/apkaxiom/axiom-go`
- Homebrew tap (initially): `brew install Fizan324926/apkaxiom/axiom-verify`
- apt-get / Copr packages
- Single binary `axiom-verify-go` for CLI users
- goreleaser builds for x86_64 + ARM64 Linux/macOS/Windows

### Test suite
- Go testing framework
- ≥ 100 unit tests
- ≥ 20 integration tests
- Race-detector clean (`go test -race`)

### Documentation
- `docs/sdk-go.md`
- godoc auto-generated

## 8. KPIs (this sub-phase)

| KPI | HARD | TARGET |
|---|---|---|
| Single-core verifications/sec via `axiom-go` | ≥ 200 | ≥ 800 |
| FFI overhead vs native Rust | < 30 % | < 10 % |
| Distribution via Go module proxy | live | live |
| Distribution via Homebrew | live | tap accepted |
| Race-detector clean | yes | yes |
| Test suite pass | 100 % | 100 % |
| Reproducible binaries | bit-identical | yes |

## 9. Working Directory & Files Produced

```
apkaxiom/
├── sdk/
│   └── axiom-go/
│       ├── go.mod
│       ├── go.sum
│       ├── BUCK
│       ├── axiomgo/
│       │   ├── axiomgo.go
│       │   ├── cert.go
│       │   ├── verify.go
│       │   └── ffi.go                    # cgo bridge
│       ├── cmd/axiom-verify-go/
│       │   └── main.go
│       └── internal/
│           └── ffi_test.go
├── .goreleaser.yaml
└── docs/
    └── sdk-go.md
```

## 10. Standalone Output

```bash
cd sdk/axiom-go && go build ./...
go run ./cmd/axiom-verify-go report.axc
# ✓ Verified — 12 claims, all valid, 38ms
```

## 11. End-to-End Test

```bash
buck2 test //sdk/axiom-go:full
# - Throughput ≥ 200/sec/core (HARD)
# - FFI overhead < 30% (HARD)
# - Race-detector clean (HARD)
# - goreleaser produces ≥ 6-platform binaries (HARD)
```

## 12. Exit Checklist

- [ ] `axiom-go` Go module compiles
- [ ] cgo (or uniffi-generated) bindings operational
- [ ] Throughput ≥ 200/sec/core (HARD)
- [ ] FFI overhead < 30 % (HARD)
- [ ] Race-detector clean (HARD)
- [ ] Distribution via Go module proxy live
- [ ] Homebrew tap published (acceptance can be slow; initial tap fine)
- [ ] ≥ 100 unit + 20 integration tests
- [ ] goreleaser builds reproducibly
- [ ] `docs/sdk-go.md` published

## 13. Hand-Off

| Consumed by | What they need |
|---|---|
| **P4.17** | Bug-bounty integrations with Go tooling |
| **P4.18** | E2E measures axiom-go throughput |
| **External Go users** | First production Go API for proof-carrying APKs |
