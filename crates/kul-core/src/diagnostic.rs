//! Diagnostic types emitted by every layer of `kul-core`.
//!
//! A [`Diagnostic`] carries an `Option<FileSpan>` (unanchored = a
//! project-wide fact with no single declaration to point at), a stable
//! code, and a message. The wrapper
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
/// `primary` is `Option` because some diagnostics describe the project or
/// the request as a whole rather than one declaration (an unknown query
/// anchor, a render-shape invariant); such diagnostics carry their context
/// in `message`.
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
    /// R05: `marriage` has `end:` but no `end_reason:`.
    pub const R05_END_WITHOUT_END_REASON: &str = "r05-end-without-end-reason";
    /// R05: `marriage` has `end_reason:` but no `end:`.
    pub const R05_END_REASON_WITHOUT_END: &str = "r05-end-reason-without-end";
}

/// One diagnostic code the toolchain can report, with a one-line summary
/// of the problem it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticCode {
    /// The stable code (`KUL-R03`).
    pub code: &'static str,
    /// What the code means, in a few words.
    pub summary: &'static str,
}

/// Every diagnostic code the Kul toolchain reports, in code order: the
/// lexer (`L`), the parser (`P`), the validator's rules (`R`), the renderer
/// (`V`) and the CLI's queries (`Q`).
///
/// This is the catalogue the docs' diagnostics reference is checked
/// against. `tests/diagnostic_catalog.rs` holds it equal to the codes the
/// workspace's sources emit, so a new code cannot ship without an entry.
pub const DIAGNOSTIC_CODES: &[DiagnosticCode] = &[
    DiagnosticCode {
        code: "KUL-L01",
        summary: "a string without its closing quote, or with an unknown escape",
    },
    DiagnosticCode {
        code: "KUL-P01",
        summary: "a line that is not a `person` or `marriage` statement",
    },
    DiagnosticCode {
        code: "KUL-P03",
        summary: "a missing or malformed identifier",
    },
    DiagnosticCode {
        code: "KUL-P04",
        summary: "something other than a field where a field belongs",
    },
    DiagnosticCode {
        code: "KUL-P05",
        summary: "a field name without its `:`",
    },
    DiagnosticCode {
        code: "KUL-P07",
        summary: "a text field whose value is not a string",
    },
    DiagnosticCode {
        code: "KUL-P08",
        summary: "a `gender:` value other than `male`, `female` or `other`",
    },
    DiagnosticCode {
        code: "KUL-P10",
        summary: "a field that does not belong on the statement",
    },
    DiagnosticCode {
        code: "KUL-P11",
        summary: "a date field without a date",
    },
    DiagnosticCode {
        code: "KUL-P12",
        summary: "an `end_reason:` without a value",
    },
    DiagnosticCode {
        code: "KUL-P13",
        summary: "more than one `birth` line under one person",
    },
    DiagnosticCode {
        code: "KUL-P14",
        summary: "a field that does not belong on `adoption`",
    },
    DiagnosticCode {
        code: "KUL-P15",
        summary: "a date that is not written `YYYY`, `YYYY-MM` or `YYYY-MM-DD`",
    },
    DiagnosticCode {
        code: "KUL-P16",
        summary: "a date that does not exist, such as month 13 or 30 February",
    },
    DiagnosticCode {
        code: "KUL-R01",
        summary: "the same id declared twice in the project",
    },
    DiagnosticCode {
        code: "KUL-R02",
        summary: "a reference to an id the project does not declare",
    },
    DiagnosticCode {
        code: "KUL-R03",
        summary: "a person without `name:` or `gender:`",
    },
    DiagnosticCode {
        code: "KUL-R04",
        summary: "a marriage whose two spouses are the same person",
    },
    DiagnosticCode {
        code: "KUL-R05",
        summary: "`end:` without `end_reason:`, or `end_reason:` without `end:`",
    },
    DiagnosticCode {
        code: "KUL-R05b",
        summary: "an `end_reason:` value other than `divorce`",
    },
    DiagnosticCode {
        code: "KUL-R06",
        summary: "a person who died before they were born",
    },
    DiagnosticCode {
        code: "KUL-R07",
        summary: "a marriage that ends before it starts",
    },
    DiagnosticCode {
        code: "KUL-R08",
        summary: "an adoption that ends before it starts",
    },
    DiagnosticCode {
        code: "KUL-R09",
        summary: "a marriage that starts before a spouse was born",
    },
    DiagnosticCode {
        code: "KUL-R10",
        summary: "a marriage that starts after a spouse died",
    },
    DiagnosticCode {
        code: "KUL-R11",
        summary: "a child born before a birth parent",
    },
    DiagnosticCode {
        code: "KUL-R12",
        summary: "an adoption that starts before an adoptive parent was born",
    },
    DiagnosticCode {
        code: "KUL-R13",
        summary: "a person who is their own ancestor",
    },
    DiagnosticCode {
        code: "KUL-R14",
        summary: "a person in two ongoing marriages who does not host both",
    },
    DiagnosticCode {
        code: "KUL-R15",
        summary: "the same field set twice on one statement",
    },
    DiagnosticCode {
        code: "KUL-V01",
        summary: "a lineage too deep to draw",
    },
    DiagnosticCode {
        code: "KUL-V02",
        summary: "the renderer was handed a graph it does not draw",
    },
    DiagnosticCode {
        code: "KUL-V03",
        summary: "the renderer found the family graph inconsistent",
    },
    DiagnosticCode {
        code: "KUL-Q01",
        summary: "a query names a person the project does not declare",
    },
    DiagnosticCode {
        code: "KUL-Q02",
        summary: "a query's filter is not one the CLI understands",
    },
];

/// Wraps a [`Diagnostic`] with the source bytes of its primary span's
/// file, for `miette` rendering. Build one per-diagnostic at the rendering
/// edge.
pub struct RenderableDiagnostic<'a> {
    /// Source bytes of the file the primary span points into. Empty for
    /// unanchored diagnostics (miette renders without a source block).
    pub source: &'a str,
    /// Display name (the `InputFile.name`; empty when unanchored).
    pub source_name: &'a str,
    pub diagnostic: &'a Diagnostic,
}

impl<'a> RenderableDiagnostic<'a> {
    /// Resolve the diagnostic's primary `FileId` against `document` to
    /// surface the right source bytes. Unanchored diagnostics render with
    /// no name and an empty source block.
    pub fn for_diagnostic(document: &'a Document, diagnostic: &'a Diagnostic) -> Self {
        let primary = diagnostic.primary;
        let (source, source_name) = match primary {
            Some(p) => (
                document.source_of(p.file).unwrap_or(""),
                document.name_of(p.file).unwrap_or(""),
            ),
            None => ("", ""),
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
