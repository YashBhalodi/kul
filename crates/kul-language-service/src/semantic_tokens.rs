//! Semantic tokens: highlighting classified from the checked syntax tree.
//!
//! Walks the file's statements and emits one token per keyword, id, field
//! name, enum value, date and string literal, in source order. It refines
//! [`lexical_tokens`](crate::lexical_tokens) once analysis has run: the tree
//! knows what the lexer can only guess at, but it skips comments and lines
//! that did not parse, so a consumer paints the lexical tokens and overrides
//! them with these where both cover the same text.

use kul_core::CheckResult;
use kul_core::ast::{AdoptionSub, BirthSub, MarriageStmt, PersonStmt, Statement};
use kul_core::field_meta::{self, ValueKind};
use kul_core::lexer::FieldName;
use kul_core::span::ByteSpan;

use crate::{Token, TokenKind, file_id, source_of};

/// The semantic tokens of `file`, sorted by position. Empty for an unknown
/// file.
#[must_use]
pub fn semantic_tokens(check: &CheckResult, file: &str) -> Vec<Token> {
    let Some(file) = file_id(check, file) else {
        return Vec::new();
    };
    let Some(source) = source_of(check, file) else {
        return Vec::new();
    };
    let mut raw: Vec<(ByteSpan, TokenKind)> = Vec::new();
    for stmt in check.resolved().statements_in(file) {
        match stmt {
            Statement::Person(p) => emit_person(&mut raw, p),
            Statement::Marriage(m) => emit_marriage(&mut raw, m),
        }
    }
    // Source order isn't guaranteed across siblings; sort once at the edge.
    raw.sort_by_key(|(span, _)| (span.start, span.end));
    raw.into_iter()
        .filter(|(span, _)| !span.is_empty() && !source[span.start..span.end].contains('\n'))
        .map(|(span, kind)| Token {
            span: span.into(),
            kind,
        })
        .collect()
}

fn emit_person(out: &mut Vec<(ByteSpan, TokenKind)>, p: &PersonStmt) {
    out.push((p.keyword_span, TokenKind::Keyword));
    out.push((p.id.span, TokenKind::Declaration));
    for f in p.fields() {
        emit_field(out, f.name_span, f.kind.value_span(), f.kind.field_name());
    }
    if let Some(birth) = &p.birth {
        emit_birth(out, birth);
    }
    for adoption in &p.adoptions {
        emit_adoption(out, adoption);
    }
}

fn emit_birth(out: &mut Vec<(ByteSpan, TokenKind)>, b: &BirthSub) {
    out.push((b.keyword_span, TokenKind::Keyword));
    out.push((b.marriage_ref.span, TokenKind::Reference));
}

fn emit_adoption(out: &mut Vec<(ByteSpan, TokenKind)>, a: &AdoptionSub) {
    out.push((a.keyword_span, TokenKind::Keyword));
    out.push((a.marriage_ref.span, TokenKind::Reference));
    for f in a.fields() {
        emit_field(out, f.name_span, f.kind.value_span(), f.kind.field_name());
    }
}

fn emit_marriage(out: &mut Vec<(ByteSpan, TokenKind)>, m: &MarriageStmt) {
    out.push((m.keyword_span, TokenKind::Keyword));
    out.push((m.id.span, TokenKind::Declaration));
    out.push((m.spouse_a.span, TokenKind::Reference));
    out.push((m.spouse_b.span, TokenKind::Reference));
    for f in m.fields() {
        emit_field(out, f.name_span, f.kind.value_span(), f.kind.field_name());
    }
}

/// Emit name + value tokens for a `field:value` pair.
fn emit_field(
    out: &mut Vec<(ByteSpan, TokenKind)>,
    name_span: ByteSpan,
    value_span: ByteSpan,
    name: FieldName,
) {
    out.push((name_span, TokenKind::FieldName));
    out.push((value_span, value_token_kind(name)));
}

/// How a field's value highlights, from its row in the field table
/// (ADR-0005). Shared with the lexical classifier.
pub(crate) fn value_token_kind(field: FieldName) -> TokenKind {
    match field_meta::meta(field).value_kind {
        ValueKind::String => TokenKind::String,
        ValueKind::Date => TokenKind::Date,
        ValueKind::Enum => TokenKind::EnumValue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FILE, check_one};

    /// Token text plus kind: snapshot-friendly, and a better diff than spans.
    #[derive(Debug)]
    struct Decoded {
        start: usize,
        end: usize,
        kind: TokenKind,
        text: String,
    }

    fn tokens_for(source: &str) -> Vec<Decoded> {
        semantic_tokens(&check_one(source), FILE)
            .into_iter()
            .map(|t| Decoded {
                start: t.span.start,
                end: t.span.end,
                kind: t.kind,
                text: source[t.span.start..t.span.end].to_owned(),
            })
            .collect()
    }

    fn texts(source: &str) -> Vec<(String, TokenKind)> {
        tokens_for(source)
            .into_iter()
            .map(|d| (d.text, d.kind))
            .collect()
    }

    #[test]
    fn empty_document_yields_no_tokens() {
        assert!(tokens_for("").is_empty());
    }

    #[test]
    fn spans_cover_the_token_text() {
        let src = "person alice name:\"Alice\" gender:female\n";
        let tokens = tokens_for(src);
        let alice = tokens.iter().find(|d| d.text == "alice").unwrap();
        assert_eq!((alice.start, alice.end), (7, 12));
        let string = tokens.iter().find(|d| d.text == "\"Alice\"").unwrap();
        assert_eq!(string.end - string.start, 7);
    }

    #[test]
    fn id_decls_are_declarations() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bob name:\"Bob\" gender:male\n\
                   marriage m alice bob start:2010\n";
        let tokens = texts(src);
        assert!(tokens.contains(&("alice".into(), TokenKind::Declaration)));
        assert!(tokens.contains(&("m".into(), TokenKind::Declaration)));
    }

    #[test]
    fn id_refs_are_references() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:2010\n\
                   person kid name:\"K\" gender:other\n  birth m\n";
        let line = src.find("marriage m").unwrap();
        let birth = src.find("birth m").unwrap();
        let refs: Vec<(String, TokenKind)> = tokens_for(src)
            .into_iter()
            .filter(|d| (d.start > line + 10 && d.start < line + 20) || d.start > birth + 5)
            .map(|d| (d.text, d.kind))
            .collect();
        assert_eq!(
            refs,
            vec![
                ("alice".into(), TokenKind::Reference),
                ("bob".into(), TokenKind::Reference),
                ("m".into(), TokenKind::Reference),
            ]
        );
    }

    #[test]
    fn keywords_fields_enums_dates_strings_classified() {
        let src = "person alice name:\"Alice\" gender:female born:1950-04-12\n\
                   person bob name:\"Bob\" gender:male\n\
                   marriage m alice bob start:1972 end:1990 end_reason:divorce\n";
        let kinds = texts(src);
        for expected in [
            ("person", TokenKind::Keyword),
            ("marriage", TokenKind::Keyword),
            ("name", TokenKind::FieldName),
            ("\"Alice\"", TokenKind::String),
            ("gender", TokenKind::FieldName),
            ("female", TokenKind::EnumValue),
            ("1950-04-12", TokenKind::Date),
            ("end_reason", TokenKind::FieldName),
            ("divorce", TokenKind::EnumValue),
        ] {
            assert!(
                kinds.contains(&(expected.0.to_owned(), expected.1)),
                "missing {expected:?}"
            );
        }
    }

    #[test]
    fn adoption_substatement_emits_keyword_reference_and_dates() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  adoption m start:2000 end:2010\n";
        let line = src.find("  adoption").unwrap();
        let row: Vec<(String, TokenKind)> = tokens_for(src)
            .into_iter()
            .filter(|d| d.start > line)
            .map(|d| (d.text, d.kind))
            .collect();
        assert_eq!(
            row,
            vec![
                ("adoption".into(), TokenKind::Keyword),
                ("m".into(), TokenKind::Reference),
                ("start".into(), TokenKind::FieldName),
                ("2000".into(), TokenKind::Date),
                ("end".into(), TokenKind::FieldName),
                ("2010".into(), TokenKind::Date),
            ]
        );
    }

    #[test]
    fn tokens_are_sorted_non_empty_and_non_overlapping() {
        let src = include_str!("../../../examples/02-three-generations/three-generations.kul");
        let tokens = tokens_for(src);
        for t in &tokens {
            assert!(t.end > t.start, "empty token: {t:?}");
        }
        for w in tokens.windows(2) {
            assert!(w[0].end <= w[1].start, "tokens overlap: {w:?}");
        }
    }

    #[test]
    fn parse_errors_still_emit_partial_stream() {
        // Half a `person` decl plus a complete one — second still gets tokens.
        let src = "person\nperson alice name:\"A\" gender:female\n";
        let kinds = texts(src);
        assert!(kinds.contains(&("alice".into(), TokenKind::Declaration)));
        assert!(kinds.contains(&("\"A\"".into(), TokenKind::String)));
    }

    #[test]
    fn snapshot_nuclear_family() {
        let src = include_str!("../../../examples/01-nuclear-family/nuclear-family.kul");
        insta::assert_debug_snapshot!(tokens_for(src));
    }

    #[test]
    fn snapshot_three_generations() {
        let src = include_str!("../../../examples/02-three-generations/three-generations.kul");
        insta::assert_debug_snapshot!(tokens_for(src));
    }

    #[test]
    fn snapshot_divorce_and_remarriage() {
        let src =
            include_str!("../../../examples/03-divorce-and-remarriage/divorce-and-remarriage.kul");
        insta::assert_debug_snapshot!(tokens_for(src));
    }

    #[test]
    fn snapshot_adoption_and_belonging() {
        let src =
            include_str!("../../../examples/04-adoption-and-belonging/adoption-and-belonging.kul");
        insta::assert_debug_snapshot!(tokens_for(src));
    }
}
