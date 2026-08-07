//! LSP completion-item list assembly for each classifier [`Context`](super::classify::Context).

use kul_core::ast::{MarriageStmt, PersonStmt};
use kul_core::date::DateLit;
use kul_core::field_meta::{self, StatementKind, ValueKind};
use kul_core::lexer::{EnumKw, FieldName};
use kul_core::semantic::ResolvedDocument;
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat};

fn item(label: &str, kind: CompletionItemKind, detail: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_owned(),
        kind: Some(kind),
        detail: Some(detail.to_owned()),
        ..Default::default()
    }
}

pub(super) fn top_level_keywords() -> Vec<CompletionItem> {
    vec![
        item(
            "person",
            CompletionItemKind::KEYWORD,
            "Declare an individual (name, gender, dates)",
        ),
        item(
            "marriage",
            CompletionItemKind::KEYWORD,
            "Declare a marriage between two people",
        ),
    ]
}

pub(super) fn sub_statement_keywords() -> Vec<CompletionItem> {
    vec![
        item(
            "birth",
            CompletionItemKind::KEYWORD,
            "Link to biological parents (a marriage id)",
        ),
        item(
            "adoption",
            CompletionItemKind::KEYWORD,
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
/// String fields use a snippet that auto-wraps the value in quotes.
fn field_completions(kind: StatementKind, existing: &[FieldName]) -> Vec<CompletionItem> {
    field_meta::fields_for(kind)
        .iter()
        .copied()
        .filter(|name| !existing.contains(name))
        .map(|name| {
            let m = field_meta::meta(name);
            let label = format!("{}:", m.name.as_str());
            match m.value_kind {
                ValueKind::String => field_with_quoted_snippet(&label, m.short_doc),
                ValueKind::Date | ValueKind::Enum => {
                    item(&label, CompletionItemKind::FIELD, m.short_doc)
                }
            }
        })
        .collect()
}

/// Field-name item that inserts `name:"$0"` and lands the cursor between the quotes.
fn field_with_quoted_snippet(label: &str, doc: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_owned(),
        kind: Some(CompletionItemKind::FIELD),
        detail: Some(doc.to_owned()),
        insert_text: Some(format!("{label}\"$0\"")),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..Default::default()
    }
}

pub(super) fn gender_values() -> Vec<CompletionItem> {
    [
        (EnumKw::Male, "Male"),
        (EnumKw::Female, "Female"),
        (EnumKw::Other, "Non-binary or unspecified"),
    ]
    .iter()
    .map(|(kw, doc)| item(kw.as_str(), CompletionItemKind::ENUM_MEMBER, doc))
    .collect()
}

pub(super) fn end_reason_values() -> Vec<CompletionItem> {
    vec![item(
        EnumKw::Divorce.as_str(),
        CompletionItemKind::ENUM_MEMBER,
        "Marriage ended in divorce",
    )]
}

/// Preselected snippet that inserts `""` and lands the cursor between the quotes.
pub(super) fn quoted_value_snippet() -> Vec<CompletionItem> {
    vec![CompletionItem {
        label: "\"…\"".to_owned(),
        kind: Some(CompletionItemKind::VALUE),
        detail: Some("Quoted value (cursor between the quotes)".to_owned()),
        insert_text: Some(r#""$0""#.to_owned()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        preselect: Some(true),
        ..Default::default()
    }]
}

/// Every declared marriage in the project (ADR-0015), with spouse names
/// and date span in the detail so the user can disambiguate.
pub(super) fn marriage_id_items(resolved: &ResolvedDocument) -> Vec<CompletionItem> {
    resolved
        .marriages()
        .map(|m| CompletionItem {
            label: m.id.name.clone(),
            kind: Some(CompletionItemKind::EVENT),
            detail: Some(marriage_detail(resolved, m)),
            ..Default::default()
        })
        .collect()
}

/// Every declared person in the project (ADR-0015), filtered by `exclude`
/// so spouse_b's list excludes the person already named as spouse_a.
pub(super) fn person_id_items(
    resolved: &ResolvedDocument,
    exclude: Option<&str>,
) -> Vec<CompletionItem> {
    resolved
        .persons()
        .filter(|p| Some(p.id.name.as_str()) != exclude)
        .map(|p| CompletionItem {
            label: p.id.name.clone(),
            kind: Some(CompletionItemKind::VARIABLE),
            detail: Some(person_detail(p)),
            ..Default::default()
        })
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
