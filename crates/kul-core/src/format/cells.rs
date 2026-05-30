//! Cell grammar: typed line atoms, canonical column tables, and the
//! AST-to-cell builders consumed by [`super::emit::Emitter`].
//!
//! A line is a sparse sequence of [`Cell`]s, each tagged with the
//! canonical column index for its statement kind. Field ordering flows
//! from [`crate::field_meta::fields_for`] so the formatter and completion
//! cannot disagree.

use std::sync::LazyLock;

use crate::ast::{
    AdoptionFieldKind, AdoptionSub, BirthSub, EndReason, Gender, MarriageFieldKind, MarriageStmt,
    PersonFieldKind, PersonStmt,
};
use crate::field_meta::{self, StatementKind};
use crate::lexer::FieldName;

#[derive(Debug, Clone)]
pub(super) struct Cell {
    pub(super) text: String,
    /// Canonical column index (see [`canonical_columns`]); the cell's
    /// `CellKind` is implicit at `canonical_columns(kind)[col]`.
    pub(super) col: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum CellKind {
    /// `person`, `marriage`, `birth`, `adoption`.
    Keyword,
    /// Bound id of `person`/`marriage`. Single space after.
    Positional,
    /// Reference positional (marriage spouses, birth/adoption parent).
    /// Single space after.
    Reference,
    /// `name:value` field. Two spaces after.
    Field(FieldName),
    /// Trailing inline comment.
    Comment,
}

/// Statement kind of a line; drives the canonical column ordering and the
/// alignment-group key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum KindTag {
    Person,
    Marriage,
    Birth,
    Adoption,
}

/// Canonical column kinds for a statement kind, in order. Field columns
/// flow from [`field_meta::fields_for`] (spec §15.2); structural cells
/// (keyword, positional, references, comment) are formatter-only.
pub(super) fn canonical_columns(kind: KindTag) -> &'static [CellKind] {
    static PERSON: LazyLock<Vec<CellKind>> = LazyLock::new(|| build_columns(KindTag::Person));
    static MARRIAGE: LazyLock<Vec<CellKind>> = LazyLock::new(|| build_columns(KindTag::Marriage));
    static BIRTH: LazyLock<Vec<CellKind>> = LazyLock::new(|| build_columns(KindTag::Birth));
    static ADOPTION: LazyLock<Vec<CellKind>> = LazyLock::new(|| build_columns(KindTag::Adoption));
    match kind {
        KindTag::Person => PERSON.as_slice(),
        KindTag::Marriage => MARRIAGE.as_slice(),
        KindTag::Birth => BIRTH.as_slice(),
        KindTag::Adoption => ADOPTION.as_slice(),
    }
}

/// Structural cells preceding the field columns. Same-keyword lines
/// always carry every prefix cell.
fn structural_prefix(kind: KindTag) -> &'static [CellKind] {
    use CellKind::*;
    match kind {
        KindTag::Person => &[Keyword, Positional],
        KindTag::Marriage => &[Keyword, Positional, Reference, Reference],
        KindTag::Birth => &[Keyword, Reference],
        KindTag::Adoption => &[Keyword, Reference],
    }
}

/// `StatementKind` for line shapes that carry fields. `Birth` has none.
fn statement_kind(kind: KindTag) -> Option<StatementKind> {
    match kind {
        KindTag::Person => Some(StatementKind::Person),
        KindTag::Marriage => Some(StatementKind::Marriage),
        KindTag::Adoption => Some(StatementKind::Adoption),
        KindTag::Birth => None,
    }
}

fn build_columns(kind: KindTag) -> Vec<CellKind> {
    let mut out: Vec<CellKind> = structural_prefix(kind).to_vec();
    if let Some(stmt_kind) = statement_kind(kind) {
        for &name in field_meta::fields_for(stmt_kind) {
            out.push(CellKind::Field(name));
        }
    }
    out.push(CellKind::Comment);
    out
}

/// Column index of `name` in `kind`'s canonical layout. Panics on
/// invalid combinations (callers are field-kind-typed, so a mismatch is
/// a programmer error).
fn field_column(kind: KindTag, name: FieldName) -> u8 {
    canonical_columns(kind)
        .iter()
        .position(|c| matches!(c, CellKind::Field(n) if *n == name))
        .expect("field is part of this kind's canonical columns") as u8
}

/// Column index of the trailing comment cell (always last).
fn comment_column(kind: KindTag) -> u8 {
    (canonical_columns(kind).len() - 1) as u8
}

/// Alignment-group key. Two lines align iff their `GroupKey`s match.
/// `parent` is `Some(id)` for sub-statements so siblings under different
/// parents never cross-align.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct GroupKey {
    pub(super) indent: usize,
    pub(super) kind: KindTag,
    pub(super) parent: Option<u32>,
}

#[derive(Debug, Clone)]
pub(super) enum RegionItem {
    /// A line that participates in an alignment group.
    Aligned {
        indent: usize,
        cells: Vec<Cell>,
        group: GroupKey,
    },
    /// A whole-line comment. Never aligned.
    Comment { indent: usize, text: String },
}

pub(super) fn build_person_cells(p: &PersonStmt, inline_comment: Option<&str>) -> Vec<Cell> {
    let kind = KindTag::Person;
    let mut cells = Vec::with_capacity(canonical_columns(kind).len());
    cells.push(Cell {
        text: "person".to_string(),
        col: 0,
    });
    cells.push(Cell {
        text: p.id.name.clone(),
        col: 1,
    });
    if let Some(s) = p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Name(s) => Some(s),
        _ => None,
    }) {
        cells.push(field_cell(kind, FieldName::Name, &quote_string(&s.value)));
    }
    if let Some(g) = p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Gender(g) => Some(g),
        _ => None,
    }) {
        cells.push(field_cell(kind, FieldName::Gender, gender_str(g.value)));
    }
    if let Some(s) = p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Family(s) => Some(s),
        _ => None,
    }) {
        cells.push(field_cell(kind, FieldName::Family, &quote_string(&s.value)));
    }
    if let Some(s) = p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Given(s) => Some(s),
        _ => None,
    }) {
        cells.push(field_cell(kind, FieldName::Given, &quote_string(&s.value)));
    }
    if let Some(d) = p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Born(d) => Some(d),
        _ => None,
    }) {
        cells.push(field_cell(kind, FieldName::Born, &d.format_canonical()));
    }
    if let Some(d) = p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Died(d) => Some(d),
        _ => None,
    }) {
        cells.push(field_cell(kind, FieldName::Died, &d.format_canonical()));
    }
    if let Some(text) = inline_comment {
        cells.push(Cell {
            text: text.to_string(),
            col: comment_column(kind),
        });
    }
    cells
}

pub(super) fn build_marriage_cells(m: &MarriageStmt, inline_comment: Option<&str>) -> Vec<Cell> {
    let kind = KindTag::Marriage;
    let mut cells = Vec::with_capacity(canonical_columns(kind).len());
    cells.push(Cell {
        text: "marriage".to_string(),
        col: 0,
    });
    cells.push(Cell {
        text: m.id.name.clone(),
        col: 1,
    });
    cells.push(Cell {
        text: m.spouse_a.name.clone(),
        col: 2,
    });
    cells.push(Cell {
        text: m.spouse_b.name.clone(),
        col: 3,
    });
    if let Some(d) = m.fields.iter().find_map(|f| match &f.kind {
        MarriageFieldKind::Start(d) => Some(d),
        _ => None,
    }) {
        cells.push(field_cell(kind, FieldName::Start, &d.format_canonical()));
    }
    if let Some(d) = m.fields.iter().find_map(|f| match &f.kind {
        MarriageFieldKind::End(d) => Some(d),
        _ => None,
    }) {
        cells.push(field_cell(kind, FieldName::End, &d.format_canonical()));
    }
    if let Some(er) = m.fields.iter().find_map(|f| match &f.kind {
        MarriageFieldKind::EndReason(v) => Some(v),
        _ => None,
    }) {
        cells.push(field_cell(
            kind,
            FieldName::EndReason,
            &end_reason_str(&er.value),
        ));
    }
    if let Some(text) = inline_comment {
        cells.push(Cell {
            text: text.to_string(),
            col: comment_column(kind),
        });
    }
    cells
}

pub(super) enum SubRef<'a> {
    Birth(&'a BirthSub),
    Adoption(&'a AdoptionSub),
}

impl SubRef<'_> {
    pub(super) fn span_start(&self) -> usize {
        match self {
            SubRef::Birth(b) => b.span.start,
            SubRef::Adoption(a) => a.span.start,
        }
    }
}

pub(super) fn collect_sub_refs(p: &PersonStmt) -> Vec<SubRef<'_>> {
    let mut subs: Vec<SubRef> = Vec::new();
    if let Some(b) = &p.birth {
        subs.push(SubRef::Birth(b));
    }
    for a in &p.adoptions {
        subs.push(SubRef::Adoption(a));
    }
    subs.sort_by_key(|s| s.span_start());
    subs
}

pub(super) fn build_sub_cells(
    sub: &SubRef<'_>,
    inline_comment: Option<&str>,
) -> (KindTag, Vec<Cell>) {
    match sub {
        SubRef::Birth(b) => {
            let kind = KindTag::Birth;
            let mut cells = vec![
                Cell {
                    text: "birth".to_string(),
                    col: 0,
                },
                Cell {
                    text: b.marriage_ref.name.clone(),
                    col: 1,
                },
            ];
            if let Some(text) = inline_comment {
                cells.push(Cell {
                    text: text.to_string(),
                    col: comment_column(kind),
                });
            }
            (kind, cells)
        }
        SubRef::Adoption(a) => {
            let kind = KindTag::Adoption;
            let mut cells = vec![
                Cell {
                    text: "adoption".to_string(),
                    col: 0,
                },
                Cell {
                    text: a.marriage_ref.name.clone(),
                    col: 1,
                },
            ];
            if let Some(d) = a.fields.iter().find_map(|f| match &f.kind {
                AdoptionFieldKind::Start(d) => Some(d),
                _ => None,
            }) {
                cells.push(field_cell(kind, FieldName::Start, &d.format_canonical()));
            }
            if let Some(d) = a.fields.iter().find_map(|f| match &f.kind {
                AdoptionFieldKind::End(d) => Some(d),
                _ => None,
            }) {
                cells.push(field_cell(kind, FieldName::End, &d.format_canonical()));
            }
            if let Some(text) = inline_comment {
                cells.push(Cell {
                    text: text.to_string(),
                    col: comment_column(kind),
                });
            }
            (kind, cells)
        }
    }
}

fn field_cell(kind: KindTag, name: FieldName, value: &str) -> Cell {
    Cell {
        text: format!("{}:{}", name.as_str(), value),
        col: field_column(kind, name),
    }
}

fn quote_string(value: &str) -> String {
    let mut s = String::with_capacity(value.len() + 2);
    s.push('"');
    for c in value.chars() {
        match c {
            '\\' => s.push_str("\\\\"),
            '"' => s.push_str("\\\""),
            other => s.push(other),
        }
    }
    s.push('"');
    s
}

fn gender_str(g: Gender) -> &'static str {
    match g {
        Gender::Male => "male",
        Gender::Female => "female",
        Gender::Other => "other",
    }
}

fn end_reason_str(e: &EndReason) -> String {
    match e {
        EndReason::Divorce => "divorce".to_string(),
        // Re-emit verbatim so KUL-R05b's anchor stays meaningful.
        EndReason::Unknown(s) => s.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(value: &str) -> String {
        value.to_owned()
    }

    #[test]
    fn quote_string_escapes_backslash_and_quote() {
        assert_eq!(quote_string(""), "\"\"");
        assert_eq!(quote_string("Alice"), "\"Alice\"");
        assert_eq!(quote_string("O\"Brien"), "\"O\\\"Brien\"");
        assert_eq!(quote_string("a\\b"), "\"a\\\\b\"");
    }

    #[test]
    fn gender_str_round_trip() {
        assert_eq!(gender_str(Gender::Male), "male");
        assert_eq!(gender_str(Gender::Female), "female");
        assert_eq!(gender_str(Gender::Other), "other");
    }

    #[test]
    fn end_reason_divorce() {
        assert_eq!(end_reason_str(&EndReason::Divorce), "divorce");
        assert_eq!(end_reason_str(&EndReason::Unknown(s("oops"))), "oops");
    }
}
