//! Basic completion for `textDocument/completion`.
//!
//! Token-stream-first classifier: the cursor often sits in whitespace or
//! a partial token where there's no clean AST node, so we walk tokens up
//! to the cursor and classify into one of a fixed set of contexts. The
//! keyword/field/enum sets are static; the marriage-id and person-id
//! sets come from the resolved document (every declared marriage and
//! person, surfaced where a reference is expected).

use kul_core::ast::{MarriageStmt, PersonStmt, Statement};
use kul_core::date::DateLit;
use kul_core::field_meta::{self, StatementKind, ValueKind};
use kul_core::lexer::{EnumKw, FieldName, Token, TokenKind, tokenize};
use kul_core::semantic::ResolvedDocument;
use kul_core::span::FileId;
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat};

/// Build a completion item list for the cursor at `byte_offset`. Empty
/// vec when the cursor lands somewhere we have nothing to offer.
pub fn complete(
    source: &str,
    file: FileId,
    resolved: &ResolvedDocument,
    byte_offset: usize,
) -> Vec<CompletionItem> {
    let tokens = tokenize(source);
    match classify(source, file, &tokens, resolved, byte_offset) {
        Context::None => Vec::new(),
        Context::TopLevelStart => top_level_keywords(),
        Context::IndentedUnderPerson => sub_statement_keywords(),
        Context::PersonFieldList { existing } => person_fields(&existing),
        Context::MarriageFieldList { existing } => marriage_fields(&existing),
        Context::AdoptionFieldList { existing } => adoption_fields(&existing),
        Context::AfterGenderColon => gender_values(),
        Context::AfterEndReasonColon => end_reason_values(),
        Context::AfterStringFieldColon => quoted_value_snippet(),
        Context::MarriageRefPosition => marriage_id_items(file, resolved),
        Context::SpousePosition { exclude } => person_id_items(file, resolved, exclude.as_deref()),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Context {
    /// Beginning of a top-level line.
    TopLevelStart,
    /// Beginning of an indented continuation under a `person` statement.
    IndentedUnderPerson,
    /// Inside the field list of a `person` statement.
    PersonFieldList { existing: Vec<FieldName> },
    /// Inside the field list of a `marriage` statement.
    MarriageFieldList { existing: Vec<FieldName> },
    /// Inside the field list of an `adoption` sub-statement.
    AdoptionFieldList { existing: Vec<FieldName> },
    /// Right after `gender:` (cursor on the value side).
    AfterGenderColon,
    /// Right after `end_reason:`.
    AfterEndReasonColon,
    /// Right after a quoted-string field's `:` (`name:`, `family:`, `given:`).
    /// Surfaces a single snippet item that wraps the value in quotes.
    AfterStringFieldColon,
    /// Inside a `birth` or `adoption` sub-statement at the marriage-ref
    /// position. Suggest declared marriage ids.
    MarriageRefPosition,
    /// Inside a `marriage` statement at a spouse position. Suggest declared
    /// persons; exclude the named person if `exclude` is set (the user
    /// already filled spouse_a, so spouse_b shouldn't repeat them).
    SpousePosition { exclude: Option<String> },
    /// No completion to offer (inside a string, on a date literal, in a
    /// comment, etc.).
    None,
}

fn classify(
    source: &str,
    file: FileId,
    tokens: &[Token],
    resolved: &ResolvedDocument,
    cursor: usize,
) -> Context {
    // Skip past EOF: nothing to complete past a finished document.
    if cursor > source.len() {
        return Context::None;
    }
    if cursor_inside_string_or_comment(source, cursor) {
        return Context::None;
    }

    // Tokens whose end <= cursor are *before* the cursor.
    let preceding: Vec<&Token> = tokens
        .iter()
        .filter(|t| t.span.end <= cursor && !matches!(t.kind, TokenKind::Eof))
        .collect();

    // 1. Detect "right after FieldKw + Colon ...". `gender:<CURSOR>`,
    //    `gender: <CURSOR>` (whitespace ok after colon), and
    //    `gender:f<CURSOR>` (cursor adjacent to a partial value) all count.
    //    `gender:female <CURSOR>` (whitespace after a complete value)
    //    does NOT — the field is closed.
    if let Some(field) = field_value_context(&preceding, cursor) {
        match field {
            FieldName::Gender => return Context::AfterGenderColon,
            FieldName::EndReason => return Context::AfterEndReasonColon,
            FieldName::Name | FieldName::Family | FieldName::Given => {
                return Context::AfterStringFieldColon;
            }
            // Date fields — nothing useful to offer.
            _ => return Context::None,
        }
    }

    // 2. Determine line state: is this line "fresh" (no significant tokens
    //    before cursor on this line), and is it indented? What's the first
    //    significant keyword on the line, if any?
    let line = current_line(&preceding);

    // 3. Determine the enclosing top-level statement via the
    //    `ResolvedDocument` query seam (ADR-0001).
    let enclosing = resolved.statement_at(file, cursor);

    let scan = positional_scan(&preceding, cursor);

    // The line's leading keyword discriminates first — partially-typed
    // lines (e.g. `marriage m <CURSOR>`) won't have parsed cleanly, so we
    // can't always rely on `enclosing` reflecting the line being typed.
    match (line.is_fresh, line.is_indented, &line.first_kw) {
        // Fresh, non-indented line at top level → top-level keywords.
        (true, false, _) => Context::TopLevelStart,

        // Fresh, indented line under a person → birth/adoption keywords.
        (true, true, _) => match enclosing {
            Some(Statement::Person(_)) => Context::IndentedUnderPerson,
            _ => Context::None,
        },

        // `birth` sub-statement: one positional (the marriage ref).
        (false, true, Some(LineKw::Birth)) => {
            if scan.filled == 0 && !scan.past_positionals {
                Context::MarriageRefPosition
            } else {
                Context::None
            }
        }

        // `adoption` sub-statement: first positional is a marriage ref,
        // then field list.
        (false, true, Some(LineKw::Adoption)) => match enclosing {
            Some(Statement::Person(p)) => {
                if scan.filled == 0 && !scan.past_positionals {
                    Context::MarriageRefPosition
                } else {
                    Context::AdoptionFieldList {
                        existing: existing_adoption_fields_at_line(p, &line),
                    }
                }
            }
            _ => Context::None,
        },

        // `marriage` line: positions 0 (its own id), 1 (spouse_a), 2
        // (spouse_b), then field list.
        (false, false, Some(LineKw::Marriage)) => {
            let existing = match enclosing {
                Some(Statement::Marriage(m)) => existing_marriage_fields(m),
                _ => Vec::new(),
            };
            if scan.past_positionals {
                Context::MarriageFieldList { existing }
            } else {
                match scan.filled {
                    // Cursor is on the marriage's own id — user is naming
                    // a new marriage; suggesting existing ids would point
                    // them at collisions.
                    0 => Context::None,
                    1 => Context::SpousePosition { exclude: None },
                    2 => Context::SpousePosition {
                        exclude: scan.seen.get(1).cloned(),
                    },
                    // Both spouses filled, no field tokens yet — between
                    // spouse_b and the first field.
                    _ => Context::MarriageFieldList { existing },
                }
            }
        }

        // `person` top-level line: field list (the id is freely-named like
        // a marriage's, so position 0 gets nothing).
        (false, false, Some(LineKw::Person)) => match enclosing {
            Some(Statement::Person(p)) => Context::PersonFieldList {
                existing: existing_person_fields(p),
            },
            _ => Context::None,
        },

        // Continuation of a previous statement (no leading keyword on this
        // line) — depends on what the enclosing statement is.
        (false, _, _) => match enclosing {
            Some(Statement::Person(p)) => Context::PersonFieldList {
                existing: existing_person_fields(p),
            },
            Some(Statement::Marriage(m)) => Context::MarriageFieldList {
                existing: existing_marriage_fields(m),
            },
            None => Context::None,
        },
    }
}

/// Scan the positional region of the cursor's line — i.e. the naked
/// identifier slots after the leading keyword, before any `field:` token.
struct PositionalScan {
    /// Number of fully-filled positional slots before the cursor (each one
    /// is a complete identifier with whitespace after it).
    filled: usize,
    /// The naked-identifier texts seen, in order. `seen[0]` for `marriage`
    /// is the marriage's own id; `seen[1]` is spouse_a.
    seen: Vec<String>,
    /// True once we've seen a `field:` or `:` token on the line — past the
    /// positional region.
    past_positionals: bool,
}

fn positional_scan(preceding: &[&Token], cursor: usize) -> PositionalScan {
    let line_start_idx = preceding
        .iter()
        .rposition(|t| matches!(t.kind, TokenKind::Newline))
        .map(|i| i + 1)
        .unwrap_or(0);
    let line_tokens = &preceding[line_start_idx..];

    // Skip the leading indent + keyword(s).
    let mut iter = line_tokens.iter();
    while let Some(t) = iter.clone().next() {
        match t.kind {
            TokenKind::Indent
            | TokenKind::PersonKw
            | TokenKind::MarriageKw
            | TokenKind::BirthKw
            | TokenKind::AdoptionKw => {
                iter.next();
            }
            _ => break,
        }
    }

    let mut filled = 0;
    let mut seen: Vec<String> = Vec::new();
    let mut past = false;

    for t in iter {
        match &t.kind {
            TokenKind::FieldKw(_) | TokenKind::Colon => {
                past = true;
                break;
            }
            TokenKind::Ident(s) | TokenKind::Bare(s) => {
                if t.span.end == cursor {
                    // Cursor adjacent to a partial identifier — the user is
                    // mid-typing this slot, so it's not yet "filled" for
                    // counting purposes (they want suggestions for *this*
                    // position, not the next one).
                } else {
                    filled += 1;
                    seen.push(s.clone());
                }
            }
            // EnumKw lexed inside a positional slot is unusual but treat it
            // as a filled slot so we don't miscount.
            TokenKind::EnumKw(_) | TokenKind::String(_) | TokenKind::Error(_)
                if t.span.end != cursor =>
            {
                filled += 1;
            }
            _ => {}
        }
    }

    PositionalScan {
        filled,
        seen,
        past_positionals: past,
    }
}

/// What's on the cursor's line, before the cursor?
struct LineInfo {
    /// True iff every token on the line so far is `Indent` or whitespace
    /// (i.e. nothing significant before the cursor on this line).
    is_fresh: bool,
    /// True iff the line begins with an `Indent` token.
    is_indented: bool,
    /// The first significant keyword on this line, if any.
    first_kw: Option<LineKw>,
    /// The span of the first significant keyword token, used to look up
    /// the matching parsed sub-statement.
    first_kw_span: Option<kul_core::span::ByteSpan>,
}

#[derive(Debug, PartialEq, Eq)]
enum LineKw {
    Person,
    Marriage,
    Birth,
    Adoption,
}

fn current_line(preceding: &[&Token]) -> LineInfo {
    let line_start_idx = preceding
        .iter()
        .rposition(|t| matches!(t.kind, TokenKind::Newline))
        .map(|i| i + 1)
        .unwrap_or(0);
    let line_tokens = &preceding[line_start_idx..];

    let is_indented = line_tokens
        .first()
        .is_some_and(|t| matches!(t.kind, TokenKind::Indent));

    let is_fresh = line_tokens
        .iter()
        .all(|t| matches!(t.kind, TokenKind::Indent));

    let first_significant = line_tokens
        .iter()
        .find(|t| !matches!(t.kind, TokenKind::Indent));

    let (first_kw, first_kw_span) = match first_significant {
        Some(t) => {
            let kw = match t.kind {
                TokenKind::PersonKw => Some(LineKw::Person),
                TokenKind::MarriageKw => Some(LineKw::Marriage),
                TokenKind::BirthKw => Some(LineKw::Birth),
                TokenKind::AdoptionKw => Some(LineKw::Adoption),
                _ => None,
            };
            (kw, Some(t.span))
        }
        None => (None, None),
    };

    LineInfo {
        is_fresh,
        is_indented,
        first_kw,
        first_kw_span,
    }
}

fn field_value_context(preceding: &[&Token], cursor: usize) -> Option<FieldName> {
    let last = preceding.last()?;
    match &last.kind {
        // `field:<CURSOR>` or `field: <CURSOR>` — cursor anywhere right
        // after the colon is fine, even with whitespace before it.
        TokenKind::Colon => {
            if preceding.len() >= 2
                && let TokenKind::FieldKw(name) = &preceding[preceding.len() - 2].kind
            {
                return Some(*name);
            }
            None
        }
        // `field:f<CURSOR>` — partial value, cursor adjacent. Walk back to
        // the FieldKw + Colon. With a whitespace gap, the field is closed.
        TokenKind::Ident(_)
        | TokenKind::Bare(_)
        | TokenKind::String(_)
        | TokenKind::EnumKw(_)
        | TokenKind::Error(_) => {
            if last.span.end != cursor {
                return None;
            }
            let mut i = preceding.len();
            while i > 0 {
                match &preceding[i - 1].kind {
                    TokenKind::Newline => return None,
                    TokenKind::Ident(_)
                    | TokenKind::Bare(_)
                    | TokenKind::String(_)
                    | TokenKind::EnumKw(_)
                    | TokenKind::Error(_)
                    | TokenKind::Indent => {
                        i -= 1;
                    }
                    TokenKind::Colon => {
                        if i >= 2
                            && let TokenKind::FieldKw(name) = &preceding[i - 2].kind
                        {
                            return Some(*name);
                        }
                        return None;
                    }
                    _ => return None,
                }
            }
            None
        }
        _ => None,
    }
}

fn cursor_inside_string_or_comment(source: &str, cursor: usize) -> bool {
    // Quick scan: are we inside an unclosed " or after # on the current line?
    let bytes = source.as_bytes();
    let line_start = bytes[..cursor.min(bytes.len())]
        .iter()
        .rposition(|&b| b == b'\n')
        .map(|i| i + 1)
        .unwrap_or(0);
    let mut in_string = false;
    let mut after_hash = false;
    for &b in &bytes[line_start..cursor.min(bytes.len())] {
        if after_hash {
            // comment runs to EOL
            return true;
        }
        match b {
            b'"' => in_string = !in_string,
            b'#' if !in_string => after_hash = true,
            _ => {}
        }
    }
    in_string
}

fn existing_person_fields(p: &PersonStmt) -> Vec<FieldName> {
    p.fields.iter().map(|f| f.kind.field_name()).collect()
}

fn existing_marriage_fields(m: &MarriageStmt) -> Vec<FieldName> {
    m.fields.iter().map(|f| f.kind.field_name()).collect()
}

fn existing_adoption_fields_at_line(p: &PersonStmt, line: &LineInfo) -> Vec<FieldName> {
    let Some(kw_span) = line.first_kw_span else {
        return Vec::new();
    };
    // Match the adoption whose keyword span starts at the same byte as
    // the line's first keyword (the parser-built `keyword_span`).
    let Some(adopt) = p
        .adoptions
        .iter()
        .find(|a| a.keyword_span.start == kw_span.start)
    else {
        return Vec::new();
    };
    adopt.fields.iter().map(|f| f.kind.field_name()).collect()
}

fn item(label: &str, kind: CompletionItemKind, detail: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_owned(),
        kind: Some(kind),
        detail: Some(detail.to_owned()),
        ..Default::default()
    }
}

fn top_level_keywords() -> Vec<CompletionItem> {
    vec![
        item(
            "person",
            CompletionItemKind::KEYWORD,
            "Declare an individual (name, gender, dates)",
        ),
        item(
            "marriage",
            CompletionItemKind::KEYWORD,
            "Declare a marriage between two people",
        ),
    ]
}

fn sub_statement_keywords() -> Vec<CompletionItem> {
    vec![
        item(
            "birth",
            CompletionItemKind::KEYWORD,
            "Link to biological parents (a marriage id)",
        ),
        item(
            "adoption",
            CompletionItemKind::KEYWORD,
            "Link to an adoptive marriage",
        ),
    ]
}

fn person_fields(existing: &[FieldName]) -> Vec<CompletionItem> {
    field_completions(StatementKind::Person, existing)
}

fn marriage_fields(existing: &[FieldName]) -> Vec<CompletionItem> {
    field_completions(StatementKind::Marriage, existing)
}

fn adoption_fields(existing: &[FieldName]) -> Vec<CompletionItem> {
    field_completions(StatementKind::Adoption, existing)
}

/// Build the completion list for one statement shape: every field valid
/// for that shape (in canonical order) minus the ones already present on
/// the line. String-typed fields use a snippet that auto-wraps the value
/// in quotes; everything else is a plain field item.
fn field_completions(kind: StatementKind, existing: &[FieldName]) -> Vec<CompletionItem> {
    field_meta::fields_for(kind)
        .iter()
        .copied()
        .filter(|name| !existing.contains(name))
        .map(|name| {
            let m = field_meta::meta(name);
            let label = format!("{}:", m.name.as_str());
            match m.value_kind {
                ValueKind::String => field_with_quoted_snippet(&label, m.short_doc),
                ValueKind::Date | ValueKind::Enum => {
                    item(&label, CompletionItemKind::FIELD, m.short_doc)
                }
            }
        })
        .collect()
}

/// Field-name item whose insertion auto-wraps the value in quotes:
/// accepting `name:` actually inserts `name:"$0"`. Cursor lands inside
/// the quotes so the user can just type the name.
fn field_with_quoted_snippet(label: &str, doc: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_owned(),
        kind: Some(CompletionItemKind::FIELD),
        detail: Some(doc.to_owned()),
        insert_text: Some(format!("{label}\"$0\"")),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..Default::default()
    }
}

fn gender_values() -> Vec<CompletionItem> {
    [
        (EnumKw::Male, "Male"),
        (EnumKw::Female, "Female"),
        (EnumKw::Other, "Non-binary or unspecified"),
    ]
    .iter()
    .map(|(kw, doc)| item(kw.as_str(), CompletionItemKind::ENUM_MEMBER, doc))
    .collect()
}

fn end_reason_values() -> Vec<CompletionItem> {
    vec![item(
        EnumKw::Divorce.as_str(),
        CompletionItemKind::ENUM_MEMBER,
        "Marriage ended in divorce",
    )]
}

/// A single preselected snippet that inserts `""` and lands the cursor
/// between the quotes — the canonical fix for "I forgot to wrap the value".
fn quoted_value_snippet() -> Vec<CompletionItem> {
    vec![CompletionItem {
        label: "\"…\"".to_owned(),
        kind: Some(CompletionItemKind::VALUE),
        detail: Some("Quoted value (cursor between the quotes)".to_owned()),
        insert_text: Some(r#""$0""#.to_owned()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        preselect: Some(true),
        ..Default::default()
    }]
}

/// Every declared marriage as a completion item — used after `birth` and
/// `adoption`. Detail shows the spouses' display names and the date span so
/// the user can disambiguate between marriages of the same person.
fn marriage_id_items(file: FileId, resolved: &ResolvedDocument) -> Vec<CompletionItem> {
    resolved
        .marriages_in(file)
        .map(|m| CompletionItem {
            label: m.id.name.clone(),
            kind: Some(CompletionItemKind::EVENT),
            detail: Some(marriage_detail(file, resolved, m)),
            ..Default::default()
        })
        .collect()
}

/// Every declared person as a completion item — used in marriage spouse
/// positions. `exclude` filters one id out (so spouse_b's list excludes the
/// person already named as spouse_a, since you can't marry yourself).
fn person_id_items(
    file: FileId,
    resolved: &ResolvedDocument,
    exclude: Option<&str>,
) -> Vec<CompletionItem> {
    resolved
        .persons_in(file)
        .filter(|p| Some(p.id.name.as_str()) != exclude)
        .map(|p| CompletionItem {
            label: p.id.name.clone(),
            kind: Some(CompletionItemKind::VARIABLE),
            detail: Some(person_detail(p)),
            ..Default::default()
        })
        .collect()
}

fn marriage_detail(_file: FileId, resolved: &ResolvedDocument, m: &MarriageStmt) -> String {
    let a = resolved
        .person(&m.spouse_a.name)
        .map(|p| p.display_name())
        .unwrap_or(m.spouse_a.name.as_str());
    let b = resolved
        .person(&m.spouse_b.name)
        .map(|p| p.display_name())
        .unwrap_or(m.spouse_b.name.as_str());
    let dates = match (
        m.start().map(DateLit::format_year),
        m.end().map(DateLit::format_year),
    ) {
        (Some(s), Some(e)) => format!(", {s}–{e}"),
        (Some(s), None) => format!(", {s}–"),
        (None, Some(e)) => format!(", ?–{e}"),
        (None, None) => String::new(),
    };
    format!("{a} + {b}{dates}")
}

fn person_detail(p: &PersonStmt) -> String {
    let name = p.display_name();
    match p.born().map(DateLit::format_year) {
        Some(b) => format!("{name}, b. {b}"),
        None => name.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::test_open_file;

    /// Take a fixture string with a `<CURSOR>` marker; return (source, offset).
    fn cursor_fixture(s: &str) -> (String, usize) {
        let offset = s.find("<CURSOR>").expect("fixture must contain <CURSOR>");
        let source = format!("{}{}", &s[..offset], &s[offset + "<CURSOR>".len()..]);
        (source, offset)
    }

    /// Take a `<CURSOR>`-marked fixture and return the list of completion
    /// items the LSP would surface for that cursor position. Centralises
    /// the tokenize/parse/resolve scaffold every per-context test in this
    /// file otherwise repeats.
    fn complete_for(src_with_marker: &str) -> Vec<CompletionItem> {
        let (source, offset) = cursor_fixture(src_with_marker);
        let doc = test_open_file(&source);
        let v = doc.view();
        complete(v.line_index.source(), v.file, v.resolved, offset)
    }

    fn run(src_with_marker: &str) -> Vec<String> {
        complete_for(src_with_marker)
            .into_iter()
            .map(|c| c.label)
            .collect()
    }

    #[test]
    fn top_level_start_blank_doc() {
        assert_eq!(
            run("<CURSOR>"),
            vec!["person".to_owned(), "marriage".into()]
        );
    }

    #[test]
    fn top_level_start_after_blank_line() {
        let src = "person a name:\"A\" gender:female\n<CURSOR>";
        let labels = run(src);
        assert!(labels.contains(&"person".to_owned()));
        assert!(labels.contains(&"marriage".to_owned()));
    }

    #[test]
    fn indented_under_person_offers_sub_keywords() {
        let src = "person a name:\"A\" gender:female\n  <CURSOR>";
        let labels = run(src);
        assert_eq!(labels, vec!["birth".to_owned(), "adoption".into()]);
    }

    #[test]
    fn person_field_list_after_id() {
        let src = "person a <CURSOR>";
        let labels = run(src);
        assert!(labels.contains(&"name:".to_owned()));
        assert!(labels.contains(&"gender:".to_owned()));
        assert!(labels.contains(&"born:".to_owned()));
    }

    #[test]
    fn person_field_list_filters_present_fields() {
        let src = "person a name:\"A\" gender:female <CURSOR>";
        let labels = run(src);
        assert!(!labels.contains(&"name:".to_owned()));
        assert!(!labels.contains(&"gender:".to_owned()));
        assert!(labels.contains(&"family:".to_owned()));
        assert!(labels.contains(&"born:".to_owned()));
    }

    #[test]
    fn marriage_field_list_after_spouses() {
        let src = "marriage m a b <CURSOR>";
        let labels = run(src);
        assert!(labels.contains(&"start:".to_owned()));
        assert!(labels.contains(&"end:".to_owned()));
        assert!(labels.contains(&"end_reason:".to_owned()));
    }

    #[test]
    fn marriage_field_list_filters_present() {
        let src = "marriage m a b start:2010 <CURSOR>";
        let labels = run(src);
        assert!(!labels.contains(&"start:".to_owned()));
        assert!(labels.contains(&"end:".to_owned()));
        assert!(labels.contains(&"end_reason:".to_owned()));
    }

    #[test]
    fn after_gender_colon_offers_enum_values() {
        let src = "person a name:\"A\" gender:<CURSOR>";
        let labels = run(src);
        assert_eq!(
            labels,
            vec!["male".to_owned(), "female".into(), "other".into()]
        );
    }

    #[test]
    fn after_gender_colon_partial_value() {
        // User typed `f` after gender:; classifier still in AfterGenderColon.
        let src = "person a name:\"A\" gender:f<CURSOR>";
        let labels = run(src);
        assert!(labels.contains(&"female".to_owned()));
    }

    #[test]
    fn after_end_reason_colon_offers_divorce() {
        let src = "marriage m a b start:2010 end:2020 end_reason:<CURSOR>";
        assert_eq!(run(src), vec!["divorce".to_owned()]);
    }

    /// Accepting `name:` from the field-name completion should drop in
    /// `name:"<cursor>"` directly — the user types the name and the closing
    /// quote is already there. Symmetric for `family:` and `given:`. Date
    /// and gender items keep their plain `field:` insertion.
    #[test]
    fn person_field_completion_wraps_string_values_in_quotes() {
        use tower_lsp::lsp_types::InsertTextFormat;
        let items = complete_for("person a <CURSOR>");

        for field in ["name:", "family:", "given:"] {
            let it = items
                .iter()
                .find(|c| c.label == field)
                .unwrap_or_else(|| panic!("missing item `{field}` in {items:#?}"));
            let expected = format!("{}\"$0\"", field);
            assert_eq!(
                it.insert_text.as_deref(),
                Some(expected.as_str()),
                "string field `{field}` should insert `{expected}`"
            );
            assert_eq!(it.insert_text_format, Some(InsertTextFormat::SNIPPET));
        }

        for field in ["born:", "died:", "gender:"] {
            let it = items
                .iter()
                .find(|c| c.label == field)
                .unwrap_or_else(|| panic!("missing item `{field}`"));
            assert!(
                it.insert_text.is_none() && it.insert_text_format.is_none(),
                "non-string field `{field}` should keep plain insertion; got: {it:#?}"
            );
        }
    }

    /// After `name:`, suggest a single preselected snippet item that wraps
    /// the value in quotes and lands the cursor between them. Same goes for
    /// `family:` and `given:`. This catches the dominant typo (forgetting
    /// quotes) the moment the user types `:` — they hit Tab and end up
    /// inside a quoted value.
    #[test]
    fn after_string_field_colon_offers_quoted_value_snippet() {
        use tower_lsp::lsp_types::InsertTextFormat;
        for field in ["name", "family", "given"] {
            let src = format!("person a {field}:<CURSOR>");
            let items = complete_for(&src);
            assert_eq!(
                items.len(),
                1,
                "expected one item for `{field}:`; got: {items:#?}"
            );
            let item = &items[0];
            assert_eq!(
                item.insert_text.as_deref(),
                Some(r#""$0""#),
                "insert_text should wrap cursor in quotes for `{field}:`"
            );
            assert_eq!(
                item.insert_text_format,
                Some(InsertTextFormat::SNIPPET),
                "insert_text_format must be Snippet for `{field}:` so $0 is honored"
            );
            assert_eq!(
                item.preselect,
                Some(true),
                "the item should be preselected so a single Tab accepts it"
            );
        }
    }

    #[test]
    fn inside_string_returns_empty() {
        let src = "person a name:\"Al<CURSOR>ice\" gender:female\n";
        assert!(run(src).is_empty());
    }

    #[test]
    fn inside_comment_returns_empty() {
        let src = "person a name:\"A\" gender:female # this is a <CURSOR>";
        assert!(run(src).is_empty());
    }

    #[test]
    fn adoption_field_list() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  adoption m <CURSOR>";
        let labels = run(src);
        assert!(labels.contains(&"start:".to_owned()));
        assert!(labels.contains(&"end:".to_owned()));
    }

    #[test]
    fn adoption_field_list_filters_present() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  adoption m start:2010 <CURSOR>";
        let labels = run(src);
        assert!(!labels.contains(&"start:".to_owned()));
        assert!(labels.contains(&"end:".to_owned()));
    }

    fn run_with_details(src_with_marker: &str) -> Vec<(String, Option<String>)> {
        complete_for(src_with_marker)
            .into_iter()
            .map(|c| (c.label, c.detail))
            .collect()
    }

    fn run_kinds(src_with_marker: &str) -> Vec<(String, CompletionItemKind)> {
        complete_for(src_with_marker)
            .into_iter()
            .map(|c| (c.label, c.kind.unwrap()))
            .collect()
    }

    #[test]
    fn after_birth_offers_marriage_ids_with_spouse_detail() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bob name:\"Bob\" gender:male\n\
                   marriage m1 alice bob start:1972 end:1990 end_reason:divorce\n\
                   marriage m2 alice bob start:2000\n\
                   person kid name:\"K\" gender:other\n  birth <CURSOR>";
        let items = run_with_details(src);
        let labels: Vec<&str> = items.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(labels, vec!["m1", "m2"]);
        // Detail strings include both spouses' display names + date span.
        assert_eq!(items[0].1.as_deref(), Some("Alice + Bob, 1972–1990"));
        assert_eq!(items[1].1.as_deref(), Some("Alice + Bob, 2000–"));
    }

    #[test]
    fn after_birth_with_partial_id_still_offers_marriages() {
        // Cursor adjacent to a partial ident — IDE still wants the full set
        // and filters by prefix client-side.
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m_alice_bob alice bob start:1972\n\
                   person kid name:\"K\" gender:other\n  birth m_<CURSOR>";
        let labels = run(src);
        assert_eq!(labels, vec!["m_alice_bob".to_owned()]);
    }

    #[test]
    fn after_adoption_offers_marriage_ids() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bob name:\"Bob\" gender:male\n\
                   marriage m alice bob start:1972\n\
                   person kid name:\"K\" gender:other\n  adoption <CURSOR>";
        let labels = run(src);
        assert_eq!(labels, vec!["m".to_owned()]);
    }

    #[test]
    fn adoption_after_marriage_ref_still_offers_fields() {
        // Once the marriage ref is filled, adoption fields take over.
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:1972\n\
                   person kid name:\"K\" gender:other\n  adoption m <CURSOR>";
        let labels = run(src);
        assert!(labels.contains(&"start:".to_owned()));
        assert!(labels.contains(&"end:".to_owned()));
        // Marriage ids should NOT appear here.
        assert!(!labels.contains(&"m".to_owned()));
    }

    #[test]
    fn after_birth_marriage_ref_offers_nothing() {
        // `birth <ref>` is a single-positional sub-statement; nothing useful
        // to offer once the ref is in place.
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:1972\n\
                   person kid name:\"K\" gender:other\n  birth m <CURSOR>";
        assert!(run(src).is_empty());
    }

    #[test]
    fn after_marriage_id_offers_persons_for_spouse_a() {
        let src = "person alice name:\"Alice\" gender:female born:1950\n\
                   person bob name:\"Bob\" gender:male born:1948\n\
                   marriage m <CURSOR>";
        let items = run_with_details(src);
        let labels: Vec<&str> = items.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(labels, vec!["alice", "bob"]);
        assert_eq!(items[0].1.as_deref(), Some("Alice, b. 1950"));
        assert_eq!(items[1].1.as_deref(), Some("Bob, b. 1948"));
    }

    #[test]
    fn after_spouse_a_excludes_self_marriage() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   person carol name:\"C\" gender:female\n\
                   marriage m alice <CURSOR>";
        let labels = run(src);
        assert!(!labels.contains(&"alice".to_owned()));
        assert!(labels.contains(&"bob".to_owned()));
        assert!(labels.contains(&"carol".to_owned()));
    }

    #[test]
    fn at_marriage_id_position_offers_nothing() {
        // The user is naming a NEW marriage; existing marriage ids would be
        // collisions, and persons/keywords are wrong here too.
        let src = "marriage <CURSOR>";
        assert!(run(src).is_empty());
    }

    #[test]
    fn after_both_spouses_falls_through_to_marriage_fields() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b <CURSOR>";
        let labels = run(src);
        assert!(labels.contains(&"start:".to_owned()));
        assert!(labels.contains(&"end:".to_owned()));
    }

    #[test]
    fn marriage_completion_kinds_are_distinct() {
        let src = "person alice name:\"A\" gender:female\n\
                   marriage m alice alice start:1972\n\
                   person kid name:\"K\" gender:other\n  birth <CURSOR>";
        let kinds = run_kinds(src);
        assert!(kinds.iter().all(|(_, k)| *k == CompletionItemKind::EVENT));
    }

    #[test]
    fn person_completion_kinds_are_variable() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m <CURSOR>";
        let kinds = run_kinds(src);
        assert!(
            kinds
                .iter()
                .all(|(_, k)| *k == CompletionItemKind::VARIABLE)
        );
    }

    #[test]
    fn snapshot_after_birth_keyword() {
        insta::assert_json_snapshot!(complete_for(
            "person alice name:\"Alice\" gender:female born:1950\n\
             person bob name:\"Bob\" gender:male born:1948\n\
             marriage m_alice_bob alice bob start:1972 end:1990 end_reason:divorce\n\
             person kid name:\"K\" gender:other\n  birth <CURSOR>",
        ));
    }

    #[test]
    fn snapshot_after_marriage_id_for_spouse_a() {
        insta::assert_json_snapshot!(complete_for(
            "person alice name:\"Alice\" gender:female born:1950\n\
             person bob name:\"Bob\" gender:male born:1948\n\
             marriage m <CURSOR>",
        ));
    }

    #[test]
    fn snapshot_after_spouse_a_excludes_self() {
        insta::assert_json_snapshot!(complete_for(
            "person alice name:\"Alice\" gender:female born:1950\n\
             person bob name:\"Bob\" gender:male born:1948\n\
             person carol name:\"Carol\" gender:female born:1975\n\
             marriage m alice <CURSOR>",
        ));
    }

    #[test]
    fn snapshot_top_level() {
        insta::assert_json_snapshot!(complete_for("<CURSOR>"));
    }

    #[test]
    fn snapshot_after_gender_colon() {
        insta::assert_json_snapshot!(complete_for("person a name:\"A\" gender:<CURSOR>"));
    }
}
