//! Kula language core: lexer, parser, semantic analyzer, validator, diagnostics.
//!
//! This crate is the reusable language-implementation library that powers the
//! `kula` CLI and (in Phase 3) the language server.

/// The version of `kula-core` linked into the consumer.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
