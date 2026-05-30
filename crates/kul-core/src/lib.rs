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
//! (manifest YAML, [InputFile, …])
//!   → manifest::validate    → (Manifest, KUL-Mxx diagnostics)
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
pub mod cycles;
pub mod date;
pub mod diagnostic;
pub mod export;
pub mod field_meta;
pub mod format;
pub mod lexer;
pub mod manifest;
pub mod node_at;
pub mod parser;
pub mod semantic;
pub mod span;
pub mod validator;

use std::sync::Arc;

use crate::ast::{Document, InputFile, KulFile};
use crate::diagnostic::{Diagnostic, fspan, manifest_codes};
use crate::manifest::Manifest;
use crate::semantic::ResolvedDocument;
use crate::span::{ByteSpan, FileId};

/// The version of `kul-core` linked into the consumer.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Outcome of the full pipeline. Holds the [`ResolvedDocument`] (cached
/// for repeat kinship queries by the LSP) plus the merged diagnostic
/// list.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub resolved: ResolvedDocument,
    pub diagnostics: Vec<Diagnostic>,
    /// Manifest from the caller's `kul.yml`, or [`Manifest::default`] if
    /// the manifest pass couldn't produce one (look in `diagnostics` for
    /// `KUL-Mxx`). Surfaced in the export envelope's `kul:` field.
    pub manifest: Manifest,
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

/// One-call entry point: validate the manifest YAML, lex/parse every
/// input, resolve, validate, return merged diagnostics + the cached
/// resolved view. Only available with the `yaml` feature (WASM builds
/// opt out and use [`check_with_manifest`]).
#[must_use = "CheckResult carries the pipeline's diagnostics — inspect them to surface errors"]
#[cfg(feature = "yaml")]
pub fn check(
    manifest_name: impl Into<String>,
    manifest_yaml: &str,
    inputs: &[InputFile],
) -> CheckResult {
    let (manifest_opt, mut diagnostics) = manifest::validate(manifest_yaml, FileId::MANIFEST);
    let manifest = manifest_opt.unwrap_or_default();
    // Empty manifest_yaml = in-memory caller not asserting a project; skip
    // M06. Non-empty yaml with zero inputs is structurally empty.
    if !manifest_yaml.is_empty() && inputs.is_empty() {
        diagnostics.push(empty_project_diagnostic(manifest_yaml.len().min(1)));
    }
    run_pipeline(
        manifest_name.into(),
        manifest_yaml,
        manifest,
        inputs,
        diagnostics,
    )
}

/// Variant of [`check`] for callers with an already-typed [`Manifest`]
/// (the WASM bridge). Skips the `KUL-Mxx` validator pass. M06 fires
/// whenever `inputs` is empty since the typed manifest itself is the
/// project-assertion. `manifest_yaml` is used only to render diagnostics
/// that anchor into `kul.yml`.
#[must_use = "CheckResult carries the pipeline's diagnostics — inspect them to surface errors"]
pub fn check_with_manifest(
    manifest_name: impl Into<String>,
    manifest_yaml: &str,
    manifest: &Manifest,
    inputs: &[InputFile],
) -> CheckResult {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    if inputs.is_empty() {
        diagnostics.push(empty_project_diagnostic(manifest_yaml.len().min(1)));
    }
    run_pipeline(
        manifest_name.into(),
        manifest_yaml,
        manifest.clone(),
        inputs,
        diagnostics,
    )
}

/// Shared tail of [`check`] and [`check_with_manifest`]. Lex/parses
/// every input, resolves, validates, and assembles the [`CheckResult`].
fn run_pipeline(
    manifest_name: String,
    manifest_yaml: &str,
    manifest: Manifest,
    inputs: &[InputFile],
    mut diagnostics: Vec<Diagnostic>,
) -> CheckResult {
    let document = build_document(manifest_name, manifest_yaml, inputs, &mut diagnostics);
    let document = Arc::new(document);

    let (resolved, resolve_diags) = semantic::resolve(document);
    diagnostics.extend(resolve_diags);
    diagnostics.extend(validator::validate(&resolved));

    CheckResult {
        resolved,
        diagnostics,
        manifest,
    }
}

/// Build the KUL-M06 "manifest but no `.kul` files" diagnostic. Span
/// anchors at byte 0 of the manifest source; `span_len` is 1 when bytes
/// exist, 0 when bytes-less (WASM bridge).
fn empty_project_diagnostic(span_len: usize) -> Diagnostic {
    Diagnostic::error(
        manifest_codes::M06_EMPTY_PROJECT,
        "project has a `kul.yml` but no `.kul` files — add at least one `.kul` file alongside the manifest",
        fspan(FileId::MANIFEST, ByteSpan::new(0, span_len)),
    )
}

fn build_document(
    manifest_name: String,
    manifest_yaml: &str,
    inputs: &[InputFile],
    diagnostics: &mut Vec<Diagnostic>,
) -> Document {
    let mut kul_files = Vec::with_capacity(inputs.len());
    for (i, input) in inputs.iter().enumerate() {
        let file = FileId((i + 1) as u32);
        let tokens = lexer::tokenize(&input.source);
        let (statements, parse_diags) = parser::parse(&tokens, file);
        diagnostics.extend(parse_diags);
        kul_files.push(Arc::new(KulFile::new(
            input.name.clone(),
            input.source.clone(),
            statements,
        )));
    }
    Document::with_manifest_source(manifest_name, manifest_yaml, kul_files)
}
