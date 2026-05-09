//! Diagnostic types emitted by every layer of `kul-core`.
//!
//! A [`Diagnostic`] is project-aware: it carries a [`FileSpan`] (or
//! `Option<FileSpan>` for diagnostics that have no anchor — manifest
//! "file not found" being the canonical case), a stable code, and a
//! message. The wrapper [`RenderableDiagnostic`] takes a reference to the
//! whole [`crate::ast::Document`] so it can resolve any file's source by
//! [`FileId`] for miette rendering — no caller has to thread around a
//! `&str` source argument that only matches one file.

use crate::ast::Document;
use crate::span::{ByteSpan, FileId, FileSpan};

/// Severity level for a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

/// A secondary span attached to a diagnostic for context (e.g. the prior
/// declaration when reporting a duplicate ID). Always file-anchored —
/// related info without a position would be a plain note in `message`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedSpan {
    pub span: FileSpan,
    pub label: String,
}

/// A diagnostic produced by the lexer, parser, semantic analyzer,
/// validator, or manifest validator pass. Stable across releases by
/// `code`.
///
/// `primary` is `Option<FileSpan>` because some diagnostics — notably
/// `KUL-M01` (the manifest is missing on disk) — have no source position
/// to anchor on. Such diagnostics carry their context entirely in
/// `message`. Every other diagnostic carries a real `FileSpan`; the LSP
/// and CLI rendering paths short-circuit unanchored ones to a
/// no-source-block layout.
///
/// `detail` is an optional sub-case discriminator, used when one rule
/// code covers multiple distinguishable conditions on the same span.
/// Consumers that change behavior per-condition (e.g. the code-action
/// provider that suggests different fixes for missing `name:` vs.
/// missing `gender:`) match on it instead of parsing the human-facing
/// `message`. Tags are declared next to the rule producer; see the
/// `detail` constants below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub primary: Option<FileSpan>,
    pub related: Vec<RelatedSpan>,
    pub detail: Option<&'static str>,
}

impl Diagnostic {
    /// Build an error diagnostic anchored at `primary`. Use
    /// [`Diagnostic::warning`] / [`Diagnostic::note`] for non-error
    /// severities, or [`Diagnostic::unanchored_error`] when the diagnostic
    /// has no source position to point at (manifest-not-found).
    pub fn error(code: &'static str, message: impl Into<String>, primary: FileSpan) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            primary: Some(primary),
            related: Vec::new(),
            detail: None,
        }
    }

    /// Build an error diagnostic with no source anchor. Used for
    /// `KUL-M01` ("manifest not found") and any future code that surfaces
    /// project-state failures the toolchain detects before opening a
    /// file. The would-be path is expected in `message`.
    pub fn unanchored_error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            primary: None,
            related: Vec::new(),
            detail: None,
        }
    }

    /// Build a warning diagnostic anchored at `primary`. Today only the
    /// `KUL-M05` (unknown manifest field) rule fires at this severity.
    pub fn warning(code: &'static str, message: impl Into<String>, primary: FileSpan) -> Self {
        Self {
            code,
            severity: Severity::Warning,
            message: message.into(),
            primary: Some(primary),
            related: Vec::new(),
            detail: None,
        }
    }

    pub fn with_related(mut self, span: FileSpan, label: impl Into<String>) -> Self {
        self.related.push(RelatedSpan {
            span,
            label: label.into(),
        });
        self
    }

    /// Tag this diagnostic with a sub-case discriminator. See the
    /// `detail::*` module constants for the canonical values.
    pub fn with_detail(mut self, detail: &'static str) -> Self {
        self.detail = Some(detail);
        self
    }
}

/// Helper: build a [`FileSpan`] from a `(file, byte-span)` pair. Common
/// enough inside the parser/validator that a free function is worth the
/// keystrokes it saves.
pub fn fspan(file: FileId, span: ByteSpan) -> FileSpan {
    FileSpan::new(file, span)
}

/// Canonical `Diagnostic::detail` tags. A tag identifies *which sub-case*
/// of a rule fired when one rule code (e.g. `KUL-R03`) covers multiple
/// conditions whose primary spans coincide. Both the validator (producer)
/// and the LSP code-action provider (consumer) reference the same
/// constants — adding a new tag is a one-line change in both places.
pub mod detail {
    /// R03: `person` is missing its required `name:` field.
    pub const R03_MISSING_NAME: &str = "r03-missing-name";
    /// R03: `person` is missing its required `gender:` field.
    pub const R03_MISSING_GENDER: &str = "r03-missing-gender";
    /// R03: `marriage` is missing its required `start:` field.
    pub const R03_MISSING_MARRIAGE_START: &str = "r03-missing-marriage-start";
    /// R05: `marriage` has `end:` but no `end_reason:`.
    pub const R05_END_WITHOUT_END_REASON: &str = "r05-end-without-end-reason";
    /// R05: `marriage` has `end_reason:` but no `end:`.
    pub const R05_END_REASON_WITHOUT_END: &str = "r05-end-reason-without-end";
}

/// Stable codes for manifest-validation diagnostics. Defined here (next to
/// `Diagnostic` itself) so the manifest validator pass and the spec
/// reference the same string constants.
///
/// See [`spec/14-project-manifest.md`](../../../spec/14-project-manifest.md)
/// and [`spec/07-validation-rules.md`](../../../spec/07-validation-rules.md)
/// for the normative descriptions.
pub mod manifest_codes {
    /// Manifest not found at the expected path. Unanchored — the would-be
    /// path is in the message.
    pub const M01_MISSING: &str = "KUL-M01";
    /// Manifest YAML failed to parse. Anchors at the line/column the
    /// YAML parser reported.
    pub const M02_MALFORMED_YAML: &str = "KUL-M02";
    /// Manifest is well-formed YAML but missing the required `kul:`
    /// field. Anchors at the manifest start.
    pub const M03_MISSING_KUL_FIELD: &str = "KUL-M03";
    /// Manifest's `kul:` value is not a recognized Kul language version.
    /// Anchors at the value.
    pub const M04_UNKNOWN_VERSION: &str = "KUL-M04";
    /// Manifest carries a top-level field the v1 schema does not know
    /// about. Severity warning. Anchors at the field key.
    pub const M05_UNKNOWN_FIELD: &str = "KUL-M05";
}

/// Wraps a [`Diagnostic`] together with the source bytes of the file its
/// primary span anchors into, for `miette` rendering.
///
/// Build one of these per-diagnostic at the rendering edge (the CLI, the
/// LSP). [`RenderableDiagnostic::for_diagnostic`] does the file lookup
/// against a [`Document`] for you; consumers that want a different
/// rendering surface (e.g. a precomputed source string) can construct
/// the struct directly.
pub struct RenderableDiagnostic<'a> {
    /// The source bytes of the file the diagnostic's primary span
    /// points into. Empty for unanchored diagnostics so miette renders
    /// without a source block.
    pub source: &'a str,
    /// Display name (the `InputFile.name` the toolchain originally fed
    /// in, or the manifest's `manifest_name` for unanchored diagnostics).
    pub source_name: &'a str,
    pub diagnostic: &'a Diagnostic,
}

impl<'a> RenderableDiagnostic<'a> {
    /// Build a [`RenderableDiagnostic`] from a [`Document`] and a
    /// [`Diagnostic`]. Looks up the diagnostic's primary `FileId` in the
    /// document to surface the right source bytes for miette to render.
    /// Unanchored diagnostics fall back to the manifest's name with an
    /// empty source block (no caret/snippet, just code + message).
    pub fn for_diagnostic(document: &'a Document, diagnostic: &'a Diagnostic) -> Self {
        let primary = diagnostic.primary;
        let (source, source_name) = match primary {
            Some(p) => (
                document.source_of(p.file).unwrap_or(""),
                document.name_of(p.file).unwrap_or(""),
            ),
            None => ("", document.manifest_name.as_str()),
        };
        Self {
            source,
            source_name,
            diagnostic,
        }
    }

    /// Direct construction — when the caller already has the source
    /// string and label in hand and doesn't need the document lookup.
    pub fn new(source: &'a str, source_name: &'a str, diagnostic: &'a Diagnostic) -> Self {
        Self {
            source,
            source_name,
            diagnostic,
        }
    }
}

impl std::fmt::Debug for RenderableDiagnostic<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderableDiagnostic")
            .field("source_name", &self.source_name)
            .field("diagnostic", &self.diagnostic)
            .finish()
    }
}

impl std::fmt::Display for RenderableDiagnostic<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.diagnostic.message)
    }
}

impl std::error::Error for RenderableDiagnostic<'_> {}

impl miette::Diagnostic for RenderableDiagnostic<'_> {
    fn code<'b>(&'b self) -> Option<Box<dyn std::fmt::Display + 'b>> {
        Some(Box::new(self.diagnostic.code))
    }

    fn severity(&self) -> Option<miette::Severity> {
        Some(match self.diagnostic.severity {
            Severity::Error => miette::Severity::Error,
            Severity::Warning => miette::Severity::Warning,
            Severity::Note => miette::Severity::Advice,
        })
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        // Unanchored diagnostics surface no source block; everything
        // they need lives in the message.
        self.diagnostic.primary?;
        // `&self.source` is `&&str`, which is `Sized` — that's the
        // shape miette needs to cast to `&dyn SourceCode`.
        Some(&self.source)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        let primary = self.diagnostic.primary?;
        let primary_label = miette::LabeledSpan::new_primary_with_span(
            Some(self.diagnostic.message.clone()),
            primary.span,
        );
        let related = self
            .diagnostic
            .related
            .iter()
            // Only related spans in the same file can be rendered by the
            // single-source miette path. Cross-file related labels would
            // need a multi-source frontend; v1's diagnostics never
            // produce them, so a filter is the simplest answer.
            .filter(move |r| r.span.file == primary.file)
            .map(|r| miette::LabeledSpan::new_with_span(Some(r.label.clone()), r.span.span));
        Some(Box::new(std::iter::once(primary_label).chain(related)))
    }
}
