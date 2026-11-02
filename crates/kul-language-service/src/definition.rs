//! Go to definition: from a reference to the declaration it names.
//!
//! Targets are project-wide (ADR-0015): a reference may resolve to a
//! declaration in a sibling `.kul` file.

use kul_core::CheckResult;

use crate::{Location, entity_under, file_id, location};

/// The declaration the reference at `offset` in `file` names, or `None` when
/// there is nothing to navigate to: not on a reference, an unresolved
/// reference, or already on the declaration.
#[must_use]
pub fn definition(check: &CheckResult, file: &str, offset: usize) -> Option<Location> {
    let entity = entity_under(check, file_id(check, file)?, offset)?;
    if entity.is_decl {
        return None;
    }
    location(check, entity.decl_span()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Span;
    use crate::test_support::{FILE, check_one, check_project, idx};

    fn def_at(source: &str, offset: usize) -> Option<Location> {
        definition(&check_one(source), FILE, offset)
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
        assert_eq!(loc.file, FILE);
        assert_eq!(loc.span, Span { start: 7, end: 12 });
    }

    #[test]
    fn marriage_ref_in_birth_jumps_to_decl_id() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  birth m\n";
        let m_ref = idx(src, "birth m") + "birth ".len();
        let loc = def_at(src, m_ref).expect("location");
        let m_decl = idx(src, "marriage m") + "marriage ".len();
        assert_eq!(
            loc.span,
            Span {
                start: m_decl,
                end: m_decl + 1
            }
        );
    }

    #[test]
    fn marriage_ref_in_adoption_jumps_to_decl_id() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  adoption m start:2000\n";
        let m_ref = idx(src, "adoption m") + "adoption ".len();
        let loc = def_at(src, m_ref).expect("location");
        assert_eq!(loc.span.start, idx(src, "marriage m") + "marriage ".len());
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

    /// Cross-file goto-definition (ADR-0015).
    #[test]
    fn jumps_to_sibling_file_declaration() {
        let alice_src = "person alice name:\"Alice\" gender:female\n";
        let marriage_src = "person bob name:\"Bob\" gender:male\nmarriage m alice bob start:2010\n";
        let check = check_project(&[("alice.kul", alice_src), ("marriage.kul", marriage_src)]);
        let alice_ref_offset = marriage_src.find(" alice ").unwrap() + 1;
        let loc = definition(&check, "marriage.kul", alice_ref_offset).expect("location");
        assert_eq!(loc.file, "alice.kul");
        assert_eq!(loc.span, Span { start: 7, end: 12 });
    }
}
