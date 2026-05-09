//! Kul language core: lexer, parser, semantic analyzer, validator,
//! diagnostics.
//!
//! This crate is the reusable language-implementation library that powers
//! the `kul` CLI and the `kul-lsp` language server. Both consumers call
//! [`check`] once per project (a manifest plus zero or more `.kul`
//! inputs) and read everything else off the resulting [`CheckResult`].
//! The [`ResolvedDocument`] inside is the kinship-query seam
//! ([ADR-0001](../../docs/adr/0001-resolved-document-as-query-seam.md))
//! and the file-identity types are the multi-file seam
//! ([ADR-0014](../../docs/adr/0014-file-identity-and-per-file-namespaces.md)).
//!
//! # Pipeline
//!
//! ```text
//! (manifest YAML, [InputFile, …])
//!   → manifest::validate    → (Manifest, manifest diagnostics with `KUL-Mxx` codes)
//!   → lexer::tokenize       → Vec<Token> per file
//!   → parser::parse         → (statements, parse diagnostics) per file
//!   → ast::Document         → multi-file container the rest of the
//!                              pipeline operates on
//!   → semantic::resolve     → (ResolvedDocument, R01 diagnostics)
//!   → validator::validate   → R02..R13 diagnostics
//! ```
//!
//! Each pass produces a strictly richer artifact; nothing earlier in the
//! pipeline ever consults something later. See `docs/architecture.md` in
//! the repository for the data-flow diagram and seam table, and
//! `CONTEXT.md` for the canonical vocabulary used in this crate.

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
use crate::diagnostic::Diagnostic;
use crate::manifest::Manifest;
use crate::semantic::ResolvedDocument;
use crate::span::FileId;

/// The version of `kul-core` linked into the consumer.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Outcome of running the full
/// `manifest-validate → lex → parse → resolve → validate` pipeline over a
/// project (a manifest plus N input files).
///
/// Holds the [`ResolvedDocument`] (which itself owns an `Arc<Document>`)
/// plus the merged diagnostic list. The resolved view is cached inside
/// the result so callers can issue queries — hover, find-references,
/// code-actions — without re-running `semantic::resolve`. This is the
/// cache the LSP relies on: each `did_change` produces one
/// `CheckResult`, and every subsequent LSP request reads through
/// `result.resolved` directly.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub resolved: ResolvedDocument,
    pub diagnostics: Vec<Diagnostic>,
    /// The project manifest the check resolved against. Sourced from the
    /// caller's `kul.yml` (or [`Manifest::default`] if the manifest pass
    /// could not produce one — in which case `diagnostics` carries the
    /// `KUL-Mxx` codes that explain why). Surfaced in the export
    /// envelope's `kul:` field.
    pub manifest: Manifest,
}

impl CheckResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| matches!(d.severity, diagnostic::Severity::Error))
    }

    /// The cached [`ResolvedDocument`] view for kinship queries. Stable
    /// across calls; no recomputation. Equivalent to `&self.resolved` —
    /// this method exists so call sites can read like a query
    /// (`.resolved()`) rather than a field access if they prefer.
    pub fn resolved(&self) -> &ResolvedDocument {
        &self.resolved
    }

    /// The underlying parsed multi-file [`Document`]. Forwarded from the
    /// resolved view so callers don't need to know about the indirection.
    pub fn document(&self) -> &Document {
        self.resolved.document()
    }
}

/// One-call entry point: run the manifest validator, lex/parse every
/// input file, resolve, validate, and return the merged diagnostics
/// together with the cached resolved view.
///
/// The `manifest_yaml` argument is the **raw bytes** the adapter loaded
/// from `kul.yml` (empty `&str` if the file was missing on disk —
/// callers in that case prepend a [`KUL-M01`](crate::diagnostic::manifest_codes)
/// diagnostic to their own out-of-band channel, since `kul-core` cannot
/// know about the would-be path). The function thread-throughs:
///
/// 1. parses the manifest YAML, collecting `KUL-Mxx` diagnostics;
/// 2. lex/parses every `InputFile` into a [`KulFile`] (each at a fresh
///    [`FileId`]);
/// 3. assembles a multi-file [`Document`] (manifest at
///    [`FileId::MANIFEST`]; `.kul` files at `FileId(1..)`);
/// 4. runs the resolver and validator; and
/// 5. returns the merged diagnostic list.
///
/// In-memory callers (the `format_source` helper, ad-hoc tests) that
/// don't have a real `kul.yml` may pass an empty `manifest_yaml` and a
/// [`Manifest::default`] in the resulting `CheckResult` — see
/// [`check_with_manifest`] when the manifest is already built.
///
/// Only available when the `yaml` feature is enabled (the default; WASM
/// builds opt out and use [`check_with_manifest`] instead, since the JS
/// host hands the bridge a typed manifest, not raw YAML).
#[cfg(feature = "yaml")]
pub fn check(
    manifest_name: impl Into<String>,
    manifest_yaml: &str,
    inputs: &[InputFile],
) -> CheckResult {
    let manifest_name = manifest_name.into();
    let (manifest_opt, mut diagnostics) = manifest::validate(manifest_yaml, FileId::MANIFEST);
    let manifest = manifest_opt.unwrap_or_default();

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

/// Variant of [`check`] for callers that already have a typed
/// [`Manifest`] in hand (the WASM bridge: the JS host hands a typed
/// manifest object across the ABI). Skips the `KUL-Mxx` validator pass —
/// the caller is responsible for routing structural manifest errors
/// through whichever surface its protocol prefers (the WASM bridge
/// raises a `tsify` exception for structurally-malformed manifests; an
/// already-typed manifest can't fail those checks). `manifest_yaml` may
/// be empty; only its bytes are used as the manifest source for any
/// `.kul`-side diagnostic that anchors into `kul.yml`.
pub fn check_with_manifest(
    manifest_name: impl Into<String>,
    manifest_yaml: &str,
    manifest: &Manifest,
    inputs: &[InputFile],
) -> CheckResult {
    let manifest_name = manifest_name.into();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let document = build_document(manifest_name, manifest_yaml, inputs, &mut diagnostics);
    let document = Arc::new(document);

    let (resolved, resolve_diags) = semantic::resolve(document);
    diagnostics.extend(resolve_diags);
    diagnostics.extend(validator::validate(&resolved));

    CheckResult {
        resolved,
        diagnostics,
        manifest: manifest.clone(),
    }
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
        kul_files.push(Arc::new(KulFile {
            name: input.name.clone(),
            source: input.source.clone(),
            statements,
        }));
    }
    Document {
        manifest_name,
        manifest_source: manifest_yaml.to_string(),
        kul_files,
    }
}
