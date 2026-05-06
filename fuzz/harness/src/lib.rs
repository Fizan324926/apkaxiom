// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! P1.13 differential fuzzing plant — shared library types.
//!
//! The harness has two run modes (selected at run time, not compile
//! time, so the same binary serves CI and the production KVM host):
//!
//!   - **dev**: in-process diff between `axiom_l0_zip_verified::
//!     consistency::parse_archive` and the AOSP libziparchive runtime
//!     probe (`target/zip-aosp-runtime-probe --archive-runtime`,
//!     P1.6). Runs anywhere; no KVM required. The classifier +
//!     finding archive + replay tool exercise the same code paths
//!     as real mode, so dev-mode disagreements are
//!     production-quality findings.
//!   - **real**: Nyx snapshot of a Cuttlefish A14 image. Requires
//!     `/dev/kvm`, libnyx, a Cuttlefish CVD; gated behind the
//!     `nyx-cuttlefish` Cargo feature **and** the `--target=cf-a14`
//!     CLI flag. Falls back to dev mode with a loud warning if the
//!     target is unavailable.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::uninlined_format_args,
    clippy::missing_errors_doc,
    clippy::too_many_arguments,
    clippy::doc_markdown,
    clippy::manual_strip,
    clippy::single_range_in_vec_init,
    clippy::manual_range_contains,
    clippy::items_after_statements,
    clippy::missing_panics_doc,
    clippy::significant_drop_in_scrutinee,
    clippy::significant_drop_tightening,
    clippy::too_long_first_doc_paragraph,
    clippy::needless_pass_by_value,
    clippy::single_match_else,
    clippy::match_wildcard_for_single_variants,
    clippy::option_if_let_else,
    clippy::manual_let_else,
    clippy::redundant_closure_for_method_calls,
    clippy::missing_const_for_fn
)]

pub mod archive;
pub mod classifier;
pub mod cuttlefish;
pub mod differ;
pub mod grammar;
pub mod mutator;
