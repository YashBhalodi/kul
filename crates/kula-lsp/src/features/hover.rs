//! Hover content for `textDocument/hover`.
//!
//! Pure dispatch over [`kula_core::node_at::Node`]: each variant maps to a
//! Markdown content builder. The async `Backend::hover` method is a thin
//! shell over [`hover`].

use kula_core::ast::{
    AdoptionField, AdoptionFieldKind, MarriageField, MarriageFieldKind, MarriageStmt, PersonField,
    PersonFieldKind, PersonStmt,
};
use kula_core::node_at::{KeywordKind, Node};
use kula_core::semantic::ResolvedDocument;
use kula_core::span::ByteSpan;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

use crate::convert::LineIndex;

const SPEC_BASE: &str = "https://github.com/YashBhalodi/kulalang/blob/main/spec";

/// Build a hover response for the cursor at `byte_offset`, or `None` if
/// nothing useful sits there. Pure: no async, no `Client`, no `tower-lsp`
/// types beyond `lsp_types`.
pub fn hover(
    resolved: &ResolvedDocument<'_>,
    line_index: &LineIndex,
    byte_offset: usize,
) -> Option<Hover> {
    let node = resolved.node_at(byte_offset)?;
    let (markdown, span) = match node {
        Node::Keyword(k, span) => (keyword_content(k), span),
        Node::VersionLiteral(v) => (
            "Kula language version this document targets.".to_owned(),
            v.version_span,
        ),
        Node::PersonDeclId(p) => (person_panel(p), p.id.span),
        Node::MarriageDeclId(m) => (marriage_panel(resolved, m), m.id.span),
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
        } => (marriage_panel(resolved, m), ident.span),
        Node::MarriageRef {
            ident,
            target: None,
        } => (unresolved_note("marriage", &ident.name), ident.span),
        Node::PersonFieldName(f) => (person_field_doc(&f.kind).to_owned(), f.name_span),
        Node::PersonFieldValue(f) => (
            person_field_value_md(line_index.source(), f),
            value_span_of_person(f),
        ),
        Node::MarriageFieldName(f) => (marriage_field_doc(&f.kind).to_owned(), f.name_span),
        Node::MarriageFieldValue(f) => (
            marriage_field_value_md(line_index.source(), f),
            value_span_of_marriage(f),
        ),
        Node::AdoptionFieldName(f) => (adoption_field_doc(&f.kind).to_owned(), f.name_span),
        Node::AdoptionFieldValue(f) => (
            adoption_field_value_md(line_index.source(), f),
            value_span_of_adoption(f),
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
        KeywordKind::Kula => format!(
            "**`kula`** — version declaration keyword.\n\nIntroduces a Kula document and binds it to a language version. See [§2 Document structure]({SPEC_BASE}/02-document-structure.md)."
        ),
        KeywordKind::Person => format!(
            "**`person`** — top-level statement declaring an individual.\n\nFollowed by an identifier and any of the person fields (`name:`, `family:`, `given:`, `gender:`, `born:`, `died:`). See [§4 Top-level statements]({SPEC_BASE}/04-top-level-statements.md)."
        ),
        KeywordKind::Marriage => format!(
            "**`marriage`** — top-level statement declaring a marriage between two persons.\n\nFollowed by an identifier, two spouse references, and any of the marriage fields (`start:`, `end:`, `end_reason:`). See [§4 Top-level statements]({SPEC_BASE}/04-top-level-statements.md)."
        ),
        KeywordKind::Birth => format!(
            "**`birth`** — sub-statement of `person` recording the biological-parent marriage.\n\nA person has at most one `birth` (spec §5.1). See [§5 Person sub-statements]({SPEC_BASE}/05-person-sub-statements.md)."
        ),
        KeywordKind::Adoption => format!(
            "**`adoption`** — sub-statement of `person` recording an adoption by a marriage.\n\nA person may have multiple adoptions; each carries a `start:` date and an optional `end:` date. See [§5 Person sub-statements]({SPEC_BASE}/05-person-sub-statements.md)."
        ),
    }
}

fn person_panel(p: &PersonStmt) -> String {
    let mut out = format!("**`person {}`**", p.id.name);
    if let Some(name) = p.name() {
        out.push_str(&format!("\n\n- name: \"{}\"", escape(&name.value)));
    }
    if let Some(family) = p.family() {
        out.push_str(&format!("\n- family: \"{}\"", escape(&family.value)));
    }
    if let Some(given) = p.given() {
        out.push_str(&format!("\n- given: \"{}\"", escape(&given.value)));
    }
    if let Some(g) = p.gender() {
        let label = match g.value {
            kula_core::ast::Gender::Male => "male",
            kula_core::ast::Gender::Female => "female",
            kula_core::ast::Gender::Other => "other",
        };
        out.push_str(&format!("\n- gender: `{label}`"));
    }
    if let Some(b) = p.born() {
        out.push_str(&format!("\n- born: `{}`", date_repr(b)));
    }
    if let Some(d) = p.died() {
        out.push_str(&format!("\n- died: `{}`", date_repr(d)));
    }
    out
}

fn marriage_panel(resolved: &ResolvedDocument<'_>, m: &MarriageStmt) -> String {
    let mut out = format!("**`marriage {}`**", m.id.name);
    let spouse_a = resolved.person(&m.spouse_a.name);
    let spouse_b = resolved.person(&m.spouse_b.name);
    out.push_str(&format!(
        "\n\n- spouse A: {}",
        spouse_repr(&m.spouse_a.name, spouse_a)
    ));
    out.push_str(&format!(
        "\n- spouse B: {}",
        spouse_repr(&m.spouse_b.name, spouse_b)
    ));
    if let Some(start) = m.start() {
        out.push_str(&format!("\n- start: `{}`", date_repr(start)));
    }
    if let Some(end) = m.end() {
        out.push_str(&format!("\n- end: `{}`", date_repr(end)));
    }
    if let Some(reason) = m.end_reason() {
        let label = match &reason.value {
            kula_core::ast::EndReason::Divorce => "divorce".to_owned(),
            kula_core::ast::EndReason::Unknown(s) => s.clone(),
        };
        out.push_str(&format!("\n- end_reason: `{label}`"));
    }
    out
}

fn spouse_repr(id: &str, target: Option<&PersonStmt>) -> String {
    match target {
        Some(p) => match p.name() {
            Some(n) => format!("`{id}` (\"{}\")", escape(&n.value)),
            None => format!("`{id}`"),
        },
        None => format!("`{id}` *(unresolved)*"),
    }
}

fn unresolved_note(kind: &str, id: &str) -> String {
    format!(
        "**`{id}`** — unresolved {kind} reference. The {kind} `{id}` is not declared in this document. See diagnostic `KULA-R02`."
    )
}

fn person_field_doc(k: &PersonFieldKind) -> &'static str {
    match k {
        PersonFieldKind::Name(_) => "**`name:`** — display name; full UTF-8 string.",
        PersonFieldKind::Family(_) => "**`family:`** — family-name component; UTF-8 string.",
        PersonFieldKind::Given(_) => "**`given:`** — given-name component; UTF-8 string.",
        PersonFieldKind::Born(_) => {
            "**`born:`** — birth date. `YYYY`, `YYYY-MM`, or `YYYY-MM-DD`; optional leading `~` for ±5y circa."
        }
        PersonFieldKind::Died(_) => {
            "**`died:`** — death date. Same date forms as `born:`. Absent means alive (spec §4.2)."
        }
        PersonFieldKind::Gender(_) => "**`gender:`** — one of `male`, `female`, `other`.",
    }
}

fn marriage_field_doc(k: &MarriageFieldKind) -> &'static str {
    match k {
        MarriageFieldKind::Start(_) => {
            "**`start:`** — date the marriage began. Required (spec §4.3)."
        }
        MarriageFieldKind::End(_) => {
            "**`end:`** — date the marriage ended. Pairs with `end_reason:`."
        }
        MarriageFieldKind::EndReason(_) => {
            "**`end_reason:`** — reason the marriage ended. Currently only `divorce`. Pairs with `end:`."
        }
    }
}

fn adoption_field_doc(k: &AdoptionFieldKind) -> &'static str {
    match k {
        AdoptionFieldKind::Start(_) => "**`start:`** — date the adoption started.",
        AdoptionFieldKind::End(_) => "**`end:`** — date the adoption ended (open-ended if absent).",
    }
}

fn person_field_value_md(source: &str, f: &PersonField) -> String {
    let doc = person_field_doc(&f.kind);
    let literal = source_slice(source, value_span_of_person(f));
    format!("{doc}\n\n`{literal}`")
}

fn marriage_field_value_md(source: &str, f: &MarriageField) -> String {
    let doc = marriage_field_doc(&f.kind);
    let literal = source_slice(source, value_span_of_marriage(f));
    format!("{doc}\n\n`{literal}`")
}

fn adoption_field_value_md(source: &str, f: &AdoptionField) -> String {
    let doc = adoption_field_doc(&f.kind);
    let literal = source_slice(source, value_span_of_adoption(f));
    format!("{doc}\n\n`{literal}`")
}

fn value_span_of_person(f: &PersonField) -> ByteSpan {
    match &f.kind {
        PersonFieldKind::Name(s) | PersonFieldKind::Family(s) | PersonFieldKind::Given(s) => s.span,
        PersonFieldKind::Born(d) | PersonFieldKind::Died(d) => d.span,
        PersonFieldKind::Gender(g) => g.span,
    }
}

fn value_span_of_marriage(f: &MarriageField) -> ByteSpan {
    match &f.kind {
        MarriageFieldKind::Start(d) | MarriageFieldKind::End(d) => d.span,
        MarriageFieldKind::EndReason(r) => r.span,
    }
}

fn value_span_of_adoption(f: &AdoptionField) -> ByteSpan {
    match &f.kind {
        AdoptionFieldKind::Start(d) | AdoptionFieldKind::End(d) => d.span,
    }
}

fn date_repr(d: &kula_core::date::DateLit) -> String {
    let mut s = String::new();
    if d.circa {
        s.push('~');
    }
    s.push_str(&format!("{:04}", d.year));
    if let Some(m) = d.month {
        s.push_str(&format!("-{m:02}"));
    }
    if let Some(day) = d.day {
        s.push_str(&format!("-{day:02}"));
    }
    s
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
    use kula_core::lexer::tokenize;
    use kula_core::parser::parse;
    use kula_core::semantic::resolve;

    fn hover_at(source: &str, offset: usize) -> Option<String> {
        let tokens = tokenize(source);
        let (document, _) = parse(&tokens);
        let (resolved, _) = resolve(&document);
        let line_index = LineIndex::new(source);
        hover(&resolved, &line_index, offset).map(|h| match h.contents {
            HoverContents::Markup(MarkupContent { value, .. }) => value,
            _ => panic!("expected markup contents"),
        })
    }

    fn idx(source: &str, pat: &str) -> usize {
        source.find(pat).expect("pattern in source")
    }

    #[test]
    fn keyword_kula() {
        let src = "kula 1\n";
        let body = hover_at(src, 0).unwrap();
        assert!(body.contains("`kula`"));
        assert!(body.contains("Document structure"));
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
    fn version_literal() {
        let src = "kula 1\n";
        let body = hover_at(src, idx(src, "1")).unwrap();
        assert!(body.contains("version"));
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
        assert!(body.contains("unresolved"));
        assert!(body.contains("KULA-R02"));
        // Doesn't dump a full panel.
        assert!(!body.contains("- name:"));
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
        assert!(body.contains("unresolved"));
        assert!(body.contains("KULA-R02"));
    }

    #[test]
    fn person_field_name_doc() {
        let src = "person alice name:\"A\" gender:female born:1900\n";
        for (field, expect) in [
            ("name:", "display name"),
            ("gender:", "male"),
            ("born:", "birth date"),
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
        assert!(body.contains("birth date"));
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
        assert!(body.contains("started"), "adoption start hover:\n{body}");
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
