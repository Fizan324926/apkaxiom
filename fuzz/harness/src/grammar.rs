// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! APK grammar loader.
//!
//! The grammar lives at `fuzz/grammars/apk-v1.lark` (Lark / EBNF
//! shape so a future Nautilus integration can ingest it directly).
//! In this sub-phase the harness only **loads** the grammar and
//! exposes a `is_valid_grammar` / `productions_count` API — full
//! grammar-aware mutation is a Nautilus-/Centipede-driven
//! activity gated on the operator one-shot at CHECKLIST §C-3.
//!
//! Loadability is enough to:
//!   1. assert the grammar file is byte-stable under regen;
//!   2. count productions for the dashboard "grammar surface" panel;
//!   3. fail the build if the grammar gets corrupted.

use std::path::Path;

/// Parsed grammar handle. Cheap; just counts productions.
#[derive(Debug, Clone)]
pub struct Grammar {
    /// Total number of `name:` productions in the grammar.
    pub productions: usize,
    /// Total number of terminals (`/.../`-quoted regex literals
    /// or `"..."` literals).
    pub terminals: usize,
    /// Original file path.
    pub source: String,
}

impl Grammar {
    /// Load the grammar from a `.lark` file.
    ///
    /// # Errors
    ///
    /// I/O failure or no productions found.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let bytes = std::fs::read(path)?;
        let s = std::str::from_utf8(&bytes)
            .map_err(|e| std::io::Error::other(format!("grammar not utf-8: {e}")))?;
        Self::parse(s, path.display().to_string())
    }

    /// Parse a grammar from an in-memory string.
    ///
    /// # Errors
    ///
    /// No productions found.
    pub fn parse(s: &str, source: String) -> std::io::Result<Self> {
        let mut productions = 0usize;
        let mut terminals = 0usize;
        for raw in s.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
                continue;
            }
            // Lark productions: `name : ...` or `NAME: ...` (the
            // latter for terminals). Be permissive here; we're
            // counting, not parsing.
            if let Some((lhs, _)) = line.split_once(':') {
                let lhs = lhs.trim();
                if lhs.is_empty() {
                    continue;
                }
                if lhs
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
                {
                    terminals += 1;
                } else if lhs
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
                {
                    productions += 1;
                }
            }
        }
        if productions == 0 && terminals == 0 {
            return Err(std::io::Error::other(
                "grammar has no productions or terminals",
            ));
        }
        Ok(Self {
            productions,
            terminals,
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_grammar() {
        let s = "
        // a comment
        archive : lfh_seq cdr_seq eocd
        lfh_seq : LFH+
        cdr_seq : CDR+

        LFH    : /\\x50\\x4b\\x03\\x04/
        CDR    : /\\x50\\x4b\\x01\\x02/
        EOCD   : /\\x50\\x4b\\x05\\x06/
        ";
        let g = Grammar::parse(s, "test".into()).unwrap();
        assert!(g.productions >= 3);
        assert!(g.terminals >= 3);
    }

    #[test]
    fn empty_grammar_rejected() {
        assert!(Grammar::parse("", "empty".into()).is_err());
    }
}
