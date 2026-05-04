//! Diagnostic types emitted by every layer of `kula-core`.
//!
//! A [`Diagnostic`] is source-agnostic: it carries spans, a stable code, and
//! a message. Consumers (the CLI, the LSP) wrap it with the source string
//! when rendering — see [`Diagnostic::with_source`] for the miette path.

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub primary: ByteSpan,
    pub related: Vec<RelatedSpan>,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>, primary: ByteSpan) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            primary,
            related: Vec::new(),
        }
    }

    pub fn with_related(mut self, span: ByteSpan, label: impl Into<String>) -> Self {
        self.related.push(RelatedSpan {
            span,
            label: label.into(),
        });
        self
    }
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
