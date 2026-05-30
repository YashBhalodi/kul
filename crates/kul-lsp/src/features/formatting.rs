//! `textDocument/formatting` handler.
//!
//! Wraps [`kul_core::format::format_source`] in a single LSP `TextEdit`
//! that replaces the entire document. The formatter is idempotent and
//! AST-preserving (ADR-0004). Refuses inputs with parse errors so the
//! editor falls back to user input instead of mangling broken source;
//! validator-rule errors (KUL-Rxx) are still formattable.

use kul_core::diagnostic::{Diagnostic, Severity};
use kul_core::span::FileId;
use tower_lsp::lsp_types::{Position, Range, TextEdit};

use crate::convert::LineIndex;

/// Format the document if it parses cleanly. Returns `None` on parse
/// errors so the editor leaves the buffer alone. Sibling-file diagnostics
/// are ignored — formatting one file doesn't depend on its siblings parsing.
pub fn formatting(
    source: &str,
    diagnostics: &[Diagnostic],
    line_index: &LineIndex,
    file: FileId,
) -> Option<Vec<TextEdit>> {
    if has_parse_errors(diagnostics, file) {
        return None;
    }
    let formatted = kul_core::format::format_source(source);
    if formatted == source {
        // Empty edit list (not a no-op edit) — some clients re-render on
        // structural no-op edits.
        return Some(Vec::new());
    }
    let end = line_index.position(source.len());
    Some(vec![TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end,
        },
        new_text: formatted,
    }])
}

fn has_parse_errors(diags: &[Diagnostic], file: FileId) -> bool {
    diags.iter().any(|d| {
        matches!(d.severity, Severity::Error)
            && (d.code.starts_with("KUL-L") || d.code.starts_with("KUL-P"))
            && d.primary.is_some_and(|p| p.file == file)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::test_open_file;

    #[test]
    fn returns_empty_edit_list_when_already_canonical() {
        let source = "person alice  name:\"A\"  gender:female\n";
        let doc = test_open_file(source);
        let v = doc.view();
        let edits = formatting(source, &doc.check.diagnostics, v.line_index, v.file).unwrap();
        assert!(edits.is_empty());
    }

    #[test]
    fn returns_full_doc_replacement_when_dirty() {
        let source = "person alice name:\"A\" gender:female\n";
        let doc = test_open_file(source);
        let v = doc.view();
        let edits = formatting(source, &doc.check.diagnostics, v.line_index, v.file).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(
            edits[0].range.start,
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            edits[0].new_text,
            "person alice  name:\"A\"  gender:female\n"
        );
    }

    #[test]
    fn refuses_to_format_input_with_parse_errors() {
        let source = "person\n";
        let doc = test_open_file(source);
        let v = doc.view();
        assert!(formatting(source, &doc.check.diagnostics, v.line_index, v.file).is_none());
    }

    #[test]
    fn formats_through_validation_errors() {
        // `person` with no fields — fires R03 but structurally sound.
        let source = "person alice\n";
        let doc = test_open_file(source);
        assert!(
            doc.check
                .diagnostics
                .iter()
                .any(|d| d.code.starts_with("KUL-R"))
        );
        let v = doc.view();
        let edits = formatting(source, &doc.check.diagnostics, v.line_index, v.file).unwrap();
        assert!(edits.is_empty());
    }
}
