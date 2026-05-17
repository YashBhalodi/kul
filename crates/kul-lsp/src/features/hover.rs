//! Hover content for `textDocument/hover`.
//!
//! Pure dispatch over [`kul_core::node_at::Node`]: each variant maps to a
//! Markdown content builder. The async `Backend::hover` method is a thin
//! shell over [`hover`].

use kul_core::ast::{AdoptionField, MarriageField, MarriageStmt, PersonField, PersonStmt};
use kul_core::field_meta;
use kul_core::node_at::{KeywordKind, Node};
use kul_core::semantic::ResolvedDocument;
use kul_core::span::ByteSpan;
use kul_core::span::FileId;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

use crate::convert::LineIndex;

const SPEC_BASE: &str = "https://github.com/YashBhalodi/kul/blob/main/spec";

/// Build a hover response for the cursor at `byte_offset`, or `None` if
/// nothing useful sits there. Pure: no async, no `Client`, no `tower-lsp`
/// types beyond `lsp_types`.
pub fn hover(
    file: FileId,
    resolved: &ResolvedDocument,
    line_index: &LineIndex,
    byte_offset: usize,
) -> Option<Hover> {
    let node = resolved.node_at(file, byte_offset)?;
    let (markdown, span) = match node {
        Node::Keyword(k, span) => (keyword_content(k), span),
        Node::PersonDeclId(p) => (person_panel(p), p.id.span),
        Node::MarriageDeclId(m) => (marriage_panel(file, resolved, m), m.id.span),
        Node::PersonRef {
            ident,
            target: Some(p),
        } => (person_panel(p), ident.span),
        Node::PersonRef {
            ident,
            target: None,
        } => (unresolved_note("person", &ident.name), ident.span),
        Node::MarriageRef {
            ident,
            target: Some(m),
        } => (marriage_panel(file, resolved, m), ident.span),
        Node::MarriageRef {
            ident,
            target: None,
        } => (unresolved_note("marriage", &ident.name), ident.span),
        Node::PersonFieldName(f) => (
            field_meta::meta(f.kind.field_name()).hover_md.to_owned(),
            f.name_span,
        ),
        Node::PersonFieldValue(f) => (
            person_field_value_md(line_index.source(), f),
            f.kind.value_span(),
        ),
        Node::MarriageFieldName(f) => (
            field_meta::meta(f.kind.field_name()).hover_md.to_owned(),
            f.name_span,
        ),
        Node::MarriageFieldValue(f) => (
            marriage_field_value_md(line_index.source(), f),
            f.kind.value_span(),
        ),
        Node::AdoptionFieldName(f) => (
            field_meta::meta(f.kind.field_name()).hover_md.to_owned(),
            f.name_span,
        ),
        Node::AdoptionFieldValue(f) => (
            adoption_field_value_md(line_index.source(), f),
            f.kind.value_span(),
        ),
    };

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: Some(line_index.range(span)),
    })
}

fn keyword_content(k: KeywordKind) -> String {
    match k {
        KeywordKind::Person => format!(
            "**`person`** — declares an individual.\n\nGive each person a unique id, then their `name:` and `gender:`. Birth and death dates are optional.\n\n```kul\nperson alice name:\"Alice Doe\" gender:female born:1980\n```\n\n[Top-level statements →]({SPEC_BASE}/04-top-level-statements.md)"
        ),
        KeywordKind::Marriage => format!(
            "**`marriage`** — declares a marriage between two people.\n\nGive each marriage a unique id, the two spouses' ids, and a `start:` date. Add `end:` and `end_reason:` if it ended.\n\n```kul\nmarriage m_alice_bob alice bob start:2010 end:2020 end_reason:divorce\n```\n\n[Top-level statements →]({SPEC_BASE}/04-top-level-statements.md)"
        ),
        KeywordKind::Birth => format!(
            "**`birth`** — links a person to their biological parents.\n\nIndent under a person and give the marriage id of the biological parents. Each person has at most one `birth`.\n\n```kul\nperson kid name:\"Kid\" gender:other\n  birth m_alice_bob\n```\n\n[Person sub-statements →]({SPEC_BASE}/05-person-sub-statements.md)"
        ),
        KeywordKind::Adoption => format!(
            "**`adoption`** — links a person to an adoptive marriage.\n\nIndent under a person and give the adoptive marriage's id and a `start:` date. Add `end:` if the adoption ended. A person may have multiple adoptions.\n\n```kul\nperson kid name:\"Kid\" gender:other\n  adoption m_carol_dave start:2005\n```\n\n[Person sub-statements →]({SPEC_BASE}/05-person-sub-statements.md)"
        ),
    }
}

fn person_panel(p: &PersonStmt) -> String {
    let mut out = match p.name() {
        Some(name) => format!("**{}** — `person {}`", escape(&name.value), p.id.name),
        None => format!("**`person {}`** *(no `name:` set)*", p.id.name),
    };
    let mut details: Vec<String> = Vec::new();
    if let Some(g) = p.gender() {
        let label = match g.value {
            kul_core::ast::Gender::Male => "male",
            kul_core::ast::Gender::Female => "female",
            kul_core::ast::Gender::Other => "other",
        };
        details.push(format!("- gender: {label}"));
    }
    if let Some(b) = p.born() {
        details.push(format!("- born: `{}`", b.format_canonical()));
    }
    if let Some(d) = p.died() {
        details.push(format!("- died: `{}`", d.format_canonical()));
    }
    if let Some(family) = p.family() {
        details.push(format!("- family name: {}", escape(&family.value)));
    }
    if let Some(given) = p.given() {
        details.push(format!("- given name: {}", escape(&given.value)));
    }
    if !details.is_empty() {
        out.push_str("\n\n");
        out.push_str(&details.join("\n"));
    }
    out
}

fn marriage_panel(_file: FileId, resolved: &ResolvedDocument, m: &MarriageStmt) -> String {
    let spouse_a = resolved.person(&m.spouse_a.name);
    let spouse_b = resolved.person(&m.spouse_b.name);
    let header = match (display_name_of(spouse_a), display_name_of(spouse_b)) {
        (Some(a), Some(b)) => format!("**{} & {}** — `marriage {}`", a, b, m.id.name),
        _ => format!("**`marriage {}`**", m.id.name),
    };
    let mut out = header;
    out.push_str(&format!(
        "\n\n- spouses: {} & {}",
        spouse_repr(&m.spouse_a.name, spouse_a),
        spouse_repr(&m.spouse_b.name, spouse_b),
    ));
    if let Some(start) = m.start() {
        out.push_str(&format!("\n- start: `{}`", start.format_canonical()));
    }
    if let Some(end) = m.end() {
        out.push_str(&format!("\n- end: `{}`", end.format_canonical()));
    }
    if let Some(reason) = m.end_reason() {
        let label = match &reason.value {
            kul_core::ast::EndReason::Divorce => "divorce".to_owned(),
            kul_core::ast::EndReason::Unknown(s) => s.clone(),
        };
        out.push_str(&format!("\n- end_reason: `{label}`"));
    }
    out
}

fn display_name_of(p: Option<&PersonStmt>) -> Option<String> {
    p?.name().map(|n| escape(&n.value))
}

fn spouse_repr(id: &str, target: Option<&PersonStmt>) -> String {
    match target {
        Some(p) => match p.name() {
            Some(n) => format!("`{id}` ({})", escape(&n.value)),
            None => format!("`{id}`"),
        },
        None => format!("`{id}` *(not declared)*"),
    }
}

fn unresolved_note(kind: &str, id: &str) -> String {
    format!(
        "**`{id}`** — no `{kind}` with this id is declared in this file.\n\nCheck for a typo, or add a `{kind} {id} …` declaration somewhere in the file.\n\nDiagnostic `KUL-R02`."
    )
}

fn person_field_value_md(source: &str, f: &PersonField) -> String {
    let doc = field_meta::meta(f.kind.field_name()).hover_md;
    let literal = source_slice(source, f.kind.value_span());
    format!("{doc}\n\n`{literal}`")
}

fn marriage_field_value_md(source: &str, f: &MarriageField) -> String {
    let doc = field_meta::meta(f.kind.field_name()).hover_md;
    let literal = source_slice(source, f.kind.value_span());
    format!("{doc}\n\n`{literal}`")
}

fn adoption_field_value_md(source: &str, f: &AdoptionField) -> String {
    let doc = field_meta::meta(f.kind.field_name()).hover_md;
    let literal = source_slice(source, f.kind.value_span());
    format!("{doc}\n\n`{literal}`")
}

fn source_slice(source: &str, span: ByteSpan) -> &str {
    let end = span.end.min(source.len());
    let start = span.start.min(end);
    &source[start..end]
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use kul_core::lexer::tokenize;
    use kul_core::parser::parse;
    use kul_core::semantic::resolve;
    use std::sync::Arc;

    fn hover_at(source: &str, offset: usize) -> Option<String> {
        use kul_core::ast::{Document, KulFile};
        let file = FileId::from_raw(1);
        let tokens = tokenize(source);
        let (statements, _) = parse(&tokens, file);
        let kf = Arc::new(KulFile::new("test.kul", source, statements));
        let document = Arc::new(Document::new("kul.yml", vec![kf]));
        let (resolved, _) = resolve(document);
        let line_index = LineIndex::new(source);
        hover(file, &resolved, &line_index, offset).map(|h| match h.contents {
            HoverContents::Markup(MarkupContent { value, .. }) => value,
            _ => panic!("expected markup contents"),
        })
    }

    fn idx(source: &str, pat: &str) -> usize {
        source.find(pat).expect("pattern in source")
    }

    #[test]
    fn keyword_person_marriage_birth_adoption() {
        let src = "person a name:\"A\" gender:female\n  birth m\n  adoption m start:2000\n\
                   marriage m a a start:1980\n";
        for (kw, expected) in [
            ("person", "`person`"),
            ("birth", "`birth`"),
            ("adoption", "`adoption`"),
            ("marriage", "`marriage`"),
        ] {
            let body = hover_at(src, idx(src, kw)).unwrap();
            assert!(body.contains(expected), "missing '{expected}' in:\n{body}");
        }
    }

    #[test]
    fn person_decl_id_panel() {
        let src = "person alice name:\"Alice\" gender:female born:1900-01-01 died:~1980\n";
        let body = hover_at(src, idx(src, "alice")).unwrap();
        assert!(body.contains("person alice"));
        assert!(body.contains("Alice"));
        assert!(body.contains("female"));
        assert!(body.contains("1900-01-01"));
        assert!(body.contains("~1980"));
    }

    #[test]
    fn marriage_decl_id_panel() {
        let src = "person a name:\"Alice\" gender:female\n\
                   person b name:\"Bob\" gender:male\n\
                   marriage m a b start:2010 end:2020 end_reason:divorce\n";
        let body = hover_at(src, idx(src, "marriage m") + "marriage ".len()).unwrap();
        assert!(body.contains("marriage m"));
        assert!(body.contains("Alice"));
        assert!(body.contains("Bob"));
        assert!(body.contains("2010"));
        assert!(body.contains("2020"));
        assert!(body.contains("divorce"));
    }

    #[test]
    fn person_ref_resolved_matches_decl_panel() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bob name:\"Bob\" gender:male\n\
                   marriage m alice bob start:2010\n";
        let marriage_line = idx(src, "marriage ");
        let alice_ref = src[marriage_line..]
            .find(" alice ")
            .map(|i| marriage_line + i + 1)
            .unwrap();
        let body = hover_at(src, alice_ref).unwrap();
        // Same content as the decl panel.
        assert!(body.contains("person alice"));
        assert!(body.contains("Alice"));
    }

    #[test]
    fn person_ref_unresolved_short_note() {
        let src = "marriage m ghost b start:2000\nperson b name:\"B\" gender:male\n";
        let marriage_line = idx(src, "marriage ");
        let ghost = src[marriage_line..]
            .find("ghost")
            .map(|i| marriage_line + i)
            .unwrap();
        let body = hover_at(src, ghost).unwrap();
        assert!(body.contains("not declared") || body.contains("no `person`"));
        assert!(body.contains("KUL-R02"));
        // Doesn't dump a full panel.
        assert!(!body.contains("- gender:"));
    }

    #[test]
    fn marriage_ref_resolved_in_birth() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  birth m\n";
        let m_ref = idx(src, "birth m") + "birth ".len();
        let body = hover_at(src, m_ref).unwrap();
        assert!(body.contains("marriage m"));
    }

    #[test]
    fn marriage_ref_unresolved_short_note() {
        let src = "person kid name:\"K\" gender:other\n  birth m_nope\n";
        let body = hover_at(src, idx(src, "m_nope")).unwrap();
        assert!(body.contains("not declared") || body.contains("no `marriage`"));
        assert!(body.contains("KUL-R02"));
    }

    #[test]
    fn person_field_name_doc() {
        let src = "person alice name:\"A\" gender:female born:1900\n";
        for (field, expect) in [
            ("name:", "display name"),
            ("gender:", "male"),
            ("born:", "date of birth"),
        ] {
            let body = hover_at(src, idx(src, field)).unwrap();
            assert!(
                body.contains(expect),
                "field `{field}` hover missing `{expect}`:\n{body}"
            );
        }
    }

    #[test]
    fn person_field_value_includes_literal() {
        let src = "person alice name:\"A\" gender:female born:~1900-06\n";
        let body = hover_at(src, idx(src, "~1900-06")).unwrap();
        assert!(body.contains("date of birth"));
        assert!(body.contains("~1900-06"));
    }

    #[test]
    fn marriage_field_name_doc() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:2010 end:2020 end_reason:divorce\n";
        for (field, expect) in [
            ("start:", "began"),
            ("end:", "ended"),
            ("end_reason:", "divorce"),
        ] {
            let body = hover_at(src, idx(src, field)).unwrap();
            assert!(
                body.contains(expect),
                "marriage field `{field}` hover missing `{expect}`:\n{body}"
            );
        }
    }

    #[test]
    fn adoption_field_name_doc() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  adoption m start:2000 end:2010\n";
        let adoption_line = idx(src, "adoption");
        let start_field = src[adoption_line..]
            .find("start:")
            .map(|i| adoption_line + i)
            .unwrap();
        let end_field = src[adoption_line..]
            .find("end:")
            .map(|i| adoption_line + i)
            .unwrap();
        let body = hover_at(src, start_field).unwrap();
        assert!(
            body.contains("took effect"),
            "adoption start hover:\n{body}"
        );
        let body = hover_at(src, end_field).unwrap();
        assert!(body.contains("ended"), "adoption end hover:\n{body}");
    }

    #[test]
    fn whitespace_returns_none() {
        let src = "person a name:\"A\" gender:female\n\nperson b name:\"B\" gender:male\n";
        let blank = src.find("\n\n").unwrap() + 1;
        assert!(hover_at(src, blank).is_none());
    }

    #[test]
    fn snapshot_person_decl_panel() {
        let src = "person alice name:\"Alice\" family:\"Doe\" given:\"A.\" gender:female born:1900-01-01 died:~1980\n";
        let body = hover_at(src, idx(src, "alice")).unwrap();
        insta::assert_snapshot!(body);
    }

    #[test]
    fn snapshot_marriage_decl_panel() {
        let src = "person a name:\"Alice Doe\" gender:female\n\
                   person b name:\"Bob Smith\" gender:male\n\
                   marriage m a b start:2010-06-15 end:2020-04-01 end_reason:divorce\n";
        let body = hover_at(src, idx(src, "marriage m") + "marriage ".len()).unwrap();
        insta::assert_snapshot!(body);
    }
}
