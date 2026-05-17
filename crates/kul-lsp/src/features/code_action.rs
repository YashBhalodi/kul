//! Code actions for `textDocument/codeAction`.
//!
//! Quick-fixes are dispatched through a small registry keyed by diagnostic
//! code (per ADR-0001 / PRD section "Code action registry"). Each entry
//! maps a `KUL-Rxx` code to a provider function that, given the
//! diagnostic and the resolved document, returns zero or more
//! [`CodeAction`]s. Adding a new fix is one entry in the registry table
//! plus the provider function.
//!
//! The action's underlying [`WorkspaceEdit`] applies one or more text
//! edits to the document; clients invoke them via the lightbulb in the
//! gutter or the right-click menu.

use std::collections::HashMap;

use kul_core::ast::{MarriageFieldKind, MarriageStmt, PersonStmt};
use kul_core::diagnostic::{Diagnostic, detail};
use kul_core::semantic::ResolvedDocument;
use kul_core::span::ByteSpan;
use kul_core::span::FileId;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Position, Range, TextEdit, Url, WorkspaceEdit,
};

use crate::convert::LineIndex;

/// Build the list of quick-fixes that apply at `request_range`.
///
/// Walks the cached diagnostics, dispatches each one whose code is in the
/// registry, and filters by overlap with the request range so the user
/// only sees actions relevant to the cursor/selection.
pub fn code_actions(
    file: FileId,
    resolved: &ResolvedDocument,
    diagnostics: &[Diagnostic],
    line_index: &LineIndex,
    uri: &Url,
    request_range: Range,
) -> Vec<CodeActionOrCommand> {
    let registry = registry();
    let mut out = Vec::new();
    for diag in diagnostics {
        let provider = match registry.get(diag.code) {
            Some(p) => *p,
            None => continue,
        };
        // Quick-fixes only fire on diagnostics anchored in the active
        // file at a position the user is currently looking at.
        let Some(primary) = diag.primary.filter(|p| p.file == file) else {
            continue;
        };
        if !ranges_overlap(line_index.range(primary.span), request_range) {
            continue;
        }
        for action in provider(file, resolved, diag, line_index, uri) {
            out.push(CodeActionOrCommand::CodeAction(action));
        }
    }
    out
}

type ProviderFn = fn(FileId, &ResolvedDocument, &Diagnostic, &LineIndex, &Url) -> Vec<CodeAction>;

/// Registry of diagnostic-code → provider. Logically a `HashMap<&str,
/// ProviderFn>`; built lazily so it's `const`-style at the call site.
fn registry() -> HashMap<&'static str, ProviderFn> {
    let mut m: HashMap<&'static str, ProviderFn> = HashMap::new();
    m.insert("KUL-R03", r03_required_fields);
    m.insert("KUL-R05", r05_end_consistency);
    m
}

/// KUL-R03: a required field is missing on a person or marriage.
///
/// R03 covers three sub-cases (missing `name:`, missing `gender:`, missing
/// marriage `start:`) all anchored on the same `id.span`. We dispatch on
/// the diagnostic's `detail` tag — see `kul_core::diagnostic::detail` for
/// the canonical values — so the code-action wiring stays sound when the
/// validator's message text changes.
fn r03_required_fields(
    file: FileId,
    resolved: &ResolvedDocument,
    diag: &Diagnostic,
    line_index: &LineIndex,
    uri: &Url,
) -> Vec<CodeAction> {
    let mut out = Vec::new();
    let Some(primary) = diag.primary else {
        return out;
    };
    let Some(p) = resolved
        .persons_in(file)
        .find(|p| p.id.span == primary.span)
    else {
        // R03 on marriage (missing `start:`) — deliberately no quick fix;
        // the user has to supply a date.
        return out;
    };
    match diag.detail {
        Some(detail::R03_MISSING_GENDER) => {
            for value in ["male", "female", "other"] {
                out.push(add_person_field(
                    file,
                    p,
                    line_index,
                    uri,
                    &format!("gender:{value}"),
                    &format!("Add `gender:{value}`"),
                    diag,
                ));
            }
        }
        Some(detail::R03_MISSING_NAME) => {
            out.push(add_person_field(
                file,
                p,
                line_index,
                uri,
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
/// Two sub-cases (extra `end:` vs. extra `end_reason:`); we dispatch on
/// `diag.detail`. Each diagnostic's primary span sits inside a single
/// marriage; we locate it by containment (the spans aren't equal — R05
/// anchors on the offending field, not the marriage's outer span).
fn r05_end_consistency(
    file: FileId,
    resolved: &ResolvedDocument,
    diag: &Diagnostic,
    line_index: &LineIndex,
    uri: &Url,
) -> Vec<CodeAction> {
    let mut out = Vec::new();
    let Some(primary) = diag.primary else {
        return out;
    };
    let Some(m) = resolved
        .marriages_in(file)
        .find(|m| span_contains(m.span, primary.span))
    else {
        return out;
    };
    match diag.detail {
        Some(detail::R05_END_WITHOUT_END_REASON) => {
            out.push(add_marriage_field(
                file,
                m,
                line_index,
                uri,
                "end_reason:divorce",
                "Add `end_reason:divorce`",
                diag,
            ));
        }
        Some(detail::R05_END_REASON_WITHOUT_END) => {
            if let Some(action) = remove_marriage_field(
                file,
                m,
                MarriageFieldKind::EndReason(default_end_reason()),
                line_index,
                uri,
                "Remove `end_reason:` field",
                diag,
            ) {
                out.push(action);
            }
        }
        _ => {}
    }
    out
}

/// Sentinel used only for matching the EndReason discriminant in
/// [`remove_marriage_field`]; the value isn't read.
fn default_end_reason() -> kul_core::ast::EndReasonValue {
    kul_core::ast::EndReasonValue {
        value: kul_core::ast::EndReason::Divorce,
        span: ByteSpan::new(0, 0),
    }
}

fn add_person_field(
    file: FileId,
    p: &PersonStmt,
    line_index: &LineIndex,
    uri: &Url,
    field_text: &str,
    title: &str,
    source: &Diagnostic,
) -> CodeAction {
    let insert_at = person_header_end(p);
    text_insertion_action(
        file,
        line_index,
        uri,
        insert_at,
        &format!(" {field_text}"),
        title,
        source,
    )
}

fn add_marriage_field(
    file: FileId,
    m: &MarriageStmt,
    line_index: &LineIndex,
    uri: &Url,
    field_text: &str,
    title: &str,
    source: &Diagnostic,
) -> CodeAction {
    let insert_at = marriage_header_end(m);
    text_insertion_action(
        file,
        line_index,
        uri,
        insert_at,
        &format!(" {field_text}"),
        title,
        source,
    )
}

fn remove_marriage_field(
    file: FileId,
    m: &MarriageStmt,
    kind_to_remove: MarriageFieldKind,
    line_index: &LineIndex,
    uri: &Url,
    title: &str,
    source: &Diagnostic,
) -> Option<CodeAction> {
    let field = m.fields.iter().find(|f| {
        matches!(
            (&f.kind, &kind_to_remove),
            (
                MarriageFieldKind::EndReason(_),
                MarriageFieldKind::EndReason(_),
            )
        )
    })?;
    let span_with_leading_space = ByteSpan::new(
        // Sweep up the single leading space that separated this field from
        // its predecessor, so the removal doesn't leave a double space.
        // Conservative: assume the parser placed at least one space.
        field.span.start.saturating_sub(1),
        field.span.end,
    );
    let edit = TextEdit {
        range: line_index.range(span_with_leading_space),
        new_text: String::new(),
    };
    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![edit]);
    // Code actions only fire on diagnostics in the active file (per the
    // overlap check in `code_actions`), so the LSP-side translation is
    // guaranteed to succeed; bubble up an empty `diagnostics` array if
    // the diagnostic happens to be unanchored.
    let lsp_diag = super::diagnostics::to_lsp_one(uri, source, line_index, file);
    let attached: Vec<tower_lsp::lsp_types::Diagnostic> = lsp_diag.into_iter().collect();
    Some(CodeAction {
        title: title.to_owned(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(attached),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }),
        ..Default::default()
    })
}

fn text_insertion_action(
    file: FileId,
    line_index: &LineIndex,
    uri: &Url,
    insert_at: usize,
    text: &str,
    title: &str,
    source: &Diagnostic,
) -> CodeAction {
    let pos = line_index.position(insert_at);
    let edit = TextEdit {
        range: Range {
            start: pos,
            end: pos,
        },
        new_text: text.to_owned(),
    };
    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![edit]);
    // Code actions only fire on diagnostics in the active file (per the
    // overlap check in `code_actions`), so the LSP-side translation is
    // guaranteed to succeed; bubble up an empty `diagnostics` array if
    // the diagnostic happens to be unanchored.
    let lsp_diag = super::diagnostics::to_lsp_one(uri, source, line_index, file);
    let attached: Vec<tower_lsp::lsp_types::Diagnostic> = lsp_diag.into_iter().collect();
    CodeAction {
        title: title.to_owned(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(attached),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// End of a person's header line (after the last header-level field, or
/// after the id if no fields are present). Sub-statements (`birth`,
/// `adoption`) are excluded — fields go before them.
fn person_header_end(p: &PersonStmt) -> usize {
    p.fields.last().map(|f| f.span.end).unwrap_or(p.id.span.end)
}

fn marriage_header_end(m: &MarriageStmt) -> usize {
    m.fields
        .last()
        .map(|f| f.span.end)
        .unwrap_or(m.spouse_b.span.end)
}

fn span_contains(outer: ByteSpan, inner: ByteSpan) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn ranges_overlap(a: Range, b: Range) -> bool {
    !(position_lt(a.end, b.start) || position_lt(b.end, a.start))
}

fn position_lt(a: Position, b: Position) -> bool {
    (a.line, a.character) < (b.line, b.character)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::test_open_file;

    fn url() -> Url {
        Url::parse("file:///t.kul").unwrap()
    }

    fn full_doc_range(line_index: &LineIndex) -> Range {
        Range {
            start: line_index.position(0),
            end: line_index.position(line_index.source().len()),
        }
    }

    /// Run the LSP feature with a request range covering the whole
    /// document, and return the `CodeAction`s (no `Command`s expected).
    fn actions_for(source: &str) -> Vec<CodeAction> {
        let doc = test_open_file(source);
        let v = doc.view();
        let request_range = full_doc_range(v.line_index);
        code_actions(
            v.file,
            v.resolved,
            &doc.check.diagnostics,
            v.line_index,
            &url(),
            request_range,
        )
        .into_iter()
        .filter_map(|a| match a {
            CodeActionOrCommand::CodeAction(a) => Some(a),
            CodeActionOrCommand::Command(_) => None,
        })
        .collect()
    }

    /// Apply every action's first text-edit to the source and return the
    /// updated string. Useful for end-to-end "fix → revalidate" tests.
    fn apply(source: &str, action: &CodeAction) -> String {
        let edit = action
            .edit
            .as_ref()
            .and_then(|we| we.changes.as_ref())
            .and_then(|m| m.values().next())
            .and_then(|v| v.first())
            .expect("action has an edit");
        // Ranges are LSP positions; for tests we use ASCII so byte=col.
        let start = byte_offset_for_position(source, edit.range.start);
        let end = byte_offset_for_position(source, edit.range.end);
        let mut out = String::with_capacity(source.len() + edit.new_text.len());
        out.push_str(&source[..start]);
        out.push_str(&edit.new_text);
        out.push_str(&source[end..]);
        out
    }

    fn byte_offset_for_position(source: &str, pos: Position) -> usize {
        let mut line = 0u32;
        let mut col = 0u32;
        for (i, c) in source.char_indices() {
            if line == pos.line && col == pos.character {
                return i;
            }
            if c == '\n' {
                line += 1;
                col = 0;
            } else {
                col += c.len_utf16() as u32;
            }
        }
        source.len()
    }

    #[test]
    fn missing_gender_offers_three_quick_fixes() {
        let src = "person alice name:\"A\"\n";
        let actions = actions_for(src);
        let titles: Vec<&str> = actions.iter().map(|a| a.title.as_str()).collect();
        assert!(titles.contains(&"Add `gender:male`"));
        assert!(titles.contains(&"Add `gender:female`"));
        assert!(titles.contains(&"Add `gender:other`"));
    }

    #[test]
    fn missing_name_offers_one_quick_fix() {
        let src = "person alice gender:female\n";
        let actions = actions_for(src);
        assert!(actions.iter().any(|a| a.title == "Add `name:\"\"`"));
    }

    #[test]
    fn end_without_end_reason_offers_add_divorce() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1972 end:1990\n";
        let actions = actions_for(src);
        let titles: Vec<&str> = actions.iter().map(|a| a.title.as_str()).collect();
        assert!(
            titles
                .iter()
                .any(|t| t.contains("Add `end_reason:divorce`"))
        );
    }

    #[test]
    fn end_reason_without_end_offers_remove_field() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1972 end_reason:divorce\n";
        let actions = actions_for(src);
        assert!(
            actions
                .iter()
                .any(|a| a.title.contains("Remove `end_reason:`")),
        );
    }

    #[test]
    fn no_diagnostic_yields_no_actions() {
        // Fully valid document — no quick fixes.
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:1972\n";
        assert!(actions_for(src).is_empty());
    }

    #[test]
    fn unrelated_diagnostic_codes_are_ignored() {
        // KUL-R04 (self-marriage) doesn't have a registered fix.
        let src = "person alice name:\"A\" gender:female\n\
                   marriage m alice alice start:1972\n";
        assert!(actions_for(src).is_empty());
    }

    #[test]
    fn add_gender_fix_is_inserted_inline_with_existing_fields() {
        let src = "person alice name:\"A\"\n";
        let actions = actions_for(src);
        let male = actions
            .iter()
            .find(|a| a.title == "Add `gender:male`")
            .expect("has gender:male action");
        let fixed = apply(src, male);
        // Field is appended at end of the header line.
        assert_eq!(fixed, "person alice name:\"A\" gender:male\n");
    }

    #[test]
    fn add_name_fix_is_inserted_at_end_of_header() {
        let src = "person alice gender:female\n";
        let actions = actions_for(src);
        let action = actions
            .iter()
            .find(|a| a.title == "Add `name:\"\"`")
            .unwrap();
        let fixed = apply(src, action);
        assert_eq!(fixed, "person alice gender:female name:\"\"\n");
    }

    #[test]
    fn end_reason_fix_resolves_diagnostic_after_apply() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1972 end:1990\n";
        let actions = actions_for(src);
        let action = actions
            .iter()
            .find(|a| a.title == "Add `end_reason:divorce`")
            .unwrap();
        let fixed = apply(src, action);
        // After applying, R05 should no longer fire.
        let result = test_open_file(&fixed).check;
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|d| d.code.starts_with("KUL-R05")),
            "diagnostics after fix: {:?}",
            result.diagnostics,
        );
    }

    #[test]
    fn remove_end_reason_fix_resolves_diagnostic_after_apply() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1972 end_reason:divorce\n";
        let actions = actions_for(src);
        let action = actions
            .iter()
            .find(|a| a.title.contains("Remove `end_reason:`"))
            .unwrap();
        let fixed = apply(src, action);
        let result = test_open_file(&fixed).check;
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|d| d.code.starts_with("KUL-R05")),
            "diagnostics after fix: {:?}",
            result.diagnostics,
        );
    }

    #[test]
    fn missing_gender_fix_resolves_diagnostic_after_apply() {
        let src = "person alice name:\"Alice\"\n";
        let actions = actions_for(src);
        let action = actions
            .iter()
            .find(|a| a.title == "Add `gender:female`")
            .unwrap();
        let fixed = apply(src, action);
        let result = test_open_file(&fixed).check;
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|d| d.code == "KUL-R03" && d.message.contains("`gender:`")),
            "diagnostics after fix: {:?}",
            result.diagnostics,
        );
    }

    #[test]
    fn range_filter_excludes_unrelated_lines() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob\n"; // bob missing both name AND gender
        let doc = test_open_file(src);
        let v = doc.view();
        // Request range covering only line 0 (alice, no diagnostics).
        let request_range = Range {
            start: v.line_index.position(0),
            end: v.line_index.position(idx(src, "\n") + 1),
        };
        let actions = code_actions(
            v.file,
            v.resolved,
            &doc.check.diagnostics,
            v.line_index,
            &url(),
            request_range,
        );
        assert!(actions.is_empty(), "expected no actions for clean line");
    }

    fn idx(source: &str, pat: &str) -> usize {
        source.find(pat).expect("pattern in source")
    }

    #[test]
    fn snapshot_quick_fixes_for_missing_gender() {
        let src = "person alice name:\"Alice\"\n";
        let titles: Vec<String> = actions_for(src).into_iter().map(|a| a.title).collect();
        insta::assert_json_snapshot!(titles);
    }

    #[test]
    fn snapshot_workspace_edits_for_end_without_end_reason() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1972 end:1990\n";
        let actions = actions_for(src);
        let edits: Vec<_> = actions.into_iter().map(|a| (a.title, a.edit)).collect();
        insta::assert_json_snapshot!(edits);
    }
}
