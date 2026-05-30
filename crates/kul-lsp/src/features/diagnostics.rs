//! Diagnostic translation: `kul_core::Diagnostic` → `lsp_types::Diagnostic`.
//!
//! Filters by `FileId` so a manifest-anchored diagnostic doesn't leak onto a
//! `.kul` URI. Related-info pointing at sibling files (common under ADR-0015
//! for R01 duplicates and R02 type-mismatch) is anchored at the sibling URI.

use kul_core::diagnostic::{Diagnostic as CoreDiagnostic, Severity as CoreSeverity};
use kul_core::span::FileId;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString, Url,
};

use crate::convert::LineIndex;
use crate::state::ProjectEntry;

/// Source name used in every published LSP diagnostic.
pub const SOURCE: &str = "kul";

/// Translate every diagnostic whose primary anchor lives in `file` into
/// LSP diagnostics. Diagnostics anchored elsewhere (other files, the
/// manifest) are skipped — LSP attaches squiggles to one URI at a time.
pub fn to_lsp(
    entry: &ProjectEntry,
    file: FileId,
    diagnostics: &[CoreDiagnostic],
) -> Vec<Diagnostic> {
    let Some(uri) = entry.url_for(file) else {
        return Vec::new();
    };
    let Some(line_index) = entry.line_index_for(file) else {
        return Vec::new();
    };
    diagnostics
        .iter()
        .filter_map(|d| translate(d, file, line_index, uri, Some(entry)))
        .collect()
}

/// Translate a single same-file diagnostic for callers without a full
/// [`ProjectEntry`] (the code-action provider). Cross-file related-info
/// falls back to the diagnostic's own URI — quick-fix codes (R03/R05)
/// never produce cross-file related-info, so the fallback is unreachable.
pub(crate) fn to_lsp_one(
    uri: &Url,
    d: &CoreDiagnostic,
    idx: &LineIndex,
    file: FileId,
) -> Option<Diagnostic> {
    translate(d, file, idx, uri, None)
}

fn translate(
    d: &CoreDiagnostic,
    file: FileId,
    primary_line_index: &LineIndex,
    primary_uri: &Url,
    entry: Option<&ProjectEntry>,
) -> Option<Diagnostic> {
    let primary = d.primary.filter(|p| p.file == file)?;
    let related = d
        .related
        .iter()
        .filter_map(|r| {
            // Anchor each related span at the URI of its own file.
            let (uri, line_index) = if r.span.file == file {
                (primary_uri.clone(), primary_line_index)
            } else {
                let entry = entry?;
                let related_uri = entry.url_for(r.span.file)?;
                let related_idx = entry.line_index_for(r.span.file)?;
                (related_uri.clone(), related_idx)
            };
            Some(DiagnosticRelatedInformation {
                location: Location {
                    uri,
                    range: line_index.range(r.span.span),
                },
                message: r.label.clone(),
            })
        })
        .collect::<Vec<_>>();

    Some(Diagnostic {
        range: primary_line_index.range(primary.span),
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
    use crate::state::{test_open_file, test_project_entry, test_url as url};
    use kul_core::diagnostic::fspan;
    use kul_core::span::ByteSpan;

    fn file_one() -> FileId {
        FileId::from_raw(1)
    }

    #[test]
    fn empty_input_produces_empty_output() {
        let entry = test_open_file("");
        let out = to_lsp(&entry, file_one(), &[]);
        assert!(out.is_empty());
    }

    #[test]
    fn passes_code_message_source() {
        let entry = test_open_file("hello");
        let diag = CoreDiagnostic::error(
            "KUL-R03",
            "missing name",
            fspan(file_one(), ByteSpan::new(0, 5)),
        );
        let lsp = to_lsp(&entry, file_one(), std::slice::from_ref(&diag));
        assert_eq!(lsp.len(), 1);
        let d = &lsp[0];
        assert_eq!(d.code, Some(NumberOrString::String("KUL-R03".into())));
        assert_eq!(d.message, "missing name");
        assert_eq!(d.source.as_deref(), Some("kul"));
        assert_eq!(d.severity, Some(DiagnosticSeverity::ERROR));
    }

    #[test]
    fn diagnostics_in_a_different_file_are_filtered_out() {
        let entry = test_open_file("hello");
        let diag = CoreDiagnostic::error(
            "KUL-M03",
            "missing kul",
            fspan(FileId::MANIFEST, ByteSpan::new(0, 1)),
        );
        let lsp = to_lsp(&entry, file_one(), std::slice::from_ref(&diag));
        assert!(lsp.is_empty());
    }

    #[test]
    fn range_uses_utf16_for_multibyte() {
        let entry = test_open_file("a🎉b");
        let diag = CoreDiagnostic::error(
            "KUL-R01",
            "duplicate",
            fspan(file_one(), ByteSpan::new(1, 5)),
        );
        let lsp = to_lsp(&entry, file_one(), std::slice::from_ref(&diag));
        let d = &lsp[0];
        assert_eq!(d.range.start.line, 0);
        assert_eq!(d.range.start.character, 1);
    }

    #[test]
    fn related_info_carries_doc_uri() {
        let entry = test_open_file("hello\nworld\n");
        let diag = CoreDiagnostic::error(
            "KUL-R01",
            "duplicate",
            fspan(file_one(), ByteSpan::new(0, 5)),
        )
        .with_related(fspan(file_one(), ByteSpan::new(6, 11)), "prior declaration");
        let lsp = to_lsp(&entry, file_one(), std::slice::from_ref(&diag));
        let related = lsp[0]
            .related_information
            .as_ref()
            .expect("related info present");
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].message, "prior declaration");
        assert_eq!(related[0].location.uri, url());
        assert_eq!(related[0].location.range.start.line, 1);
    }

    /// Cross-file related-info (ADR-0015): related span in a sibling file
    /// surfaces with the sibling's URI.
    #[test]
    fn related_info_in_sibling_file_carries_sibling_uri() {
        let entry = test_project_entry(&[
            ("first.kul", "person alice name:\"A\" gender:female\n"),
            ("second.kul", "person alice name:\"A2\" gender:male\n"),
        ]);
        let f1 = FileId::from_raw(1);
        let f2 = FileId::from_raw(2);
        let diag = CoreDiagnostic::error(
            "KUL-R01",
            "duplicate id `alice`",
            fspan(f2, ByteSpan::new(7, 12)),
        )
        .with_related(fspan(f1, ByteSpan::new(7, 12)), "first declared here");

        let lsp = to_lsp(&entry, f2, std::slice::from_ref(&diag));
        let related = lsp[0]
            .related_information
            .as_ref()
            .expect("related info present");
        let first_url = Url::parse("file:///first.kul").unwrap();
        let second_url = Url::parse("file:///second.kul").unwrap();
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].location.uri, first_url);
        assert_eq!(entry.url_for(f2).cloned().unwrap(), second_url);
    }
}
