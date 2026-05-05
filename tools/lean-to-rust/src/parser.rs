// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! P1.9 Lean parser for the verified-parser subset.
//!
//! Recursive-descent over the token stream produced by [`crate::lexer`].
//! Parses the patterns actually used in
//! `theorems/Apkaxiom/Zip/LocalHeader.lean`. Anything outside the
//! supported shape is rejected with a span — the parser's job is to
//! ensure the translator never sees an unmodelled construct.

#![allow(missing_docs, clippy::too_long_first_doc_paragraph)]

use crate::ast::*;
use crate::lexer::{TokKind, Token};

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("parse error at {line}:{col}: expected {expected}, got {got}")]
    Unexpected {
        line: usize,
        col: usize,
        expected: String,
        got: String,
    },
    #[error("parse error at {line}:{col}: {message}")]
    Generic {
        line: usize,
        col: usize,
        message: String,
    },
    #[error("parse error: unexpected end of input (expected {expected})")]
    Eof { expected: String },
    #[error("parse error at {line}:{col}: unsupported construct ({reason}). The extractor handles only the verified-parser sublanguage; refactor or add support.")]
    Unsupported {
        line: usize,
        col: usize,
        reason: String,
    },
}

pub fn parse_module(tokens: &[Token]) -> Result<Module, ParseError> {
    let mut p = Parser::new(tokens);
    let mut items = Vec::new();
    let mut pending_doc: Option<String> = None;
    let mut last_i = usize::MAX;
    while !p.is_eof() {
        // Stuck-detector: if a parse_item call fails to advance the
        // cursor, we'd loop forever.
        if p.i == last_i {
            return Err(ParseError::Generic {
                line: p.peek().map(|t| t.line).unwrap_or(0),
                col: p.peek().map(|t| t.col).unwrap_or(0),
                message: format!(
                    "parse_item failed to advance at token #{} ({})",
                    p.i,
                    p.peek()
                        .map(|t| t.kind.to_string())
                        .unwrap_or_else(|| "?".into())
                ),
            });
        }
        last_i = p.i;
        if p.peek_kind() == Some(&TokKind::Newline) {
            p.bump();
            continue;
        }
        let _ = pending_doc.take();
        items.push(p.parse_item()?);
    }
    Ok(Module { items })
}

struct Parser<'a> {
    toks: &'a [Token],
    i: usize,
}

impl<'a> Parser<'a> {
    fn new(toks: &'a [Token]) -> Self {
        Self { toks, i: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        // Skip newlines for most lookups; specific spots that care
        // about newlines (do-block boundaries) call `peek_raw`.
        let mut j = self.i;
        while j < self.toks.len() && self.toks[j].kind == TokKind::Newline {
            j += 1;
        }
        self.toks.get(j)
    }

    fn peek_kind(&self) -> Option<&TokKind> {
        self.peek().map(|t| &t.kind)
    }

    fn peek_raw(&self) -> Option<&Token> {
        self.toks.get(self.i)
    }

    fn bump(&mut self) -> Option<&Token> {
        // Skip newlines first if peek-skips them.
        while self.i < self.toks.len() && self.toks[self.i].kind == TokKind::Newline {
            self.i += 1;
        }
        let tok = self.toks.get(self.i)?;
        self.i += 1;
        Some(tok)
    }

    fn bump_raw(&mut self) -> Option<&Token> {
        let tok = self.toks.get(self.i)?;
        self.i += 1;
        Some(tok)
    }

    fn is_eof(&self) -> bool {
        self.peek().is_none()
    }

    fn span_of(tok: Option<&Token>) -> (usize, usize) {
        tok.map_or((0, 0), |t| (t.line, t.col))
    }

    fn err_unexpected(&self, expected: &str) -> ParseError {
        let tok = self.peek();
        let (line, col) = Self::span_of(tok);
        let got = tok
            .map(|t| t.kind.to_string())
            .unwrap_or_else(|| "<eof>".to_string());
        if tok.is_none() {
            ParseError::Eof {
                expected: expected.to_string(),
            }
        } else {
            ParseError::Unexpected {
                line,
                col,
                expected: expected.to_string(),
                got,
            }
        }
    }

    fn err_unsupported(&self, reason: &str) -> ParseError {
        let (line, col) = Self::span_of(self.peek());
        ParseError::Unsupported {
            line,
            col,
            reason: reason.to_string(),
        }
    }

    fn expect_kind(&mut self, want: &TokKind, expected_label: &str) -> Result<(), ParseError> {
        match self.peek_kind() {
            Some(k) if k == want => {
                self.bump();
                Ok(())
            }
            _ => Err(self.err_unexpected(expected_label)),
        }
    }

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        match self.peek_kind() {
            Some(TokKind::Ident(s)) => {
                let s = s.clone();
                self.bump();
                Ok(s)
            }
            _ => Err(self.err_unexpected("identifier")),
        }
    }

    fn at_keyword(&self, kw: &str) -> bool {
        matches!(self.peek_kind(), Some(TokKind::Ident(s)) if s == kw)
    }

    fn eat_keyword(&mut self, kw: &str) -> bool {
        if self.at_keyword(kw) {
            self.bump();
            true
        } else {
            false
        }
    }

    // ---------------- top-level items ----------------

    fn parse_item(&mut self) -> Result<Item, ParseError> {
        if self.at_keyword("namespace") {
            self.bump();
            let name = self.expect_ident()?;
            return Ok(Item::Namespace(name));
        }
        if self.at_keyword("end") {
            self.bump();
            let name = self.expect_ident()?;
            return Ok(Item::EndNamespace(name));
        }
        if self.at_keyword("import") {
            self.bump();
            let name = self.expect_ident()?;
            return Ok(Item::ImportSkipped(name));
        }
        if self.at_keyword("open") {
            // `open Foo.Bar` — skip until newline-equivalent or
            // next top-level keyword.
            self.bump();
            let _ = self.expect_ident()?;
            return Ok(Item::ImportSkipped("open".into()));
        }
        if self.at_keyword("instance") {
            let line = Self::span_of(self.peek()).0;
            self.bump(); // consume the `instance` keyword first
            self.skip_until_next_item();
            return Ok(Item::InstanceSkipped { line });
        }
        if self.at_keyword("theorem") || self.at_keyword("example") {
            let line = Self::span_of(self.peek()).0;
            self.bump(); // consume the keyword
            self.skip_until_next_item();
            return Ok(Item::TheoremSkipped { line });
        }
        if self.at_keyword("def") {
            self.bump();
            return Ok(Item::Def(self.parse_def()?));
        }
        if self.at_keyword("structure") {
            self.bump();
            return Ok(Item::Struct(self.parse_struct()?));
        }
        if self.at_keyword("inductive") {
            self.bump();
            return Ok(Item::Inductive(self.parse_inductive()?));
        }
        Err(self.err_unsupported("expected top-level item (def / structure / inductive / namespace / import / instance / theorem)"))
    }

    /// Skip tokens until we reach the next top-level keyword. Used
    /// for items the extractor doesn't process (theorem, instance,
    /// example).
    fn skip_until_next_item(&mut self) {
        // Track bracket depth so we don't get fooled by structures
        // inside `instance` bodies.
        let mut depth = 0i32;
        loop {
            let Some(tok) = self.peek_raw() else {
                return;
            };
            match &tok.kind {
                TokKind::LParen | TokKind::LBrace | TokKind::LBracket => {
                    depth += 1;
                    self.bump_raw();
                }
                TokKind::RParen | TokKind::RBrace | TokKind::RBracket => {
                    depth -= 1;
                    self.bump_raw();
                }
                TokKind::Newline if depth == 0 => {
                    // Look ahead — is the next non-newline token a
                    // new top-level keyword?
                    let mut j = self.i + 1;
                    while j < self.toks.len() && self.toks[j].kind == TokKind::Newline {
                        j += 1;
                    }
                    if let Some(t) = self.toks.get(j) {
                        if let TokKind::Ident(s) = &t.kind {
                            if matches!(
                                s.as_str(),
                                "def"
                                    | "structure"
                                    | "inductive"
                                    | "instance"
                                    | "theorem"
                                    | "example"
                                    | "namespace"
                                    | "end"
                                    | "import"
                                    | "open"
                            ) {
                                return;
                            }
                        }
                    }
                    self.bump_raw();
                }
                _ => {
                    self.bump_raw();
                }
            }
        }
    }

    // ---------------- def / structure / inductive ----------------

    fn parse_def(&mut self) -> Result<DefItem, ParseError> {
        let name = self.expect_ident()?;
        let mut params = Vec::new();
        // Parameter groups: `(name : Ty)` or `(a b : Ty)`.
        while let Some(TokKind::LParen) = self.peek_kind() {
            self.bump();
            let mut names = Vec::new();
            while let Some(TokKind::Ident(s)) = self.peek_kind() {
                names.push(s.clone());
                self.bump();
            }
            self.expect_kind(&TokKind::Colon, "':'")?;
            let ty = self.parse_type()?;
            self.expect_kind(&TokKind::RParen, "')'")?;
            for n in names {
                params.push(Param {
                    name: n,
                    ty: ty.clone(),
                });
            }
        }
        self.expect_kind(&TokKind::Colon, "':' before return type")?;
        let return_ty = self.parse_type()?;
        // Lean's "function-by-match" sugar:
        //   def f : T → R
        //     | pat1 => body1
        //     | pat2 => body2
        // is equivalent to
        //   def f : T → R := fun x => match x with | pat1 => …
        // Detect this by looking at the token immediately after the
        // return type — if it's `|`, parse the arms and synthesize
        // a `match` on a fresh implicit parameter.
        if matches!(self.peek_kind(), Some(TokKind::Pipe)) {
            let mut arms = Vec::new();
            while matches!(self.peek_kind(), Some(TokKind::Pipe)) {
                self.bump();
                let pat = self.parse_pattern()?;
                self.expect_kind(&TokKind::Arrow, "'=>' between pattern and body")?;
                let body = self.parse_expr()?;
                arms.push(MatchArm { pat, body });
            }
            // The implicit argument's name is `__arg`; the
            // translator recognises this.
            let body = Expr::Match {
                scrutinee: Box::new(Expr::Ident("__arg".into())),
                arms,
            };
            return Ok(DefItem {
                name,
                params,
                return_ty,
                body,
                doc: None,
            });
        }
        self.expect_kind(&TokKind::Assign, "':='")?;
        let body = self.parse_expr()?;
        Ok(DefItem {
            name,
            params,
            return_ty,
            body,
            doc: None,
        })
    }

    fn parse_struct(&mut self) -> Result<StructItem, ParseError> {
        let name = self.expect_ident()?;
        // `where` introduces fields.
        if !self.eat_keyword("where") {
            return Err(self.err_unexpected("'where' after structure name"));
        }
        let mut fields = Vec::new();
        // Each field: `name : Type` (one per line). Multi-name
        // groups like `a b c : Type` are also allowed.
        loop {
            // Stop at `deriving`, end-of-input, or next top-level item.
            if self.at_keyword("deriving") || self.is_eof() {
                break;
            }
            if let Some(TokKind::Ident(s)) = self.peek_kind() {
                if matches!(
                    s.as_str(),
                    "def"
                        | "structure"
                        | "inductive"
                        | "instance"
                        | "theorem"
                        | "example"
                        | "namespace"
                        | "end"
                ) {
                    break;
                }
            }
            // Collect names up to ':'.
            let mut names = Vec::new();
            while let Some(TokKind::Ident(s)) = self.peek_kind() {
                names.push(s.clone());
                self.bump();
            }
            if names.is_empty() {
                break;
            }
            self.expect_kind(&TokKind::Colon, "':' between field name and type")?;
            let ty = self.parse_type()?;
            for n in names {
                fields.push(StructField {
                    name: n,
                    ty: ty.clone(),
                });
            }
        }
        let derives = if self.eat_keyword("deriving") {
            self.parse_derive_list()?
        } else {
            Vec::new()
        };
        Ok(StructItem {
            name,
            fields,
            doc: None,
            derives,
        })
    }

    fn parse_inductive(&mut self) -> Result<InductiveItem, ParseError> {
        let name = self.expect_ident()?;
        // Optional `: Type` annotation.
        if matches!(self.peek_kind(), Some(TokKind::Colon)) {
            self.bump();
            let _ = self.parse_type()?;
        }
        if !self.eat_keyword("where") {
            return Err(self.err_unexpected("'where' after inductive name"));
        }
        let mut ctors = Vec::new();
        // Each constructor begins with `|`.
        while matches!(self.peek_kind(), Some(TokKind::Pipe)) {
            self.bump();
            let ctor_name = self.expect_ident()?;
            // Constructor arg list — for our subset all ctors are
            // nullary (LocalHeader.lean's `ParseError`). Reject
            // anything that looks like an arg list.
            if matches!(self.peek_kind(), Some(TokKind::Colon)) {
                return Err(
                    self.err_unsupported("inductive constructor with explicit type signature")
                );
            }
            ctors.push(InductiveCtor {
                name: ctor_name,
                args: Vec::new(),
            });
        }
        let derives = if self.eat_keyword("deriving") {
            self.parse_derive_list()?
        } else {
            Vec::new()
        };
        Ok(InductiveItem {
            name,
            ctors,
            doc: None,
            derives,
        })
    }

    fn parse_derive_list(&mut self) -> Result<Vec<String>, ParseError> {
        let mut out = vec![self.expect_ident()?];
        while matches!(self.peek_kind(), Some(TokKind::Comma)) {
            self.bump();
            out.push(self.expect_ident()?);
        }
        Ok(out)
    }

    // ---------------- types ----------------

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        let mut t = self.parse_type_atom()?;
        // Right-associative `→` reduces to function-typed values; we
        // don't model function types as runtime values (only at the
        // `def` signature level), so reject them here. A `→` between
        // types in a definition return position appears in
        // `ParseError → UInt8`, which is the type of a function we
        // *do* lower (see ParseError.tag). Accept it as a function
        // type and let the caller decide.
        if matches!(self.peek_kind(), Some(TokKind::Arrow)) {
            self.bump();
            let rhs = self.parse_type()?;
            // Encode as Named so the translator can recognise the
            // `EnumName -> ResultType` pattern used by
            // ParseError.tag.
            let lhs_name = match &t {
                Type::Named(n) => n.clone(),
                _ => return Err(self.err_unsupported("function type with non-named domain")),
            };
            let rhs_name = match &rhs {
                Type::UInt(_) => format!("{:?}", rhs),
                Type::Named(n) => n.clone(),
                _ => return Err(self.err_unsupported("function type with complex codomain")),
            };
            // Stash as a `Named` so callers get a single string they
            // can pattern-match on.
            t = Type::Named(format!("{lhs_name} -> {rhs_name}"));
        }
        // `A × B` product.
        if matches!(self.peek_kind(), Some(TokKind::Times)) {
            self.bump();
            let rhs = self.parse_type_atom()?;
            t = Type::Prod(Box::new(t), Box::new(rhs));
        }
        Ok(t)
    }

    fn parse_type_atom(&mut self) -> Result<Type, ParseError> {
        if matches!(self.peek_kind(), Some(TokKind::LParen)) {
            self.bump();
            let inner = self.parse_type()?;
            self.expect_kind(&TokKind::RParen, "')'")?;
            return Ok(inner);
        }
        let name = self.expect_ident()?;
        match name.as_str() {
            "UInt8" => Ok(Type::UInt(8)),
            "UInt16" => Ok(Type::UInt(16)),
            "UInt32" => Ok(Type::UInt(32)),
            "UInt64" => Ok(Type::UInt(64)),
            "Nat" => Ok(Type::Nat),
            "Bool" => Ok(Type::Bool),
            "String" => Ok(Type::String_),
            "ByteArray" => Ok(Type::ByteArray),
            "Option" => {
                let inner = self.parse_type_atom()?;
                Ok(Type::Option(Box::new(inner)))
            }
            "Except" => {
                let err_ty = self.parse_type_atom()?;
                let ok_ty = self.parse_type_atom()?;
                Ok(Type::Except(Box::new(err_ty), Box::new(ok_ty)))
            }
            other => Ok(Type::Named(other.to_string())),
        }
    }

    // ---------------- expressions (Pratt) ----------------

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_prefix()?;
        loop {
            let op_kind = match self.peek_kind() {
                Some(k) => k.clone(),
                None => break,
            };
            let (op, l_bp, r_bp) = match infix_bp(&op_kind) {
                Some(t) => t,
                None => break,
            };
            if l_bp < min_bp {
                break;
            }
            self.bump();
            let rhs = self.parse_expr_bp(r_bp)?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        // Keywords that start expressions.
        if self.at_keyword("if") {
            return self.parse_if();
        }
        if self.at_keyword("let") {
            return self.parse_let_expr();
        }
        if self.at_keyword("match") {
            return self.parse_match();
        }
        if self.at_keyword("return") {
            self.bump();
            let inner = self.parse_expr()?;
            return Ok(Expr::Return(Box::new(inner)));
        }
        // `Id.run do …` — the lexer merges dotted idents, so the
        // most common shape is a single `Ident("Id.run")` followed
        // by keyword `do`. We also accept the un-merged form for
        // robustness.
        if self.at_keyword("Id.run") {
            self.bump();
            if !self.eat_keyword("do") {
                return Err(self.err_unexpected("'do' after `Id.run`"));
            }
            let stmts = self.parse_do_block()?;
            return Ok(Expr::IdRunDo { stmts });
        }
        if self.at_keyword("Id") {
            self.bump();
            self.expect_kind(&TokKind::Dot, "'.'")?;
            let after = self.expect_ident()?;
            if after != "run" {
                return Err(self.err_unsupported("`Id.<x>` other than `Id.run`"));
            }
            if !self.eat_keyword("do") {
                return Err(self.err_unexpected("'do' after `Id.run`"));
            }
            let stmts = self.parse_do_block()?;
            return Ok(Expr::IdRunDo { stmts });
        }
        if self.at_keyword("true") {
            self.bump();
            return Ok(Expr::BoolLit(true));
        }
        if self.at_keyword("false") {
            self.bump();
            return Ok(Expr::BoolLit(false));
        }
        // Unary `-` only on numeric literals — we don't model
        // arbitrary negation in the subset.
        if matches!(self.peek_kind(), Some(TokKind::Minus)) {
            self.bump();
            let inner = self.parse_prefix()?;
            return Ok(Expr::Neg(Box::new(inner)));
        }
        if matches!(self.peek_kind(), Some(TokKind::Bang)) {
            self.bump();
            let inner = self.parse_prefix()?;
            return Ok(Expr::Not(Box::new(inner)));
        }
        // Atoms.
        let mut head = self.parse_atom()?;
        // Application — Lean uses juxtaposition. Stop on operators,
        // newlines, closing brackets, comma, semicolon, `with`,
        // `then`, `else`, etc.
        loop {
            if self.application_continues() {
                let arg = self.parse_atom()?;
                head = Expr::App {
                    head: Box::new(head),
                    arg: Box::new(arg),
                };
            } else {
                break;
            }
        }
        Ok(head)
    }

    fn application_continues(&self) -> bool {
        let tok = match self.peek_raw() {
            Some(t) => t,
            None => return false,
        };
        match &tok.kind {
            TokKind::LParen
            | TokKind::LBrace
            | TokKind::LBracket
            | TokKind::Hash // `#[ … ]` array literal
            | TokKind::IntLit(_)
            | TokKind::StrLit(_) => true,
            TokKind::Ident(s) => !is_expr_terminator_keyword(s),
            TokKind::Dot => true, // `.some` ctor
            _ => false,
        }
    }

    fn parse_atom(&mut self) -> Result<Expr, ParseError> {
        // Skip leading newlines — at the expression level (outside
        // do-blocks) newlines are insignificant.
        let tok = self.peek().cloned();
        let kind = match tok {
            Some(t) => t.kind.clone(),
            None => return Err(self.err_unexpected("expression")),
        };
        match kind {
            TokKind::IntLit(s) => {
                self.bump();
                Ok(Expr::IntLit(s))
            }
            TokKind::StrLit(s) => {
                self.bump();
                Ok(Expr::StrLit(s))
            }
            TokKind::Ident(s) => {
                self.bump();
                Ok(Expr::Ident(s))
            }
            TokKind::Dot => {
                self.bump();
                let ctor = self.expect_ident()?;
                Ok(Expr::DotCtor { ctor })
            }
            TokKind::LParen => {
                self.bump();
                if matches!(self.peek_kind(), Some(TokKind::RParen)) {
                    self.bump();
                    return Err(self.err_unsupported("unit value `()`"));
                }
                let first = self.parse_expr()?;
                if matches!(self.peek_kind(), Some(TokKind::Comma)) {
                    let mut elts = vec![first];
                    while matches!(self.peek_kind(), Some(TokKind::Comma)) {
                        self.bump();
                        elts.push(self.parse_expr()?);
                    }
                    self.expect_kind(&TokKind::RParen, "')'")?;
                    return Ok(Expr::Tuple(elts));
                }
                self.expect_kind(&TokKind::RParen, "')'")?;
                Ok(Expr::Paren(Box::new(first)))
            }
            TokKind::LBrace => {
                self.bump();
                let fields = self.parse_struct_lit_fields()?;
                self.expect_kind(&TokKind::RBrace, "'}'")?;
                Ok(Expr::StructLit {
                    type_hint: None,
                    fields,
                })
            }
            TokKind::LBracket => Err(self.err_unsupported("list literal `[ … ]`")),
            TokKind::Hash => {
                self.bump();
                if !matches!(self.peek_kind(), Some(TokKind::LBracket)) {
                    return Err(self.err_unexpected("'[' after '#'"));
                }
                self.bump();
                let mut elts = Vec::new();
                while !matches!(self.peek_kind(), Some(TokKind::RBracket)) {
                    if elts.is_empty() && matches!(self.peek_kind(), Some(TokKind::RBracket)) {
                        break;
                    }
                    elts.push(self.parse_expr()?);
                    if matches!(self.peek_kind(), Some(TokKind::Comma)) {
                        self.bump();
                    } else {
                        break;
                    }
                }
                self.expect_kind(&TokKind::RBracket, "']' to close `#[ … ]`")?;
                Ok(Expr::ArrayLit(elts))
            }
            _ => Err(self.err_unexpected("atom")),
        }
    }

    fn parse_struct_lit_fields(&mut self) -> Result<Vec<StructLitField>, ParseError> {
        let mut out = Vec::new();
        while !matches!(self.peek_kind(), Some(TokKind::RBrace)) {
            let name = self.expect_ident()?;
            // Optional `:= value`. Without it Lean treats `{ x }` as
            // `{ x := x }` (punning).
            let value = if matches!(self.peek_kind(), Some(TokKind::Assign)) {
                self.bump();
                Some(self.parse_expr()?)
            } else {
                None
            };
            out.push(StructLitField { name, value });
            if matches!(self.peek_kind(), Some(TokKind::Comma)) {
                self.bump();
            }
        }
        Ok(out)
    }

    fn parse_if(&mut self) -> Result<Expr, ParseError> {
        if !self.eat_keyword("if") {
            return Err(self.err_unexpected("'if'"));
        }
        let cond = self.parse_expr()?;
        if !self.eat_keyword("then") {
            return Err(self.err_unexpected("'then'"));
        }
        let then_ = self.parse_expr()?;
        if !self.eat_keyword("else") {
            return Err(self.err_unexpected("'else'"));
        }
        let else_ = self.parse_expr()?;
        Ok(Expr::If {
            cond: Box::new(cond),
            then_: Box::new(then_),
            else_: Box::new(else_),
        })
    }

    /// Parse a top-level `let … := … <body>`. Inside `do`-blocks
    /// we use `parse_do_let` which has slightly different syntax.
    fn parse_let_expr(&mut self) -> Result<Expr, ParseError> {
        if !self.eat_keyword("let") {
            return Err(self.err_unexpected("'let'"));
        }
        let binder = self.parse_let_binder()?;
        let ty = if matches!(self.peek_kind(), Some(TokKind::Colon)) {
            self.bump();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect_kind(&TokKind::Assign, "':=' in let")?;
        let value = self.parse_expr()?;
        // Lean syntax for `let .some var := expr | bail`: the bail
        // comes AFTER the value.
        let bail = if matches!(binder, LetBinder::SomePat(_))
            && matches!(self.peek_kind(), Some(TokKind::Pipe))
        {
            self.bump();
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };
        let body = self.parse_expr()?;
        Ok(Expr::Let {
            binder,
            ty,
            value: Box::new(value),
            bail,
            body: Box::new(body),
        })
    }

    fn parse_let_binder(&mut self) -> Result<LetBinder, ParseError> {
        if matches!(self.peek_kind(), Some(TokKind::Dot)) {
            self.bump();
            let ctor = self.expect_ident()?;
            if ctor != "some" {
                return Err(self.err_unsupported("let-pattern other than `let .some x := … | …`"));
            }
            let var = self.expect_ident()?;
            return Ok(LetBinder::SomePat(var));
        }
        let n = self.expect_ident()?;
        Ok(LetBinder::Ident(n))
    }

    fn parse_match(&mut self) -> Result<Expr, ParseError> {
        if !self.eat_keyword("match") {
            return Err(self.err_unexpected("'match'"));
        }
        let scrutinee = self.parse_expr()?;
        if !self.eat_keyword("with") {
            return Err(self.err_unexpected("'with'"));
        }
        let mut arms = Vec::new();
        while matches!(self.peek_kind(), Some(TokKind::Pipe)) {
            self.bump();
            let pat = self.parse_pattern()?;
            // `=>` (we lex it as `Arrow`).
            self.expect_kind(&TokKind::Arrow, "'=>' or '→' between pattern and arm body")?;
            let body = self.parse_expr()?;
            arms.push(MatchArm { pat, body });
        }
        Ok(Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
        })
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        // `_`, `.ctor [args]`, identifier.
        if matches!(self.peek_kind(), Some(TokKind::Ident(s)) if s == "_") {
            self.bump();
            return Ok(Pattern::Wildcard);
        }
        if matches!(self.peek_kind(), Some(TokKind::Dot)) {
            self.bump();
            let name = self.expect_ident()?;
            let mut args = Vec::new();
            while !matches!(
                self.peek_kind(),
                Some(TokKind::Pipe) | Some(TokKind::Arrow) | None
            ) {
                args.push(self.parse_pattern()?);
            }
            return Ok(Pattern::Ctor { name, args });
        }
        let n = self.expect_ident()?;
        Ok(Pattern::Ident(n))
    }

    // ---------------- do-block ----------------

    fn parse_do_block(&mut self) -> Result<Vec<DoStmt>, ParseError> {
        // We treat newlines as statement separators inside a do.
        // The block ends when we see either:
        //   - end of input
        //   - a keyword that can't start a do-stmt (caller's
        //     responsibility — usually we hit EOF or a top-level
        //     def, since `Id.run do` is always the body of a def)
        let mut stmts = Vec::new();
        loop {
            // skip blank lines
            while self.peek_raw().is_some_and(|t| t.kind == TokKind::Newline) {
                self.bump_raw();
            }
            // End markers — once we hit a top-level keyword the do
            // block is done.
            let Some(tok) = self.peek_raw() else {
                break;
            };
            if let TokKind::Ident(s) = &tok.kind {
                if matches!(
                    s.as_str(),
                    "def"
                        | "structure"
                        | "inductive"
                        | "instance"
                        | "theorem"
                        | "example"
                        | "namespace"
                        | "end"
                        | "import"
                ) {
                    break;
                }
            }
            // Parse one stmt.
            let stmt = self.parse_do_stmt()?;
            let was_return = matches!(stmt, DoStmt::Return(_));
            stmts.push(stmt);
            if was_return {
                // No more stmts allowed after return at this depth —
                // but Lean's `Id.run do` body usually has a tail
                // return. We continue parsing to handle nested
                // structures correctly; a stray statement after
                // `return` would be unreachable but we don't reject.
            }
        }
        Ok(stmts)
    }

    fn parse_do_stmt(&mut self) -> Result<DoStmt, ParseError> {
        if self.at_keyword("if") {
            self.bump();
            let cond = self.parse_expr()?;
            if !self.eat_keyword("then") {
                return Err(self.err_unexpected("'then' in do-if"));
            }
            // Then branch: either a single expression or a do-block.
            // In LocalHeader.lean it's always a single `return …`.
            let then_stmt = self.parse_do_stmt()?;
            // Optional else.
            let else_stmts = if self.eat_keyword("else") {
                let s = self.parse_do_stmt()?;
                Some(vec![s])
            } else {
                None
            };
            return Ok(DoStmt::If {
                cond,
                then_: vec![then_stmt],
                else_: else_stmts,
            });
        }
        if self.at_keyword("return") {
            self.bump();
            let e = self.parse_expr()?;
            return Ok(DoStmt::Return(e));
        }
        if self.at_keyword("let") {
            self.bump();
            let binder = self.parse_let_binder()?;
            let ty = if matches!(self.peek_kind(), Some(TokKind::Colon)) {
                self.bump();
                Some(self.parse_type()?)
            } else {
                None
            };
            self.expect_kind(&TokKind::Assign, "':=' in let")?;
            let value = self.parse_expr()?;
            // Lean: `let .some var := expr | bail`. The pipe + bail
            // expr come AFTER the value.
            let bail = if matches!(binder, LetBinder::SomePat(_))
                && matches!(self.peek_kind(), Some(TokKind::Pipe))
            {
                self.bump();
                Some(self.parse_expr()?)
            } else {
                None
            };
            return Ok(DoStmt::Let {
                binder,
                ty,
                value,
                bail,
            });
        }
        // Tail expression.
        let e = self.parse_expr()?;
        Ok(DoStmt::Tail(e))
    }
}

fn is_expr_terminator_keyword(s: &str) -> bool {
    matches!(
        s,
        "then"
            | "else"
            | "with"
            | "do"
            | "where"
            | "deriving"
            | "by"
            | "def"
            | "structure"
            | "inductive"
            | "instance"
            | "theorem"
            | "example"
            | "namespace"
            | "end"
            | "import"
            | "let"
            | "match"
            | "return"
            | "if"
    )
}

fn infix_bp(k: &TokKind) -> Option<(BinOp, u8, u8)> {
    // (op, left-bp, right-bp) — left-associative when l_bp == r_bp - 1.
    Some(match k {
        TokKind::OrOr => (BinOp::Or, 1, 2),
        TokKind::AndAnd => (BinOp::And, 3, 4),
        TokKind::EqEq => (BinOp::Eq, 5, 6),
        TokKind::NEq => (BinOp::NEq, 5, 6),
        TokKind::Lt => (BinOp::Lt, 7, 8),
        TokKind::Le => (BinOp::Le, 7, 8),
        TokKind::Gt => (BinOp::Gt, 7, 8),
        TokKind::Ge => (BinOp::Ge, 7, 8),
        TokKind::BitOr => (BinOp::BitOr, 9, 10),
        TokKind::BitXor => (BinOp::BitXor, 11, 12),
        TokKind::BitAnd => (BinOp::BitAnd, 13, 14),
        TokKind::Shl => (BinOp::Shl, 15, 16),
        TokKind::Shr => (BinOp::Shr, 15, 16),
        TokKind::Plus => (BinOp::Add, 17, 18),
        TokKind::Minus => (BinOp::Sub, 17, 18),
        TokKind::Star => (BinOp::Mul, 19, 20),
        TokKind::Slash => (BinOp::Div, 19, 20),
        TokKind::Percent => (BinOp::Mod, 19, 20),
        _ => return None,
    })
}
