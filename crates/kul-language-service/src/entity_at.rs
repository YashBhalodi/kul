//! Entity-at: the person or marriage under the cursor, by id.
//!
//! The inverse of [`locate`](crate::locate): it lets the canvas mark the
//! entity the editor cursor rests on. Resolution is project-wide (ADR-0015).
//! A keyword, field, whitespace, end of file or unresolved reference yields
//! nothing.

use kul_core::CheckResult;
use kul_core::semantic::EntityKind as CoreEntityKind;
use serde::Serialize;
#[cfg(feature = "tsify")]
use tsify::Tsify;

use crate::{entity_under, file_id};

/// The two kinds of entity a Kul project declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub enum EntityKind {
    /// A `person` declaration.
    Person,
    /// A `marriage` declaration.
    Marriage,
}

impl From<CoreEntityKind> for EntityKind {
    fn from(kind: CoreEntityKind) -> Self {
        match kind {
            CoreEntityKind::Person => EntityKind::Person,
            CoreEntityKind::Marriage => EntityKind::Marriage,
        }
    }
}

/// A declared entity, by id. The id keys `data-person-id` /
/// `data-marriage-id` in the rendered SVG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    /// The declaration's id.
    pub id: String,
    /// Person or marriage.
    pub kind: EntityKind,
}

/// The entity whose declaration or resolved reference sits at `offset` in
/// `file`. An unresolved reference yields `None`, so the canvas never marks
/// an entity that does not exist.
#[must_use]
pub fn entity_at(check: &CheckResult, file: &str, offset: usize) -> Option<Entity> {
    entity_under(check, file_id(check, file)?, offset)
        .filter(|e| e.target.is_some())
        .map(|e| Entity {
            id: e.name.to_owned(),
            kind: e.kind.into(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FILE, check_one, check_project, idx};

    fn entity_at_offset(source: &str, offset: usize) -> Option<(String, EntityKind)> {
        entity_at(&check_one(source), FILE, offset).map(|e| (e.id, e.kind))
    }

    #[test]
    fn person_decl_resolves_to_person_entity() {
        let src = "person alice name:\"A\" gender:female\n";
        let got = entity_at_offset(src, idx(src, "alice")).expect("entity");
        assert_eq!(got, ("alice".to_owned(), EntityKind::Person));
    }

    #[test]
    fn marriage_decl_resolves_to_marriage_entity() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n";
        let m_decl = idx(src, "marriage m") + "marriage ".len();
        let got = entity_at_offset(src, m_decl).expect("entity");
        assert_eq!(got, ("m".to_owned(), EntityKind::Marriage));
    }

    #[test]
    fn resolved_person_ref_resolves_to_target_entity() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:2010\n";
        let marriage_line = idx(src, "marriage ");
        let alice_ref = src[marriage_line..]
            .find(" alice ")
            .map(|i| marriage_line + i + 1)
            .unwrap();
        let got = entity_at_offset(src, alice_ref).expect("entity");
        assert_eq!(got, ("alice".to_owned(), EntityKind::Person));
    }

    #[test]
    fn resolved_marriage_ref_resolves_to_target_entity() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  birth m\n";
        let m_ref = idx(src, "birth m") + "birth ".len();
        let got = entity_at_offset(src, m_ref).expect("entity");
        assert_eq!(got, ("m".to_owned(), EntityKind::Marriage));
    }

    #[test]
    fn unresolved_reference_resolves_to_none() {
        let src = "marriage m ghost b start:2000\nperson b name:\"B\" gender:male\n";
        let marriage_line = idx(src, "marriage ");
        let ghost = src[marriage_line..]
            .find("ghost")
            .map(|i| marriage_line + i)
            .unwrap();
        assert!(entity_at_offset(src, ghost).is_none());
    }

    #[test]
    fn keyword_resolves_to_none() {
        let src = "person alice name:\"A\" gender:female\n";
        assert!(entity_at_offset(src, 0).is_none());
    }

    #[test]
    fn field_name_resolves_to_none() {
        let src = "person alice name:\"A\" gender:female\n";
        assert!(entity_at_offset(src, idx(src, "name:")).is_none());
    }

    #[test]
    fn field_value_resolves_to_none() {
        let src = "person alice name:\"Alice\" gender:female\n";
        assert!(entity_at_offset(src, idx(src, "\"Alice\"")).is_none());
    }

    #[test]
    fn whitespace_resolves_to_none() {
        let src = "person a name:\"A\" gender:female\n\nperson b name:\"B\" gender:male\n";
        let blank = src.find("\n\n").unwrap() + 1;
        assert!(entity_at_offset(src, blank).is_none());
    }

    #[test]
    fn eof_resolves_to_none() {
        let src = "person a name:\"A\" gender:female\n";
        assert!(entity_at_offset(src, src.len()).is_none());
    }

    /// Cross-file resolution (ADR-0015): cursor in one file, decl in another.
    #[test]
    fn cross_file_resolved_reference() {
        let alice_src = "person alice name:\"Alice\" gender:female\n";
        let marriage_src = "person bob name:\"Bob\" gender:male\nmarriage m alice bob start:2010\n";
        let check = check_project(&[("alice.kul", alice_src), ("marriage.kul", marriage_src)]);
        let alice_ref_offset = marriage_src.find(" alice ").unwrap() + 1;
        let entity = entity_at(&check, "marriage.kul", alice_ref_offset).expect("entity");
        assert_eq!(entity.id, "alice");
        assert_eq!(entity.kind, EntityKind::Person);
    }
}
