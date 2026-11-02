//! Completion: what can be typed at the cursor.
//!
//! Token-stream-first classifier (ADR-0002): the cursor usually sits in
//! whitespace or a partial token where there's no clean syntax-tree node, so
//! the classifier walks tokens up to the cursor and sorts the position into a
//! fixed set of contexts; each context has its own item list.

mod classify;
mod items;

use classify::{Context, classify};
use kul_core::CheckResult;
use kul_core::lexer::tokenize;
use serde::Serialize;
#[cfg(feature = "tsify")]
use tsify::Tsify;

use crate::{file_id, source_of};

/// What a [`CompletionItem`] inserts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub enum CompletionKind {
    /// `person`, `marriage`, `birth`, `adoption`.
    Keyword,
    /// A field name with its colon: `name:`, `born:`, …
    Field,
    /// An enumerated value: `female`, `divorce`, …
    EnumValue,
    /// A value shape, such as an empty quoted string.
    Value,
    /// A declared marriage's id.
    Marriage,
    /// A declared person's id.
    Person,
}

/// Text to insert in place of an item's label, with where the cursor lands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct Insertion {
    /// The text.
    pub text: String,
    /// Offset into `text` where the cursor goes after inserting it.
    pub cursor: usize,
}

/// One completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct CompletionItem {
    /// What the list shows, and what is inserted when `insert` is absent.
    pub label: String,
    /// What it inserts.
    pub kind: CompletionKind,
    /// A one-line description (a field's meaning, a person's name and birth
    /// year, a marriage's spouses and years).
    pub detail: String,
    /// Text to insert instead of the label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert: Option<Insertion>,
    /// Whether the list should select this item first.
    pub preselect: bool,
}

/// The completions at `offset` in `file`. The full set for the context: the
/// consumer filters by the partial word already typed.
#[must_use]
pub fn complete(check: &CheckResult, file: &str, offset: usize) -> Vec<CompletionItem> {
    let Some(file) = file_id(check, file) else {
        return Vec::new();
    };
    let Some(source) = source_of(check, file) else {
        return Vec::new();
    };
    let resolved = check.resolved();
    let tokens = tokenize(source);
    match classify(source, file, &tokens, resolved, offset) {
        Context::None => Vec::new(),
        Context::TopLevelStart => items::top_level_keywords(),
        Context::IndentedUnderPerson => items::sub_statement_keywords(),
        Context::PersonFieldList { existing } => items::person_fields(&existing),
        Context::MarriageFieldList { existing } => items::marriage_fields(&existing),
        Context::AdoptionFieldList { existing } => items::adoption_fields(&existing),
        Context::AfterGenderColon => items::gender_values(),
        Context::AfterEndReasonColon => items::end_reason_values(),
        Context::AfterStringFieldColon => items::quoted_value(),
        Context::MarriageRefPosition => items::marriage_id_items(resolved),
        Context::SpousePosition { exclude } => items::person_id_items(resolved, exclude.as_deref()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FILE, check_one, cursor_fixture};

    fn complete_for(src_with_marker: &str) -> Vec<CompletionItem> {
        let (source, offset) = cursor_fixture(src_with_marker);
        complete(&check_one(&source), FILE, offset)
    }

    fn run(src_with_marker: &str) -> Vec<String> {
        complete_for(src_with_marker)
            .into_iter()
            .map(|c| c.label)
            .collect()
    }

    #[test]
    fn top_level_keyword_labels_and_order() {
        assert_eq!(
            run("<CURSOR>"),
            vec!["person".to_owned(), "marriage".into()]
        );
    }

    #[test]
    fn indented_under_person_offers_sub_keywords() {
        let src = "person a name:\"A\" gender:female\n  <CURSOR>";
        assert_eq!(run(src), vec!["birth".to_owned(), "adoption".into()]);
    }

    #[test]
    fn person_field_list_after_id() {
        let labels = run("person a <CURSOR>");
        assert!(labels.contains(&"name:".to_owned()));
        assert!(labels.contains(&"gender:".to_owned()));
        assert!(labels.contains(&"born:".to_owned()));
    }

    #[test]
    fn person_field_list_filters_present_fields() {
        let labels = run("person a name:\"A\" gender:female <CURSOR>");
        assert!(!labels.contains(&"name:".to_owned()));
        assert!(!labels.contains(&"gender:".to_owned()));
        assert!(labels.contains(&"family:".to_owned()));
        assert!(labels.contains(&"born:".to_owned()));
    }

    #[test]
    fn marriage_field_list_after_spouses() {
        let labels = run("marriage m a b <CURSOR>");
        assert!(labels.contains(&"start:".to_owned()));
        assert!(labels.contains(&"end:".to_owned()));
        assert!(labels.contains(&"end_reason:".to_owned()));
    }

    #[test]
    fn marriage_field_list_filters_present() {
        let labels = run("marriage m a b start:2010 <CURSOR>");
        assert!(!labels.contains(&"start:".to_owned()));
        assert!(labels.contains(&"end:".to_owned()));
        assert!(labels.contains(&"end_reason:".to_owned()));
    }

    #[test]
    fn after_gender_colon_offers_enum_values() {
        assert_eq!(
            run("person a name:\"A\" gender:<CURSOR>"),
            vec!["male".to_owned(), "female".into(), "other".into()]
        );
    }

    #[test]
    fn after_end_reason_colon_offers_divorce() {
        let src = "marriage m a b start:2010 end:2020 end_reason:<CURSOR>";
        assert_eq!(run(src), vec!["divorce".to_owned()]);
    }

    #[test]
    fn person_field_completion_wraps_string_values_in_quotes() {
        let items = complete_for("person a <CURSOR>");

        for field in ["name:", "family:", "given:"] {
            let it = items
                .iter()
                .find(|c| c.label == field)
                .unwrap_or_else(|| panic!("missing item `{field}` in {items:#?}"));
            assert_eq!(
                it.insert,
                Some(Insertion {
                    text: format!("{field}\"\""),
                    cursor: field.len() + 1,
                }),
                "string field `{field}` should insert its quotes with the cursor between them"
            );
        }

        for field in ["born:", "died:", "gender:"] {
            let it = items
                .iter()
                .find(|c| c.label == field)
                .unwrap_or_else(|| panic!("missing item `{field}`"));
            assert!(
                it.insert.is_none(),
                "non-string field `{field}` should insert its label; got: {it:#?}"
            );
        }
    }

    #[test]
    fn after_string_field_colon_offers_quoted_value() {
        for field in ["name", "family", "given"] {
            let items = complete_for(&format!("person a {field}:<CURSOR>"));
            assert_eq!(
                items.len(),
                1,
                "expected one item for `{field}:`; got: {items:#?}"
            );
            let item = &items[0];
            assert_eq!(
                item.insert,
                Some(Insertion {
                    text: "\"\"".to_owned(),
                    cursor: 1
                }),
                "should insert quotes with the cursor between them for `{field}:`"
            );
            assert!(
                item.preselect,
                "the item should be preselected so a single Tab accepts it"
            );
        }
    }

    #[test]
    fn adoption_field_list() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  adoption m <CURSOR>";
        let labels = run(src);
        assert!(labels.contains(&"start:".to_owned()));
        assert!(labels.contains(&"end:".to_owned()));
    }

    #[test]
    fn adoption_field_list_filters_present() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  adoption m start:2010 <CURSOR>";
        let labels = run(src);
        assert!(!labels.contains(&"start:".to_owned()));
        assert!(labels.contains(&"end:".to_owned()));
    }

    fn run_with_details(src_with_marker: &str) -> Vec<(String, String)> {
        complete_for(src_with_marker)
            .into_iter()
            .map(|c| (c.label, c.detail))
            .collect()
    }

    fn run_kinds(src_with_marker: &str) -> Vec<CompletionKind> {
        complete_for(src_with_marker)
            .into_iter()
            .map(|c| c.kind)
            .collect()
    }

    #[test]
    fn after_birth_offers_marriage_ids_with_spouse_detail() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bob name:\"Bob\" gender:male\n\
                   marriage m1 alice bob start:1972 end:1990 end_reason:divorce\n\
                   marriage m2 alice bob start:2000\n\
                   person kid name:\"K\" gender:other\n  birth <CURSOR>";
        assert_eq!(
            run_with_details(src),
            vec![
                ("m1".to_owned(), "Alice + Bob, 1972–1990".to_owned()),
                ("m2".to_owned(), "Alice + Bob, 2000–".to_owned()),
            ]
        );
    }

    #[test]
    fn after_birth_with_partial_id_still_offers_marriages() {
        // Cursor adjacent to a partial id — the consumer wants the full set
        // and filters by prefix itself.
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m_alice_bob alice bob start:1972\n\
                   person kid name:\"K\" gender:other\n  birth m_<CURSOR>";
        assert_eq!(run(src), vec!["m_alice_bob".to_owned()]);
    }

    #[test]
    fn after_adoption_offers_marriage_ids() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bob name:\"Bob\" gender:male\n\
                   marriage m alice bob start:1972\n\
                   person kid name:\"K\" gender:other\n  adoption <CURSOR>";
        assert_eq!(run(src), vec!["m".to_owned()]);
    }

    #[test]
    fn adoption_after_marriage_ref_still_offers_fields() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:1972\n\
                   person kid name:\"K\" gender:other\n  adoption m <CURSOR>";
        let labels = run(src);
        assert!(labels.contains(&"start:".to_owned()));
        assert!(labels.contains(&"end:".to_owned()));
        assert!(!labels.contains(&"m".to_owned()));
    }

    #[test]
    fn after_marriage_id_offers_persons_for_spouse_a() {
        let src = "person alice name:\"Alice\" gender:female born:1950\n\
                   person bob name:\"Bob\" gender:male born:1948\n\
                   marriage m <CURSOR>";
        assert_eq!(
            run_with_details(src),
            vec![
                ("alice".to_owned(), "Alice, b. 1950".to_owned()),
                ("bob".to_owned(), "Bob, b. 1948".to_owned()),
            ]
        );
    }

    #[test]
    fn after_spouse_a_excludes_self_marriage() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   person carol name:\"C\" gender:female\n\
                   marriage m alice <CURSOR>";
        let labels = run(src);
        assert!(!labels.contains(&"alice".to_owned()));
        assert!(labels.contains(&"bob".to_owned()));
        assert!(labels.contains(&"carol".to_owned()));
    }

    #[test]
    fn after_both_spouses_falls_through_to_marriage_fields() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b <CURSOR>";
        let labels = run(src);
        assert!(labels.contains(&"start:".to_owned()));
        assert!(labels.contains(&"end:".to_owned()));
    }

    #[test]
    fn marriage_ids_complete_as_marriages() {
        let src = "person alice name:\"A\" gender:female\n\
                   marriage m alice alice start:1972\n\
                   person kid name:\"K\" gender:other\n  birth <CURSOR>";
        assert_eq!(run_kinds(src), vec![CompletionKind::Marriage]);
    }

    #[test]
    fn person_ids_complete_as_persons() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m <CURSOR>";
        assert_eq!(
            run_kinds(src),
            vec![CompletionKind::Person, CompletionKind::Person]
        );
    }

    #[test]
    fn unknown_file_offers_nothing() {
        assert!(complete(&check_one(""), "other.kul", 0).is_empty());
    }

    #[test]
    fn snapshot_after_birth_keyword() {
        insta::assert_json_snapshot!(complete_for(
            "person alice name:\"Alice\" gender:female born:1950\n\
             person bob name:\"Bob\" gender:male born:1948\n\
             marriage m_alice_bob alice bob start:1972 end:1990 end_reason:divorce\n\
             person kid name:\"K\" gender:other\n  birth <CURSOR>",
        ));
    }

    #[test]
    fn snapshot_after_marriage_id_for_spouse_a() {
        insta::assert_json_snapshot!(complete_for(
            "person alice name:\"Alice\" gender:female born:1950\n\
             person bob name:\"Bob\" gender:male born:1948\n\
             marriage m <CURSOR>",
        ));
    }

    #[test]
    fn snapshot_after_spouse_a_excludes_self() {
        insta::assert_json_snapshot!(complete_for(
            "person alice name:\"Alice\" gender:female born:1950\n\
             person bob name:\"Bob\" gender:male born:1948\n\
             person carol name:\"Carol\" gender:female born:1975\n\
             marriage m alice <CURSOR>",
        ));
    }

    #[test]
    fn snapshot_top_level() {
        insta::assert_json_snapshot!(complete_for("<CURSOR>"));
    }

    #[test]
    fn snapshot_after_gender_colon() {
        insta::assert_json_snapshot!(complete_for("person a name:\"A\" gender:<CURSOR>"));
    }

    #[test]
    fn snapshot_person_fields_with_insertions() {
        insta::assert_json_snapshot!(complete_for("person a <CURSOR>"));
    }
}
