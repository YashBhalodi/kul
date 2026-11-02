//! Completion-item list assembly for each classifier [`Context`](super::classify::Context).

use kul_core::ast::{MarriageStmt, PersonStmt};
use kul_core::date::DateLit;
use kul_core::field_meta::{self, StatementKind, ValueKind};
use kul_core::lexer::{EnumKw, FieldName};
use kul_core::semantic::ResolvedDocument;

use super::{CompletionItem, CompletionKind, Insertion};

fn item(label: &str, kind: CompletionKind, detail: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_owned(),
        kind,
        detail: detail.to_owned(),
        insert: None,
        preselect: false,
    }
}

pub(super) fn top_level_keywords() -> Vec<CompletionItem> {
    vec![
        item(
            "person",
            CompletionKind::Keyword,
            "Declare an individual (name, gender, dates)",
        ),
        item(
            "marriage",
            CompletionKind::Keyword,
            "Declare a marriage between two people",
        ),
    ]
}

pub(super) fn sub_statement_keywords() -> Vec<CompletionItem> {
    vec![
        item(
            "birth",
            CompletionKind::Keyword,
            "Link to biological parents (a marriage id)",
        ),
        item(
            "adoption",
            CompletionKind::Keyword,
            "Link to an adoptive marriage",
        ),
    ]
}

pub(super) fn person_fields(existing: &[FieldName]) -> Vec<CompletionItem> {
    field_completions(StatementKind::Person, existing)
}

pub(super) fn marriage_fields(existing: &[FieldName]) -> Vec<CompletionItem> {
    field_completions(StatementKind::Marriage, existing)
}

pub(super) fn adoption_fields(existing: &[FieldName]) -> Vec<CompletionItem> {
    field_completions(StatementKind::Adoption, existing)
}

/// Every valid field for the shape, minus the ones already on the line.
/// String fields insert `name:""` with the cursor between the quotes.
fn field_completions(kind: StatementKind, existing: &[FieldName]) -> Vec<CompletionItem> {
    field_meta::fields_for(kind)
        .iter()
        .copied()
        .filter(|name| !existing.contains(name))
        .map(|name| {
            let m = field_meta::meta(name);
            let label = format!("{}:", m.name.as_str());
            let mut field = item(&label, CompletionKind::Field, m.short_doc);
            if m.value_kind == ValueKind::String {
                field.insert = Some(Insertion {
                    text: format!("{label}\"\""),
                    cursor: label.len() + 1,
                });
            }
            field
        })
        .collect()
}

pub(super) fn gender_values() -> Vec<CompletionItem> {
    [
        (EnumKw::Male, "Male"),
        (EnumKw::Female, "Female"),
        (EnumKw::Other, "Non-binary or unspecified"),
    ]
    .iter()
    .map(|(kw, doc)| item(kw.as_str(), CompletionKind::EnumValue, doc))
    .collect()
}

pub(super) fn end_reason_values() -> Vec<CompletionItem> {
    vec![item(
        EnumKw::Divorce.as_str(),
        CompletionKind::EnumValue,
        "Marriage ended in divorce",
    )]
}

/// Preselected item that inserts `""` with the cursor between the quotes.
pub(super) fn quoted_value() -> Vec<CompletionItem> {
    vec![CompletionItem {
        insert: Some(Insertion {
            text: "\"\"".to_owned(),
            cursor: 1,
        }),
        preselect: true,
        ..item(
            "\"…\"",
            CompletionKind::Value,
            "Quoted value (cursor between the quotes)",
        )
    }]
}

/// Every declared marriage in the project (ADR-0015), with spouse names
/// and date span in the detail so the author can tell them apart.
pub(super) fn marriage_id_items(resolved: &ResolvedDocument) -> Vec<CompletionItem> {
    resolved
        .marriages()
        .map(|m| {
            item(
                &m.id.name,
                CompletionKind::Marriage,
                &marriage_detail(resolved, m),
            )
        })
        .collect()
}

/// Every declared person in the project (ADR-0015), filtered by `exclude`
/// so the second spouse's list leaves out the person named first.
pub(super) fn person_id_items(
    resolved: &ResolvedDocument,
    exclude: Option<&str>,
) -> Vec<CompletionItem> {
    resolved
        .persons()
        .filter(|p| Some(p.id.name.as_str()) != exclude)
        .map(|p| item(&p.id.name, CompletionKind::Person, &person_detail(p)))
        .collect()
}

fn marriage_detail(resolved: &ResolvedDocument, m: &MarriageStmt) -> String {
    let a = resolved
        .person(&m.spouse_a.name)
        .map(|p| p.display_name())
        .unwrap_or(m.spouse_a.name.as_str());
    let b = resolved
        .person(&m.spouse_b.name)
        .map(|p| p.display_name())
        .unwrap_or(m.spouse_b.name.as_str());
    let dates = match (
        m.start().map(DateLit::format_year),
        m.end().map(DateLit::format_year),
    ) {
        (Some(s), Some(e)) => format!(", {s}–{e}"),
        (Some(s), None) => format!(", {s}–"),
        (None, Some(e)) => format!(", ?–{e}"),
        (None, None) => String::new(),
    };
    format!("{a} + {b}{dates}")
}

fn person_detail(p: &PersonStmt) -> String {
    let name = p.display_name();
    match p.born().map(DateLit::format_year) {
        Some(b) => format!("{name}, b. {b}"),
        None => name.to_owned(),
    }
}
