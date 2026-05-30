//! `kul/entityAt` custom LSP request.
//!
//! Inverse of [`crate::features::locate`]: turns a cursor position into the
//! project-wide entity id under it (ADR-0015) so the preview panel can
//! highlight the matching card. A cursor on a keyword, field, whitespace,
//! EOF, or unresolved reference yields `entity: null`.

use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::{Position, Url};

use crate::state::ProjectEntry;

/// Request parameters for `kul/entityAt`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityAtParams {
    /// The document the cursor is in. Must already be open. Resolution is
    /// project-wide (ADR-0015).
    pub uri: Url,
    pub position: Position,
}

/// The entity under the cursor — its resolved declaration id and kind.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    /// The resolved declaration id, keying `data-person-id` / `data-marriage-id` in SVG.
    pub id: String,
    /// `"person"` or `"marriage"`.
    pub kind: String,
}

/// `kul/entityAt` response. `entity` is `null` when the cursor is not on a
/// resolved person/marriage id.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityAtResponse {
    pub entity: Option<Entity>,
}

/// Resolve the cursor in `params` to the entity (decl or resolved
/// reference) under it. A non-entity position yields `entity: None` —
/// that is a successful "nothing selected", not an error. Gating on
/// `target.is_some()` keeps an unresolved reference from reporting a
/// phantom highlight.
pub fn entity_at(entry: &ProjectEntry, params: &EntityAtParams) -> EntityAtResponse {
    let entity = entry
        .cursor_for_uri(&params.uri, params.position)
        .and_then(|c| c.entity())
        .filter(|e| e.target.is_some())
        .map(|e| Entity {
            id: e.name.to_owned(),
            kind: e.kind.as_str().to_owned(),
        });
    EntityAtResponse { entity }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{idx, position_for, test_open_file, test_project_entry, test_url as url};

    fn entity_at_offset(source: &str, offset: usize) -> Option<(String, String)> {
        let doc = test_open_file(source);
        entity_at(
            &doc,
            &EntityAtParams {
                uri: url(),
                position: position_for(source, offset),
            },
        )
        .entity
        .map(|e| (e.id, e.kind))
    }

    #[test]
    fn person_decl_resolves_to_person_entity() {
        let src = "person alice name:\"A\" gender:female\n";
        let got = entity_at_offset(src, idx(src, "alice")).expect("entity");
        assert_eq!(got, ("alice".to_owned(), "person".to_owned()));
    }

    #[test]
    fn marriage_decl_resolves_to_marriage_entity() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n";
        let m_decl = idx(src, "marriage m") + "marriage ".len();
        let got = entity_at_offset(src, m_decl).expect("entity");
        assert_eq!(got, ("m".to_owned(), "marriage".to_owned()));
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
        assert_eq!(got, ("alice".to_owned(), "person".to_owned()));
    }

    #[test]
    fn resolved_marriage_ref_resolves_to_target_entity() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n\
                   person kid name:\"K\" gender:other\n  birth m\n";
        let m_ref = idx(src, "birth m") + "birth ".len();
        let got = entity_at_offset(src, m_ref).expect("entity");
        assert_eq!(got, ("m".to_owned(), "marriage".to_owned()));
    }

    #[test]
    fn unresolved_reference_resolves_to_null() {
        let src = "marriage m ghost b start:2000\nperson b name:\"B\" gender:male\n";
        let marriage_line = idx(src, "marriage ");
        let ghost = src[marriage_line..]
            .find("ghost")
            .map(|i| marriage_line + i)
            .unwrap();
        assert!(entity_at_offset(src, ghost).is_none());
    }

    #[test]
    fn keyword_resolves_to_null() {
        let src = "person alice name:\"A\" gender:female\n";
        assert!(entity_at_offset(src, 0).is_none());
    }

    #[test]
    fn field_name_resolves_to_null() {
        let src = "person alice name:\"A\" gender:female\n";
        assert!(entity_at_offset(src, idx(src, "name:")).is_none());
    }

    #[test]
    fn field_value_resolves_to_null() {
        let src = "person alice name:\"Alice\" gender:female\n";
        assert!(entity_at_offset(src, idx(src, "\"Alice\"")).is_none());
    }

    #[test]
    fn whitespace_resolves_to_null() {
        let src = "person a name:\"A\" gender:female\n\nperson b name:\"B\" gender:male\n";
        let blank = src.find("\n\n").unwrap() + 1;
        assert!(entity_at_offset(src, blank).is_none());
    }

    #[test]
    fn eof_resolves_to_null() {
        let src = "person a name:\"A\" gender:female\n";
        assert!(entity_at_offset(src, src.len()).is_none());
    }

    /// Cross-file resolution (ADR-0015): cursor in one file, decl in another.
    #[test]
    fn cross_file_resolved_reference() {
        let alice_src = "person alice name:\"Alice\" gender:female\n";
        let marriage_src = "person bob name:\"Bob\" gender:male\nmarriage m alice bob start:2010\n";
        let entry = test_project_entry(&[("alice.kul", alice_src), ("marriage.kul", marriage_src)]);
        let marriage_url = Url::parse("file:///marriage.kul").unwrap();
        let alice_ref_offset = marriage_src.find(" alice ").unwrap() + 1;
        let entity = entity_at(
            &entry,
            &EntityAtParams {
                uri: marriage_url,
                position: position_for(marriage_src, alice_ref_offset),
            },
        )
        .entity
        .expect("entity");
        assert_eq!(entity.id, "alice");
        assert_eq!(entity.kind, "person");
    }
}
