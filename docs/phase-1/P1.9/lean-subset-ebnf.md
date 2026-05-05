# P1.9 Lean Subset — EBNF Grammar

> Formal grammar of the Lean 4 sublanguage that
> [`tools/lean-to-rust`](../../../tools/lean-to-rust/) accepts. Anyone
> modifying the verified-parser theorems must keep their Lean
> source within this grammar, or the extractor rejects the file
> with a span'd parse error.
>
> This grammar is the contract that ADR-0025 v2 (in
> [`CHECKLIST.md`](./CHECKLIST.md) §D) refers to. It documents what
> "verified-parser sublanguage of Lean" means, mechanically.

## Notation

EBNF, with the following conventions:

- `'literal'` — exact source text
- `name` — non-terminal
- `name?` — optional
- `name*` — zero or more
- `name+` — one or more
- `(...)` — grouping
- `a | b` — alternation

## Top-level

```ebnf
module     ::= item*
item       ::= namespace | end | import | open | def | structure | inductive
             | instance | theorem | example
namespace  ::= 'namespace' qualified_ident
end        ::= 'end' qualified_ident
import     ::= 'import' qualified_ident
open       ::= 'open' qualified_ident
```

`instance`, `theorem`, and `example` are **recognised but skipped** —
the extractor records them as elided and their content is not
lowered to Rust.

## Definitions

```ebnf
def        ::= 'def' qualified_ident params? ':' type ( assign_body | match_body )
params     ::= param_group+
param_group::= '(' ident+ ':' type ')'
assign_body::= ':=' expr
match_body ::= ('|' pattern '=>' expr)+
```

The `match_body` form is Lean's "function-by-match" sugar:

```lean
def f : T → R
  | pat1 => body1
  | pat2 => body2
```

is sugar for

```lean
def f : T → R := fun x => match x with | pat1 => body1 | pat2 => body2
```

The extractor synthesises a `match` on a fresh implicit parameter
named `__arg`.

## Structures and inductives

```ebnf
structure  ::= 'structure' ident 'where' field+ deriving?
field      ::= ident+ ':' type
deriving   ::= 'deriving' ident (',' ident)*

inductive  ::= 'inductive' ident (':' type)? 'where' ctor+ deriving?
ctor       ::= '|' ident
```

Constructor arguments are **not supported** — every inductive
constructor we use is nullary (the only kind in
`LocalHeader.lean`'s `ParseError`).

`deriving` clauses are parsed but only `Inhabited`, `Repr`,
`DecidableEq` are recognised; others are silently elided.

## Types

```ebnf
type       ::= type_atom ('×' type_atom)? ('→' type)?
type_atom  ::= 'UInt8' | 'UInt16' | 'UInt32' | 'UInt64'
             | 'Nat' | 'Bool' | 'String' | 'ByteArray'
             | 'Option' type_atom
             | 'Except' type_atom type_atom
             | qualified_ident                -- user-defined type name
             | '(' type ')'
```

Lean's `Except E T` lowers to Rust's `Result<T, E>` (argument
order swapped). `ByteArray` lowers to `&[u8]` in parameter
position and `Vec<u8>` in return / field position.

`A × B` produces a 2-tuple. Triple+ products are not supported
(we only encounter `Lfh × Nat` in `LocalHeader.lean`).

## Expressions

```ebnf
expr       ::= prefix_expr (binop expr)*
prefix_expr::= '-' prefix_expr
             | '!' prefix_expr
             | atom application_arg*
application_arg
           ::= atom

atom       ::= int_lit | bool_lit | str_lit
             | ident | '.' ident                     -- DotCtor
             | '(' expr (',' expr)* ')'              -- paren / tuple
             | '{' struct_lit_field (',' struct_lit_field)* '}'
             | '#' '[' expr (',' expr)* ']'          -- ArrayLit
             | 'if' expr 'then' expr 'else' expr
             | 'let' let_binder (':' type)? ':=' expr
                   ('|' expr)?                       -- bail (only with .some)
                   expr
             | 'match' expr 'with' match_arm+
             | 'return' expr
             | 'Id.run' 'do' do_stmt+
             | 'true' | 'false'

let_binder ::= ident | '.' 'some' ident

match_arm  ::= '|' pattern '=>' expr
pattern    ::= '_' | ident | '.' ident pattern*

struct_lit_field
           ::= ident (':=' expr)?                    -- punning if no ':='

binop      ::= '+' | '-' | '*' | '/' | '%'
             | '==' | '!=' | '<' | '<=' | '>' | '>='
             | '&&' | '||'
             | '&&&' | '|||' | '^^^' | '<<<' | '>>>'
```

### `do` blocks

```ebnf
do_stmt    ::= 'let' let_binder (':' type)? ':=' expr ('|' expr)?
             | 'if' expr 'then' do_stmt ('else' do_stmt)?
             | 'return' expr
             | expr                                   -- tail
```

`Id.run do` blocks are flattened into straight-line Rust function
bodies. `return e` becomes `return e;`. The final tail expression
becomes the function's return value.

## Lexical elements

```ebnf
ident         ::= ascii_alpha (ascii_alpha | ascii_digit | '_')*
                  ('!' | '?')*                       -- trailing method markers
qualified_ident
              ::= ident ('.' ident)*

int_lit       ::= dec_lit | hex_lit
dec_lit       ::= ascii_digit+
hex_lit       ::= '0x' hex_digit+

bool_lit      ::= 'true' | 'false'
str_lit       ::= '"' str_char* '"'
str_char      ::= /* anything except `"` and `\`, plus
                     escape sequences */

comment_block ::= '/-' /* nestable */ '-/'
comment_line  ::= '--' /* to end of line */

unicode_op    ::= '≠' | '≤' | '≥' | '→' | '←' | '×'
                | '∧' | '∨'                          -- recognised, lowered
                                                      -- to ASCII equivalents
```

The lexer accepts trailing `!` and `?` on identifiers (Lean's
method-style `bs.get!`, `Option.get?`) **only** when not followed
by `=` (so `foo != bar` lexes as ident + NEq).

## Out-of-scope (intentional rejections)

The following Lean 4 constructs are **explicitly outside the
supported subset**. The extractor must reject them with a
span'd error rather than silently fall through:

- Lambdas / anonymous functions (`fun x => …`)
- `where` clauses inside expressions / definitions
- Mutual recursion (`mutual … end`)
- Well-founded fixpoints (`termination_by` / `decreasing_by`)
- Type classes & class definitions (`class Foo where …`)
- Macros / `syntax` declarations
- Tactics / `by …` blocks (theorems are skipped, not extracted)
- Dependent types in user code (Lean's full Π/Σ types)
- Mathlib-specific tactics like `ext`, `decide`, `simp_arith`
- Universe polymorphism (`(α : Type u)`)
- Polymorphic functions over arbitrary type variables (we
  hand-translate `Option α` etc. but don't accept new ones)
- Unicode identifiers (only ASCII identifiers are tokenised)
- Char literals (`'a'`)
- Raw string literals (`r"..."`)
- List literals `[a, b]` (only `#[a, b]` array literals)

Any source using these constructs in our extractable modules
must be refactored before the extractor will accept it.

## Hand-off table — Lean → Rust

| Lean construct | Rust target | Notes |
|---|---|---|
| `def f : T := lit` (nullary, primitive `T`) | `pub const F: T = lit;` | uppercase via `lean_to_rust_field` |
| `def f (x : A) (y : B) : R := body` | `pub fn f(x: A, y: B) -> R { body }` | snake_case identifiers |
| `def Type.method : Type → R \| pat => body \| …` | `impl Type { pub const fn method(self) -> R { match self { … } } }` | enum-method form |
| `structure S where x : T …` | `#[derive(Debug,Clone,PartialEq,Eq)] pub struct S { pub x: T, … }` | Inhabited stripped (no Default for ByteArray) |
| `inductive I where \| a \| b` | `#[derive(Debug,Clone,Copy,PartialEq,Eq)] pub enum I { A, B }` | nullary ctors only |
| `Id.run do …` | straight-line block | `return e` → `return e;`, tail → fall-through |
| `let .some x := e \| bail; body` | `let Some(x) = e else { bail }; body` | Rust 1.65+ let-else |
| `match e with \| .ctor x => body` | `match e { Type::Ctor(x) => body }` | env-resolved enum prefix |
| `bs.size` | `bs.len()` | method dispatch in translator |
| `b0.toUInt16` | `u16::from(b0)` | numeric type cast |
| `nameLen.toNat` | `(name_len as usize)` | unsigned-to-usize via `as` |
| `bs.get! o` | `bs[o]` | bounds-check left to caller |
| `bs.extract a b` | `bs[a..b].to_vec()` | slice → owned |
| `ByteArray.mk arr` | `arr` | identity for our `&[u8]` lowering |
| `.error e` | `Err(e)` | `Except` → `Result` |
| `.ok v` | `Ok(v)` | |
| `.some x` | `Some(x)` | |
| `.none` | `None` | |
| `0x04034b50` | `0x04034b50` | hex preserved verbatim |

## Updating this grammar

The extractor's parser is the source of truth (see
[`tools/lean-to-rust/src/parser.rs`](../../../tools/lean-to-rust/src/parser.rs)).
This document mirrors the parser's accepted shapes; if the parser
gains new productions, this file must be updated in the same
commit. A future automation step (P1.13+) could derive this EBNF
mechanically from the parser's recursive-descent structure.
