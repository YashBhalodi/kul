//! Format one file to the canonical layout.
//!
//! Wraps [`kul_core::format::format_source`] in one edit that replaces the
//! whole file. The formatter is idempotent and AST-preserving (ADR-0004).
//! Refuses a file with lex or parse errors, so broken source is left as the
//! author typed it; validator-rule errors (KUL-Rxx) are still formattable.

use kul_core::CheckResult;
use kul_core::diagnostic::Severity;
use kul_core::span::FileId;

use crate::{Span, TextEdit, file_id, source_of};

/// The edits that format `file`: none when it is already canonical, one
/// whole-file replacement otherwise. `None` when the file has lex or parse
/// errors (or the project has no such file). Errors in sibling files don't
/// matter — formatting one file doesn't depend on its siblings parsing.
#[must_use]
pub fn format(check: &CheckResult, file: &str) -> Option<Vec<TextEdit>> {
    let file = file_id(check, file)?;
    if has_parse_errors(check, file) {
        return None;
    }
    let source = source_of(check, file)?;
    let formatted = kul_core::format::format_source(source);
    if formatted == source {
        return Some(Vec::new());
    }
    Some(vec![TextEdit {
        span: Span {
            start: 0,
            end: source.len(),
        },
        text: formatted,
    }])
}

fn has_parse_errors(check: &CheckResult, file: FileId) -> bool {
    check.diagnostics.iter().any(|d| {
        matches!(d.severity, Severity::Error)
            && (d.code.starts_with("KUL-L") || d.code.starts_with("KUL-P"))
            && d.primary.is_some_and(|p| p.file == file)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FILE, check_one, check_project};

    #[test]
    fn returns_no_edits_when_already_canonical() {
        let source = "person alice  name:\"A\"  gender:female\n";
        assert_eq!(format(&check_one(source), FILE), Some(Vec::new()));
    }

    #[test]
    fn returns_whole_file_replacement_when_dirty() {
        let source = "person alice name:\"A\" gender:female\n";
        let edits = format(&check_one(source), FILE).unwrap();
        assert_eq!(
            edits,
            vec![TextEdit {
                span: Span {
                    start: 0,
                    end: source.len()
                },
                text: "person alice  name:\"A\"  gender:female\n".to_owned(),
            }]
        );
    }

    #[test]
    fn refuses_to_format_input_with_parse_errors() {
        assert!(format(&check_one("person\n"), FILE).is_none());
    }

    #[test]
    fn formats_through_validation_errors() {
        // `person` with no fields — fires R03 but structurally sound.
        let source = "person alice\n";
        let check = check_one(source);
        assert!(
            check
                .diagnostics
                .iter()
                .any(|d| d.code.starts_with("KUL-R"))
        );
        assert_eq!(format(&check, FILE), Some(Vec::new()));
    }

    #[test]
    fn a_sibling_parse_error_does_not_block_formatting() {
        let check = check_project(&[
            ("a.kul", "person alice name:\"A\" gender:female\n"),
            ("b.kul", "person\n"),
        ]);
        assert_eq!(format(&check, "a.kul").map(|e| e.len()), Some(1));
        assert!(format(&check, "b.kul").is_none());
    }
}
