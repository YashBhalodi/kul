//! Quick fixes: one-step repairs for diagnostics that have an obvious one.
//!
//! Dispatch is a direct `match` on the diagnostic code, one arm per `KUL-Rxx`
//! that ships a fix, and on the diagnostic's `detail` tag for sub-cases
//! (ADR-0006), so the wiring survives validator message changes.

use kul_core::CheckResult;
use kul_core::ast::{MarriageFieldKind, MarriageStmt, PersonStmt};
use kul_core::diagnostic::{Diagnostic as CoreDiagnostic, detail};
use kul_core::span::{ByteSpan, FileId};
use serde::Serialize;
#[cfg(feature = "tsify")]
use tsify::Tsify;

use crate::diagnostics::{Diagnostic, diagnostic};
use crate::{Span, TextEdit, file_id};

/// One repair for one diagnostic. Its edits all apply to the file the fix
/// was asked for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct QuickFix {
    /// What the fix does (`Add `gender:female``).
    pub title: String,
    /// The diagnostic it repairs.
    pub diagnostic: Diagnostic,
    /// Its edits to the file, in source order.
    pub edits: Vec<TextEdit>,
}

/// Every quick fix for a diagnostic in `file` whose range touches `span`.
#[must_use]
pub fn quick_fixes(check: &CheckResult, file: &str, span: Span) -> Vec<QuickFix> {
    let Some(file) = file_id(check, file) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for diag in &check.diagnostics {
        let Some(primary) = diag.primary.filter(|p| p.file == file) else {
            continue;
        };
        if !Span::from(primary.span).touches(span) {
            continue;
        }
        let fixes = match diag.code {
            "KUL-R03" => r03_required_fields(check, file, diag),
            "KUL-R05" => r05_end_consistency(check, file, diag),
            _ => continue,
        };
        out.extend(fixes);
    }
    out
}

/// KUL-R03: required field missing on a person or marriage.
///
/// Three sub-cases (missing `name:`, missing `gender:`, missing marriage
/// `start:`) all anchored on the same `id.span`; we dispatch on `diag.detail`.
fn r03_required_fields(check: &CheckResult, file: FileId, diag: &CoreDiagnostic) -> Vec<QuickFix> {
    let mut out = Vec::new();
    let Some(primary) = diag.primary else {
        return out;
    };
    let Some(p) = check
        .resolved()
        .persons_in(file)
        .find(|p| p.id.span == primary.span)
    else {
        // R03 on marriage (missing `start:`) — no quick fix; the author must supply a date.
        return out;
    };
    match diag.detail {
        Some(detail::R03_MISSING_GENDER) => {
            for value in ["male", "female", "other"] {
                out.push(add_person_field(
                    check,
                    p,
                    &format!("gender:{value}"),
                    &format!("Add `gender:{value}`"),
                    diag,
                ));
            }
        }
        Some(detail::R03_MISSING_NAME) => {
            out.push(add_person_field(
                check,
                p,
                "name:\"\"",
                "Add `name:\"\"`",
                diag,
            ));
        }
        _ => {}
    }
    out
}

/// KUL-R05: `end:` and `end_reason:` must both be present or both absent.
///
/// Locates the enclosing marriage by containment — R05 anchors on the
/// offending field, not the marriage's outer span.
fn r05_end_consistency(check: &CheckResult, file: FileId, diag: &CoreDiagnostic) -> Vec<QuickFix> {
    let mut out = Vec::new();
    let Some(primary) = diag.primary else {
        return out;
    };
    let Some(m) = check
        .resolved()
        .marriages_in(file)
        .find(|m| span_contains(m.span, primary.span))
    else {
        return out;
    };
    match diag.detail {
        Some(detail::R05_END_WITHOUT_END_REASON) => {
            out.push(add_marriage_field(
                check,
                m,
                "end_reason:divorce",
                "Add `end_reason:divorce`",
                diag,
            ));
        }
        Some(detail::R05_END_REASON_WITHOUT_END) => {
            if let Some(fix) = remove_end_reason(check, m, "Remove `end_reason:` field", diag) {
                out.push(fix);
            }
        }
        _ => {}
    }
    out
}

fn add_person_field(
    check: &CheckResult,
    p: &PersonStmt,
    field_text: &str,
    title: &str,
    source: &CoreDiagnostic,
) -> QuickFix {
    // After the last header field, or after the id if there are none.
    // Sub-statements (`birth`, `adoption`) come after.
    let at = p.last_field_end().unwrap_or(p.id.span.end);
    insertion(check, at, &format!(" {field_text}"), title, source)
}

fn add_marriage_field(
    check: &CheckResult,
    m: &MarriageStmt,
    field_text: &str,
    title: &str,
    source: &CoreDiagnostic,
) -> QuickFix {
    let at = m.last_field_end().unwrap_or(m.spouse_b.span.end);
    insertion(check, at, &format!(" {field_text}"), title, source)
}

fn remove_end_reason(
    check: &CheckResult,
    m: &MarriageStmt,
    title: &str,
    source: &CoreDiagnostic,
) -> Option<QuickFix> {
    let field = m
        .fields()
        .find(|f| matches!(f.kind, MarriageFieldKind::EndReason(_)))?;
    // Sweep up the single leading space so removal doesn't leave a double space.
    let span = ByteSpan::new(field.span.start.saturating_sub(1), field.span.end);
    Some(QuickFix {
        title: title.to_owned(),
        diagnostic: diagnostic(check, source),
        edits: vec![TextEdit {
            span: span.into(),
            text: String::new(),
        }],
    })
}

fn insertion(
    check: &CheckResult,
    at: usize,
    text: &str,
    title: &str,
    source: &CoreDiagnostic,
) -> QuickFix {
    QuickFix {
        title: title.to_owned(),
        diagnostic: diagnostic(check, source),
        edits: vec![TextEdit {
            span: Span { start: at, end: at },
            text: text.to_owned(),
        }],
    }
}

fn span_contains(outer: ByteSpan, inner: ByteSpan) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FILE, check_one, idx};

    fn fixes_for(source: &str) -> Vec<QuickFix> {
        let whole = Span {
            start: 0,
            end: source.len(),
        };
        quick_fixes(&check_one(source), FILE, whole)
    }

    fn apply(source: &str, fix: &QuickFix) -> String {
        let edit = fix.edits.first().expect("fix has an edit");
        let mut out = String::with_capacity(source.len() + edit.text.len());
        out.push_str(&source[..edit.span.start]);
        out.push_str(&edit.text);
        out.push_str(&source[edit.span.end..]);
        out
    }

    fn has_code(source: &str, code: &str) -> bool {
        check_one(source).diagnostics.iter().any(|d| d.code == code)
    }

    #[test]
    fn missing_gender_offers_three_quick_fixes() {
        let titles: Vec<String> = fixes_for("person alice name:\"A\"\n")
            .into_iter()
            .map(|f| f.title)
            .collect();
        for value in ["male", "female", "other"] {
            assert!(titles.contains(&format!("Add `gender:{value}`")));
        }
    }

    #[test]
    fn missing_name_offers_one_quick_fix() {
        let fixes = fixes_for("person alice gender:female\n");
        assert!(fixes.iter().any(|f| f.title == "Add `name:\"\"`"));
    }

    #[test]
    fn end_without_end_reason_offers_add_divorce() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1972 end:1990\n";
        assert!(
            fixes_for(src)
                .iter()
                .any(|f| f.title == "Add `end_reason:divorce`")
        );
    }

    #[test]
    fn end_reason_without_end_offers_remove_field() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1972 end_reason:divorce\n";
        assert!(
            fixes_for(src)
                .iter()
                .any(|f| f.title.contains("Remove `end_reason:`"))
        );
    }

    #[test]
    fn no_diagnostic_yields_no_fixes() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:1972\n";
        assert!(fixes_for(src).is_empty());
    }

    #[test]
    fn unrelated_diagnostic_codes_are_ignored() {
        // KUL-R04 (self-marriage) has no registered fix.
        let src = "person alice name:\"A\" gender:female\n\
                   marriage m alice alice start:1972\n";
        assert!(fixes_for(src).is_empty());
    }

    #[test]
    fn add_gender_fix_is_inserted_inline_with_existing_fields() {
        let src = "person alice name:\"A\"\n";
        let fixes = fixes_for(src);
        let male = fixes
            .iter()
            .find(|f| f.title == "Add `gender:male`")
            .unwrap();
        assert_eq!(apply(src, male), "person alice name:\"A\" gender:male\n");
    }

    #[test]
    fn add_name_fix_is_inserted_at_end_of_header() {
        let src = "person alice gender:female\n";
        let fixes = fixes_for(src);
        let fix = fixes.iter().find(|f| f.title == "Add `name:\"\"`").unwrap();
        assert_eq!(apply(src, fix), "person alice gender:female name:\"\"\n");
    }

    #[test]
    fn end_reason_fix_resolves_diagnostic_after_apply() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1972 end:1990\n";
        let fixes = fixes_for(src);
        let fix = fixes
            .iter()
            .find(|f| f.title == "Add `end_reason:divorce`")
            .unwrap();
        assert!(!has_code(&apply(src, fix), "KUL-R05"));
    }

    #[test]
    fn remove_end_reason_fix_resolves_diagnostic_after_apply() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1972 end_reason:divorce\n";
        let fixes = fixes_for(src);
        let fix = fixes
            .iter()
            .find(|f| f.title.contains("Remove `end_reason:`"))
            .unwrap();
        assert!(!has_code(&apply(src, fix), "KUL-R05"));
    }

    #[test]
    fn missing_gender_fix_resolves_diagnostic_after_apply() {
        let src = "person alice name:\"Alice\"\n";
        let fixes = fixes_for(src);
        let fix = fixes
            .iter()
            .find(|f| f.title == "Add `gender:female`")
            .unwrap();
        let fixed = apply(src, fix);
        assert!(
            !check_one(&fixed)
                .diagnostics
                .iter()
                .any(|d| d.code == "KUL-R03" && d.message.contains("`gender:`")),
        );
    }

    #[test]
    fn span_filter_excludes_unrelated_lines() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob\n"; // bob missing both name AND gender
        let first_line = Span {
            start: 0,
            end: idx(src, "\n") + 1,
        };
        assert!(quick_fixes(&check_one(src), FILE, first_line).is_empty());
    }

    #[test]
    fn snapshot_quick_fixes_for_missing_gender() {
        let titles: Vec<String> = fixes_for("person alice name:\"Alice\"\n")
            .into_iter()
            .map(|f| f.title)
            .collect();
        insta::assert_json_snapshot!(titles);
    }

    #[test]
    fn snapshot_fixes_for_end_without_end_reason() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1972 end:1990\n";
        insta::assert_json_snapshot!(fixes_for(src));
    }
}
