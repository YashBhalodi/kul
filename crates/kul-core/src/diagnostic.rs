//! Diagnostic types emitted by every layer of `kul-core`.
//!
//! A [`Diagnostic`] carries an `Option<FileSpan>` (unanchored = manifest
//! "file not found"), a stable code, and a message. The wrapper
//! [`RenderableDiagnostic`] resolves source bytes by [`FileId`] against a
//! [`crate::ast::Document`] for miette rendering.

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
/// declaration when reporting a duplicate ID).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedSpan {
    pub span: FileSpan,
    pub label: String,
}

/// A diagnostic produced by any `kul-core` pass. Stable by `code`.
///
/// `primary` is `Option` because `KUL-M01` (manifest missing on disk) has
/// no source position; such diagnostics carry context in `message`.
///
/// `detail` is an optional sub-case discriminator used when one code
/// covers multiple conditions on the same span — consumers match on it
/// instead of parsing `message`. See the `detail` module constants.
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
    /// Build an error diagnostic anchored at `primary`.
    #[must_use]
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

    /// Build an error diagnostic with no source anchor (e.g. `KUL-M01`,
    /// manifest not found). The would-be path goes in `message`.
    #[must_use]
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

    /// Build a warning diagnostic anchored at `primary`.
    #[must_use]
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

    #[must_use]
    pub fn with_related(mut self, span: FileSpan, label: impl Into<String>) -> Self {
        self.related.push(RelatedSpan {
            span,
            label: label.into(),
        });
        self
    }

    /// Tag this diagnostic with a sub-case discriminator (see `detail::*`).
    #[must_use]
    pub fn with_detail(mut self, detail: &'static str) -> Self {
        self.detail = Some(detail);
        self
    }
}

/// Build a [`FileSpan`] from a `(file, byte-span)` pair.
#[must_use]
pub fn fspan(file: FileId, span: ByteSpan) -> FileSpan {
    FileSpan::new(file, span)
}

/// Canonical `Diagnostic::detail` tags identifying which sub-case of a
/// rule fired when one code covers multiple conditions on coinciding
/// spans.
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

/// Stable codes for manifest-validation diagnostics. See
/// [`spec/14-project-manifest.md`](../../../spec/14-project-manifest.md).
pub mod manifest_codes {
    /// Manifest not found at the expected path (unanchored).
    pub const M01_MISSING: &str = "KUL-M01";
    /// Manifest YAML failed to parse.
    pub const M02_MALFORMED_YAML: &str = "KUL-M02";
    /// Manifest missing the required `kul:` field.
    pub const M03_MISSING_KUL_FIELD: &str = "KUL-M03";
    /// Manifest's `kul:` value is not a recognized version.
    pub const M04_UNKNOWN_VERSION: &str = "KUL-M04";
    /// Unknown top-level manifest field (warning).
    pub const M05_UNKNOWN_FIELD: &str = "KUL-M05";
    /// Project has `kul.yml` but zero sibling `.kul` files.
    pub const M06_EMPTY_PROJECT: &str = "KUL-M06";
}

/// Wraps a [`Diagnostic`] with the source bytes of its primary span's
/// file, for `miette` rendering. Build one per-diagnostic at the rendering
/// edge.
pub struct RenderableDiagnostic<'a> {
    /// Source bytes of the file the primary span points into. Empty for
    /// unanchored diagnostics (miette renders without a source block).
    pub source: &'a str,
    /// Display name (the `InputFile.name`, or `manifest_name` when
    /// unanchored).
    pub source_name: &'a str,
    pub diagnostic: &'a Diagnostic,
}

impl<'a> RenderableDiagnostic<'a> {
    /// Resolve the diagnostic's primary `FileId` against `document` to
    /// surface the right source bytes. Unanchored diagnostics fall back
    /// to the manifest name with an empty source block.
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

    /// Direct construction when the caller already has source and label.
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
        self.diagnostic.primary?;
        // `&self.source` is `&&str` — the `Sized` shape miette needs.
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
            // miette's SourceCode is single-file; cross-file related spans
            // (real under ADR-0015) get surfaced by the CLI as a "see also"
            // line instead of being drawn into this source block.
            .filter(move |r| r.span.file == primary.file)
            .map(|r| miette::LabeledSpan::new_with_span(Some(r.label.clone()), r.span.span));
        Some(Box::new(std::iter::once(primary_label).chain(related)))
    }
}
