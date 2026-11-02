//! Kul language core: lexer, parser, semantic analyzer, validator,
//! diagnostics.
//!
//! Reusable library powering the `kul` CLI and the `kul-lsp` language
//! server. Both call [`check`] once per project and read everything off
//! the resulting [`CheckResult`]. The [`ResolvedDocument`] is the
//! kinship-query seam (ADR-0001); file-identity is the multi-file seam
//! (ADR-0014).
//!
//! # Pipeline
//!
//! ```text
//! [InputFile, …]           (ordered by file name: the file order)
//!   → lexer::tokenize       → Vec<Token> per file
//!   → parser::parse         → (statements, parse diagnostics) per file
//!   → ast::Document         → multi-file container
//!   → semantic::resolve     → (ResolvedDocument, R01 diagnostics)
//!   → validator::validate   → R02..R13 diagnostics
//! ```
//!
//! Each pass strictly enriches; nothing earlier consults anything later.
//! See `docs/architecture.md` and `CONTEXT.md`.

pub mod ast;
mod cycles;
pub mod date;
pub mod diagnostic;
pub mod export;
pub mod field_meta;
pub mod format;
pub mod lexer;
pub mod node_at;
pub mod parser;
mod projection;
pub mod query;
pub mod semantic;
pub mod span;
pub mod validator;

use std::sync::Arc;

use crate::ast::{Document, InputFile, KulFile};
use crate::diagnostic::Diagnostic;
use crate::semantic::ResolvedDocument;
use crate::span::FileId;

/// The version of `kul-core` linked into the consumer.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Outcome of the full pipeline. Holds the [`ResolvedDocument`] (cached
/// for repeat kinship queries) plus the merged diagnostic list.
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

    /// Cached [`ResolvedDocument`] for kinship queries.
    pub fn resolved(&self) -> &ResolvedDocument {
        &self.resolved
    }

    /// Underlying parsed multi-file [`Document`].
    pub fn document(&self) -> &Document {
        self.resolved.document()
    }
}

/// One-call entry point: lex/parse every file of a project, resolve,
/// validate, and return the merged diagnostics plus the cached resolved
/// view.
///
/// A project is a flat set of uniquely named `.kul` files. `check` puts
/// `inputs` in **file order** — the byte-wise lexicographic order of their
/// names (spec §2, *Projects*) — before anything else, so the result is
/// independent of the order the caller listed them in: `FileId(i)` is the
/// `i`-th file in file order, and project-wide declaration order (which
/// file's duplicate is "the second", which entity an export lists first)
/// follows it. An empty project is a valid, empty family.
#[must_use = "CheckResult carries the pipeline's diagnostics — inspect them to surface errors"]
pub fn check(inputs: &[InputFile]) -> CheckResult {
    let mut ordered: Vec<&InputFile> = inputs.iter().collect();
    ordered.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));

    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let document = Arc::new(build_document(&ordered, &mut diagnostics));

    let (resolved, resolve_diags) = semantic::resolve(document);
    diagnostics.extend(resolve_diags);
    diagnostics.extend(validator::validate(&resolved));

    CheckResult {
        resolved,
        diagnostics,
    }
}

fn build_document(inputs: &[&InputFile], diagnostics: &mut Vec<Diagnostic>) -> Document {
    let kul_files = inputs
        .iter()
        .zip(0u32..)
        .map(|(input, idx)| {
            let tokens = lexer::tokenize(&input.source);
            let (statements, parse_diags) = parser::parse(tokens, FileId(idx));
            diagnostics.extend(parse_diags);
            Arc::new(KulFile::new(
                input.name.clone(),
                Arc::clone(&input.source),
                statements,
            ))
        })
        .collect();
    Document::new(kul_files)
}
