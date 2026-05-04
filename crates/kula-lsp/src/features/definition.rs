//! Go-to-definition for `textDocument/definition`.
//!
//! Pure dispatch: the resolved-doc query already pairs each reference with
//! its target (when one exists). All this module does is turn a `Node::*Ref`
//! with `target: Some(_)` into the corresponding declaration `Location`.

use kula_core::node_at::Node;
use kula_core::semantic::ResolvedDocument;
use tower_lsp::lsp_types::{Location, Url};

use crate::convert::LineIndex;

/// Resolve the cursor position to the declaration `Location`, or `None`
/// when there is nothing to navigate to (declaration site, unresolved
/// reference, keyword, field, whitespace, EOF).
pub fn definition(
    resolved: &ResolvedDocument<'_>,
    line_index: &LineIndex,
    uri: &Url,
    byte_offset: usize,
) -> Option<Location> {
    let node = resolved.node_at(byte_offset)?;
    let target_span = match node {
        Node::PersonRef {
            target: Some(p), ..
        } => p.id.span,
        Node::MarriageRef {
            target: Some(m), ..
        } => m.id.span,
        _ => return None,
    };
    Some(Location {
        uri: uri.clone(),
        range: line_index.range(target_span),
    })
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

    fn def_at(source: &str, offset: usize) -> Option<Location> {
        let tokens = tokenize(source);
        let (document, _) = parse(&tokens);
        let (resolved, _) = resolve(&document);
        let line_index = LineIndex::new(source);
        definition(&resolved, &line_index, &url(), offset)
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
        assert!(def_at(src, src.len()).is_none());
        assert!(def_at(src, src.len() + 999).is_none());
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
}
