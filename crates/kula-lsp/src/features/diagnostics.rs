//! Diagnostic translation: `kula_core::Diagnostic` → `lsp_types::Diagnostic`.
//!
//! Pure function called from `server::Backend` after every parse. The
//! caller publishes the result via `Client::publish_diagnostics`.

use kula_core::diagnostic::{Diagnostic as CoreDiagnostic, Severity as CoreSeverity};
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString, Url,
};

use crate::convert::LineIndex;

/// Source name used in every published LSP diagnostic. Editors use this to
/// group diagnostics from the same producer.
pub const SOURCE: &str = "kula";

/// Translate a slice of `kula-core` diagnostics into LSP diagnostics.
///
/// The `uri` is the document the diagnostics belong to; it's used as the
/// `Location.uri` for related-info entries (which are constrained to point
/// inside the same document for v1 — there are no cross-file links).
pub fn to_lsp(
    uri: &Url,
    diagnostics: &[CoreDiagnostic],
    line_index: &LineIndex,
) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .map(|d| translate_one(uri, d, line_index))
        .collect()
}

fn translate_one(uri: &Url, d: &CoreDiagnostic, idx: &LineIndex) -> Diagnostic {
    let related = d
        .related
        .iter()
        .map(|r| DiagnosticRelatedInformation {
            location: Location {
                uri: uri.clone(),
                range: idx.range(r.span),
            },
            message: r.label.clone(),
        })
        .collect::<Vec<_>>();

    Diagnostic {
        range: idx.range(d.primary),
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
    }
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
    use kula_core::span::ByteSpan;

    fn url() -> Url {
        Url::parse("file:///t.kula").unwrap()
    }

    fn idx(source: &str) -> LineIndex {
        LineIndex::new(source)
    }

    #[test]
    fn empty_input_produces_empty_output() {
        let out = to_lsp(&url(), &[], &idx(""));
        assert!(out.is_empty());
    }

    #[test]
    fn passes_code_message_source() {
        let core = CoreDiagnostic::error("KULA-R03", "missing name", ByteSpan::new(0, 5));
        let lsp = &to_lsp(&url(), std::slice::from_ref(&core), &idx("hello"))[0];
        assert_eq!(lsp.code, Some(NumberOrString::String("KULA-R03".into())));
        assert_eq!(lsp.message, "missing name");
        assert_eq!(lsp.source.as_deref(), Some("kula"));
        assert_eq!(lsp.severity, Some(DiagnosticSeverity::ERROR));
    }

    #[test]
    fn range_uses_utf16_for_multibyte() {
        let source = "a🎉b";
        let core = CoreDiagnostic::error("KULA-R01", "duplicate", ByteSpan::new(1, 5));
        let lsp = &to_lsp(&url(), std::slice::from_ref(&core), &idx(source))[0];
        // 🎉 starts at byte 1 (UTF-16 col 1) and spans 4 bytes / 2 UTF-16
        // units. `b` lives at byte 5 (UTF-16 col 3).
        assert_eq!(lsp.range.start.line, 0);
        assert_eq!(lsp.range.start.character, 1);
        assert_eq!(lsp.range.end.line, 0);
        assert_eq!(lsp.range.end.character, 3);
    }

    #[test]
    fn related_info_carries_doc_uri() {
        let core = CoreDiagnostic::error("KULA-R01", "duplicate", ByteSpan::new(0, 5))
            .with_related(ByteSpan::new(6, 11), "prior declaration");
        let lsp = &to_lsp(&url(), std::slice::from_ref(&core), &idx("hello\nworld\n"))[0];
        let related = lsp
            .related_information
            .as_ref()
            .expect("related info present");
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].message, "prior declaration");
        assert_eq!(related[0].location.uri, url());
        assert_eq!(related[0].location.range.start.line, 1);
    }

    #[test]
    fn no_related_info_field_when_empty() {
        let core = CoreDiagnostic::error("KULA-R03", "missing", ByteSpan::new(0, 1));
        let lsp = &to_lsp(&url(), std::slice::from_ref(&core), &idx("a"))[0];
        assert!(lsp.related_information.is_none());
    }

    #[test]
    fn snapshot_multi_error_fixture() {
        let source = "kula 1
person dup_a name:\"A\" gender:female
person dup_a name:\"A2\" gender:female
person bad_dates name:\"B\" gender:female born:2000 died:1950
person noname
marriage bad_self bad_dates bad_dates start:2010
";
        let core = kula_core::check(source);
        let lsp = to_lsp(&url(), &core.diagnostics, &idx(source));
        insta::assert_json_snapshot!(lsp);
    }

    #[test]
    fn one_thousand_statement_check_and_translate_under_budget() {
        let mut source = String::from("kula 1\n");
        for i in 0..1000 {
            use std::fmt::Write as _;
            let _ = writeln!(&mut source, "person p{i} name:\"P{i}\" gender:female");
        }
        let start = std::time::Instant::now();
        let core = kula_core::check(&source);
        let line_index = LineIndex::new(&source);
        let _ = to_lsp(&url(), &core.diagnostics, &line_index);
        let elapsed = start.elapsed();

        eprintln!("1000-statement parse + check + to_lsp: {elapsed:?}");
        // PRD target is 100ms. CI runners and debug builds are slower than
        // a developer laptop, so assert a generous 500ms ceiling — enough
        // to catch a 5x regression without flaking.
        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "1000-statement budget exceeded: {elapsed:?}"
        );
    }
}
