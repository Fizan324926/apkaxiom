// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Intent filters and their data sub-elements.

/// A single `<intent-filter>` block.
///
/// Stable for v0.1. The order of `actions` / `categories` / `data` is the
/// source order — *not* sorted — because Android's matching rules are
/// order-insensitive but the source order is meaningful for forensic
/// fingerprinting.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IntentFilter {
    /// `<action android:name="…"/>` values.
    pub actions: Vec<String>,
    /// `<category android:name="…"/>` values.
    pub categories: Vec<String>,
    /// `<data .../>` filters.
    pub data: Vec<DataFilter>,
    /// Filter priority (Android-default 0; higher wins).
    pub priority: i32,
}

/// `<data>` matcher inside an [`IntentFilter`].
///
/// Every field is optional — Android's matching algorithm allows any
/// subset to be specified.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DataFilter {
    /// `android:scheme="https"` — e.g. `"http"`, `"https"`, `"content"`.
    pub scheme: Option<String>,
    /// `android:host="example.com"`.
    pub host: Option<String>,
    /// `android:port="443"`.
    pub port: Option<String>,
    /// `android:path="/exact"`.
    pub path: Option<String>,
    /// `android:pathPrefix="/api/"`.
    pub path_prefix: Option<String>,
    /// `android:pathPattern="/.*"`.
    pub path_pattern: Option<String>,
    /// `android:mimeType="image/png"`.
    pub mime_type: Option<String>,
}

impl IntentFilter {
    /// Construct an empty filter.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            actions: Vec::new(),
            categories: Vec::new(),
            data: Vec::new(),
            priority: 0,
        }
    }

    /// Builder: append an action.
    #[must_use]
    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.actions.push(action.into());
        self
    }

    /// Builder: append a category.
    #[must_use]
    pub fn with_category(mut self, cat: impl Into<String>) -> Self {
        self.categories.push(cat.into());
        self
    }

    /// Builder: append a data filter.
    #[must_use]
    pub fn with_data(mut self, d: DataFilter) -> Self {
        self.data.push(d);
        self
    }

    /// Returns `true` if this filter is browsable (has VIEW + DEFAULT +
    /// BROWSABLE categories with an http/https scheme — the canonical
    /// "deep link" shape).
    #[must_use]
    pub fn is_browsable_deeplink(&self) -> bool {
        let has_view = self
            .actions
            .iter()
            .any(|a| a == "android.intent.action.VIEW");
        let has_default = self
            .categories
            .iter()
            .any(|c| c == "android.intent.category.DEFAULT");
        let has_browsable = self
            .categories
            .iter()
            .any(|c| c == "android.intent.category.BROWSABLE");
        let has_http = self
            .data
            .iter()
            .any(|d| matches!(d.scheme.as_deref(), Some("http" | "https")));
        has_view && has_default && has_browsable && has_http
    }
}

impl DataFilter {
    /// Construct an empty data filter.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            scheme: None,
            host: None,
            port: None,
            path: None,
            path_prefix: None,
            path_pattern: None,
            mime_type: None,
        }
    }

    /// Builder: set scheme.
    #[must_use]
    pub fn with_scheme(mut self, s: impl Into<String>) -> Self {
        self.scheme = Some(s.into());
        self
    }

    /// Builder: set host.
    #[must_use]
    pub fn with_host(mut self, s: impl Into<String>) -> Self {
        self.host = Some(s.into());
        self
    }

    /// Builder: set path.
    #[must_use]
    pub fn with_path(mut self, s: impl Into<String>) -> Self {
        self.path = Some(s.into());
        self
    }

    /// Builder: set MIME type.
    #[must_use]
    pub fn with_mime(mut self, s: impl Into<String>) -> Self {
        self.mime_type = Some(s.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browsable_deeplink_recognised() {
        let f = IntentFilter::new()
            .with_action("android.intent.action.VIEW")
            .with_category("android.intent.category.DEFAULT")
            .with_category("android.intent.category.BROWSABLE")
            .with_data(
                DataFilter::new()
                    .with_scheme("https")
                    .with_host("example.com"),
            );
        assert!(f.is_browsable_deeplink());
    }

    #[test]
    fn missing_browsable_category_is_not_a_deeplink() {
        let f = IntentFilter::new()
            .with_action("android.intent.action.VIEW")
            .with_category("android.intent.category.DEFAULT")
            .with_data(DataFilter::new().with_scheme("https"));
        assert!(!f.is_browsable_deeplink());
    }
}
