//! Token-stream-first completion classifier (ADR-0002).
//!
//! Walks tokens up to the cursor and classifies into a fixed set of
//! contexts. The AST (`ResolvedDocument`) is a secondary signal for
//! enclosing statement and already-declared fields.

use kul_core::ast::{PersonStmt, Statement};
use kul_core::lexer::{FieldName, Token, TokenKind};
use kul_core::semantic::ResolvedDocument;
use kul_core::span::FileId;

/// Cursor context for completion — the classifier's public interface.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Context {
    TopLevelStart,
    IndentedUnderPerson,
    PersonFieldList {
        existing: Vec<FieldName>,
    },
    MarriageFieldList {
        existing: Vec<FieldName>,
    },
    AdoptionFieldList {
        existing: Vec<FieldName>,
    },
    AfterGenderColon,
    AfterEndReasonColon,
    /// After a quoted-string field's `:` (`name:`, `family:`, `given:`) —
    /// surfaces a snippet that wraps the value in quotes.
    AfterStringFieldColon,
    MarriageRefPosition,
    /// Spouse position in a `marriage`; `exclude` filters spouse_a so
    /// spouse_b won't repeat them.
    SpousePosition {
        exclude: Option<String>,
    },
    None,
}

/// Classify the cursor into a [`Context`] from the token stream (first)
/// and enclosing AST (second).
pub(crate) fn classify(
    source: &str,
    file: FileId,
    tokens: &[Token],
    resolved: &ResolvedDocument,
    cursor: usize,
) -> Context {
    if cursor > source.len() {
        return Context::None;
    }
    if cursor_inside_string_or_comment(source, cursor) {
        return Context::None;
    }

    let preceding: Vec<&Token> = tokens
        .iter()
        .filter(|t| t.span.end <= cursor && !matches!(t.kind, TokenKind::Eof))
        .collect();

    // After `field:` — `gender:<CURSOR>`, `gender: <CURSOR>`, and
    // `gender:f<CURSOR>` all count; `gender:female <CURSOR>` does not
    // (whitespace after a complete value closes the field).
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

    let line = current_line(&preceding);
    let enclosing = resolved.statement_at(file, cursor);
    let scan = positional_scan(&preceding, cursor);

    // Discriminate on the line's leading keyword first — partially-typed
    // lines won't have parsed cleanly, so `enclosing` may not reflect them.
    match (line.is_fresh, line.is_indented, &line.first_kw) {
        (true, false, _) => Context::TopLevelStart,

        (true, true, _) => match enclosing {
            Some(Statement::Person(_)) => Context::IndentedUnderPerson,
            _ => Context::None,
        },

        (false, true, Some(LineKw::Birth)) => {
            if scan.filled == 0 && !scan.past_positionals {
                Context::MarriageRefPosition
            } else {
                Context::None
            }
        }

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

        // `marriage` line: pos 0 (own id), 1 (spouse_a), 2 (spouse_b), then fields.
        (false, false, Some(LineKw::Marriage)) => {
            let existing = match enclosing {
                Some(Statement::Marriage(m)) => m.declared_field_names().collect::<Vec<_>>(),
                _ => Vec::new(),
            };
            if scan.past_positionals {
                Context::MarriageFieldList { existing }
            } else {
                match scan.filled {
                    // Marriage's own id — suggesting existing ids would surface collisions.
                    0 => Context::None,
                    1 => Context::SpousePosition { exclude: None },
                    2 => Context::SpousePosition {
                        exclude: scan.seen.get(1).cloned(),
                    },
                    _ => Context::MarriageFieldList { existing },
                }
            }
        }

        (false, false, Some(LineKw::Person)) => match enclosing {
            Some(Statement::Person(p)) => Context::PersonFieldList {
                existing: p.declared_field_names().collect::<Vec<_>>(),
            },
            _ => Context::None,
        },

        // Continuation line (no leading keyword) — depends on enclosing statement.
        (false, _, _) => match enclosing {
            Some(Statement::Person(p)) => Context::PersonFieldList {
                existing: p.declared_field_names().collect::<Vec<_>>(),
            },
            Some(Statement::Marriage(m)) => Context::MarriageFieldList {
                existing: m.declared_field_names().collect::<Vec<_>>(),
            },
            None => Context::None,
        },
    }
}

/// Scan of the positional region — naked identifier slots after the leading
/// keyword, before any `field:` token.
struct PositionalScan {
    filled: usize,
    seen: Vec<String>,
    past_positionals: bool,
}

fn positional_scan(preceding: &[&Token], cursor: usize) -> PositionalScan {
    let line_start_idx = preceding
        .iter()
        .rposition(|t| matches!(t.kind, TokenKind::Newline))
        .map(|i| i + 1)
        .unwrap_or(0);
    let line_tokens = &preceding[line_start_idx..];

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
                    // Cursor adjacent to a partial identifier — user is
                    // mid-typing this slot, so don't count it as filled.
                } else {
                    filled += 1;
                    seen.push(s.clone());
                }
            }
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

/// What's on the cursor's line, before the cursor.
struct LineInfo {
    is_fresh: bool,
    is_indented: bool,
    first_kw: Option<LineKw>,
    /// Span of the first keyword token, used to find the parsed sub-statement.
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
        // `field:<CURSOR>` or `field: <CURSOR>`.
        TokenKind::Colon => {
            if preceding.len() >= 2
                && let TokenKind::FieldKw(name) = &preceding[preceding.len() - 2].kind
            {
                return Some(*name);
            }
            None
        }
        // `field:f<CURSOR>` — partial value, cursor adjacent. With a
        // whitespace gap, the field is closed.
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

fn existing_adoption_fields_at_line(p: &PersonStmt, line: &LineInfo) -> Vec<FieldName> {
    let Some(kw_span) = line.first_kw_span else {
        return Vec::new();
    };
    let Some(adopt) = p
        .adoptions
        .iter()
        .find(|a| a.keyword_span.start == kw_span.start)
    else {
        return Vec::new();
    };
    adopt.declared_field_names().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::test_open_file;
    use kul_core::lexer::tokenize;

    /// Take a fixture string with a `<CURSOR>` marker; return (source, offset).
    fn cursor_fixture(s: &str) -> (String, usize) {
        let offset = s.find("<CURSOR>").expect("fixture must contain <CURSOR>");
        let source = format!("{}{}", &s[..offset], &s[offset + "<CURSOR>".len()..]);
        (source, offset)
    }

    fn context_for(src_with_marker: &str) -> Context {
        let (source, offset) = cursor_fixture(src_with_marker);
        let doc = test_open_file(&source);
        let v = doc.view();
        let tokens = tokenize(v.line_index.source());
        classify(v.line_index.source(), v.file, &tokens, v.resolved, offset)
    }

    #[test]
    fn top_level_start_blank_doc() {
        assert_eq!(context_for("<CURSOR>"), Context::TopLevelStart);
    }

    #[test]
    fn top_level_start_after_blank_line() {
        let src = "person a name:\"A\" gender:female\n<CURSOR>";
        assert_eq!(context_for(src), Context::TopLevelStart);
    }

    #[test]
    fn indented_under_person() {
        let src = "person a name:\"A\" gender:female\n  <CURSOR>";
        assert_eq!(context_for(src), Context::IndentedUnderPerson);
    }

    #[test]
    fn after_gender_colon() {
        let src = "person a name:\"A\" gender:<CURSOR>";
        assert_eq!(context_for(src), Context::AfterGenderColon);
    }

    #[test]
    fn after_gender_colon_partial_value() {
        let src = "person a name:\"A\" gender:f<CURSOR>";
        assert_eq!(context_for(src), Context::AfterGenderColon);
    }

    #[test]
    fn after_end_reason_colon() {
        let src = "marriage m a b start:2010 end:2020 end_reason:<CURSOR>";
        assert_eq!(context_for(src), Context::AfterEndReasonColon);
    }

    #[test]
    fn after_string_field_colon() {
        for field in ["name", "family", "given"] {
            let src = format!("person a {field}:<CURSOR>");
            assert_eq!(
                context_for(&src),
                Context::AfterStringFieldColon,
                "expected AfterStringFieldColon for `{field}:`"
            );
        }
    }

    #[test]
    fn inside_string_is_none() {
        let src = "person a name:\"Al<CURSOR>ice\" gender:female\n";
        assert_eq!(context_for(src), Context::None);
    }

    #[test]
    fn inside_comment_is_none() {
        let src = "person a name:\"A\" gender:female # this is a <CURSOR>";
        assert_eq!(context_for(src), Context::None);
    }

    #[test]
    fn at_marriage_id_position_is_none() {
        // Naming a NEW marriage — existing ids would collide.
        assert_eq!(context_for("marriage <CURSOR>"), Context::None);
    }

    #[test]
    fn after_birth_marriage_ref_is_none() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:1972\n\
                   person kid name:\"K\" gender:other\n  birth m <CURSOR>";
        assert_eq!(context_for(src), Context::None);
    }

    #[test]
    fn after_birth_is_marriage_ref_position() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:1972\n\
                   person kid name:\"K\" gender:other\n  birth <CURSOR>";
        assert_eq!(context_for(src), Context::MarriageRefPosition);
    }

    #[test]
    fn after_marriage_id_is_spouse_position() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bob name:\"Bob\" gender:male\n\
                   marriage m <CURSOR>";
        assert_eq!(context_for(src), Context::SpousePosition { exclude: None });
    }

    #[test]
    fn after_spouse_a_excludes_self() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   person carol name:\"C\" gender:female\n\
                   marriage m alice <CURSOR>";
        assert_eq!(
            context_for(src),
            Context::SpousePosition {
                exclude: Some("alice".to_owned()),
            }
        );
    }
}
