//! Basic completion for `textDocument/completion`.
//!
//! Token-stream-first classifier: the cursor often sits in whitespace or
//! a partial token where there's no clean AST node, so we walk tokens up
//! to the cursor and classify into one of seven contexts. The seven sets
//! are static — ID-reference completion (suggesting declared marriage IDs
//! after `birth`, etc.) is Phase 4.

use kula_core::ast::{
    AdoptionFieldKind, MarriageFieldKind, MarriageStmt, PersonFieldKind, PersonStmt, Statement,
};
use kula_core::lexer::{EnumKw, FieldName, Token, TokenKind, tokenize};
use kula_core::semantic::ResolvedDocument;
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind};

/// Build a completion item list for the cursor at `byte_offset`. Empty
/// vec when the cursor lands somewhere we have nothing to offer.
pub fn complete(
    source: &str,
    resolved: &ResolvedDocument<'_>,
    byte_offset: usize,
) -> Vec<CompletionItem> {
    let tokens = tokenize(source);
    match classify(source, &tokens, resolved, byte_offset) {
        Context::None => Vec::new(),
        Context::TopLevelStart => top_level_keywords(),
        Context::IndentedUnderPerson => sub_statement_keywords(),
        Context::PersonFieldList { existing } => person_fields(&existing),
        Context::MarriageFieldList { existing } => marriage_fields(&existing),
        Context::AdoptionFieldList { existing } => adoption_fields(&existing),
        Context::AfterGenderColon => gender_values(),
        Context::AfterEndReasonColon => end_reason_values(),
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
    /// No completion to offer (inside a string, on a date literal, in a
    /// comment, etc.).
    None,
}

fn classify(
    source: &str,
    tokens: &[Token],
    resolved: &ResolvedDocument<'_>,
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
            // Other field types (string, date) — nothing useful to offer.
            _ => return Context::None,
        }
    }

    // 2. Determine line state: is this line "fresh" (no significant tokens
    //    before cursor on this line), and is it indented? What's the first
    //    significant keyword on the line, if any?
    let line = current_line(&preceding);

    // 3. Determine the enclosing top-level statement.
    let enclosing = enclosing_statement(resolved, cursor);

    match (line.is_fresh, line.is_indented, &line.first_kw, enclosing) {
        // Fresh, non-indented line at top level → top-level keywords.
        (true, false, _, _) => Context::TopLevelStart,

        // Fresh, indented line under a person → birth/adoption.
        (true, true, _, Some(Enclosing::Person(_))) => Context::IndentedUnderPerson,

        // Indented, non-fresh: the line starts with `adoption <m_ref> ...`.
        // Offer adoption field names, filtered.
        (false, true, Some(LineKw::Adoption), Some(Enclosing::Person(p))) => {
            Context::AdoptionFieldList {
                existing: existing_adoption_fields_at_line(p, &line),
            }
        }

        // Continuing a person's same-line field list.
        (false, false, _, Some(Enclosing::Person(p))) => Context::PersonFieldList {
            existing: existing_person_fields(p),
        },

        // Continuing a marriage's same-line field list.
        (false, false, _, Some(Enclosing::Marriage(m))) => Context::MarriageFieldList {
            existing: existing_marriage_fields(m),
        },

        _ => Context::None,
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
    first_kw_span: Option<kula_core::span::ByteSpan>,
}

#[derive(Debug, PartialEq, Eq)]
enum LineKw {
    Person,
    Marriage,
    Kula,
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
                TokenKind::KulaKw => Some(LineKw::Kula),
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

enum Enclosing<'a> {
    Person(&'a PersonStmt),
    Marriage(&'a MarriageStmt),
}

fn enclosing_statement<'a>(
    resolved: &'a ResolvedDocument<'_>,
    cursor: usize,
) -> Option<Enclosing<'a>> {
    // Most recent statement whose span starts at or before the cursor.
    let stmts = &resolved.document().statements;
    let mut chosen: Option<&Statement> = None;
    for s in stmts {
        let span = match s {
            Statement::Person(p) => p.span,
            Statement::Marriage(m) => m.span,
        };
        if span.start <= cursor {
            chosen = Some(s);
        }
    }
    match chosen? {
        Statement::Person(p) => Some(Enclosing::Person(p)),
        Statement::Marriage(m) => Some(Enclosing::Marriage(m)),
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
    p.fields
        .iter()
        .map(|f| match &f.kind {
            PersonFieldKind::Name(_) => FieldName::Name,
            PersonFieldKind::Family(_) => FieldName::Family,
            PersonFieldKind::Given(_) => FieldName::Given,
            PersonFieldKind::Born(_) => FieldName::Born,
            PersonFieldKind::Died(_) => FieldName::Died,
            PersonFieldKind::Gender(_) => FieldName::Gender,
        })
        .collect()
}

fn existing_marriage_fields(m: &MarriageStmt) -> Vec<FieldName> {
    m.fields
        .iter()
        .map(|f| match &f.kind {
            MarriageFieldKind::Start(_) => FieldName::Start,
            MarriageFieldKind::End(_) => FieldName::End,
            MarriageFieldKind::EndReason(_) => FieldName::EndReason,
        })
        .collect()
}

fn existing_adoption_fields_at_line(p: &PersonStmt, line: &LineInfo) -> Vec<FieldName> {
    let kw_span = match line.first_kw_span {
        Some(s) => s,
        None => return Vec::new(),
    };
    // Match the adoption whose keyword span starts at the same byte as
    // the line's first keyword (the parser-built `keyword_span`).
    let adopt = p
        .adoptions
        .iter()
        .find(|a| a.keyword_span.start == kw_span.start);
    match adopt {
        Some(a) => a
            .fields
            .iter()
            .map(|f| match &f.kind {
                AdoptionFieldKind::Start(_) => FieldName::Start,
                AdoptionFieldKind::End(_) => FieldName::End,
            })
            .collect(),
        None => Vec::new(),
    }
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
        item("kula", CompletionItemKind::KEYWORD, "Version declaration"),
        item("person", CompletionItemKind::KEYWORD, "Declare a person"),
        item(
            "marriage",
            CompletionItemKind::KEYWORD,
            "Declare a marriage",
        ),
    ]
}

fn sub_statement_keywords() -> Vec<CompletionItem> {
    vec![
        item(
            "birth",
            CompletionItemKind::KEYWORD,
            "Biological-parent marriage",
        ),
        item(
            "adoption",
            CompletionItemKind::KEYWORD,
            "Adoption sub-statement",
        ),
    ]
}

fn person_fields(existing: &[FieldName]) -> Vec<CompletionItem> {
    let all: &[(FieldName, &str)] = &[
        (FieldName::Name, "Display name (UTF-8 string)"),
        (FieldName::Family, "Family-name component"),
        (FieldName::Given, "Given-name component"),
        (FieldName::Gender, "male / female / other"),
        (FieldName::Born, "Birth date"),
        (FieldName::Died, "Death date (absent = alive)"),
    ];
    all.iter()
        .filter(|(f, _)| !existing.contains(f))
        .map(|(f, doc)| item(&format!("{}:", f.as_str()), CompletionItemKind::FIELD, doc))
        .collect()
}

fn marriage_fields(existing: &[FieldName]) -> Vec<CompletionItem> {
    let all: &[(FieldName, &str)] = &[
        (FieldName::Start, "Date the marriage began (required)"),
        (FieldName::End, "Date the marriage ended"),
        (FieldName::EndReason, "Reason: divorce"),
    ];
    all.iter()
        .filter(|(f, _)| !existing.contains(f))
        .map(|(f, doc)| item(&format!("{}:", f.as_str()), CompletionItemKind::FIELD, doc))
        .collect()
}

fn adoption_fields(existing: &[FieldName]) -> Vec<CompletionItem> {
    let all: &[(FieldName, &str)] = &[
        (FieldName::Start, "Date the adoption started"),
        (FieldName::End, "Date the adoption ended"),
    ];
    all.iter()
        .filter(|(f, _)| !existing.contains(f))
        .map(|(f, doc)| item(&format!("{}:", f.as_str()), CompletionItemKind::FIELD, doc))
        .collect()
}

fn gender_values() -> Vec<CompletionItem> {
    [EnumKw::Male, EnumKw::Female, EnumKw::Other]
        .iter()
        .map(|kw| item(kw.as_str(), CompletionItemKind::ENUM_MEMBER, "Gender"))
        .collect()
}

fn end_reason_values() -> Vec<CompletionItem> {
    vec![item(
        EnumKw::Divorce.as_str(),
        CompletionItemKind::ENUM_MEMBER,
        "End reason",
    )]
}

#[cfg(test)]
mod tests {
    use super::*;
    use kula_core::lexer::tokenize;
    use kula_core::parser::parse;
    use kula_core::semantic::resolve;

    /// Take a fixture string with a `<CURSOR>` marker; return (source, offset).
    fn cursor_fixture(s: &str) -> (String, usize) {
        let offset = s.find("<CURSOR>").expect("fixture must contain <CURSOR>");
        let source = format!("{}{}", &s[..offset], &s[offset + "<CURSOR>".len()..]);
        (source, offset)
    }

    fn run(src_with_marker: &str) -> Vec<String> {
        let (source, offset) = cursor_fixture(src_with_marker);
        let tokens = tokenize(&source);
        let (document, _) = parse(&tokens);
        let (resolved, _) = resolve(&document);
        complete(&source, &resolved, offset)
            .into_iter()
            .map(|c| c.label)
            .collect()
    }

    #[test]
    fn top_level_start_blank_doc() {
        assert_eq!(
            run("<CURSOR>"),
            vec!["kula".to_owned(), "person".into(), "marriage".into()]
        );
    }

    #[test]
    fn top_level_start_after_blank_line() {
        let src = "person a name:\"A\" gender:female\n<CURSOR>";
        let labels = run(src);
        assert!(labels.contains(&"kula".to_owned()));
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

    #[test]
    fn after_name_colon_returns_empty() {
        // We don't suggest values for free-form fields like name:.
        let src = "person a name:<CURSOR>";
        assert!(run(src).is_empty());
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

    #[test]
    fn snapshot_top_level() {
        let (source, offset) = cursor_fixture("<CURSOR>");
        let tokens = tokenize(&source);
        let (document, _) = parse(&tokens);
        let (resolved, _) = resolve(&document);
        let items = complete(&source, &resolved, offset);
        insta::assert_json_snapshot!(items);
    }

    #[test]
    fn snapshot_after_gender_colon() {
        let (source, offset) = cursor_fixture("person a name:\"A\" gender:<CURSOR>");
        let tokens = tokenize(&source);
        let (document, _) = parse(&tokens);
        let (resolved, _) = resolve(&document);
        let items = complete(&source, &resolved, offset);
        insta::assert_json_snapshot!(items);
    }
}
