//! Locate: from an entity id to its declaration, for canvas-to-source
//! navigation.
//!
//! Persons and marriages share one project-wide namespace (ADR-0015), so an
//! id alone names the entity.

use kul_core::CheckResult;

use crate::{Location, location};

/// The id token of the declaration of `id`, or `None` when no person or
/// marriage has that id — a stale id is "nothing to jump to", not an error.
#[must_use]
pub fn locate(check: &CheckResult, id: &str) -> Option<Location> {
    let entity = check.resolved().entity(id)?;
    location(check, entity.span())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Span;
    use crate::test_support::{FILE, check_one, check_project, idx};

    #[test]
    fn person_id_resolves_to_decl_id_token() {
        let src = "person alice name:\"A\" gender:female\n";
        let loc = locate(&check_one(src), "alice").expect("location");
        assert_eq!(loc.file, FILE);
        assert_eq!(loc.span, Span { start: 7, end: 12 });
    }

    #[test]
    fn marriage_id_resolves_to_decl_id_token() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n";
        let loc = locate(&check_one(src), "m").expect("location");
        assert_eq!(loc.span.start, idx(src, "marriage m") + "marriage ".len());
    }

    /// Cross-file resolution (ADR-0015): id declared in a sibling file.
    #[test]
    fn resolves_id_declared_in_sibling_file() {
        let alice_src = "person alice name:\"Alice\" gender:female\n";
        let marriage_src = "person bob name:\"Bob\" gender:male\nmarriage m alice bob start:2010\n";
        let check = check_project(&[("alice.kul", alice_src), ("marriage.kul", marriage_src)]);
        let loc = locate(&check, "alice").expect("location");
        assert_eq!(loc.file, "alice.kul");
        assert_eq!(loc.span, Span { start: 7, end: 12 });
    }

    #[test]
    fn unknown_id_resolves_to_none() {
        let src = "person alice name:\"A\" gender:female\n";
        assert!(locate(&check_one(src), "nobody").is_none());
    }
}
