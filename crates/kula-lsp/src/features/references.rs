//! Find references for `textDocument/references`.
//!
//! Pure dispatch over [`kula_core::node_at::Node`]: a cursor on a person
//! id (decl or ref) returns every spouse position that names them; a
//! cursor on a marriage id returns every `birth`/`adoption` ref that
//! points at that marriage. Per LSP `ReferenceContext.includeDeclaration`,
//! the declaration site is included only when asked.

use kula_core::node_at::Node;
use kula_core::semantic::{EntityKind, ResolvedDocument};
use kula_core::span::ByteSpan;
use tower_lsp::lsp_types::{Location, Url};

use crate::convert::LineIndex;

/// Resolve the cursor to the list of reference `Location`s, or `None` when
/// the cursor isn't on something the user could find references for
/// (keywords, fields, whitespace, EOF). Returns `Some(empty)` when the
/// cursor *is* on a referenceable id but nothing else uses it.
pub fn references(
    resolved: &ResolvedDocument<'_>,
    line_index: &LineIndex,
    uri: &Url,
    byte_offset: usize,
    include_declaration: bool,
) -> Option<Vec<Location>> {
    let node = resolved.node_at(byte_offset)?;
    let (id, kind, decl_span) = match node {
        Node::PersonDeclId(p) => (p.id.name.as_str(), EntityKind::Person, Some(p.id.span)),
        Node::MarriageDeclId(m) => (m.id.name.as_str(), EntityKind::Marriage, Some(m.id.span)),
        Node::PersonRef { ident, target } => (
            ident.name.as_str(),
            EntityKind::Person,
            target.map(|p| p.id.span),
        ),
        Node::MarriageRef { ident, target } => (
            ident.name.as_str(),
            EntityKind::Marriage,
            target.map(|m| m.id.span),
        ),
        _ => return None,
    };

    let mut spans: Vec<ByteSpan> = resolved.references_to(id, kind);
    if include_declaration && let Some(d) = decl_span {
        spans.push(d);
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
    use kula_core::lexer::tokenize;
    use kula_core::parser::parse;
    use kula_core::semantic::resolve;

    fn url() -> Url {
        Url::parse("file:///t.kula").unwrap()
    }

    fn refs_at(source: &str, offset: usize, include_decl: bool) -> Option<Vec<(u32, u32)>> {
        let tokens = tokenize(source);
        let (document, _) = parse(&tokens);
        let (resolved, _) = resolve(&document);
        let line_index = LineIndex::new(source);
        references(&resolved, &line_index, &url(), offset, include_decl).map(|locs| {
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
        let tokens = tokenize(src);
        let (document, _) = parse(&tokens);
        let (resolved, _) = resolve(&document);
        let line_index = LineIndex::new(src);
        let locs = references(&resolved, &line_index, &url(), idx(src, "alice"), false).unwrap();
        assert!(locs.iter().all(|l| l.uri == url()));
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
