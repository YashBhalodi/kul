//! Hover content for `textDocument/hover`.
//!
//! Dispatch over [`kul_core::node_at::Node`]: each shape (keyword, id,
//! field) maps to a Markdown content builder.

use kul_core::ast::{MarriageStmt, PersonStmt, Statement};
use kul_core::field_meta;
use kul_core::node_at::{FieldNode, KeywordKind, Node};
use kul_core::semantic::ResolvedDocument;
use kul_core::span::ByteSpan;
use kul_core::span::FileId;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

use crate::convert::LineIndex;

const SPEC_BASE: &str = "https://github.com/YashBhalodi/kul/blob/main/spec";

/// Build a hover response for the cursor at `byte_offset`.
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
            target: Some((_, p)),
        } => {
            let mut panel = person_panel(p);
            if let Some(role) = spouse_role_line(resolved, file, byte_offset) {
                panel.push_str("\n\n");
                panel.push_str(&role);
            }
            (panel, ident.span)
        }
        Node::PersonRef {
            ident,
            target: None,
        } => (unresolved_note("person", &ident.name), ident.span),
        Node::MarriageRef {
            ident,
            target: Some((_, m)),
        } => (marriage_panel(file, resolved, m), ident.span),
        Node::MarriageRef {
            ident,
            target: None,
        } => (unresolved_note("marriage", &ident.name), ident.span),
        Node::PersonFieldName(_)
        | Node::PersonFieldValue(_)
        | Node::MarriageFieldName(_)
        | Node::MarriageFieldValue(_)
        | Node::AdoptionFieldName(_)
        | Node::AdoptionFieldValue(_) => {
            let field = node
                .field_node()
                .expect("field_node returns Some for the six field Node variants");
            field_hover(field, line_index.source())
        }
    };

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: Some(line_index.range(span)),
    })
}

/// On the `name:` side, just the field's doc. On the value side, append
/// the literal source text in a code span.
fn field_hover(field: FieldNode, source: &str) -> (String, ByteSpan) {
    let doc = field_meta::meta(field.name).hover_md;
    if field.is_name {
        (doc.to_owned(), field.name_span)
    } else {
        let literal = source_slice(source, field.value_span);
        (format!("{doc}\n\n`{literal}`"), field.value_span)
    }
}

fn keyword_content(k: KeywordKind) -> String {
    match k {
        KeywordKind::Person => format!(
            "**`person`** — declares an individual.\n\nGive each person a unique id, then their `name:` and `gender:`. Birth and death dates are optional.\n\n```kul\nperson alice name:\"Alice Doe\" gender:female born:1980\n```\n\n[Top-level statements →]({SPEC_BASE}/04-top-level-statements.md)"
        ),
        KeywordKind::Marriage => format!(
            "**`marriage`** — declares a marriage between two people.\n\nGive each marriage a unique id, the two spouses' ids, and a `start:` date. Add `end:` and `end_reason:` if it ended.\n\nThe first-listed spouse is the marriage's host; the second joins the host's family.\n\n```kul\nmarriage m_alice_bob alice bob start:2010 end:2020 end_reason:divorce\n```\n\n[Top-level statements →]({SPEC_BASE}/04-top-level-statements.md)"
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
        "\n\n- spouses: {} (host) & {}",
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

fn spouse_role_line(
    resolved: &ResolvedDocument,
    file: FileId,
    byte_offset: usize,
) -> Option<String> {
    let Statement::Marriage(m) = resolved.statement_at(file, byte_offset)? else {
        return None;
    };
    if m.spouse_a.span.start <= byte_offset && byte_offset < m.spouse_a.span.end {
        Some(format!("Host of marriage `{}`.", m.id.name))
    } else if m.spouse_b.span.start <= byte_offset && byte_offset < m.spouse_b.span.end {
        Some(format!("Joining spouse in marriage `{}`.", m.id.name))
    } else {
        None
    }
}

fn unresolved_note(kind: &str, id: &str) -> String {
    format!(
        "**`{id}`** — no `{kind}` with this id is declared in this file.\n\nCheck for a typo, or add a `{kind} {id} …` declaration somewhere in the file.\n\nDiagnostic `KUL-R02`."
    )
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
    use crate::state::{idx, test_open_file};

    fn hover_at(source: &str, offset: usize) -> Option<String> {
        let doc = test_open_file(source);
        let v = doc.view();
        hover(v.file, v.resolved, v.line_index, offset).map(|h| match h.contents {
            HoverContents::Markup(MarkupContent { value, .. }) => value,
            _ => panic!("expected markup contents"),
        })
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

    #[test]
    fn marriage_keyword_hover_mentions_host_rule() {
        let src = "marriage m a a start:1980\n";
        let body = hover_at(src, idx(src, "marriage")).unwrap();
        assert!(
            body.contains("host"),
            "marriage keyword hover should mention host rule:\n{body}"
        );
    }

    #[test]
    fn spouse_a_hover_marks_host_role() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bob name:\"Bob\" gender:male\n\
                   marriage m alice bob start:2010\n";
        let marriage_line = idx(src, "marriage ");
        let alice_ref = src[marriage_line..]
            .find(" alice ")
            .map(|i| marriage_line + i + 1)
            .unwrap();
        let body = hover_at(src, alice_ref).unwrap();
        assert!(
            body.contains("Host of marriage `m`"),
            "spouse_a hover should mark host role:\n{body}"
        );
    }

    #[test]
    fn spouse_b_hover_marks_joining_role() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bob name:\"Bob\" gender:male\n\
                   marriage m alice bob start:2010\n";
        let marriage_line = idx(src, "marriage ");
        let bob_ref = src[marriage_line..]
            .find(" bob")
            .map(|i| marriage_line + i + 1)
            .unwrap();
        let body = hover_at(src, bob_ref).unwrap();
        assert!(
            body.contains("Joining spouse in marriage `m`"),
            "spouse_b hover should mark joining role:\n{body}"
        );
    }

    #[test]
    fn snapshot_spouse_role_host() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bob name:\"Bob\" gender:male\n\
                   marriage m alice bob start:2010\n";
        let marriage_line = idx(src, "marriage ");
        let alice_ref = src[marriage_line..]
            .find(" alice ")
            .map(|i| marriage_line + i + 1)
            .unwrap();
        let body = hover_at(src, alice_ref).unwrap();
        insta::assert_snapshot!(body);
    }

    #[test]
    fn snapshot_spouse_role_joining() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bob name:\"Bob\" gender:male\n\
                   marriage m alice bob start:2010\n";
        let marriage_line = idx(src, "marriage ");
        let bob_ref = src[marriage_line..]
            .find(" bob")
            .map(|i| marriage_line + i + 1)
            .unwrap();
        let body = hover_at(src, bob_ref).unwrap();
        insta::assert_snapshot!(body);
    }
}
