//! Semantic tokens for `textDocument/semanticTokens/full`.
//!
//! Walks the AST in source order and emits one token per keyword, identifier,
//! field name, enum value, date, and string literal, with the type taxonomy
//! fixed by `docs/roadmap/04-polished-lsp.md`. The legend is the ordered
//! list of `SemanticTokenType`s the server reports in `initializeResult`;
//! clients map a `tokenType` index back through it.
//!
//! Pure dispatch over the parsed AST — no async, no LSP plumbing beyond
//! `lsp_types`. The encoded stream is line/character-delta-compressed per
//! the LSP spec.

use kula_core::ast::{AdoptionSub, BirthSub, MarriageStmt, PersonStmt, Statement, VersionDecl};
use kula_core::field_meta::{self, ValueKind};
use kula_core::semantic::ResolvedDocument;
use kula_core::span::ByteSpan;
use tower_lsp::lsp_types::{
    Position, SemanticToken, SemanticTokenType, SemanticTokens, SemanticTokensLegend,
};

use crate::convert::LineIndex;

/// The legend the server advertises. A token's `token_type` field indexes
/// into this list. The order is part of the protocol contract — appending
/// new types is fine; reordering breaks every connected client.
pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::KEYWORD,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::CLASS,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::PARAMETER,
            SemanticTokenType::NUMBER,
            SemanticTokenType::STRING,
        ],
        token_modifiers: Vec::new(),
    }
}

const TT_KEYWORD: u32 = 0;
const TT_PROPERTY: u32 = 1;
const TT_ENUM_MEMBER: u32 = 2;
const TT_CLASS: u32 = 3;
const TT_FUNCTION: u32 = 4;
const TT_VARIABLE: u32 = 5;
const TT_PARAMETER: u32 = 6;
const TT_NUMBER: u32 = 7;
const TT_STRING: u32 = 8;

#[derive(Debug, Clone, Copy)]
struct RawToken {
    span: ByteSpan,
    token_type: u32,
}

/// Build the semantic-token stream for the document. The result is an
/// LSP-encoded `SemanticTokens` payload — `data` is a flat `Vec<u32>` of
/// 5-tuples after delta encoding.
pub fn semantic_tokens(resolved: &ResolvedDocument<'_>, line_index: &LineIndex) -> SemanticTokens {
    let mut raw: Vec<RawToken> = Vec::new();
    if let Some(version) = &resolved.document().version {
        emit_version(&mut raw, version);
    }
    for stmt in resolved.statements() {
        match stmt {
            Statement::Person(p) => emit_person(&mut raw, p),
            Statement::Marriage(m) => emit_marriage(&mut raw, m),
        }
    }
    // Source order isn't guaranteed across siblings (sub-statement order is
    // birth-then-adoptions, which can interleave with the next person's
    // header in pathological inputs). Sort once at the edge to keep the
    // emit functions oblivious to ordering.
    raw.sort_by_key(|t| (t.span.start, t.span.end));
    SemanticTokens {
        result_id: None,
        data: encode(&raw, line_index),
    }
}

fn emit_version(out: &mut Vec<RawToken>, v: &VersionDecl) {
    out.push(RawToken {
        span: v.keyword_span,
        token_type: TT_KEYWORD,
    });
    out.push(RawToken {
        span: v.version_span,
        token_type: TT_NUMBER,
    });
}

fn emit_person(out: &mut Vec<RawToken>, p: &PersonStmt) {
    out.push(RawToken {
        span: p.keyword_span,
        token_type: TT_KEYWORD,
    });
    out.push(RawToken {
        span: p.id.span,
        token_type: TT_CLASS,
    });
    for f in &p.fields {
        emit_field(out, f.name_span, f.kind.value_span(), f.kind.field_name());
    }
    if let Some(birth) = &p.birth {
        emit_birth(out, birth);
    }
    for adoption in &p.adoptions {
        emit_adoption(out, adoption);
    }
}

fn emit_birth(out: &mut Vec<RawToken>, b: &BirthSub) {
    out.push(RawToken {
        span: b.keyword_span,
        token_type: TT_KEYWORD,
    });
    out.push(RawToken {
        span: b.marriage_ref.span,
        token_type: TT_PARAMETER,
    });
}

fn emit_adoption(out: &mut Vec<RawToken>, a: &AdoptionSub) {
    out.push(RawToken {
        span: a.keyword_span,
        token_type: TT_KEYWORD,
    });
    out.push(RawToken {
        span: a.marriage_ref.span,
        token_type: TT_PARAMETER,
    });
    for f in &a.fields {
        emit_field(out, f.name_span, f.kind.value_span(), f.kind.field_name());
    }
}

fn emit_marriage(out: &mut Vec<RawToken>, m: &MarriageStmt) {
    out.push(RawToken {
        span: m.keyword_span,
        token_type: TT_KEYWORD,
    });
    out.push(RawToken {
        span: m.id.span,
        token_type: TT_FUNCTION,
    });
    out.push(RawToken {
        span: m.spouse_a.span,
        token_type: TT_VARIABLE,
    });
    out.push(RawToken {
        span: m.spouse_b.span,
        token_type: TT_VARIABLE,
    });
    for f in &m.fields {
        emit_field(out, f.name_span, f.kind.value_span(), f.kind.field_name());
    }
}

/// Emit the two tokens for a single `field:value` pair: the property name
/// and the value (typed by the field's [`ValueKind`]).
fn emit_field(
    out: &mut Vec<RawToken>,
    name_span: ByteSpan,
    value_span: ByteSpan,
    name: kula_core::lexer::FieldName,
) {
    out.push(RawToken {
        span: name_span,
        token_type: TT_PROPERTY,
    });
    out.push(RawToken {
        span: value_span,
        token_type: token_type_for(field_meta::meta(name).value_kind),
    });
}

fn token_type_for(kind: ValueKind) -> u32 {
    match kind {
        ValueKind::String => TT_STRING,
        ValueKind::Date => TT_NUMBER,
        ValueKind::Enum => TT_ENUM_MEMBER,
    }
}

fn encode(raw: &[RawToken], line_index: &LineIndex) -> Vec<SemanticToken> {
    let mut out = Vec::with_capacity(raw.len());
    let mut prev = Position {
        line: 0,
        character: 0,
    };
    for tok in raw {
        let range = line_index.range(tok.span);
        // The LSP spec requires multi-line tokens to be split; none of our
        // literals span lines (the lexer rejects newlines inside strings,
        // and dates / idents are by construction single-line).
        if range.start.line != range.end.line {
            continue;
        }
        let length = range.end.character.saturating_sub(range.start.character);
        if length == 0 {
            continue;
        }
        let delta_line = range.start.line - prev.line;
        let delta_start = if delta_line == 0 {
            range.start.character - prev.character
        } else {
            range.start.character
        };
        out.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: tok.token_type,
            token_modifiers_bitset: 0,
        });
        prev = range.start;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use kula_core::lexer::tokenize;
    use kula_core::parser::parse;
    use kula_core::semantic::resolve;

    /// Decoded view of a token: the kind name (looked up via the legend) and
    /// the literal source slice it covers. Snapshot-friendly, and a much
    /// better diff than 5-tuples of integers.
    #[derive(Debug)]
    struct Decoded {
        line: u32,
        character: u32,
        length: u32,
        kind: &'static str,
        text: String,
    }

    fn type_name(idx: u32) -> &'static str {
        match idx {
            TT_KEYWORD => "keyword",
            TT_PROPERTY => "property",
            TT_ENUM_MEMBER => "enumMember",
            TT_CLASS => "class",
            TT_FUNCTION => "function",
            TT_VARIABLE => "variable",
            TT_PARAMETER => "parameter",
            TT_NUMBER => "number",
            TT_STRING => "string",
            _ => "unknown",
        }
    }

    fn decode(source: &str, tokens: &SemanticTokens) -> Vec<Decoded> {
        let mut line: u32 = 0;
        let mut character: u32 = 0;
        let line_starts = {
            let mut v = vec![0usize];
            for (i, b) in source.bytes().enumerate() {
                if b == b'\n' {
                    v.push(i + 1);
                }
            }
            v
        };
        let mut out = Vec::new();
        for t in &tokens.data {
            if t.delta_line > 0 {
                line += t.delta_line;
                character = t.delta_start;
            } else {
                character += t.delta_start;
            }
            let line_start = line_starts[line as usize];
            let line_end = line_starts
                .get(line as usize + 1)
                .copied()
                .unwrap_or(source.len());
            let line_text = &source[line_start..line_end];
            let mut utf16: u32 = 0;
            let mut byte_start = line_start;
            for c in line_text.chars() {
                if utf16 >= character {
                    break;
                }
                utf16 += c.len_utf16() as u32;
                byte_start += c.len_utf8();
            }
            let mut byte_end = byte_start;
            let mut utf16_left = t.length;
            for c in source[byte_start..].chars() {
                if utf16_left == 0 {
                    break;
                }
                let units = c.len_utf16() as u32;
                if units > utf16_left {
                    break;
                }
                utf16_left -= units;
                byte_end += c.len_utf8();
            }
            out.push(Decoded {
                line,
                character,
                length: t.length,
                kind: type_name(t.token_type),
                text: source[byte_start..byte_end].to_owned(),
            });
        }
        out
    }

    fn tokens_for(source: &str) -> (SemanticTokens, Vec<Decoded>) {
        let tokens = tokenize(source);
        let (document, _) = parse(&tokens);
        let (resolved, _) = resolve(&document);
        let line_index = LineIndex::new(source);
        let semantic = semantic_tokens(&resolved, &line_index);
        let decoded = decode(source, &semantic);
        (semantic, decoded)
    }

    #[test]
    fn legend_matches_taxonomy() {
        let l = legend();
        assert_eq!(l.token_types.len(), 9);
        assert_eq!(
            l.token_types[TT_KEYWORD as usize],
            SemanticTokenType::KEYWORD
        );
        assert_eq!(l.token_types[TT_CLASS as usize], SemanticTokenType::CLASS);
        assert_eq!(
            l.token_types[TT_FUNCTION as usize],
            SemanticTokenType::FUNCTION
        );
        assert!(l.token_modifiers.is_empty());
    }

    #[test]
    fn empty_document_yields_no_tokens() {
        let (sem, decoded) = tokens_for("");
        assert!(sem.data.is_empty());
        assert!(decoded.is_empty());
    }

    #[test]
    fn decoded_position_and_length_match_source_slice() {
        let src = "person alice name:\"Alice\" gender:female\n";
        let (_, decoded) = tokens_for(src);
        let alice_decl = decoded.iter().find(|d| d.text == "alice").unwrap();
        assert_eq!(
            (alice_decl.line, alice_decl.character, alice_decl.length),
            (0, 7, 5)
        );
        let alice_str = decoded.iter().find(|d| d.text == "\"Alice\"").unwrap();
        // 7 utf-16 code units for the quoted form.
        assert_eq!(alice_str.length, 7);
    }

    #[test]
    fn person_decl_is_class_marriage_decl_is_function() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bob name:\"Bob\" gender:male\n\
                   marriage m alice bob start:2010\n";
        let (_, decoded) = tokens_for(src);
        let alice_decl = decoded.iter().find(|d| d.text == "alice" && d.line == 0);
        assert_eq!(alice_decl.unwrap().kind, "class");
        let m_decl = decoded.iter().find(|d| d.text == "m" && d.line == 2);
        assert_eq!(m_decl.unwrap().kind, "function");
    }

    #[test]
    fn person_ref_is_variable_marriage_ref_is_parameter() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:2010\n\
                   person kid name:\"K\" gender:other\n  birth m\n";
        let (_, decoded) = tokens_for(src);
        // The two `alice` and `bob` mentions on the marriage line are spouse
        // references — they should be `variable`, not `class`.
        let spouse_refs: Vec<_> = decoded
            .iter()
            .filter(|d| d.line == 2 && (d.text == "alice" || d.text == "bob"))
            .collect();
        assert_eq!(spouse_refs.len(), 2);
        for r in spouse_refs {
            assert_eq!(r.kind, "variable");
        }
        // The `m` on the `birth m` line is a marriage reference.
        let marriage_ref = decoded
            .iter()
            .find(|d| d.line == 4 && d.text == "m")
            .unwrap();
        assert_eq!(marriage_ref.kind, "parameter");
    }

    #[test]
    fn keywords_fields_enums_dates_strings_classified() {
        let src = "kula 0.1\n\
                   person alice name:\"Alice\" gender:female born:1950-04-12\n\
                   person bob name:\"Bob\" gender:male\n\
                   marriage m alice bob start:1972 end:1990 end_reason:divorce\n";
        let (_, decoded) = tokens_for(src);
        let kinds: Vec<_> = decoded.iter().map(|d| (d.text.as_str(), d.kind)).collect();
        assert!(kinds.contains(&("kula", "keyword")));
        assert!(kinds.contains(&("0.1", "number")));
        assert!(kinds.contains(&("person", "keyword")));
        assert!(kinds.contains(&("marriage", "keyword")));
        assert!(kinds.contains(&("name", "property")));
        assert!(kinds.contains(&("\"Alice\"", "string")));
        assert!(kinds.contains(&("gender", "property")));
        assert!(kinds.contains(&("female", "enumMember")));
        assert!(kinds.contains(&("1950-04-12", "number")));
        assert!(kinds.contains(&("end_reason", "property")));
        assert!(kinds.contains(&("divorce", "enumMember")));
    }

    #[test]
    fn adoption_substatement_emits_keyword_param_and_dates() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  adoption m start:2000 end:2010\n";
        let (_, decoded) = tokens_for(src);
        let row: Vec<_> = decoded.iter().filter(|d| d.line == 4).collect();
        let texts: Vec<_> = row.iter().map(|d| (d.text.as_str(), d.kind)).collect();
        assert_eq!(
            texts,
            vec![
                ("adoption", "keyword"),
                ("m", "parameter"),
                ("start", "property"),
                ("2000", "number"),
                ("end", "property"),
                ("2010", "number"),
            ]
        );
    }

    #[test]
    fn tokens_are_sorted_and_non_overlapping() {
        let src = include_str!("../../../../examples/03-three-generations.kula");
        let (sem, _) = tokens_for(src);
        // After delta encoding, every entry has either delta_line > 0 or
        // (delta_line == 0 && delta_start > 0), and length > 0. That's
        // exactly the LSP-spec requirement on the encoded stream.
        for t in &sem.data {
            assert!(t.length > 0, "zero-length token: {t:?}");
            assert!(
                t.delta_line > 0
                    || t.delta_start > 0
                    || t == sem.data.first().unwrap_or(&SemanticToken {
                        delta_line: 0,
                        delta_start: 0,
                        length: 0,
                        token_type: 0,
                        token_modifiers_bitset: 0
                    }),
                "tokens overlap: {t:?}"
            );
        }
    }

    #[test]
    fn parse_errors_still_emit_partial_stream() {
        // Half a `person` decl plus a complete one — the second still gets
        // tokens even though the first half-statement parses with errors.
        let src = "person\nperson alice name:\"A\" gender:female\n";
        let (_, decoded) = tokens_for(src);
        // We don't pin the recovery shape, but the second line's tokens
        // should be present.
        assert!(
            decoded
                .iter()
                .any(|d| d.text == "alice" && d.kind == "class")
        );
        assert!(
            decoded
                .iter()
                .any(|d| d.text == "\"A\"" && d.kind == "string")
        );
    }

    #[test]
    fn snapshot_single_couple() {
        let src = include_str!("../../../../examples/01-single-couple.kula");
        let (_, decoded) = tokens_for(src);
        insta::assert_debug_snapshot!(decoded);
    }

    #[test]
    fn snapshot_nuclear_family() {
        let src = include_str!("../../../../examples/02-nuclear-family.kula");
        let (_, decoded) = tokens_for(src);
        insta::assert_debug_snapshot!(decoded);
    }

    #[test]
    fn snapshot_three_generations() {
        let src = include_str!("../../../../examples/03-three-generations.kula");
        let (_, decoded) = tokens_for(src);
        insta::assert_debug_snapshot!(decoded);
    }

    #[test]
    fn snapshot_polygamous_family() {
        let src = include_str!("../../../../examples/04-polygamous-family.kula");
        let (_, decoded) = tokens_for(src);
        insta::assert_debug_snapshot!(decoded);
    }
}
