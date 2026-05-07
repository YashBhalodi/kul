//! Diagnostic types emitted by every layer of `kul-core`.
//!
//! A [`Diagnostic`] is source-agnostic: it carries spans, a stable code, and
//! a message. Consumers (the CLI, the LSP) wrap it with the source string
//! when rendering — see [`RenderableDiagnostic`] for the miette path.

use crate::span::ByteSpan;

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
    pub span: ByteSpan,
    pub label: String,
}

/// A diagnostic produced by the lexer, parser, semantic analyzer, or
/// validator. Stable across releases by `code`.
///
/// `detail` is an optional sub-case discriminator, used when one rule code
/// covers multiple distinguishable conditions on the same span. Consumers
/// that change behavior per-condition (e.g. the code-action provider that
/// suggests different fixes for missing `name:` vs. missing `gender:`)
/// match on it instead of parsing the human-facing `message`. Tags are
/// declared next to the rule producer; see the `detail` constants below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub primary: ByteSpan,
    pub related: Vec<RelatedSpan>,
    pub detail: Option<&'static str>,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>, primary: ByteSpan) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            primary,
            related: Vec::new(),
            detail: None,
        }
    }

    pub fn with_related(mut self, span: ByteSpan, label: impl Into<String>) -> Self {
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

/// Wraps a [`Diagnostic`] with the source string for `miette` rendering.
///
/// Build one of these per-diagnostic at the rendering edge (the CLI, the
/// LSP). The wrapper owns nothing it does not need to: the source string is
/// borrowed.
pub struct RenderableDiagnostic<'a> {
    pub source: &'a str,
    pub source_name: &'a str,
    pub diagnostic: &'a Diagnostic,
}

impl<'a> RenderableDiagnostic<'a> {
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
        Some(&self.source)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        let primary = miette::LabeledSpan::new_primary_with_span(
            Some(self.diagnostic.message.clone()),
            self.diagnostic.primary,
        );
        let related = self
            .diagnostic
            .related
            .iter()
            .map(|r| miette::LabeledSpan::new_with_span(Some(r.label.clone()), r.span));
        Some(Box::new(std::iter::once(primary).chain(related)))
    }
}
