//! `textDocument/formatting` handler.
//!
//! Wraps [`kula_core::format::format_source`] in a single LSP `TextEdit`
//! that replaces the entire document. The formatter is idempotent and
//! AST-preserving (per ADR 0004), so a whole-document replacement is the
//! simplest correct shape — it sidesteps the diff-minimization machinery
//! editors implement themselves when the response just describes the
//! before-and-after.
//!
//! Refuses to format inputs with parse errors, returning an empty edit
//! list so the editor falls back to whatever the user typed instead of
//! silently mangling broken source. Validator-rule errors (KULA-Rxx) are
//! ignored — they don't prevent the AST from being structurally
//! formattable.

use kula_core::diagnostic::{Diagnostic, Severity};
use tower_lsp::lsp_types::{Position, Range, TextEdit};

use crate::convert::LineIndex;

/// Format the document if it parses cleanly. Returns `None` if the parse
/// produced any error-severity lex/parse diagnostics; in that case the
/// editor receives an empty response and leaves the buffer alone.
pub fn formatting(
    source: &str,
    diagnostics: &[Diagnostic],
    line_index: &LineIndex,
) -> Option<Vec<TextEdit>> {
    if has_parse_errors(diagnostics) {
        return None;
    }
    let formatted = kula_core::format::format_source(source);
    if formatted == source {
        // Already canonical — return an empty edit list rather than a no-op
        // edit. Some clients re-render even when an edit is structurally a
        // no-op; an empty list short-circuits that.
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

fn has_parse_errors(diags: &[Diagnostic]) -> bool {
    diags.iter().any(|d| {
        matches!(d.severity, Severity::Error)
            && (d.code.starts_with("KULA-L") || d.code.starts_with("KULA-P"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx(source: &str) -> LineIndex {
        LineIndex::new(source)
    }

    #[test]
    fn returns_empty_edit_list_when_already_canonical() {
        let source = "person alice  name:\"A\"  gender:female\n";
        let result = kula_core::check(source);
        let edits = formatting(source, &result.diagnostics, &idx(source)).unwrap();
        assert!(edits.is_empty());
    }

    #[test]
    fn returns_full_doc_replacement_when_dirty() {
        let source = "person alice name:\"A\" gender:female\n";
        let result = kula_core::check(source);
        let edits = formatting(source, &result.diagnostics, &idx(source)).unwrap();
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
        let result = kula_core::check(source);
        assert!(formatting(source, &result.diagnostics, &idx(source)).is_none());
    }

    #[test]
    fn formats_through_validation_errors() {
        // A `person` with no fields — fires R03 but the structure is sound.
        let source = "person alice\n";
        let result = kula_core::check(source);
        // Sanity check: there's a validator error here.
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code.starts_with("KULA-R"))
        );
        // ...but the formatter still runs.
        let edits = formatting(source, &result.diagnostics, &idx(source)).unwrap();
        // Already canonical (`person alice\n`).
        assert!(edits.is_empty());
    }
}
