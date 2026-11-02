//! Symbols: one file's outline.
//!
//! Labels prefer display names (the `name:` field; for marriages, the
//! spouses' display names) and fall back to ids. Always returns something,
//! even when the file has errors — a partial outline beats an empty one.

use kul_core::CheckResult;
use kul_core::ast::{AdoptionSub, BirthSub, MarriageStmt, PersonStmt, Statement};
use kul_core::date::DateLit;
use kul_core::semantic::ResolvedDocument;
use serde::Serialize;
#[cfg(feature = "tsify")]
use tsify::Tsify;

use crate::{Span, file_id};

/// What an [`OutlineSymbol`] stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub enum SymbolKind {
    /// A `person` declaration.
    Person,
    /// A `marriage` declaration.
    Marriage,
    /// A person's `birth` sub-statement.
    Birth,
    /// A person's `adoption` sub-statement.
    Adoption,
}

/// One outline entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct OutlineSymbol {
    /// The label: a display name, or the id when there is none.
    pub name: String,
    /// A short date summary (`1900–1990`, `b. 1950`), when there are dates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// What it stands for.
    pub kind: SymbolKind,
    /// The whole statement, sub-statements included.
    pub span: Span,
    /// The part to select when jumping to it: the id, or the keyword of a
    /// sub-statement.
    pub selection: Span,
    /// A person's `birth` and `adoption` sub-statements.
    pub children: Vec<OutlineSymbol>,
}

/// The outline of `file`, in source order. Empty for an unknown file.
#[must_use]
pub fn symbols(check: &CheckResult, file: &str) -> Vec<OutlineSymbol> {
    let Some(file) = file_id(check, file) else {
        return Vec::new();
    };
    let resolved = check.resolved();
    resolved
        .statements_in(file)
        .map(|stmt| match stmt {
            Statement::Person(p) => person_symbol(p),
            Statement::Marriage(m) => marriage_symbol(resolved, m),
        })
        .collect()
}

fn person_symbol(p: &PersonStmt) -> OutlineSymbol {
    // An outline entry needs a visible label, so an empty or whitespace-only
    // `name:""` reads as absent and the id stands in.
    let name = p
        .name()
        .map(|n| n.value.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| p.id.name.clone());
    let mut children = Vec::new();
    if let Some(birth) = &p.birth {
        children.push(birth_symbol(birth));
    }
    for adoption in &p.adoptions {
        children.push(adoption_symbol(adoption));
    }
    OutlineSymbol {
        name,
        detail: person_detail(p),
        kind: SymbolKind::Person,
        span: p.span.into(),
        selection: p.id.span.into(),
        children,
    }
}

fn marriage_symbol(resolved: &ResolvedDocument, m: &MarriageStmt) -> OutlineSymbol {
    let a = display_name_or(resolved, &m.spouse_a.name);
    let b = display_name_or(resolved, &m.spouse_b.name);
    OutlineSymbol {
        name: format!("{a} & {b}"),
        detail: date_range(m.start(), m.end()),
        kind: SymbolKind::Marriage,
        span: m.span.into(),
        selection: m.id.span.into(),
        children: Vec::new(),
    }
}

fn birth_symbol(b: &BirthSub) -> OutlineSymbol {
    OutlineSymbol {
        name: format!("birth {}", b.marriage_ref.name),
        detail: None,
        kind: SymbolKind::Birth,
        span: b.span.into(),
        selection: b.keyword_span.into(),
        children: Vec::new(),
    }
}

fn adoption_symbol(a: &AdoptionSub) -> OutlineSymbol {
    OutlineSymbol {
        name: format!("adoption {}", a.marriage_ref.name),
        detail: date_range(a.start(), a.end()),
        kind: SymbolKind::Adoption,
        span: a.span.into(),
        selection: a.keyword_span.into(),
        children: Vec::new(),
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

/// `start–end` for a marriage or adoption, `start` alone while it lasts.
fn date_range(start: Option<&DateLit>, end: Option<&DateLit>) -> Option<String> {
    match (
        start.map(DateLit::format_year),
        end.map(DateLit::format_year),
    ) {
        (Some(s), Some(e)) => Some(format!("{s}–{e}")),
        (Some(s), None) => Some(s),
        (None, Some(e)) => Some(format!("?–{e}")),
        (None, None) => None,
    }
}

fn display_name_or(resolved: &ResolvedDocument, id: &str) -> String {
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
    use crate::test_support::{FILE, check_one};

    fn symbols_for(source: &str) -> Vec<OutlineSymbol> {
        symbols(&check_one(source), FILE)
    }

    fn names(syms: &[OutlineSymbol]) -> Vec<&str> {
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
        assert_eq!(syms[0].kind, SymbolKind::Person);
    }

    #[test]
    fn person_without_name_falls_back_to_id() {
        let src = "person alice gender:female\n";
        assert_eq!(names(&symbols_for(src)), vec!["alice"]);
    }

    #[test]
    fn person_with_empty_name_literal_falls_back_to_id() {
        let src = "person yash name:\"\" gender:male\n";
        assert_eq!(names(&symbols_for(src)), vec!["yash"]);
    }

    #[test]
    fn person_with_whitespace_only_name_falls_back_to_id() {
        let src = "person yash name:\"   \" gender:male\n";
        assert_eq!(names(&symbols_for(src)), vec!["yash"]);
    }

    #[test]
    fn marriage_with_empty_spouse_names_falls_back_to_ids() {
        let src = "person a name:\"\" gender:female\n\
                   person b name:\"\" gender:male\n\
                   marriage m a b start:2010\n";
        let marriage = symbols_for(src)
            .into_iter()
            .find(|s| s.kind == SymbolKind::Marriage)
            .unwrap();
        assert_eq!(marriage.name, "a & b");
    }

    #[test]
    fn marriage_label_uses_spouse_display_names() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bob name:\"Bob\" gender:male\n\
                   marriage m alice bob start:2010\n";
        let syms = symbols_for(src);
        let marriage = syms
            .iter()
            .find(|s| s.kind == SymbolKind::Marriage)
            .unwrap();
        assert_eq!(marriage.name, "Alice & Bob");
    }

    #[test]
    fn marriage_label_falls_back_to_spouse_id_when_unresolved() {
        let src = "marriage m ghost b start:2010\nperson b name:\"Bob\" gender:male\n";
        let marriage = symbols_for(src)
            .into_iter()
            .find(|s| s.kind == SymbolKind::Marriage)
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
            .filter(|s| s.kind == SymbolKind::Marriage)
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
        assert_eq!(kid.children.len(), 1);
        assert_eq!(kid.children[0].name, "birth m");
        assert_eq!(kid.children[0].kind, SymbolKind::Birth);
    }

    #[test]
    fn adoption_nests_under_person_with_date_detail() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  adoption m start:2000 end:2010\n";
        let syms = symbols_for(src);
        let kid = syms.iter().find(|s| s.name == "K").unwrap();
        assert_eq!(kid.children.len(), 1);
        assert_eq!(kid.children[0].name, "adoption m");
        assert_eq!(kid.children[0].kind, SymbolKind::Adoption);
        assert_eq!(kid.children[0].detail.as_deref(), Some("2000–2010"));
    }

    #[test]
    fn person_with_no_substatements_has_no_children() {
        let src = "person alice name:\"Alice\" gender:female\n";
        assert!(symbols_for(src)[0].children.is_empty());
    }

    #[test]
    fn selection_points_at_id_not_full_statement() {
        let src = "person alice name:\"Alice\" gender:female\n";
        assert_eq!(symbols_for(src)[0].selection, Span { start: 7, end: 12 });
    }

    #[test]
    fn person_span_extends_over_substatements() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  birth m\n";
        let syms = symbols_for(src);
        let kid = syms.iter().find(|s| s.name == "K").unwrap();
        assert!(src[kid.span.start..kid.span.end].contains("birth m"));
    }

    #[test]
    fn snapshot_divorce_and_remarriage() {
        let src =
            include_str!("../../../examples/03-divorce-and-remarriage/divorce-and-remarriage.kul");
        insta::assert_json_snapshot!(symbols_for(src));
    }

    #[test]
    fn snapshot_nuclear_family() {
        let src = include_str!("../../../examples/01-nuclear-family/nuclear-family.kul");
        insta::assert_json_snapshot!(symbols_for(src));
    }

    #[test]
    fn snapshot_three_generations() {
        let src = include_str!("../../../examples/02-three-generations/three-generations.kul");
        insta::assert_json_snapshot!(symbols_for(src));
    }

    #[test]
    fn snapshot_adoption_and_belonging() {
        let src =
            include_str!("../../../examples/04-adoption-and-belonging/adoption-and-belonging.kul");
        insta::assert_json_snapshot!(symbols_for(src));
    }
}
