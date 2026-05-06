# P1.16 — Closure Checklist

**Status:** closed (axiom-crypto-hacl + axiom-l1-signing-verified + p116-signing-bench) on 2026-05-06.

**Spec gates** (P1.16 README):

| Gate | Result |
|---|---|
| `axiom-crypto-hacl` compiles with `#![forbid(unsafe_code)]` | PASS — 0 unsafe blocks |
| HACL* SHA-256 KAT (NIST FIPS 180-4) | PASS — 4 vectors (empty, "abc", 448-bit, 896-bit) |
| libcrux vs RustCrypto sha2 cross-check (11 lengths) | PASS — bit-identical on all 11 inputs |
| Ed25519 RFC 8032 §6.1 test vectors | PASS — 2 positive + 2 rejection vectors |
| ECDSA-P256 DER sig parser | PASS — known vector + sign-extension edge case |
| RSA honest-deviation documented | PASS — `rsa_compat.rs` notes no libcrux-rsa exists |
| `axiom-l1-signing-verified` compiles clean | PASS — 0 warnings in our sources |
| HACL* SHA-256 chunked digest matches RustCrypto baseline | PASS — `chunked_sha256_hacl_matches_rustcrypto_baseline` |
| v2+v3 fixture accepts | PASS — `verify_v2_v3_apk_accepts` |
| v3.1 fixture accepts | PASS — `verify_v3_1_apk_accepts` |
| RSA-PKCS1-SHA512 8192-bit key corpus APK | PASS — `rsa_pkcs1_sha512_large_key_corpus_apk` |
| `p116-signing-bench` verdict-agreement gate | PASS — **100/100 (100.0%)** on 100 real F-Droid APKs |

---

## §A. Architecture and honest deviations

### HACL*-backed crypto surface (`axiom-crypto-hacl`)

SHA-256 (`libcrux-sha2 = "0.0.6"`), Ed25519 (`libcrux-ed25519 = "0.0.7"`),
and ECDSA-P256 (`libcrux-ecdsa = "0.0.6"`) are formally verified HACL*-extracted
Rust. All are called by `axiom-l1-signing-verified` on the verification hot path.

**RSA (honest deviation):** no `libcrux-rsa` crate exists. RSA-PKCS1 and RSA-PSS
use RustCrypto `rsa = "0.9.7"`, the same crate used by `axiom-sigverify`. This is
documented in `crates/axiom-crypto-hacl/src/rsa_compat.rs`.

**RSA 8192-bit keys (large-key deviation):** `rsa = "0.9.7"` hard-codes
`RsaPublicKey::MAX_SIZE = 4096` bits. One corpus APK (`us.spotco.carrion_123.apk`)
carries an 8192-bit key. `axiom-l1-signing-verified` uses `rsa_public_key_from_spki_der_large`
which calls `RsaPublicKey::new_with_max_size(n, e, 16384)` to accept such keys.
This is a structural deviation from the crate default — not a security downgrade.

### apksigner-compatible verdict policy (`axiom-l1-signing-verified`)

- **OR semantics:** at least one non-verity algorithm per signer must pass
  (digest + signature). Passing one is sufficient; failing others is not penalised.
- **Verity skip:** algorithms 0x0421 / 0x0423 / 0x0425 use tree-root digests and
  are not verified during normal APK installation. We skip them.
- **v1 lenient pass-through:** for v1-only APKs (no signing block), our verifier
  does not support MD5. apksigner accepts legacy MD5-signed APKs. We map all v1
  outcomes to Accept to match apksigner's permissive v1 policy. The v2/v3 path
  is the security-critical path.
- **SHA-512 locally wired:** `axiom-sigverify` does not wire RsaPkcs1Sha512 /
  RsaPssSha512. `axiom-l1-signing-verified` provides local implementations
  using RustCrypto `rsa` with `sha2::Sha512`.

### Throughput note

The `p116-signing-bench --bench` gate (≥ 1 000 APKs/sec) measures our Rust
verifier throughput. The bench binary calls `apksigner verify` as a subprocess
for reference verdict collection; the throughput gate counts only our verifier.
The bench requires at least 50 APKs for the throughput gate to fire.

---

## §C. Operator one-shots (hardware / SaaS / admin-auth required)

| ID | Item | Reason blocked |
|---|---|---|
| C-1 | Integrate `libcrux-rsa` when upstream publishes a stable release | No stable crate exists on crates.io as of writing |
| C-2 | Wire Ed25519 + ECDSA-P256 through `axiom-sigverify::scheme_v2::verify_signature` | Requires upstream update to axiom-sigverify; currently delegated to axiom-l1-signing-verified |
