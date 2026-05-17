//! Go-to-definition for `textDocument/definition`.
//!
//! Pure dispatch: the resolved-doc query already pairs each reference with
//! its target (when one exists). All this module does is turn a `Node::*Ref`
//! with `target: Some(_)` into the corresponding declaration `Location`.
//!
//! With project-wide resolution (ADR-0015), the target of a reference may
//! live in a sibling `.kul` file. The lookup walks through
//! [`ResolvedDocument::entity`] to find which `FileId` owns the
//! declaration, then maps that `FileId` back to the project URL via
//! [`ProjectEntry`].

use tower_lsp::lsp_types::{Location, Position, Url};

use crate::state::ProjectEntry;

/// Resolve the cursor position to the declaration `Location`, or `None`
/// when there is nothing to navigate to (declaration site, unresolved
/// reference, keyword, field, whitespace, EOF, URI not in the project).
pub fn definition(entry: &ProjectEntry, uri: &Url, position: Position) -> Option<Location> {
    let c = entry.cursor_for_uri(uri, position)?;
    let entity = c
        .resolved
        .node_at(c.file, c.offset)?
        .entity_reference(c.file)?;
    // Goto-def from a decl is a no-op; resolved refs jump to the target.
    if entity.is_decl {
        return None;
    }
    // `entity.target` is the resolved decl (may live in any project
    // file). Re-query the resolver to find the file the declaration
    // sits in; with project-wide namespaces (ADR-0015) the answer is no
    // longer "the same file as the reference".
    let _ = entity.target?;
    let target_entity = c.resolved.entity(entity.name)?;
    let target_url = entry.url_for(target_entity.file)?;
    let target_line_index = entry.line_index_for(target_entity.file)?;
    Some(Location {
        uri: target_url.clone(),
        range: target_line_index.range(target_entity.id.span),
    })
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

    fn def_at(source: &str, offset: usize) -> Option<Location> {
        let doc = test_open_file(source);
        definition(&doc, &url(), position_for(source, offset))
    }

    fn idx(source: &str, pat: &str) -> usize {
        source.find(pat).expect("pattern in source")
    }

    #[test]
    fn person_ref_jumps_to_decl_id_span() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:2010\n";
        let marriage_line = idx(src, "marriage ");
        let alice_ref = src[marriage_line..]
            .find(" alice ")
            .map(|i| marriage_line + i + 1)
            .unwrap();
        let loc = def_at(src, alice_ref).expect("location");

        // Range should point at "alice" in `person alice ...` (line 0, cols 7..12).
        assert_eq!(loc.uri, url());
        assert_eq!(loc.range.start.line, 0);
        assert_eq!(loc.range.start.character, 7);
        assert_eq!(loc.range.end.line, 0);
        assert_eq!(loc.range.end.character, 12);
    }

    #[test]
    fn marriage_ref_in_birth_jumps_to_decl_id() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  birth m\n";
        let m_ref = idx(src, "birth m") + "birth ".len();
        let loc = def_at(src, m_ref).expect("location");

        // The marriage `m` id lives on line 2; find its column.
        let marriage_line_start = src.find("marriage m").unwrap();
        let line_text_before_m = &src[..marriage_line_start];
        let line_count = line_text_before_m.matches('\n').count();
        assert_eq!(loc.range.start.line as usize, line_count);
        // Column is the byte offset of `m` within line 2 = "marriage ".len().
        assert_eq!(loc.range.start.character as usize, "marriage ".len());
    }

    #[test]
    fn marriage_ref_in_adoption_jumps_to_decl_id() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  adoption m start:2000\n";
        let m_ref = idx(src, "adoption m") + "adoption ".len();
        let loc = def_at(src, m_ref).expect("location");
        // `m` decl is at line 2; same as birth case.
        assert_eq!(loc.range.start.line, 2);
    }

    #[test]
    fn unresolved_person_ref_returns_none() {
        let src = "marriage m ghost b start:2000\nperson b name:\"B\" gender:male\n";
        let marriage_line = idx(src, "marriage ");
        let ghost = src[marriage_line..]
            .find("ghost")
            .map(|i| marriage_line + i)
            .unwrap();
        assert!(def_at(src, ghost).is_none());
    }

    #[test]
    fn unresolved_marriage_ref_returns_none() {
        let src = "person a name:\"A\" gender:female\n  birth m_nope\n";
        assert!(def_at(src, idx(src, "m_nope")).is_none());
    }

    #[test]
    fn declaration_site_returns_none() {
        let src = "person alice name:\"A\" gender:female\n";
        // Cursor on the decl id itself — you don't go to def of a def.
        assert!(def_at(src, idx(src, "alice")).is_none());
    }

    #[test]
    fn marriage_decl_site_returns_none() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n";
        let m_decl = idx(src, "marriage m") + "marriage ".len();
        assert!(def_at(src, m_decl).is_none());
    }

    #[test]
    fn keyword_returns_none() {
        let src = "person alice name:\"A\" gender:female\n";
        assert!(def_at(src, 0).is_none()); // `person` keyword
    }

    #[test]
    fn field_name_returns_none() {
        let src = "person alice name:\"A\" gender:female\n";
        assert!(def_at(src, idx(src, "name:")).is_none());
    }

    #[test]
    fn field_value_returns_none() {
        let src = "person alice name:\"Alice\" gender:female\n";
        assert!(def_at(src, idx(src, "\"Alice\"")).is_none());
    }

    #[test]
    fn whitespace_returns_none() {
        let src = "person a name:\"A\" gender:female\n\nperson b name:\"B\" gender:male\n";
        let blank = src.find("\n\n").unwrap() + 1;
        assert!(def_at(src, blank).is_none());
    }

    #[test]
    fn eof_returns_none() {
        let src = "person a name:\"A\" gender:female\n";
        // `position_for` clamps past EOF to last position; cursor at end
        // still resolves to None because the entity_reference query is
        // empty there.
        assert!(def_at(src, src.len()).is_none());
    }

    #[test]
    fn returned_uri_matches_input_uri() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:2010\n";
        let marriage_line = idx(src, "marriage ");
        let a_ref = src[marriage_line..]
            .find(" a ")
            .map(|i| marriage_line + i + 1)
            .unwrap();
        let loc = def_at(src, a_ref).expect("location");
        assert_eq!(loc.uri, url());
    }

    /// Cross-file goto-definition: a reference in one file jumps to a
    /// declaration in a sibling file. This is the project-wide-namespace
    /// payoff (ADR-0015 + issue #85) — the editor experience catches up
    /// to the CLI/WASM crate.
    #[test]
    fn jumps_to_sibling_file_declaration() {
        let alice_src = "person alice name:\"Alice\" gender:female\n";
        let marriage_src = "person bob name:\"Bob\" gender:male\nmarriage m alice bob start:2010\n";
        let entry = test_project_entry(&[("alice.kul", alice_src), ("marriage.kul", marriage_src)]);
        let marriage_url = Url::parse("file:///marriage.kul").unwrap();
        let alice_url = Url::parse("file:///alice.kul").unwrap();
        // Cursor on `alice` inside `marriage m alice bob`.
        let alice_ref_offset = marriage_src.find(" alice ").unwrap() + 1;
        let loc = definition(
            &entry,
            &marriage_url,
            position_for(marriage_src, alice_ref_offset),
        )
        .expect("location");
        assert_eq!(loc.uri, alice_url);
        // alice's decl id span starts at byte 7 of `alice.kul`.
        assert_eq!(loc.range.start.line, 0);
        assert_eq!(loc.range.start.character, 7);
    }
}
