//! Find references for `textDocument/references`.
//!
//! Project-wide (ADR-0015): walks every file in the project, anchoring
//! each result at its file's URL.

use tower_lsp::lsp_types::{Location, Position, Url};

use crate::state::ProjectEntry;

/// Resolve the cursor to the list of reference `Location`s, or `None`
/// for a non-referenceable cursor. `Some(empty)` when the id has no
/// other uses.
pub fn references(
    entry: &ProjectEntry,
    uri: &Url,
    position: Position,
    include_declaration: bool,
) -> Option<Vec<Location>> {
    let c = entry.cursor_for_uri(uri, position)?;
    let entity = c.entity()?;

    let mut spans = c.resolved.references_to(entity.name, entity.kind);
    if include_declaration && let Some(d) = entity.decl_span() {
        spans.push(d);
    }
    spans.sort_by_key(|s| (s.file.as_u32(), s.span.start));
    spans.dedup();

    Some(
        spans
            .into_iter()
            .filter_map(|fs| entry.location_for(fs))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{idx, position_for, test_open_file, test_project_entry, test_url as url};

    fn refs_at(source: &str, offset: usize, include_decl: bool) -> Option<Vec<(u32, u32)>> {
        let doc = test_open_file(source);
        references(&doc, &url(), position_for(source, offset), include_decl).map(|locs| {
            locs.into_iter()
                .map(|l| (l.range.start.line, l.range.start.character))
                .collect()
        })
    }

    #[test]
    fn person_decl_finds_all_spouse_positions() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   person carol name:\"C\" gender:female\n\
                   marriage m1 alice bob start:1972\n\
                   marriage m2 alice carol start:2000\n";
        let got = refs_at(src, idx(src, "alice"), false).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].0, 3);
        assert_eq!(got[1].0, 4);
    }

    #[test]
    fn person_decl_includes_declaration_when_asked() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:1972\n";
        let with_decl = refs_at(src, idx(src, "alice"), true).unwrap();
        let without = refs_at(src, idx(src, "alice"), false).unwrap();
        assert_eq!(with_decl.len(), without.len() + 1);
        assert_eq!(with_decl[0], (0, 7));
    }

    #[test]
    fn person_ref_returns_same_results_as_decl() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:1972\n";
        let from_decl = refs_at(src, idx(src, "alice"), false).unwrap();
        let marriage_line = idx(src, "marriage ");
        let alice_ref = src[marriage_line..]
            .find(" alice ")
            .map(|i| marriage_line + i + 1)
            .unwrap();
        let from_ref = refs_at(src, alice_ref, false).unwrap();
        assert_eq!(from_decl, from_ref);
    }

    #[test]
    fn marriage_decl_finds_birth_and_adoption_refs() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1972\n\
                   person kid1 name:\"K1\" gender:other\n  birth m\n\
                   person kid2 name:\"K2\" gender:other\n  adoption m start:2000\n";
        let m_decl = idx(src, "marriage m") + "marriage ".len();
        let got = refs_at(src, m_decl, false).unwrap();
        assert_eq!(got.len(), 2);
    }

    #[test]
    fn unresolved_person_ref_still_finds_uses() {
        let src = "marriage m ghost b start:1972\nperson b name:\"B\" gender:male\n";
        let marriage_line = idx(src, "marriage ");
        let ghost = src[marriage_line..]
            .find("ghost")
            .map(|i| marriage_line + i)
            .unwrap();
        let got = refs_at(src, ghost, false).unwrap();
        assert_eq!(got.len(), 1);
        // include_declaration adds nothing for an unresolved ref.
        let with = refs_at(src, ghost, true).unwrap();
        assert_eq!(with.len(), 1);
    }

    #[test]
    fn keyword_returns_none() {
        let src = "person alice name:\"A\" gender:female\n";
        assert!(refs_at(src, 0, true).is_none());
    }

    #[test]
    fn field_name_returns_none() {
        let src = "person alice name:\"A\" gender:female\n";
        assert!(refs_at(src, idx(src, "name:"), true).is_none());
    }

    #[test]
    fn field_value_returns_none() {
        let src = "person alice name:\"Alice\" gender:female\n";
        assert!(refs_at(src, idx(src, "\"Alice\""), true).is_none());
    }

    #[test]
    fn whitespace_returns_none() {
        let src = "person a name:\"A\" gender:female\n\nperson b name:\"B\" gender:male\n";
        let blank = src.find("\n\n").unwrap() + 1;
        assert!(refs_at(src, blank, true).is_none());
    }

    #[test]
    fn returned_uri_matches_input() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:1972\n";
        let doc = test_open_file(src);
        let raw = references(&doc, &url(), position_for(src, idx(src, "alice")), false).unwrap();
        assert!(!raw.is_empty());
        assert!(raw.iter().all(|l| l.uri == url()));
    }

    #[test]
    fn references_are_sorted_by_position() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m1 alice bob start:1972\n\
                   marriage m2 bob alice start:2000\n";
        let got = refs_at(src, idx(src, "alice"), true).unwrap();
        for w in got.windows(2) {
            assert!(w[0] <= w[1], "not sorted: {:?}", got);
        }
    }

    /// Cross-file find-references (ADR-0015).
    #[test]
    fn finds_references_across_files() {
        let alice_src = "person alice name:\"Alice\" gender:female\n";
        let marriage_src = "person bob name:\"Bob\" gender:male\nmarriage m alice bob start:2010\n";
        let entry = test_project_entry(&[("alice.kul", alice_src), ("marriage.kul", marriage_src)]);
        let alice_url = Url::parse("file:///alice.kul").unwrap();
        let marriage_url = Url::parse("file:///marriage.kul").unwrap();
        let locs = references(
            &entry,
            &alice_url,
            position_for(alice_src, alice_src.find("alice").unwrap()),
            false,
        )
        .unwrap();
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].uri, marriage_url);
    }

    /// Cross-file `includeDeclaration`: cursor on a reference, decl in sibling.
    #[test]
    fn include_declaration_picks_up_sibling_file_decl() {
        let alice_src = "person alice name:\"Alice\" gender:female\n";
        let marriage_src = "person bob name:\"Bob\" gender:male\nmarriage m alice bob start:2010\n";
        let entry = test_project_entry(&[("alice.kul", alice_src), ("marriage.kul", marriage_src)]);
        let alice_url = Url::parse("file:///alice.kul").unwrap();
        let marriage_url = Url::parse("file:///marriage.kul").unwrap();
        let alice_ref_offset = marriage_src.find(" alice ").unwrap() + 1;
        let locs = references(
            &entry,
            &marriage_url,
            position_for(marriage_src, alice_ref_offset),
            true,
        )
        .unwrap();
        assert_eq!(locs.len(), 2);
        assert!(locs.iter().any(|l| l.uri == alice_url));
        assert!(locs.iter().any(|l| l.uri == marriage_url));
    }
}
