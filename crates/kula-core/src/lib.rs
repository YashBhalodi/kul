//! Kula language core: lexer, parser, semantic analyzer, validator, diagnostics.
//!
//! This crate is the reusable language-implementation library that powers the
//! `kula` CLI and (in Phase 3) the language server.

pub mod ast;
pub mod diagnostic;
pub mod lexer;
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
