//! Go-to-definition for `textDocument/definition`.
//!
//! Targets are project-wide (ADR-0015): a reference may resolve to a
//! declaration in a sibling `.kul` file.

use tower_lsp::lsp_types::{Location, Position, Url};

use crate::state::ProjectEntry;

/// Resolve the cursor position to the declaration `Location`, or `None`
/// when there is nothing to navigate to.
pub fn definition(entry: &ProjectEntry, uri: &Url, position: Position) -> Option<Location> {
    let c = entry.cursor_for_uri(uri, position)?;
    let entity = c.entity()?;
    // Goto-def from a decl is a no-op.
    if entity.is_decl {
        return None;
    }
    let decl = entity.decl_span()?;
    entry.location_for(decl)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{idx, position_for, test_open_file, test_project_entry, test_url as url};

    fn def_at(source: &str, offset: usize) -> Option<Location> {
        let doc = test_open_file(source);
        definition(&doc, &url(), position_for(source, offset))
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

        let marriage_line_start = src.find("marriage m").unwrap();
        let line_text_before_m = &src[..marriage_line_start];
        let line_count = line_text_before_m.matches('\n').count();
        assert_eq!(loc.range.start.line as usize, line_count);
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
        assert!(def_at(src, 0).is_none());
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

    /// Cross-file goto-definition (ADR-0015).
    #[test]
    fn jumps_to_sibling_file_declaration() {
        let alice_src = "person alice name:\"Alice\" gender:female\n";
        let marriage_src = "person bob name:\"Bob\" gender:male\nmarriage m alice bob start:2010\n";
        let entry = test_project_entry(&[("alice.kul", alice_src), ("marriage.kul", marriage_src)]);
        let marriage_url = Url::parse("file:///marriage.kul").unwrap();
        let alice_url = Url::parse("file:///alice.kul").unwrap();
        let alice_ref_offset = marriage_src.find(" alice ").unwrap() + 1;
        let loc = definition(
            &entry,
            &marriage_url,
            position_for(marriage_src, alice_ref_offset),
        )
        .expect("location");
        assert_eq!(loc.uri, alice_url);
        assert_eq!(loc.range.start.line, 0);
        assert_eq!(loc.range.start.character, 7);
    }
}
