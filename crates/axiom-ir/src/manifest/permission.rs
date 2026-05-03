// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Permissions: declarations and references.

/// A permission *declared* by this app via `<permission>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permission {
    /// Fully-qualified permission name.
    pub name: String,
    /// Protection level — gates how aggressively Android enforces it.
    pub protection: ProtectionLevel,
    /// Optional permission group (`<permission android:permissionGroup>`).
    pub group: Option<String>,
}

/// Android permission protection levels.
///
/// The set is closed for v0.1 — new levels (e.g. `Knownsigner`,
/// `Privileged`) require a schema-version bump.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProtectionLevel {
    /// `normal` — auto-granted at install time. Lowest risk.
    Normal,
    /// `dangerous` — requires runtime user grant.
    Dangerous,
    /// `signature` — only apps signed with the same cert may hold.
    Signature,
    /// `signatureOrSystem` — legacy compound level.
    SignatureOrSystem,
    /// `internal` — system-internal only (added in API 31+).
    Internal,
}

/// Reference to a permission — either symbolic ("permission name") or
/// resolved via the resource dialect.
///
/// Lowering ([`crate::lowering::resolve`]) consumes a [`PermissionRef`]
/// and either threads it through unchanged (symbolic) or replaces a
/// `@string/...`-style reference with the literal string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionRef {
    /// Symbolic permission — the manifest declares it directly.
    Symbolic(String),
    /// Resource-resolved permission name — produced by lowering.
    Resolved(String),
}

impl ProtectionLevel {
    /// Returns `true` if this level admits arbitrary third-party apps.
    /// `normal` and `dangerous` are the third-party-grantable levels.
    #[must_use]
    pub const fn is_grantable_to_third_parties(self) -> bool {
        matches!(self, Self::Normal | Self::Dangerous)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_is_not_third_party_grantable() {
        assert!(!ProtectionLevel::Signature.is_grantable_to_third_parties());
        assert!(!ProtectionLevel::Internal.is_grantable_to_third_parties());
        assert!(ProtectionLevel::Normal.is_grantable_to_third_parties());
        assert!(ProtectionLevel::Dangerous.is_grantable_to_third_parties());
    }
}
