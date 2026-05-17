//! Find references for `textDocument/references`.
//!
//! Pure dispatch over [`kul_core::node_at::Node`]: a cursor on a person
//! id (decl or ref) returns every spouse position that names them; a
//! cursor on a marriage id returns every `birth`/`adoption` ref that
//! points at that marriage. Per LSP `ReferenceContext.includeDeclaration`,
//! the declaration site is included only when asked.

use kul_core::semantic::ResolvedDocument;
use kul_core::span::ByteSpan;
use kul_core::span::FileId;
use tower_lsp::lsp_types::{Location, Url};

use crate::convert::LineIndex;

/// Resolve the cursor to the list of reference `Location`s, or `None` when
/// the cursor isn't on something the user could find references for
/// (keywords, fields, whitespace, EOF). Returns `Some(empty)` when the
/// cursor *is* on a referenceable id but nothing else uses it.
///
/// The resolver's `references_to` query is project-wide (per ADR-0015);
/// this feature filters to the active URI's `FileId` because the LSP
/// cache is still URI-keyed. Cross-file find-references lands with PRD
/// 0001 slice 5 (#85), at which point this filter goes away.
pub fn references(
    file: FileId,
    resolved: &ResolvedDocument,
    line_index: &LineIndex,
    uri: &Url,
    byte_offset: usize,
    include_declaration: bool,
) -> Option<Vec<Location>> {
    let entity = resolved
        .node_at(file, byte_offset)?
        .entity_reference(file)?;

    let mut spans: Vec<ByteSpan> = resolved
        .references_to(entity.name, entity.kind)
        .into_iter()
        .filter(|fs| fs.file == file)
        .map(|fs| fs.span)
        .collect();
    if include_declaration && let Some(d) = entity.decl_span() {
        spans.push(d.span);
    }
    spans.sort_by_key(|s| s.start);
    spans.dedup();

    Some(
        spans
            .into_iter()
            .map(|s| Location {
                uri: uri.clone(),
                range: line_index.range(s),
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::test_open_file;

    fn url() -> Url {
        Url::parse("file:///t.kul").unwrap()
    }

    fn refs_at(source: &str, offset: usize, include_decl: bool) -> Option<Vec<(u32, u32)>> {
        let doc = test_open_file(source);
        let v = doc.view();
        references(
            v.file,
            v.resolved,
            v.line_index,
            &url(),
            offset,
            include_decl,
        )
        .map(|locs| {
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
        let locs = refs_at(src, idx(src, "alice"), false).unwrap();
        // refs_at maps Location → (line, char); re-run the underlying
        // call to get URIs back.
        let doc = test_open_file(src);
        let v = doc.view();
        let raw = references(
            v.file,
            v.resolved,
            v.line_index,
            &url(),
            idx(src, "alice"),
            false,
        )
        .unwrap();
        assert_eq!(raw.len(), locs.len());
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
}
