// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! P1.9 Lean lexer for the verified-parser subset.
//!
//! Tokenizes the subset of Lean 4 used in
//! `theorems/Apkaxiom/Zip/LocalHeader.lean`. Handles:
//!
//!   - ASCII identifiers (with `.` for qualified names)
//!   - Decimal and hex integer literals
//!   - Block comments `/- ... -/` (nestable per Lean spec)
//!   - Line comments `--`
//!   - Punctuation `( ) { } [ ] # : ; , .`
//!   - ASCII operators `:= = + - * / < > <= >= ! | ||| <<< &&& ^^^`
//!   - Unicode operators `≠` (U+2260), `≤` (U+2264), `≥` (U+2265),
//!     `→` (U+2192), `←` (U+2190), `×` (U+00D7)
//!   - Keywords: `def structure inductive theorem instance example
//!     where with do let match if then else return fun by decide
//!     native_decide deriving namespace end open import some none
//!     ok error true false`
//!
//! Tokens carry source spans so the parser can report errors with
//! line / column numbers on rejection.

#![allow(clippy::too_long_first_doc_paragraph)]

use std::fmt;

/// One source-position token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// What kind of token.
    pub kind: TokKind,
    /// Source line (1-indexed).
    pub line: usize,
    /// Source column (1-indexed, byte offset).
    pub col: usize,
}

/// All token shapes the lexer emits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokKind {
    /// Identifier — including dotted (`Apkaxiom.Zip.foo`) and reserved
    /// words. The parser distinguishes keywords from idents at use site.
    Ident(String),
    /// Decimal or hex integer literal, stored as the raw substring
    /// (so `0x04034b50` keeps its hex form for emission).
    IntLit(String),
    /// String literal contents (escape-decoded by the parser; the
    /// lexer emits the raw inner string).
    StrLit(String),

    // Punctuation
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `#`
    Hash,
    /// `:`
    Colon,
    /// `;`
    Semicolon,
    /// `,`
    Comma,
    /// `.`
    Dot,
    /// `..`
    DotDot,
    /// `|`
    Pipe,

    // Operators
    /// `:=`
    Assign,
    /// `=`
    Eq,
    /// `==`
    EqEq,
    /// `≠` or `!=`
    NEq,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `≤` or `<=`
    Le,
    /// `≥` or `>=`
    Ge,
    /// `→` or `->`
    Arrow,
    /// `←` or `<-`
    LArrow,
    /// `×`
    Times,
    /// `&&` or `∧`
    AndAnd,
    /// `||` or `∨`
    OrOr,
    /// `&&&` (BitAnd)
    BitAnd,
    /// `|||` (BitOr)
    BitOr,
    /// `^^^` (BitXor)
    BitXor,
    /// `<<<`
    Shl,
    /// `>>>`
    Shr,
    /// `!`
    Bang,

    /// Newline marker. The parser treats Lean as significant-newline
    /// only inside `do` blocks; everywhere else newlines are
    /// whitespace. We emit a single `Newline` per blank line so the
    /// `do`-block parser can find boundaries; multiple consecutive
    /// newlines collapse.
    Newline,
}

impl fmt::Display for TokKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokKind::Ident(s) => write!(f, "{s}"),
            TokKind::IntLit(s) => write!(f, "{s}"),
            TokKind::StrLit(s) => write!(f, "\"{s}\""),
            TokKind::LParen => write!(f, "("),
            TokKind::RParen => write!(f, ")"),
            TokKind::LBrace => write!(f, "{{"),
            TokKind::RBrace => write!(f, "}}"),
            TokKind::LBracket => write!(f, "["),
            TokKind::RBracket => write!(f, "]"),
            TokKind::Hash => write!(f, "#"),
            TokKind::Colon => write!(f, ":"),
            TokKind::Semicolon => write!(f, ";"),
            TokKind::Comma => write!(f, ","),
            TokKind::Dot => write!(f, "."),
            TokKind::DotDot => write!(f, ".."),
            TokKind::Pipe => write!(f, "|"),
            TokKind::Assign => write!(f, ":="),
            TokKind::Eq => write!(f, "="),
            TokKind::EqEq => write!(f, "=="),
            TokKind::NEq => write!(f, "≠"),
            TokKind::Plus => write!(f, "+"),
            TokKind::Minus => write!(f, "-"),
            TokKind::Star => write!(f, "*"),
            TokKind::Slash => write!(f, "/"),
            TokKind::Percent => write!(f, "%"),
            TokKind::Lt => write!(f, "<"),
            TokKind::Gt => write!(f, ">"),
            TokKind::Le => write!(f, "≤"),
            TokKind::Ge => write!(f, "≥"),
            TokKind::Arrow => write!(f, "→"),
            TokKind::LArrow => write!(f, "←"),
            TokKind::Times => write!(f, "×"),
            TokKind::AndAnd => write!(f, "&&"),
            TokKind::OrOr => write!(f, "||"),
            TokKind::BitAnd => write!(f, "&&&"),
            TokKind::BitOr => write!(f, "|||"),
            TokKind::BitXor => write!(f, "^^^"),
            TokKind::Shl => write!(f, "<<<"),
            TokKind::Shr => write!(f, ">>>"),
            TokKind::Bang => write!(f, "!"),
            TokKind::Newline => write!(f, "\\n"),
        }
    }
}

/// Lexer error — the parser surfaces these with source spans.
#[derive(Debug, thiserror::Error)]
pub enum LexError {
    /// Encountered a character we don't recognise as part of the
    /// supported sublanguage. Includes the offending char + position.
    #[error("lex error at {line}:{col}: unexpected character {ch:?}")]
    Unexpected {
        /// Source line.
        line: usize,
        /// Source column.
        col: usize,
        /// The rejected character.
        ch: char,
    },
    /// `/-` block comment with no matching `-/`.
    #[error("lex error: unterminated block comment starting at {line}:{col}")]
    UnterminatedBlockComment {
        /// Where the unterminated block started.
        line: usize,
        /// Column of the unterminated block.
        col: usize,
    },
    /// String literal with no closing quote.
    #[error("lex error at {line}:{col}: unterminated string literal")]
    UnterminatedString {
        /// Source line.
        line: usize,
        /// Source column.
        col: usize,
    },
}

/// Tokenize an entire source string. The output ends with no
/// terminator token — callers use `slice.iter()` and detect EOF via
/// length.
pub fn tokenize(src: &str) -> Result<Vec<Token>, LexError> {
    let mut out = Vec::new();
    let mut line = 1usize;
    let mut col = 1usize;
    let bytes = src.as_bytes();
    let mut i = 0usize;
    let mut last_was_newline = false;

    while i < bytes.len() {
        let start_line = line;
        let start_col = col;
        let c = bytes[i] as char;

        // ---------- whitespace + newlines ----------
        if c == '\n' {
            if !last_was_newline {
                out.push(Token {
                    kind: TokKind::Newline,
                    line,
                    col,
                });
                last_was_newline = true;
            }
            i += 1;
            line += 1;
            col = 1;
            continue;
        }
        if c.is_ascii_whitespace() {
            i += 1;
            col += 1;
            continue;
        }

        // ---------- comments ----------
        if c == '-' && i + 1 < bytes.len() && bytes[i + 1] as char == '-' {
            // line comment
            while i < bytes.len() && bytes[i] as char != '\n' {
                i += 1;
                col += 1;
            }
            continue;
        }
        if c == '/' && i + 1 < bytes.len() && bytes[i + 1] as char == '-' {
            // nestable block comment
            let mut depth = 1usize;
            i += 2;
            col += 2;
            while i < bytes.len() && depth > 0 {
                let ch = bytes[i] as char;
                if ch == '\n' {
                    line += 1;
                    col = 1;
                    i += 1;
                    continue;
                }
                if ch == '/' && i + 1 < bytes.len() && bytes[i + 1] as char == '-' {
                    depth += 1;
                    i += 2;
                    col += 2;
                    continue;
                }
                if ch == '-' && i + 1 < bytes.len() && bytes[i + 1] as char == '/' {
                    depth -= 1;
                    i += 2;
                    col += 2;
                    continue;
                }
                i += 1;
                col += 1;
            }
            if depth > 0 {
                return Err(LexError::UnterminatedBlockComment {
                    line: start_line,
                    col: start_col,
                });
            }
            continue;
        }

        last_was_newline = false;

        // ---------- ASCII identifiers ----------
        if c == '_' || c.is_ascii_alphabetic() {
            let start = i;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                if ch == '_' || ch.is_ascii_alphanumeric() {
                    i += 1;
                    col += 1;
                } else if ch == '.' && i + 1 < bytes.len() {
                    let next = bytes[i + 1] as char;
                    // dotted ident only if next char starts a new ident segment
                    if next == '_' || next.is_ascii_alphabetic() {
                        i += 1;
                        col += 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            // Lean's method-style names allow trailing `!` (e.g.
            // `ByteArray.get!`) or `?` as part of the identifier.
            // Only consume `!` when it's NOT followed by `=` (so
            // that `foo != bar` still lexes as ident + NEq).
            while i < bytes.len() {
                let ch = bytes[i];
                if ch == b'?' {
                    i += 1;
                    col += 1;
                } else if ch == b'!' {
                    // Don't eat `!` if it starts `!=` operator.
                    if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                        break;
                    }
                    i += 1;
                    col += 1;
                } else {
                    break;
                }
            }
            let s = src[start..i].to_string();
            out.push(Token {
                kind: TokKind::Ident(s),
                line: start_line,
                col: start_col,
            });
            continue;
        }

        // ---------- numeric literals ----------
        if c.is_ascii_digit() {
            let start = i;
            // hex prefix
            if c == '0' && i + 1 < bytes.len() && bytes[i + 1] as char == 'x' {
                i += 2;
                col += 2;
                while i < bytes.len() && (bytes[i] as char).is_ascii_hexdigit() {
                    i += 1;
                    col += 1;
                }
            } else {
                while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                    i += 1;
                    col += 1;
                }
            }
            let s = src[start..i].to_string();
            out.push(Token {
                kind: TokKind::IntLit(s),
                line: start_line,
                col: start_col,
            });
            continue;
        }

        // ---------- string literals ----------
        if c == '"' {
            let start_off = i;
            i += 1;
            col += 1;
            while i < bytes.len() && bytes[i] as char != '"' {
                if bytes[i] as char == '\\' && i + 1 < bytes.len() {
                    i += 2;
                    col += 2;
                    continue;
                }
                if bytes[i] as char == '\n' {
                    return Err(LexError::UnterminatedString {
                        line: start_line,
                        col: start_col,
                    });
                }
                i += 1;
                col += 1;
            }
            if i >= bytes.len() {
                return Err(LexError::UnterminatedString {
                    line: start_line,
                    col: start_col,
                });
            }
            let raw = src[start_off + 1..i].to_string();
            i += 1;
            col += 1;
            out.push(Token {
                kind: TokKind::StrLit(raw),
                line: start_line,
                col: start_col,
            });
            continue;
        }

        // ---------- multi-byte unicode operators ----------
        if c as u32 >= 0x80 {
            // Decode one UTF-8 codepoint.
            let (cp, len) = decode_utf8(&bytes[i..]);
            let kind = match cp {
                0x2260 => Some(TokKind::NEq),    // ≠
                0x2264 => Some(TokKind::Le),     // ≤
                0x2265 => Some(TokKind::Ge),     // ≥
                0x2192 => Some(TokKind::Arrow),  // →
                0x2190 => Some(TokKind::LArrow), // ←
                0x00d7 => Some(TokKind::Times),  // ×
                0x2227 => Some(TokKind::AndAnd), // ∧
                0x2228 => Some(TokKind::OrOr),   // ∨
                _ => None,
            };
            if let Some(k) = kind {
                out.push(Token {
                    kind: k,
                    line: start_line,
                    col: start_col,
                });
                i += len;
                col += 1;
                continue;
            }
            return Err(LexError::Unexpected {
                line,
                col,
                ch: char::from_u32(cp).unwrap_or('?'),
            });
        }

        // ---------- ASCII multi-char operators (longest match) ----------
        if let Some((kind, n)) = match_op(&bytes[i..]) {
            out.push(Token {
                kind,
                line: start_line,
                col: start_col,
            });
            i += n;
            col += n;
            continue;
        }

        // ---------- single-char punctuation ----------
        let kind = match c {
            '(' => TokKind::LParen,
            ')' => TokKind::RParen,
            '{' => TokKind::LBrace,
            '}' => TokKind::RBrace,
            '[' => TokKind::LBracket,
            ']' => TokKind::RBracket,
            '#' => TokKind::Hash,
            ':' => TokKind::Colon,
            ';' => TokKind::Semicolon,
            ',' => TokKind::Comma,
            '.' => TokKind::Dot,
            '+' => TokKind::Plus,
            '-' => TokKind::Minus,
            '*' => TokKind::Star,
            '/' => TokKind::Slash,
            '%' => TokKind::Percent,
            '<' => TokKind::Lt,
            '>' => TokKind::Gt,
            '=' => TokKind::Eq,
            '|' => TokKind::Pipe,
            '!' => TokKind::Bang,
            _ => {
                return Err(LexError::Unexpected { line, col, ch: c });
            }
        };
        out.push(Token {
            kind,
            line: start_line,
            col: start_col,
        });
        i += 1;
        col += 1;
    }
    Ok(out)
}

/// Try to match a multi-byte ASCII operator at `bytes[0..]`. Returns
/// the matched token kind and consumed byte count, or `None`.
fn match_op(bytes: &[u8]) -> Option<(TokKind, usize)> {
    // Longest-match first — order matters.
    let s = std::str::from_utf8(bytes).ok()?;
    for (lit, kind) in [
        ("|||", TokKind::BitOr),
        ("&&&", TokKind::BitAnd),
        ("^^^", TokKind::BitXor),
        ("<<<", TokKind::Shl),
        (">>>", TokKind::Shr),
        (":=", TokKind::Assign),
        ("==", TokKind::EqEq),
        ("!=", TokKind::NEq),
        ("<=", TokKind::Le),
        (">=", TokKind::Ge),
        ("->", TokKind::Arrow),
        ("=>", TokKind::Arrow),
        ("<-", TokKind::LArrow),
        ("&&", TokKind::AndAnd),
        ("||", TokKind::OrOr),
        ("..", TokKind::DotDot),
    ] {
        if s.starts_with(lit) {
            return Some((kind, lit.len()));
        }
    }
    None
}

/// Decode one UTF-8 codepoint from `bytes[0..]`. Returns
/// `(codepoint, byte-length)`. Returns `(0xFFFD, 1)` on invalid UTF-8
/// to fail loudly downstream rather than silently consuming a single
/// byte.
fn decode_utf8(bytes: &[u8]) -> (u32, usize) {
    if bytes.is_empty() {
        return (0xFFFD, 0);
    }
    let b0 = bytes[0];
    if b0 < 0x80 {
        return (b0 as u32, 1);
    }
    let (n, mask) = match b0 {
        0xC0..=0xDF => (2usize, 0x1F),
        0xE0..=0xEF => (3usize, 0x0F),
        0xF0..=0xF7 => (4usize, 0x07),
        _ => return (0xFFFD, 1),
    };
    if bytes.len() < n {
        return (0xFFFD, 1);
    }
    let mut cp = (b0 as u32) & mask;
    for &b in &bytes[1..n] {
        if b & 0xC0 != 0x80 {
            return (0xFFFD, 1);
        }
        cp = (cp << 6) | ((b as u32) & 0x3F);
    }
    (cp, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(s: &str) -> Vec<TokKind> {
        tokenize(s)
            .unwrap()
            .into_iter()
            .filter(|t| t.kind != TokKind::Newline)
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn ident_and_dotted() {
        assert_eq!(
            lex("foo Bar.baz"),
            vec![
                TokKind::Ident("foo".into()),
                TokKind::Ident("Bar.baz".into()),
            ]
        );
    }

    #[test]
    fn hex_and_dec_literals() {
        assert_eq!(
            lex("42 0x04034b50"),
            vec![
                TokKind::IntLit("42".into()),
                TokKind::IntLit("0x04034b50".into()),
            ]
        );
    }

    #[test]
    fn ascii_and_unicode_operators() {
        // Note `≠` and `!=` are both lexed as NEq.
        assert_eq!(
            lex(":= ≠ != → -> ≤ <= |||"),
            vec![
                TokKind::Assign,
                TokKind::NEq,
                TokKind::NEq,
                TokKind::Arrow,
                TokKind::Arrow,
                TokKind::Le,
                TokKind::Le,
                TokKind::BitOr,
            ]
        );
    }

    #[test]
    fn nested_block_comments() {
        let src = "foo /- a /- b -/ c -/ bar";
        assert_eq!(
            lex(src),
            vec![TokKind::Ident("foo".into()), TokKind::Ident("bar".into()),]
        );
    }

    #[test]
    fn line_comments() {
        let src = "foo -- discard rest\nbar";
        assert_eq!(
            lex(src),
            vec![TokKind::Ident("foo".into()), TokKind::Ident("bar".into()),]
        );
    }

    #[test]
    fn unicode_operator_codepoints() {
        // Every Unicode operator we recognise round-trips through
        // `decode_utf8` correctly.
        let cases = [
            ("≠", TokKind::NEq),
            ("≤", TokKind::Le),
            ("≥", TokKind::Ge),
            ("→", TokKind::Arrow),
            ("←", TokKind::LArrow),
            ("×", TokKind::Times),
        ];
        for (src, expect) in cases {
            assert_eq!(lex(src), vec![expect.clone()], "for {src:?}");
        }
    }

    #[test]
    fn rejects_unknown_unicode() {
        assert!(matches!(tokenize("π"), Err(LexError::Unexpected { .. })));
    }
}
