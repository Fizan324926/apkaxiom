# APKAXIOM Rebuilder Federation — Registry

> Per [ADR-0011](../ADR-0011-rebuilder-federation.md). Public roster of
> independent third parties who have rebuilt APKAXIOM from source and
> published a signed attestation matching (or, equally valuable,
> diverging from) the canonical reference hashes.

To attest as a rebuilder, see the operator workflow in
[ADR-0011 §"Operator workflow"](../ADR-0011-rebuilder-federation.md).

This file is the **trust set in plain sight**: anyone reading it can
verify each entry's bundle, re-derive the OIDC identity from the Fulcio
cert, and re-check the Rekor log entry.

## Schema

| Field | Meaning |
|-------|---------|
| Operator | Public identity (GitHub handle, ORCID, etc.) |
| Platform | One of `linux-x86_64`, `linux-aarch64`, `darwin-arm64` |
| git SHA | The APKAXIOM commit they attested |
| Result | `pass` / `diverged` |
| Bundle URL | Where the `.bundle` (cosign keyless) is hosted |
| Verified by | The G13 reviewer who admitted this entry |

## Roster

| Operator | Platform | git SHA | Result | Bundle | Verified by |
|----------|----------|---------|--------|--------|-------------|
| _empty — first attestations land after CI is live_ | — | — | — | — | — |

---

## How verification proceeds

For any row, anyone can run:

```bash
# Replace <bundle-url> + <hash-file> with the values from the roster row.
curl -sSL <bundle-url> -o /tmp/attestation.bundle
cosign verify-blob \
  --bundle /tmp/attestation.bundle \
  --certificate-identity-regexp '.+' \
  --certificate-oidc-issuer-regexp 'token\.actions\.githubusercontent\.com|accounts\.google\.com|gitlab\.com' \
  <attestation-json>
# If verify-blob exits 0, the attestation's signature checks out.
```

The G13 reviewer ran this exact command before adding any row.

## How to challenge an entry

Open a PR removing the row with a commit message of the form:

```
P1.1: revoke rebuilder attestation <operator>/<platform>/<sha>

Reason: <evidence — e.g. cosign verify-blob fails today, Rekor entry not
found, operator has retracted the attestation, etc.>
```

A G13 reviewer makes the call.
