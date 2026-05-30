//! Single source of truth for the per-field taxonomy.
//!
//! Hover, completion, and semantic-token features all consume rows from
//! [`META`]. Adding a field: extend [`FieldName`] in the lexer, add a
//! [`META`] row, and add the variant to the relevant `*_FIELDS` slice
//! (plus the matching AST enum and parser).

use crate::lexer::FieldName;

/// The shape of a field's value as written in source. Drives syntax
/// highlighting and completion (e.g. whether to quote the insertion).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    /// `name:"Alice"`.
    String,
    /// `YYYY[-MM[-DD]]`, optionally `~`-prefixed.
    Date,
    /// Enum keyword (`gender`, `end_reason`).
    Enum,
}

/// Statement shape a field appears on. The same `FieldName` may appear
/// on multiple shapes (`start`/`end` on both marriage and adoption); the
/// `(StatementKind, FieldName)` pair selects a unique row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementKind {
    Person,
    Marriage,
    Adoption,
}

/// One row of the field taxonomy.
#[derive(Debug, Clone, Copy)]
pub struct FieldMeta {
    pub name: FieldName,
    pub value_kind: ValueKind,
    /// One-line description for completion-item details.
    pub short_doc: &'static str,
    /// Self-contained Markdown for hover popovers (includes the bold
    /// header and any examples).
    pub hover_md: &'static str,
}

/// Look up the metadata row for a field name. Total over [`FieldName`].
pub fn meta(name: FieldName) -> &'static FieldMeta {
    META.iter()
        .find(|m| m.name == name)
        .expect("every FieldName must have a row in META")
}

/// Fields valid on a statement shape, in canonical order.
pub fn fields_for(kind: StatementKind) -> &'static [FieldName] {
    match kind {
        StatementKind::Person => PERSON_FIELDS,
        StatementKind::Marriage => MARRIAGE_FIELDS,
        StatementKind::Adoption => ADOPTION_FIELDS,
    }
}

const PERSON_FIELDS: &[FieldName] = &[
    FieldName::Name,
    FieldName::Gender,
    FieldName::Family,
    FieldName::Given,
    FieldName::Born,
    FieldName::Died,
];

const MARRIAGE_FIELDS: &[FieldName] = &[FieldName::Start, FieldName::End, FieldName::EndReason];

const ADOPTION_FIELDS: &[FieldName] = &[FieldName::Start, FieldName::End];

const META: &[FieldMeta] = &[
    FieldMeta {
        name: FieldName::Name,
        value_kind: ValueKind::String,
        short_doc: "Full display name — required",
        hover_md: "**`name:`** — the person's full display name. Any text in double quotes.\n\nExample: `name:\"Alice Doe\"`",
    },
    FieldMeta {
        name: FieldName::Family,
        value_kind: ValueKind::String,
        short_doc: "Family name (last name)",
        hover_md: "**`family:`** — family name (last name / surname). Any text in double quotes. Optional.\n\nExample: `family:\"Doe\"`",
    },
    FieldMeta {
        name: FieldName::Given,
        value_kind: ValueKind::String,
        short_doc: "Given name (first name)",
        hover_md: "**`given:`** — given name (first name). Any text in double quotes. Optional.\n\nExample: `given:\"Alice\"`",
    },
    FieldMeta {
        name: FieldName::Gender,
        value_kind: ValueKind::Enum,
        short_doc: "male, female, or other — required",
        hover_md: "**`gender:`** — one of `male`, `female`, or `other`.",
    },
    FieldMeta {
        name: FieldName::Born,
        value_kind: ValueKind::Date,
        short_doc: "Date of birth (YYYY, YYYY-MM, or YYYY-MM-DD)",
        hover_md: "**`born:`** — date of birth. Use `YYYY`, `YYYY-MM`, or `YYYY-MM-DD`.\n\nPrefix with `~` for an approximate date (e.g. `~1980` means roughly 1975–1985).",
    },
    FieldMeta {
        name: FieldName::Died,
        value_kind: ValueKind::Date,
        short_doc: "Date of death — omit if the person is still alive",
        hover_md: "**`died:`** — date of death. Same formats as `born:`. Omit this field if the person is still alive.",
    },
    FieldMeta {
        name: FieldName::Start,
        value_kind: ValueKind::Date,
        short_doc: "Start date — required on marriages",
        hover_md: "**`start:`** — start date. On a marriage, the date the marriage began (required). On an adoption, the date the adoption took effect.\n\nUse `YYYY`, `YYYY-MM`, or `YYYY-MM-DD`. Prefix with `~` for an approximate date.",
    },
    FieldMeta {
        name: FieldName::End,
        value_kind: ValueKind::Date,
        short_doc: "End date — pair with end_reason: on marriages",
        hover_md: "**`end:`** — end date. On a marriage, the date the marriage ended (pair with `end_reason:`). On an adoption, the date the adoption ended.\n\nOmit when the marriage or adoption is still in effect.",
    },
    FieldMeta {
        name: FieldName::EndReason,
        value_kind: ValueKind::Enum,
        short_doc: "Why the marriage ended — currently only `divorce`",
        hover_md: "**`end_reason:`** — why the marriage ended. The only value in v1 is `divorce`. Must be paired with `end:`.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_field_name_has_a_row() {
        for name in [
            FieldName::Name,
            FieldName::Family,
            FieldName::Given,
            FieldName::Born,
            FieldName::Died,
            FieldName::Gender,
            FieldName::Start,
            FieldName::End,
            FieldName::EndReason,
        ] {
            let m = meta(name);
            assert_eq!(m.name, name);
            assert!(!m.short_doc.is_empty());
            assert!(!m.hover_md.is_empty());
        }
    }

    #[test]
    fn statement_field_lists_only_reference_valid_field_names() {
        for kind in [
            StatementKind::Person,
            StatementKind::Marriage,
            StatementKind::Adoption,
        ] {
            for &name in fields_for(kind) {
                let _ = meta(name);
            }
        }
    }

    #[test]
    fn person_fields_match_spec_field_order() {
        // Spec §15.2 fixes the canonical person-field order.
        assert_eq!(
            fields_for(StatementKind::Person),
            &[
                FieldName::Name,
                FieldName::Gender,
                FieldName::Family,
                FieldName::Given,
                FieldName::Born,
                FieldName::Died,
            ]
        );
    }
}
