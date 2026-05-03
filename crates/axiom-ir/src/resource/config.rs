// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Resource configurations — density / locale / orientation / sdk.

/// One row in the configuration matrix.
///
/// `qualifier` is the human-readable form (`"en-rUS-mdpi-port-v21"` etc.)
/// — its components are flattened into the typed fields below for ease
/// of analysis. Both representations are kept so canonical bytes can
/// round-trip the original AOSP-binary qualifier exactly.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Configuration {
    /// Original AOSP qualifier string (e.g. `"en-rUS-mdpi-port-v21"`).
    /// `"default"` for the unqualified fallback.
    pub qualifier: String,
    /// Density in DPI. 0 means "any/unspecified".
    pub density_dpi: u32,
    /// BCP-47 locale tag (`"en-US"`). `None` means "any".
    pub locale: Option<String>,
    /// `"port"` / `"land"` / `"square"`. `None` means "any".
    pub orientation: Option<String>,
    /// Minimum SDK level the configuration was generated against.
    pub min_sdk: u8,
}

impl Configuration {
    /// Construct an unqualified default configuration for the given SDK.
    #[must_use]
    pub fn default_for_sdk(sdk: u8) -> Self {
        Self {
            qualifier: "default".into(),
            density_dpi: 0,
            locale: None,
            orientation: None,
            min_sdk: sdk,
        }
    }

    /// Builder: set the qualifier string.
    #[must_use]
    pub fn with_qualifier(mut self, q: impl Into<String>) -> Self {
        self.qualifier = q.into();
        self
    }

    /// Builder: set density.
    #[must_use]
    pub const fn with_density(mut self, dpi: u32) -> Self {
        self.density_dpi = dpi;
        self
    }

    /// Builder: set locale.
    #[must_use]
    pub fn with_locale(mut self, l: impl Into<String>) -> Self {
        self.locale = Some(l.into());
        self
    }

    /// Builder: set orientation.
    #[must_use]
    pub fn with_orientation(mut self, o: impl Into<String>) -> Self {
        self.orientation = Some(o.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_unspecified() {
        let c = Configuration::default_for_sdk(21);
        assert_eq!(c.qualifier, "default");
        assert_eq!(c.density_dpi, 0);
        assert!(c.locale.is_none());
        assert!(c.orientation.is_none());
        assert_eq!(c.min_sdk, 21);
    }
}
