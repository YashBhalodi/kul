//! Diagnostic translation: `kul_core::Diagnostic` → `lsp_types::Diagnostic`.
//!
//! Pure function called from `server::Backend` after every parse. The
//! caller publishes the result via `Client::publish_diagnostics`.
//!
//! Per the file-identity refactor (issue #70), each LSP `Url` corresponds
//! to one `kul_core::span::FileId`. We filter the diagnostic stream to
//! the diagnostics anchored at that id (so a manifest diagnostic at
//! `FileId::MANIFEST` doesn't leak into a `.kul` URI's squiggle list)
//! and clamp related-info entries to the same file (the LSP protocol's
//! `Location` model expects an in-document URI).

use kul_core::diagnostic::{Diagnostic as CoreDiagnostic, Severity as CoreSeverity};
use kul_core::span::FileId;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString, Url,
};

use crate::convert::LineIndex;

/// Source name used in every published LSP diagnostic. Editors use this
/// to group diagnostics from the same producer.
pub const SOURCE: &str = "kul";

/// Translate a slice of `kul-core` diagnostics into LSP diagnostics for
/// the URI at `file`.
///
/// Diagnostics whose primary anchor is in another file are skipped:
/// LSP can only attach squiggles to the document the request came in
/// for. Unanchored diagnostics (e.g. `KUL-M01`) are also skipped on a
/// `.kul` URI — `KUL-M01` is the CLI's responsibility (the LSP detects
/// missing manifests through its own channel; surfacing them as
/// squiggles in the editor would be confusing for a file the editor is
/// happy to open).
pub fn to_lsp(
    uri: &Url,
    diagnostics: &[CoreDiagnostic],
    line_index: &LineIndex,
    file: FileId,
) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter_map(|d| to_lsp_one(uri, d, line_index, file))
        .collect()
}

/// Translate a single `kul-core` diagnostic. Returns `None` when the
/// diagnostic's primary anchor lives in a different file (or has no
/// anchor at all).
pub(crate) fn to_lsp_one(
    uri: &Url,
    d: &CoreDiagnostic,
    idx: &LineIndex,
    file: FileId,
) -> Option<Diagnostic> {
    let primary = d.primary.filter(|p| p.file == file)?;
    let related = d
        .related
        .iter()
        // Cross-file related-info isn't expressible without a multi-URI
        // surface; v1's resolver never produces them in practice.
        .filter(|r| r.span.file == file)
        .map(|r| DiagnosticRelatedInformation {
            location: Location {
                uri: uri.clone(),
                range: idx.range(r.span.span),
            },
            message: r.label.clone(),
        })
        .collect::<Vec<_>>();

    Some(Diagnostic {
        range: idx.range(primary.span),
        severity: Some(severity_to_lsp(d.severity)),
        code: Some(NumberOrString::String(d.code.to_owned())),
        code_description: None,
        source: Some(SOURCE.to_owned()),
        message: d.message.clone(),
        related_information: if related.is_empty() {
            None
        } else {
            Some(related)
        },
        tags: None,
        data: None,
    })
}

fn severity_to_lsp(s: CoreSeverity) -> DiagnosticSeverity {
    match s {
        CoreSeverity::Error => DiagnosticSeverity::ERROR,
        CoreSeverity::Warning => DiagnosticSeverity::WARNING,
        CoreSeverity::Note => DiagnosticSeverity::INFORMATION,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kul_core::diagnostic::fspan;
    use kul_core::span::ByteSpan;

    fn url() -> Url {
        Url::parse("file:///t.kul").unwrap()
    }

    fn idx(source: &str) -> LineIndex {
        LineIndex::new(source)
    }

    fn file_one() -> FileId {
        FileId::from_raw(1)
    }

    #[test]
    fn empty_input_produces_empty_output() {
        let out = to_lsp(&url(), &[], &idx(""), file_one());
        assert!(out.is_empty());
    }

    #[test]
    fn passes_code_message_source() {
        let core = CoreDiagnostic::error(
            "KUL-R03",
            "missing name",
            fspan(file_one(), ByteSpan::new(0, 5)),
        );
        let lsp = to_lsp(
            &url(),
            std::slice::from_ref(&core),
            &idx("hello"),
            file_one(),
        );
        assert_eq!(lsp.len(), 1);
        let d = &lsp[0];
        assert_eq!(d.code, Some(NumberOrString::String("KUL-R03".into())));
        assert_eq!(d.message, "missing name");
        assert_eq!(d.source.as_deref(), Some("kul"));
        assert_eq!(d.severity, Some(DiagnosticSeverity::ERROR));
    }

    #[test]
    fn diagnostics_in_a_different_file_are_filtered_out() {
        let manifest_diag = CoreDiagnostic::error(
            "KUL-M03",
            "missing kul",
            fspan(FileId::MANIFEST, ByteSpan::new(0, 1)),
        );
        // Active URI corresponds to FileId(1) — the manifest-anchored
        // diagnostic must be filtered out.
        let lsp = to_lsp(
            &url(),
            std::slice::from_ref(&manifest_diag),
            &idx("hello"),
            file_one(),
        );
        assert!(
            lsp.is_empty(),
            "manifest-anchored diagnostic must not surface on a .kul URI"
        );
    }

    #[test]
    fn range_uses_utf16_for_multibyte() {
        let core = CoreDiagnostic::error(
            "KUL-R01",
            "duplicate",
            fspan(file_one(), ByteSpan::new(1, 5)),
        );
        let lsp = to_lsp(
            &url(),
            std::slice::from_ref(&core),
            &idx("a🎉b"),
            file_one(),
        );
        let d = &lsp[0];
        assert_eq!(d.range.start.line, 0);
        assert_eq!(d.range.start.character, 1);
    }

    #[test]
    fn related_info_carries_doc_uri() {
        let core = CoreDiagnostic::error(
            "KUL-R01",
            "duplicate",
            fspan(file_one(), ByteSpan::new(0, 5)),
        )
        .with_related(fspan(file_one(), ByteSpan::new(6, 11)), "prior declaration");
        let lsp = to_lsp(
            &url(),
            std::slice::from_ref(&core),
            &idx("hello\nworld\n"),
            file_one(),
        );
        let related = lsp[0]
            .related_information
            .as_ref()
            .expect("related info present");
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].message, "prior declaration");
        assert_eq!(related[0].location.uri, url());
        assert_eq!(related[0].location.range.start.line, 1);
    }
}
