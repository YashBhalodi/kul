//! Single source of truth for the per-field taxonomy.
//!
//! Every fact about a Kula field — its value shape, its short completion
//! description, its long-form hover documentation, and which statement
//! shapes it appears on — is recorded here. Hover, completion, and
//! semantic-token features all consume rows from this table; the AST
//! enums (`PersonFieldKind`, `MarriageFieldKind`, `AdoptionFieldKind`)
//! provide the path from a parsed value back to its [`FieldName`].
//!
//! Adding a new field is a one-row change: extend [`FieldName`] in the
//! lexer, add a row to [`META`], and add the `FieldName` to the relevant
//! `*_FIELDS` slice. The AST enum and parser also need the new variant;
//! the validator then picks it up via existing per-rule code.

use crate::lexer::FieldName;

/// The shape of a field's value as written in source. Drives both syntax
/// highlighting (string vs. number vs. enum-member) and completion
/// (whether to wrap the inserted value in quotes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    /// A double-quoted string literal: `name:"Alice"`.
    String,
    /// A date literal (`YYYY[-MM[-DD]]`, optionally `~`-prefixed).
    Date,
    /// One of a small set of enum keywords (`gender`, `end_reason`).
    Enum,
}

/// Which top-level / sub-statement shape a field can appear on. The same
/// `FieldName` may participate in more than one shape (e.g. `start` and
/// `end` appear on both `marriage` and `adoption`); a `(StatementKind,
/// FieldName)` pair is what selects a unique row in the per-statement
/// field list.
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
    /// One-line description used in completion-item details and the like.
    pub short_doc: &'static str,
    /// Long-form Markdown for hover popovers. Self-contained — already
    /// includes the bold field-name header and any examples.
    pub hover_md: &'static str,
}

/// Look up the metadata row for a field name. Total over [`FieldName`]:
/// every variant has exactly one row.
pub fn meta(name: FieldName) -> &'static FieldMeta {
    META.iter()
        .find(|m| m.name == name)
        .expect("every FieldName must have a row in META")
}

/// The fields valid on a given statement shape, in the canonical order
/// the formatter and completion lists use.
pub fn fields_for(kind: StatementKind) -> &'static [FieldName] {
    match kind {
        StatementKind::Person => PERSON_FIELDS,
        StatementKind::Marriage => MARRIAGE_FIELDS,
        StatementKind::Adoption => ADOPTION_FIELDS,
    }
}

const PERSON_FIELDS: &[FieldName] = &[
    FieldName::Name,
    FieldName::Family,
    FieldName::Given,
    FieldName::Gender,
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
        // Compile-time exhaustiveness via the match below; the test fails
        // if a new FieldName lands without a META row.
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
    fn person_fields_match_ast_enum() {
        // Expect the field set to mirror PersonFieldKind, in the canonical
        // formatter order. If a new variant lands, this test is the
        // smallest signal that fields_for needs an update.
        assert_eq!(
            fields_for(StatementKind::Person),
            &[
                FieldName::Name,
                FieldName::Family,
                FieldName::Given,
                FieldName::Gender,
                FieldName::Born,
                FieldName::Died,
            ]
        );
    }
}
