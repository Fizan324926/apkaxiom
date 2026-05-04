// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `axiom-zip-ref` — P1.5 Rust reference parser for ZIP local file
//! headers (LFH) and end-of-central-directory (EOCD) records.
//!
//! This crate is the *non-Lean* leg of the differential harness. It
//! mirrors `theorems/Apkaxiom/Zip/LocalHeader.lean` and
//! `theorems/Apkaxiom/Zip/Eocd.lean` byte-for-byte; the tag enums
//! agree numerically with the Lean `ParseError.tag` definitions, so
//! the harness can compare error categories across languages.
//!
//! Soundness contract (asserted by the harness, not by this crate):
//!   ∀ bytes : the Lean `parseLfh / parseEocd` result and the Rust
//!   `parse_lfh / parse_eocd` result agree on (verdict, recovered
//!   structure, error category) for every input in the 1500+
//!   corpus under `corpus/zip/`.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]

pub mod eocd;
pub mod lfh;
