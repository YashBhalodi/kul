//! Document symbols for `textDocument/documentSymbol`.
//!
//! Labels prefer display names (the `name:` field; for marriages, the
//! spouses' display names) and fall back to ids. Always returns something,
//! even when the document has errors — partial results beat an empty pane.

use kul_core::ast::{AdoptionSub, BirthSub, MarriageStmt, PersonStmt, Statement};
use kul_core::date::DateLit;
use kul_core::semantic::ResolvedDocument;
use kul_core::span::FileId;
use tower_lsp::lsp_types::{DocumentSymbol, SymbolKind};

use crate::convert::LineIndex;

/// Build the document-symbol tree for the outline view.
pub fn document_symbols(
    file: FileId,
    resolved: &ResolvedDocument,
    line_index: &LineIndex,
) -> Vec<DocumentSymbol> {
    resolved
        .statements_in(file)
        .map(|stmt| match stmt {
            Statement::Person(p) => person_symbol(line_index, p),
            Statement::Marriage(m) => marriage_symbol(file, resolved, line_index, m),
        })
        .collect()
}

fn person_symbol(line_index: &LineIndex, p: &PersonStmt) -> DocumentSymbol {
    // Sanitise empty / whitespace-only `name:""` to the person id. The LSP
    // `DocumentSymbol.name` field has a non-empty invariant (LSP spec) that
    // VSCode validates client-side, so we treat unusable names as "absent"
    // at this wire boundary rather than pushing the policy into core.
    let name = p
        .name()
        .map(|n| n.value.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| p.id.name.clone());
    let mut children = Vec::new();
    if let Some(birth) = &p.birth {
        children.push(birth_symbol(line_index, birth));
    }
    for adoption in &p.adoptions {
        children.push(adoption_symbol(line_index, adoption));
    }
    // reason: tower-lsp deprecates `DocumentSymbol.deprecated` but still requires the field.
    #[allow(deprecated)]
    DocumentSymbol {
        name,
        detail: person_detail(p),
        kind: SymbolKind::VARIABLE,
        tags: None,
        deprecated: None,
        range: line_index.range(p.span),
        selection_range: line_index.range(p.id.span),
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    }
}

fn marriage_symbol(
    file: FileId,
    resolved: &ResolvedDocument,
    line_index: &LineIndex,
    m: &MarriageStmt,
) -> DocumentSymbol {
    let a = display_name_or(file, resolved, &m.spouse_a.name);
    let b = display_name_or(file, resolved, &m.spouse_b.name);
    // reason: tower-lsp deprecates `DocumentSymbol.deprecated`, but the struct literal
    // still requires the field, so the lint is allowed at this marriage construction site.
    #[allow(deprecated)]
    DocumentSymbol {
        name: format!("{a} & {b}"),
        detail: marriage_detail(m),
        kind: SymbolKind::EVENT,
        tags: None,
        deprecated: None,
        range: line_index.range(m.span),
        selection_range: line_index.range(m.id.span),
        children: None,
    }
}

fn birth_symbol(line_index: &LineIndex, b: &BirthSub) -> DocumentSymbol {
    // reason: tower-lsp deprecates `DocumentSymbol.deprecated`, but the struct literal
    // still requires the field, so the lint is allowed at this birth construction site.
    #[allow(deprecated)]
    DocumentSymbol {
        name: format!("birth {}", b.marriage_ref.name),
        detail: None,
        kind: SymbolKind::FIELD,
        tags: None,
        deprecated: None,
        range: line_index.range(b.span),
        selection_range: line_index.range(b.keyword_span),
        children: None,
    }
}

fn adoption_symbol(line_index: &LineIndex, a: &AdoptionSub) -> DocumentSymbol {
    // reason: tower-lsp deprecates `DocumentSymbol.deprecated`, but the struct literal
    // still requires the field, so the lint is allowed at this adoption construction site.
    #[allow(deprecated)]
    DocumentSymbol {
        name: format!("adoption {}", a.marriage_ref.name),
        detail: adoption_detail(a),
        kind: SymbolKind::FIELD,
        tags: None,
        deprecated: None,
        range: line_index.range(a.span),
        selection_range: line_index.range(a.keyword_span),
        children: None,
    }
}

fn person_detail(p: &PersonStmt) -> Option<String> {
    let born = p.born().map(DateLit::format_year);
    let died = p.died().map(DateLit::format_year);
    match (born, died) {
        (Some(b), Some(d)) => Some(format!("{b}–{d}")),
        (Some(b), None) => Some(format!("b. {b}")),
        (None, Some(d)) => Some(format!("d. {d}")),
        (None, None) => None,
    }
}

fn marriage_detail(m: &MarriageStmt) -> Option<String> {
    let start = m.start().map(DateLit::format_year);
    let end = m.end().map(DateLit::format_year);
    match (start, end) {
        (Some(s), Some(e)) => Some(format!("{s}–{e}")),
        (Some(s), None) => Some(s),
        (None, Some(e)) => Some(format!("?–{e}")),
        (None, None) => None,
    }
}

fn adoption_detail(a: &AdoptionSub) -> Option<String> {
    let start = a.start().map(DateLit::format_year);
    let end = a.end().map(DateLit::format_year);
    match (start, end) {
        (Some(s), Some(e)) => Some(format!("{s}–{e}")),
        (Some(s), None) => Some(s),
        (None, Some(e)) => Some(format!("?–{e}")),
        (None, None) => None,
    }
}

fn display_name_or(_file: FileId, resolved: &ResolvedDocument, id: &str) -> String {
    resolved
        .person(id)
        .map(|p| p.display_name())
        .filter(|n| !n.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| id.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::test_open_file;

    fn symbols_for(source: &str) -> Vec<DocumentSymbol> {
        let doc = test_open_file(source);
        let v = doc.view();
        document_symbols(v.file, v.resolved, v.line_index)
    }

    fn names(syms: &[DocumentSymbol]) -> Vec<&str> {
        syms.iter().map(|s| s.name.as_str()).collect()
    }

    #[test]
    fn empty_document_yields_empty_outline() {
        assert!(symbols_for("").is_empty());
        assert!(symbols_for("kul 1\n").is_empty());
    }

    #[test]
    fn top_level_persons_use_display_name() {
        let src = "person alice name:\"Alice Sharma\" gender:female\n\
                   person bob name:\"Bob Sharma\" gender:male\n";
        let syms = symbols_for(src);
        assert_eq!(names(&syms), vec!["Alice Sharma", "Bob Sharma"]);
        assert_eq!(syms[0].kind, SymbolKind::VARIABLE);
    }

    #[test]
    fn person_without_name_falls_back_to_id() {
        let src = "person alice gender:female\n";
        let syms = symbols_for(src);
        assert_eq!(names(&syms), vec!["alice"]);
    }

    #[test]
    fn person_with_empty_name_literal_falls_back_to_id() {
        let src = "person yash name:\"\" gender:male\n";
        let syms = symbols_for(src);
        assert_eq!(names(&syms), vec!["yash"]);
    }

    #[test]
    fn person_with_whitespace_only_name_falls_back_to_id() {
        let src = "person yash name:\"   \" gender:male\n";
        let syms = symbols_for(src);
        assert_eq!(names(&syms), vec!["yash"]);
    }

    #[test]
    fn marriage_with_empty_spouse_names_falls_back_to_ids() {
        let src = "person a name:\"\" gender:female\n\
                   person b name:\"\" gender:male\n\
                   marriage m a b start:2010\n";
        let marriage = symbols_for(src)
            .into_iter()
            .find(|s| s.kind == SymbolKind::EVENT)
            .unwrap();
        assert_eq!(marriage.name, "a & b");
    }

    #[test]
    fn marriage_label_uses_spouse_display_names() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bob name:\"Bob\" gender:male\n\
                   marriage m alice bob start:2010\n";
        let syms = symbols_for(src);
        let marriage = syms.iter().find(|s| s.kind == SymbolKind::EVENT).unwrap();
        assert_eq!(marriage.name, "Alice & Bob");
    }

    #[test]
    fn marriage_label_falls_back_to_spouse_id_when_unresolved() {
        let src = "marriage m ghost b start:2010\nperson b name:\"Bob\" gender:male\n";
        let marriage = symbols_for(src)
            .into_iter()
            .find(|s| s.kind == SymbolKind::EVENT)
            .unwrap();
        assert_eq!(marriage.name, "ghost & Bob");
    }

    #[test]
    fn person_detail_combines_born_and_died() {
        let src = "person alice name:\"A\" gender:female born:1900 died:1990\n\
                   person bob name:\"B\" gender:male born:~1948\n\
                   person carol name:\"C\" gender:female died:2020\n\
                   person dan name:\"D\" gender:male\n";
        let syms = symbols_for(src);
        assert_eq!(syms[0].detail.as_deref(), Some("1900–1990"));
        assert_eq!(syms[1].detail.as_deref(), Some("b. ~1948"));
        assert_eq!(syms[2].detail.as_deref(), Some("d. 2020"));
        assert!(syms[3].detail.is_none());
    }

    #[test]
    fn marriage_detail_uses_year_span() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m1 a b start:2010 end:2020 end_reason:divorce\n\
                   marriage m2 a b start:2022\n";
        let marriages: Vec<_> = symbols_for(src)
            .into_iter()
            .filter(|s| s.kind == SymbolKind::EVENT)
            .collect();
        assert_eq!(marriages[0].detail.as_deref(), Some("2010–2020"));
        assert_eq!(marriages[1].detail.as_deref(), Some("2022"));
    }

    #[test]
    fn birth_nests_under_person() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  birth m\n";
        let syms = symbols_for(src);
        let kid = syms.iter().find(|s| s.name == "K").unwrap();
        let kids = kid.children.as_ref().expect("kid has children");
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].name, "birth m");
        assert_eq!(kids[0].kind, SymbolKind::FIELD);
    }

    #[test]
    fn adoption_nests_under_person_with_date_detail() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  adoption m start:2000 end:2010\n";
        let syms = symbols_for(src);
        let kid = syms.iter().find(|s| s.name == "K").unwrap();
        let kids = kid.children.as_ref().expect("kid has children");
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].name, "adoption m");
        assert_eq!(kids[0].detail.as_deref(), Some("2000–2010"));
    }

    #[test]
    fn person_with_no_substatements_has_no_children() {
        let src = "person alice name:\"Alice\" gender:female\n";
        let syms = symbols_for(src);
        assert!(syms[0].children.is_none());
    }

    #[test]
    fn selection_range_points_at_id_not_full_statement() {
        let src = "person alice name:\"Alice\" gender:female\n";
        let syms = symbols_for(src);
        let sel = syms[0].selection_range;
        assert_eq!(sel.start.line, 0);
        assert_eq!(sel.start.character, 7);
        assert_eq!(sel.end.character, 12);
    }

    #[test]
    fn person_range_extends_over_substatements() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  birth m\n";
        let syms = symbols_for(src);
        let kid = syms.iter().find(|s| s.name == "K").unwrap();
        assert!(kid.range.end.line > kid.range.start.line);
    }

    #[test]
    fn snapshot_divorce_and_remarriage() {
        let src = include_str!(
            "../../../../examples/03-divorce-and-remarriage/divorce-and-remarriage.kul"
        );
        let syms = symbols_for(src);
        insta::assert_json_snapshot!(syms);
    }

    #[test]
    fn snapshot_nuclear_family() {
        let src = include_str!("../../../../examples/01-nuclear-family/nuclear-family.kul");
        let syms = symbols_for(src);
        insta::assert_json_snapshot!(syms);
    }

    #[test]
    fn snapshot_three_generations() {
        let src = include_str!("../../../../examples/02-three-generations/three-generations.kul");
        let syms = symbols_for(src);
        insta::assert_json_snapshot!(syms);
    }

    #[test]
    fn snapshot_adoption_and_belonging() {
        let src = include_str!(
            "../../../../examples/04-adoption-and-belonging/adoption-and-belonging.kul"
        );
        let syms = symbols_for(src);
        insta::assert_json_snapshot!(syms);
    }
}
