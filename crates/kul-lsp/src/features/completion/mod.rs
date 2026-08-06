//! Completion for `textDocument/completion`.
//!
//! Token-stream-first classifier (ADR-0002): the cursor often sits in
//! whitespace or a partial token where there's no clean AST node, so we
//! walk tokens up to the cursor and classify into a fixed set of contexts.

mod classify;
mod items;

use classify::{Context, classify};
use kul_core::lexer::tokenize;
use kul_core::semantic::ResolvedDocument;
use kul_core::span::FileId;
use tower_lsp::lsp_types::CompletionItem;

/// Build a completion item list for the cursor at `byte_offset`.
pub fn complete(
    source: &str,
    file: FileId,
    resolved: &ResolvedDocument,
    byte_offset: usize,
) -> Vec<CompletionItem> {
    let tokens = tokenize(source);
    match classify(source, file, &tokens, resolved, byte_offset) {
        Context::None => Vec::new(),
        Context::TopLevelStart => items::top_level_keywords(),
        Context::IndentedUnderPerson => items::sub_statement_keywords(),
        Context::PersonFieldList { existing } => items::person_fields(&existing),
        Context::MarriageFieldList { existing } => items::marriage_fields(&existing),
        Context::AdoptionFieldList { existing } => items::adoption_fields(&existing),
        Context::AfterGenderColon => items::gender_values(),
        Context::AfterEndReasonColon => items::end_reason_values(),
        Context::AfterStringFieldColon => items::quoted_value_snippet(),
        Context::MarriageRefPosition => items::marriage_id_items(resolved),
        Context::SpousePosition { exclude } => items::person_id_items(resolved, exclude.as_deref()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::test_open_file;
    use tower_lsp::lsp_types::CompletionItemKind;

    /// Take a fixture string with a `<CURSOR>` marker; return (source, offset).
    fn cursor_fixture(s: &str) -> (String, usize) {
        let offset = s.find("<CURSOR>").expect("fixture must contain <CURSOR>");
        let source = format!("{}{}", &s[..offset], &s[offset + "<CURSOR>".len()..]);
        (source, offset)
    }

    fn complete_for(src_with_marker: &str) -> Vec<CompletionItem> {
        let (source, offset) = cursor_fixture(src_with_marker);
        let doc = test_open_file(&source);
        let v = doc.view();
        complete(v.line_index.source(), v.file, v.resolved, offset)
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
        let labels = run(src);
        assert_eq!(labels, vec!["birth".to_owned(), "adoption".into()]);
    }

    #[test]
    fn person_field_list_after_id() {
        let src = "person a <CURSOR>";
        let labels = run(src);
        assert!(labels.contains(&"name:".to_owned()));
        assert!(labels.contains(&"gender:".to_owned()));
        assert!(labels.contains(&"born:".to_owned()));
    }

    #[test]
    fn person_field_list_filters_present_fields() {
        let src = "person a name:\"A\" gender:female <CURSOR>";
        let labels = run(src);
        assert!(!labels.contains(&"name:".to_owned()));
        assert!(!labels.contains(&"gender:".to_owned()));
        assert!(labels.contains(&"family:".to_owned()));
        assert!(labels.contains(&"born:".to_owned()));
    }

    #[test]
    fn marriage_field_list_after_spouses() {
        let src = "marriage m a b <CURSOR>";
        let labels = run(src);
        assert!(labels.contains(&"start:".to_owned()));
        assert!(labels.contains(&"end:".to_owned()));
        assert!(labels.contains(&"end_reason:".to_owned()));
    }

    #[test]
    fn marriage_field_list_filters_present() {
        let src = "marriage m a b start:2010 <CURSOR>";
        let labels = run(src);
        assert!(!labels.contains(&"start:".to_owned()));
        assert!(labels.contains(&"end:".to_owned()));
        assert!(labels.contains(&"end_reason:".to_owned()));
    }

    #[test]
    fn after_gender_colon_offers_enum_values() {
        let src = "person a name:\"A\" gender:<CURSOR>";
        let labels = run(src);
        assert_eq!(
            labels,
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
        use tower_lsp::lsp_types::InsertTextFormat;
        let items = complete_for("person a <CURSOR>");

        for field in ["name:", "family:", "given:"] {
            let it = items
                .iter()
                .find(|c| c.label == field)
                .unwrap_or_else(|| panic!("missing item `{field}` in {items:#?}"));
            let expected = format!("{}\"$0\"", field);
            assert_eq!(
                it.insert_text.as_deref(),
                Some(expected.as_str()),
                "string field `{field}` should insert `{expected}`"
            );
            assert_eq!(it.insert_text_format, Some(InsertTextFormat::SNIPPET));
        }

        for field in ["born:", "died:", "gender:"] {
            let it = items
                .iter()
                .find(|c| c.label == field)
                .unwrap_or_else(|| panic!("missing item `{field}`"));
            assert!(
                it.insert_text.is_none() && it.insert_text_format.is_none(),
                "non-string field `{field}` should keep plain insertion; got: {it:#?}"
            );
        }
    }

    #[test]
    fn after_string_field_colon_offers_quoted_value_snippet() {
        use tower_lsp::lsp_types::InsertTextFormat;
        for field in ["name", "family", "given"] {
            let src = format!("person a {field}:<CURSOR>");
            let items = complete_for(&src);
            assert_eq!(
                items.len(),
                1,
                "expected one item for `{field}:`; got: {items:#?}"
            );
            let item = &items[0];
            assert_eq!(
                item.insert_text.as_deref(),
                Some(r#""$0""#),
                "insert_text should wrap cursor in quotes for `{field}:`"
            );
            assert_eq!(
                item.insert_text_format,
                Some(InsertTextFormat::SNIPPET),
                "insert_text_format must be Snippet for `{field}:` so $0 is honored"
            );
            assert_eq!(
                item.preselect,
                Some(true),
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

    fn run_with_details(src_with_marker: &str) -> Vec<(String, Option<String>)> {
        complete_for(src_with_marker)
            .into_iter()
            .map(|c| (c.label, c.detail))
            .collect()
    }

    fn run_kinds(src_with_marker: &str) -> Vec<(String, CompletionItemKind)> {
        complete_for(src_with_marker)
            .into_iter()
            .map(|c| (c.label, c.kind.unwrap()))
            .collect()
    }

    #[test]
    fn after_birth_offers_marriage_ids_with_spouse_detail() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bob name:\"Bob\" gender:male\n\
                   marriage m1 alice bob start:1972 end:1990 end_reason:divorce\n\
                   marriage m2 alice bob start:2000\n\
                   person kid name:\"K\" gender:other\n  birth <CURSOR>";
        let items = run_with_details(src);
        let labels: Vec<&str> = items.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(labels, vec!["m1", "m2"]);
        assert_eq!(items[0].1.as_deref(), Some("Alice + Bob, 1972–1990"));
        assert_eq!(items[1].1.as_deref(), Some("Alice + Bob, 2000–"));
    }

    #[test]
    fn after_birth_with_partial_id_still_offers_marriages() {
        // Cursor adjacent to partial ident — IDE wants the full set and
        // filters by prefix client-side.
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m_alice_bob alice bob start:1972\n\
                   person kid name:\"K\" gender:other\n  birth m_<CURSOR>";
        let labels = run(src);
        assert_eq!(labels, vec!["m_alice_bob".to_owned()]);
    }

    #[test]
    fn after_adoption_offers_marriage_ids() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bob name:\"Bob\" gender:male\n\
                   marriage m alice bob start:1972\n\
                   person kid name:\"K\" gender:other\n  adoption <CURSOR>";
        let labels = run(src);
        assert_eq!(labels, vec!["m".to_owned()]);
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
        let items = run_with_details(src);
        let labels: Vec<&str> = items.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(labels, vec!["alice", "bob"]);
        assert_eq!(items[0].1.as_deref(), Some("Alice, b. 1950"));
        assert_eq!(items[1].1.as_deref(), Some("Bob, b. 1948"));
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
    fn marriage_completion_kinds_are_distinct() {
        let src = "person alice name:\"A\" gender:female\n\
                   marriage m alice alice start:1972\n\
                   person kid name:\"K\" gender:other\n  birth <CURSOR>";
        let kinds = run_kinds(src);
        assert!(kinds.iter().all(|(_, k)| *k == CompletionItemKind::EVENT));
    }

    #[test]
    fn person_completion_kinds_are_variable() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m <CURSOR>";
        let kinds = run_kinds(src);
        assert!(
            kinds
                .iter()
                .all(|(_, k)| *k == CompletionItemKind::VARIABLE)
        );
    }

    /// Keep snapshots next to sibling LSP features under `features/snapshots/`.
    macro_rules! assert_completion_snapshot {
        ($($tt:tt)*) => {
            insta::with_settings!({ snapshot_path => "../snapshots" }, {
                insta::assert_json_snapshot!($($tt)*);
            });
        };
    }

    #[test]
    fn snapshot_after_birth_keyword() {
        assert_completion_snapshot!(complete_for(
            "person alice name:\"Alice\" gender:female born:1950\n\
             person bob name:\"Bob\" gender:male born:1948\n\
             marriage m_alice_bob alice bob start:1972 end:1990 end_reason:divorce\n\
             person kid name:\"K\" gender:other\n  birth <CURSOR>",
        ));
    }

    #[test]
    fn snapshot_after_marriage_id_for_spouse_a() {
        assert_completion_snapshot!(complete_for(
            "person alice name:\"Alice\" gender:female born:1950\n\
             person bob name:\"Bob\" gender:male born:1948\n\
             marriage m <CURSOR>",
        ));
    }

    #[test]
    fn snapshot_after_spouse_a_excludes_self() {
        assert_completion_snapshot!(complete_for(
            "person alice name:\"Alice\" gender:female born:1950\n\
             person bob name:\"Bob\" gender:male born:1948\n\
             person carol name:\"Carol\" gender:female born:1975\n\
             marriage m alice <CURSOR>",
        ));
    }

    #[test]
    fn snapshot_top_level() {
        assert_completion_snapshot!(complete_for("<CURSOR>"));
    }

    #[test]
    fn snapshot_after_gender_colon() {
        assert_completion_snapshot!(complete_for("person a name:\"A\" gender:<CURSOR>"));
    }
}
