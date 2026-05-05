//! Kula language core: lexer, parser, semantic analyzer, validator, diagnostics.
//!
//! This crate is the reusable language-implementation library that powers the
//! `kula` CLI and the `kula-lsp` language server. Both consumers call
//! [`check`] once per source string; everything else hangs off the resulting
//! `CheckResult`. The [`ResolvedDocument`] inside the result is the kinship-
//! query seam ([ADR-0001](../../docs/adr/0001-resolved-document-as-query-seam.md)).
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
pub mod field_meta;
pub mod format;
pub mod lexer;
pub mod node_at;
pub mod parser;
pub mod semantic;
pub mod span;
pub mod validator;

use std::sync::Arc;

use crate::ast::Document;
use crate::diagnostic::Diagnostic;
use crate::semantic::ResolvedDocument;

/// The version of `kula-core` linked into the consumer.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Outcome of running the full `lex → parse → resolve → validate` pipeline
/// over a source string.
///
/// Holds the [`ResolvedDocument`] (which itself owns an `Arc<Document>`) plus
/// the merged diagnostic list. The resolved view is cached inside the result
/// so callers can issue queries — hover, find-references, code-actions —
/// without re-running `semantic::resolve`. This is the cache the LSP relies
/// on: each `did_change` produces one `CheckResult`, and every subsequent
/// LSP request reads through `result.resolved` directly.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub resolved: ResolvedDocument,
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| matches!(d.severity, diagnostic::Severity::Error))
    }

    /// The cached [`ResolvedDocument`] view for kinship queries. Stable
    /// across calls; no recomputation. Equivalent to `&self.resolved` —
    /// this method exists so call sites can read like a query (`.resolved()`)
    /// rather than a field access if they prefer.
    pub fn resolved(&self) -> &ResolvedDocument {
        &self.resolved
    }

    /// The underlying parsed [`Document`]. Forwarded from the resolved view
    /// so callers don't need to know about the indirection.
    pub fn document(&self) -> &Document {
        self.resolved.document()
    }
}

/// One-call entry point: lex, parse, resolve, validate, return the merged
/// diagnostics together with the cached resolved view.
pub fn check(source: &str) -> CheckResult {
    let tokens = lexer::tokenize(source);
    let (document, mut diagnostics) = parser::parse(&tokens);
    let document = Arc::new(document);
    let (resolved, resolve_diags) = semantic::resolve(document);
    diagnostics.extend(resolve_diags);
    diagnostics.extend(validator::validate(&resolved));
    CheckResult {
        resolved,
        diagnostics,
    }
}
