// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Indexed string pool for the resource dialect.

/// String pool — an ordered list of UTF-8 strings.
///
/// Within v0.1 the pool is a plain `Vec<String>`. We do **not** model
/// AOSP's encoded-style runs (e.g. quoted-string escape sequences) at
/// this layer — string-pool decoding is an L1 concern; the IR receives
/// already-decoded UTF-8.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StringPool {
    /// Strings in pool index order.
    pub strings: Vec<String>,
}

impl StringPool {
    /// Empty pool.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            strings: Vec::new(),
        }
    }

    /// Append a string and return its pool index.
    pub fn intern(&mut self, s: impl Into<String>) -> u32 {
        let idx = self.strings.len() as u32;
        self.strings.push(s.into());
        idx
    }

    /// Look up by index.
    #[must_use]
    pub fn get(&self, idx: u32) -> Option<&str> {
        self.strings.get(idx as usize).map(String::as_str)
    }

    /// Number of strings in the pool.
    #[must_use]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Returns `true` if the pool is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_returns_monotonic_indices() {
        let mut p = StringPool::new();
        let i = p.intern("a");
        let j = p.intern("b");
        assert_eq!(i, 0);
        assert_eq!(j, 1);
        assert_eq!(p.get(0), Some("a"));
        assert_eq!(p.get(1), Some("b"));
    }
}
