//! Find references for `textDocument/references`.
//!
//! Pure dispatch over [`kul_core::node_at::Node`]: a cursor on a person
//! id (decl or ref) returns every spouse position that names them; a
//! cursor on a marriage id returns every `birth`/`adoption` ref that
//! points at that marriage. Per LSP `ReferenceContext.includeDeclaration`,
//! the declaration site is included only when asked.
//!
//! With project-wide resolution (ADR-0015), `references_to` walks every
//! file in the project. This module assembles a single `Vec<Location>`
//! across all of them, anchoring each result at the URL its file maps
//! to in the project entry.

use tower_lsp::lsp_types::{Location, Position, Url};

use crate::state::ProjectEntry;

/// Resolve the cursor to the list of reference `Location`s, or `None` when
/// the cursor isn't on something the user could find references for
/// (keywords, fields, whitespace, EOF). Returns `Some(empty)` when the
/// cursor *is* on a referenceable id but nothing else uses it.
///
/// Results span every file in the project: rename and find-references
/// no longer stop at the active URI's boundary.
pub fn references(
    entry: &ProjectEntry,
    uri: &Url,
    position: Position,
    include_declaration: bool,
) -> Option<Vec<Location>> {
    let c = entry.cursor_for_uri(uri, position)?;
    let entity = c
        .resolved
        .node_at(c.file, c.offset)?
        .entity_reference(c.file)?;

    let mut spans = c.resolved.references_to(entity.name, entity.kind);
    if include_declaration && let Some(d) = entity.decl_span() {
        // `decl_span()`'s `file` field reports the active URI's file when
        // the cursor is on a decl, and the active URI's file again when
        // the cursor is on a reference (the helper is conservative
        // about cross-file lookups). Re-resolve via `entity()` to find
        // the real decl file under project-wide namespaces.
        if entity.is_decl {
            spans.push(d);
        } else if let Some(target_entity) = c.resolved.entity(entity.name) {
            spans.push(kul_core::span::FileSpan::new(
                target_entity.file,
                target_entity.id.span,
            ));
        }
    }
    spans.sort_by_key(|s| (s.file.as_u32(), s.span.start));
    spans.dedup();

    Some(
        spans
            .into_iter()
            .filter_map(|fs| {
                let url = entry.url_for(fs.file)?;
                let line_index = entry.line_index_for(fs.file)?;
                Some(Location {
                    uri: url.clone(),
                    range: line_index.range(fs.span),
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{test_open_file, test_project_entry};
    use tower_lsp::lsp_types::Position;

    fn url() -> Url {
        Url::parse("file:///t.kul").unwrap()
    }

    fn position_for(source: &str, offset: usize) -> Position {
        let mut line = 0u32;
        let mut character = 0u32;
        for (i, b) in source.bytes().enumerate() {
            if i == offset {
                break;
            }
            if b == b'\n' {
                line += 1;
                character = 0;
            } else {
                character += 1;
            }
        }
        Position { line, character }
    }

    fn refs_at(source: &str, offset: usize, include_decl: bool) -> Option<Vec<(u32, u32)>> {
        let doc = test_open_file(source);
        references(&doc, &url(), position_for(source, offset), include_decl).map(|locs| {
            locs.into_iter()
                .map(|l| (l.range.start.line, l.range.start.character))
                .collect()
        })
    }

    fn idx(source: &str, pat: &str) -> usize {
        source.find(pat).expect("pattern in source")
    }

    #[test]
    fn person_decl_finds_all_spouse_positions() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   person carol name:\"C\" gender:female\n\
                   marriage m1 alice bob start:1972\n\
                   marriage m2 alice carol start:2000\n";
        let got = refs_at(src, idx(src, "alice"), false).unwrap();
        // Two spouse positions in the two marriages.
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].0, 3); // line 3: "marriage m1 alice ..."
        assert_eq!(got[1].0, 4); // line 4: "marriage m2 alice ..."
    }

    #[test]
    fn person_decl_includes_declaration_when_asked() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:1972\n";
        let with_decl = refs_at(src, idx(src, "alice"), true).unwrap();
        let without = refs_at(src, idx(src, "alice"), false).unwrap();
        // `with_decl` has one extra location at the declaration site.
        assert_eq!(with_decl.len(), without.len() + 1);
        assert_eq!(with_decl[0], (0, 7)); // alice's decl id span
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
        // No `ghost` declaration, but the user wants to find every place it's
        // mentioned. Returns the spouse position(s) where the name appears.
        let src = "marriage m ghost b start:1972\nperson b name:\"B\" gender:male\n";
        let marriage_line = idx(src, "marriage ");
        let ghost = src[marriage_line..]
            .find("ghost")
            .map(|i| marriage_line + i)
            .unwrap();
        let got = refs_at(src, ghost, false).unwrap();
        assert_eq!(got.len(), 1);
        // include_declaration on an unresolved ref doesn't add anything.
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
        // Sorted: decl on line 0 first, then m1 on line 2, then m2 on line 3.
        for w in got.windows(2) {
            assert!(w[0] <= w[1], "not sorted: {:?}", got);
        }
    }

    /// Cross-file find-references: a person declared in one file is
    /// referenced in another; both files participate in the result.
    #[test]
    fn finds_references_across_files() {
        let alice_src = "person alice name:\"Alice\" gender:female\n";
        let marriage_src = "person bob name:\"Bob\" gender:male\nmarriage m alice bob start:2010\n";
        let entry = test_project_entry(&[("alice.kul", alice_src), ("marriage.kul", marriage_src)]);
        let alice_url = Url::parse("file:///alice.kul").unwrap();
        let marriage_url = Url::parse("file:///marriage.kul").unwrap();
        // Cursor on `alice` decl in alice.kul; expect one reference in marriage.kul.
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
}
