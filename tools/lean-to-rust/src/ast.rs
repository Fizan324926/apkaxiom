// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! P1.9 Lean AST for the verified-parser subset.
//!
//! Models the surface syntax of `theorems/Apkaxiom/Zip/LocalHeader.lean`.
//! Anything outside this AST shape is a `parse error`, never a silent
//! pass-through. The lowering step (`translator.rs`) converts these
//! nodes into Rust source.

#![allow(missing_docs)]

/// A whole Lean source file, as parsed.
#[derive(Debug, Clone)]
pub struct Module {
    /// Items in source order. The translator preserves order so the
    /// extracted Rust file matches the Lean reading order.
    pub items: Vec<Item>,
}

/// One top-level definition. The translator handles each variant
/// independently; unsupported items fail at parse time with a span.
#[derive(Debug, Clone)]
pub enum Item {
    /// `def name (params) : ReturnType := body`
    Def(DefItem),
    /// `structure Name where field : Type ...`
    Struct(StructItem),
    /// `inductive Name [: Type] where | ctor | ctor`
    Inductive(InductiveItem),
    /// `instance ... where ...` — recognised but not extracted.
    /// Stored for diagnostics.
    InstanceSkipped { line: usize },
    /// `theorem ...` and `example ...` — recognised but not extracted.
    TheoremSkipped { line: usize },
    /// `namespace Foo` / `end Foo` — affect emission's module path.
    Namespace(String),
    /// `end Foo`.
    EndNamespace(String),
    /// `import X` — recognised but ignored (Rust doesn't import Lean
    /// modules; the extractor builds a single-crate output).
    ImportSkipped(String),
}

#[derive(Debug, Clone)]
pub struct DefItem {
    pub name: String,
    pub params: Vec<Param>,
    pub return_ty: Type,
    pub body: Expr,
    pub doc: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StructItem {
    pub name: String,
    pub fields: Vec<StructField>,
    pub doc: Option<String>,
    /// `deriving Inhabited, …` clauses — translated into Rust derives.
    pub derives: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct InductiveItem {
    pub name: String,
    pub ctors: Vec<InductiveCtor>,
    pub doc: Option<String>,
    pub derives: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct InductiveCtor {
    pub name: String,
    /// Constructor arguments. Empty for nullary constructors (the
    /// only kind LocalHeader.lean uses).
    pub args: Vec<Type>,
}

/// `(name : Type)` or `(a b : Type)` — Lean lets you group
/// same-typed binders.
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

/// Surface-level Lean types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// `UInt8` / `UInt16` / `UInt32` / `UInt64`.
    UInt(u8),
    /// `Nat`. Lowered to `usize` in our subset.
    Nat,
    /// `Bool`.
    Bool,
    /// `String`.
    String_,
    /// `ByteArray`. Lowered to `Vec<u8>` for owning storage and
    /// `&[u8]` for borrowed views; the translator picks based on
    /// position (parameter vs return / body).
    ByteArray,
    /// `Option T`.
    Option(Box<Type>),
    /// `Except E T`. Mapped to `Result<T, E>` in Rust. Lean's
    /// argument order is `(error, ok)`; Rust is `(ok, error)` —
    /// the translator swaps.
    Except(Box<Type>, Box<Type>),
    /// `A × B` product. We support 2-tuples as Lean's
    /// `Prod` (the only case in LocalHeader.lean).
    Prod(Box<Type>, Box<Type>),
    /// User-defined name: `Lfh`, `ParseError`, `Function.Injective …`.
    /// The translator recognises `Lfh` etc. against the parsed
    /// inductives/structs.
    Named(String),
}

/// Surface-level Lean expressions, restricted to what
/// LocalHeader.lean actually uses. The translator emits Rust for
/// each variant; unsupported variants cannot be constructed because
/// the parser doesn't accept the corresponding syntax.
#[derive(Debug, Clone)]
pub enum Expr {
    /// `42`, `0x04034b50`. Stored as raw text so the emitter keeps
    /// the source form (hex / decimal) intact.
    IntLit(String),
    /// `true` / `false`.
    BoolLit(bool),
    /// `"…"` — emitted as a Rust `&'static str`.
    StrLit(String),
    /// Unqualified or dotted identifier.
    Ident(String),
    /// `.some` / `.none` / `.error` / `.ok` / `.shortHeader` etc —
    /// Lean's anonymous-projector dot. Resolved against the expected
    /// type at translation time.
    DotCtor { ctor: String },
    /// `f x y z` — left-associative application.
    App { head: Box<Expr>, arg: Box<Expr> },
    /// Binary operator.
    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// Unary `-`.
    Neg(Box<Expr>),
    /// Unary `!`.
    Not(Box<Expr>),
    /// `if cond then t else e`.
    If {
        cond: Box<Expr>,
        then_: Box<Expr>,
        else_: Box<Expr>,
    },
    /// `let name [: ty] := value; body` (or trailing newline-as-`;`
    /// inside `do` blocks). `bail` is `Some(_)` only when the
    /// binder is `LetBinder::SomePat` and Lean's `| bail-expr`
    /// suffix is present.
    Let {
        binder: LetBinder,
        ty: Option<Type>,
        value: Box<Expr>,
        bail: Option<Box<Expr>>,
        body: Box<Expr>,
    },
    /// `match scrutinee with | pat1 => body1 | pat2 => body2`.
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    /// `Id.run do …` — wrap a stmt-list in `Id.run`.
    IdRunDo { stmts: Vec<DoStmt> },
    /// `return e` — only valid inside an `IdRunDo`. The parser only
    /// accepts this token in that context.
    Return(Box<Expr>),
    /// `{ field1 := v1, field2 := v2, … }` or
    /// `{ field1, field2, … }` (Lean punning).
    StructLit {
        /// Optional `(x : Lfh) := { … }` ascription if the parser
        /// inferred the type from context.
        type_hint: Option<String>,
        fields: Vec<StructLitField>,
    },
    /// `(e)` — kept so we can preserve grouping during emission.
    Paren(Box<Expr>),
    /// `(a, b)` — tuple constructor (used only for the `Lfh × Nat`
    /// return shape of `parseLfh`).
    Tuple(Vec<Expr>),
    /// `#[a, b, c]` — Lean's `Array α` literal. Used in
    /// `minimalLfhBytes`. Translator emits `vec![…]` for byte
    /// arrays, panics on other element types.
    ArrayLit(Vec<Expr>),
}

#[derive(Debug, Clone)]
pub enum LetBinder {
    /// `let name := …` — bind a fresh identifier.
    Ident(String),
    /// `let .some name := … | else_branch` — pattern-bind from
    /// `Option`. The bail expression after `|` is parsed *after*
    /// the value (Lean syntax: `let .some var := expr | bail`).
    /// Stored in the enclosing `Expr::Let` / `DoStmt::Let`'s
    /// `bail` field.
    SomePat(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NEq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pat: Pattern,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    /// `_` — catches anything.
    Wildcard,
    /// `.shortHeader` — ctor pattern with no args.
    Ctor { name: String, args: Vec<Pattern> },
    /// Bare identifier — binds the scrutinee to a fresh name.
    Ident(String),
}

/// One statement inside `Id.run do`.
#[derive(Debug, Clone)]
pub enum DoStmt {
    /// `let _ := …` or `let .some x := … | early_return`.
    Let {
        binder: LetBinder,
        ty: Option<Type>,
        value: Expr,
        /// Present only when `binder == SomePat(_)`.
        bail: Option<Expr>,
    },
    /// `if cond then early_return` (no `else`) or `if cond then …
    /// else …` as a statement.
    If {
        cond: Expr,
        then_: Vec<DoStmt>,
        else_: Option<Vec<DoStmt>>,
    },
    /// `return e` — early-exit from the do-block.
    Return(Expr),
    /// Tail expression — the do-block's value is whatever this
    /// expression evaluates to. Only valid as the last statement.
    Tail(Expr),
}

#[derive(Debug, Clone)]
pub struct StructLitField {
    pub name: String,
    /// `None` means Lean's punning `{ x }` shorthand (= `{ x := x }`).
    pub value: Option<Expr>,
}
