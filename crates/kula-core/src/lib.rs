//! Kula language core: lexer, parser, semantic analyzer, validator, diagnostics.
//!
//! This crate is the reusable language-implementation library that powers the
//! `kula` CLI and the `kula-lsp` language server. Both consumers call
//! [`check`] once per source string; everything else hangs off the resulting
//! `CheckResult` (an AST and a diagnostic list) or the `ResolvedDocument`
//! query surface in [`semantic`].
//!
//! # Pipeline
//!
//! ```text
//! source: &str
//!   → lexer::tokenize  → Vec<Token>
//!   → parser::parse    → (Document, Vec<Diagnostic>)
//!   → semantic::resolve → (ResolvedDocument, Vec<Diagnostic>)
//!   → validator::validate → Vec<Diagnostic>
//! ```
//!
//! Each pass produces a strictly richer artifact; nothing earlier in the
//! pipeline ever consults something later. See `docs/architecture.md` in the
//! repository for the data-flow diagram and seam table, and `CONTEXT.md` for
//! the canonical vocabulary used in this crate.

pub mod ast;
pub mod cycles;
pub mod date;
pub mod diagnostic;
pub mod lexer;
pub mod node_at;
pub mod parser;
pub mod semantic;
pub mod span;
pub mod validator;

use crate::ast::Document;
use crate::diagnostic::Diagnostic;

/// The version of `kula-core` linked into the consumer.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Outcome of running the full `lex → parse → resolve → validate` pipeline
/// over a source string. The `Document` is always returned (it may be
/// partial); diagnostics describe everything wrong with the input.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub document: Document,
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| matches!(d.severity, diagnostic::Severity::Error))
    }
}

/// One-call entry point: lex, parse, resolve, validate, return the merged
/// diagnostics.
pub fn check(source: &str) -> CheckResult {
    let tokens = lexer::tokenize(source);
    let (document, mut diagnostics) = parser::parse(&tokens);
    let (resolved, resolve_diags) = semantic::resolve(&document);
    diagnostics.extend(resolve_diags);
    diagnostics.extend(validator::validate(&resolved));
    CheckResult {
        document,
        diagnostics,
    }
}
