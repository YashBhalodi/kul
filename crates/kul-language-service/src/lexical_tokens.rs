//! Lexical tokens: highlighting classified from the lexer alone.
//!
//! Cheap enough to run on every keystroke, before any analysis, and it covers
//! what the syntax tree cannot: comments and lines that do not parse.
//! [`semantic_tokens`](crate::semantic_tokens) refines it after analysis.
//!
//! Classification looks only at the token and the tokens just before it on
//! its line: the first id after `person` or `marriage` is the declaration,
//! every other id a reference, and a value after `field:` highlights as the
//! field table says that field's values do (ADR-0005).

use kul_core::lexer::{FieldName, TokenKind as LexKind, tokenize};
use kul_core::span::ByteSpan;

use crate::semantic_tokens::value_token_kind;
use crate::{Token, TokenKind};

/// The lexical tokens of `source`, in source order.
#[must_use]
pub fn lexical_tokens(source: &str) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::new();
    let mut gap_start = 0;
    // The first id after `person` / `marriage` on this line is declared.
    let mut declaring = false;
    // `field` then `:` — the next token is that field's value.
    let mut field: Option<FieldName> = None;
    let mut after_colon: Option<FieldName> = None;

    for token in tokenize(source) {
        push_comment(source, gap_start, token.span.start, &mut out);
        gap_start = token.span.end;

        let value_of = after_colon.take();
        let kind = match &token.kind {
            LexKind::PersonKw | LexKind::MarriageKw => {
                declaring = true;
                Some(TokenKind::Keyword)
            }
            LexKind::BirthKw | LexKind::AdoptionKw => {
                declaring = false;
                Some(TokenKind::Keyword)
            }
            LexKind::FieldKw(name) => {
                field = Some(*name);
                Some(TokenKind::FieldName)
            }
            LexKind::Colon => {
                after_colon = field.take();
                None
            }
            LexKind::Newline => {
                declaring = false;
                None
            }
            LexKind::Indent | LexKind::Eof => None,
            _ if value_of.is_some() => value_of.map(value_token_kind),
            LexKind::Ident(_) if declaring => {
                declaring = false;
                Some(TokenKind::Declaration)
            }
            LexKind::Ident(_) => Some(TokenKind::Reference),
            LexKind::EnumKw(_) => Some(TokenKind::EnumValue),
            LexKind::String(_) => Some(TokenKind::String),
            LexKind::Bare(text) if text.starts_with(|c: char| c.is_ascii_digit() || c == '~') => {
                Some(TokenKind::Date)
            }
            // An unterminated string lexes as an error that starts at its quote.
            LexKind::Error(_) if source[token.span.start..].starts_with('"') => {
                Some(TokenKind::String)
            }
            LexKind::Bare(_) | LexKind::Error(_) => None,
        };
        if !matches!(token.kind, LexKind::FieldKw(_)) {
            field = None;
        }
        if let Some(kind) = kind {
            push(source, token.span, kind, &mut out);
        }
    }
    out
}

/// A `#` comment in the whitespace between two tokens. The lexer skips
/// comments, so anything in a gap from `#` on is one.
fn push_comment(source: &str, start: usize, end: usize, out: &mut Vec<Token>) {
    if let Some(hash) = source[start..end].find('#') {
        let comment = source[start + hash..end].trim_end();
        let span = ByteSpan::new(start + hash, start + hash + comment.len());
        push(source, span, TokenKind::Comment, out);
    }
}

/// Push a token, cut at its first line break: an unterminated string runs
/// to the end of the file, but highlights only to the end of its line.
fn push(source: &str, span: ByteSpan, kind: TokenKind, out: &mut Vec<Token>) {
    let text = &source[span.start..span.end];
    let line = text.split(['\r', '\n']).next().unwrap_or_default();
    if !line.is_empty() {
        out.push(Token {
            span: ByteSpan::new(span.start, span.start + line.len()).into(),
            kind,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_tokens;
    use crate::test_support::{FILE, check_one};

    fn texts(source: &str) -> Vec<(&str, TokenKind)> {
        lexical_tokens(source)
            .into_iter()
            .map(|t| (&source[t.span.start..t.span.end], t.kind))
            .collect()
    }

    #[test]
    fn classifies_a_person_line() {
        let src = "person alice name:\"Alice\" gender:female born:~1950-04\n";
        assert_eq!(
            texts(src),
            vec![
                ("person", TokenKind::Keyword),
                ("alice", TokenKind::Declaration),
                ("name", TokenKind::FieldName),
                ("\"Alice\"", TokenKind::String),
                ("gender", TokenKind::FieldName),
                ("female", TokenKind::EnumValue),
                ("born", TokenKind::FieldName),
                ("~1950-04", TokenKind::Date),
            ]
        );
    }

    #[test]
    fn classifies_a_marriage_line_and_sub_statements() {
        let src = "marriage m alice bob start:1972 end_reason:divorce\n  birth m\n  adoption m start:2000\n";
        assert_eq!(
            texts(src),
            vec![
                ("marriage", TokenKind::Keyword),
                ("m", TokenKind::Declaration),
                ("alice", TokenKind::Reference),
                ("bob", TokenKind::Reference),
                ("start", TokenKind::FieldName),
                ("1972", TokenKind::Date),
                ("end_reason", TokenKind::FieldName),
                ("divorce", TokenKind::EnumValue),
                ("birth", TokenKind::Keyword),
                ("m", TokenKind::Reference),
                ("adoption", TokenKind::Keyword),
                ("m", TokenKind::Reference),
                ("start", TokenKind::FieldName),
                ("2000", TokenKind::Date),
            ]
        );
    }

    #[test]
    fn comments_are_tokens_and_hashes_in_strings_are_not() {
        let src = "# The founders\nperson a name:\"No #1\" gender:male # first  \n";
        assert_eq!(
            texts(src),
            vec![
                ("# The founders", TokenKind::Comment),
                ("person", TokenKind::Keyword),
                ("a", TokenKind::Declaration),
                ("name", TokenKind::FieldName),
                ("\"No #1\"", TokenKind::String),
                ("gender", TokenKind::FieldName),
                ("male", TokenKind::EnumValue),
                ("# first", TokenKind::Comment),
            ]
        );
    }

    #[test]
    fn a_line_that_does_not_parse_still_highlights() {
        let src = "person\nmarriage m alice\nperson kid name:\"unterminated\n";
        let kinds = texts(src);
        assert!(kinds.contains(&("person", TokenKind::Keyword)));
        assert!(kinds.contains(&("m", TokenKind::Declaration)));
        assert!(kinds.contains(&("alice", TokenKind::Reference)));
        assert!(kinds.contains(&("\"unterminated", TokenKind::String)));
    }

    #[test]
    fn an_unknown_end_reason_value_highlights_as_an_enum_value() {
        let src = "marriage m a b start:1990 end:2000 end_reason:annulment\n";
        assert!(texts(src).contains(&("annulment", TokenKind::EnumValue)));
    }

    #[test]
    fn crlf_line_endings_end_comments_and_lines() {
        let src = "person a name:\"A\" gender:male # note\r\nperson b name:\"B\" gender:female\r\n";
        let kinds = texts(src);
        assert!(kinds.contains(&("# note", TokenKind::Comment)));
        assert!(kinds.contains(&("b", TokenKind::Declaration)));
    }

    /// On parsed source the two classifiers agree wherever both speak, so
    /// semantic tokens refine lexical ones without contradicting them.
    #[test]
    fn agrees_with_semantic_tokens_on_every_example() {
        for src in [
            include_str!("../../../examples/01-nuclear-family/nuclear-family.kul"),
            include_str!("../../../examples/04-adoption-and-belonging/adoption-and-belonging.kul"),
            include_str!("../../../examples/06-polygamous-household/polygamous-household.kul"),
            include_str!(
                "../../../examples/09-family-across-a-century/family-across-a-century.kul"
            ),
        ] {
            let lexical = lexical_tokens(src);
            let semantic = semantic_tokens(&check_one(src), FILE);
            assert!(!semantic.is_empty());
            for token in &semantic {
                assert!(
                    lexical.contains(token),
                    "semantic token {token:?} ({:?}) has no lexical twin",
                    &src[token.span.start..token.span.end]
                );
            }
        }
    }
}
