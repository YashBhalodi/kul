//! `kul/entityAt` custom LSP request.
//!
//! Maps a source cursor position to the project-wide entity id (a person
//! or a marriage) under it, so the VSCode preview panel can highlight the
//! matching card or marriage bar. This is the inverse of
//! [`crate::features::locate`]: `kul/locate` turns a clicked entity id into
//! a source [`Location`](tower_lsp::lsp_types::Location); `kul/entityAt`
//! turns a cursor position into an entity id.
//!
//! Resolution reuses the same cursor seam goto-definition walks:
//! [`ProjectEntry::cursor_for_uri`] → [`Cursor::entity`]. The reported id
//! is the resolved declaration's id — the decl id when the cursor is on a
//! declaration, the resolved target's decl id when it is on a reference.
//! A cursor on a keyword, field name/value, whitespace, EOF, or an
//! *unresolved* reference yields `entity: null`.
//!
//! Resolution is project-wide (ADR-0015): a cursor in a sibling `.kul`
//! file resolves to the same entity the preview (opened on another file)
//! rendered. Only a URI that is not open in the language server is a
//! request error (mirrors [`crate::features::render`] and
//! [`crate::features::locate`]).

use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::{Position, Url};

use crate::state::ProjectEntry;

/// Request parameters for `kul/entityAt`. Camel-case to match LSP custom
/// requests, which conventionally mirror the protocol's casing.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityAtParams {
    /// The document the cursor is in. Must already be open
    /// (`textDocument/didOpen`). Resolution is project-wide, so a cursor
    /// in a sibling file still resolves against the whole project.
    pub uri: Url,
    /// The cursor position (LSP 0-based line/character).
    pub position: Position,
}

/// The entity under the cursor — its resolved declaration id and kind.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    /// The resolved declaration id (a person id or a marriage id), keying
    /// the rendered SVG's `data-person-id` / `data-marriage-id`.
    pub id: String,
    /// `"person"` or `"marriage"`.
    pub kind: String,
}

/// `kul/entityAt` response. `entity` is `null` when the cursor is not on a
/// resolved person/marriage id (keyword, field, whitespace, EOF, or an
/// unresolved reference).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityAtResponse {
    /// The entity under the cursor, or `None` for a non-entity position.
    pub entity: Option<Entity>,
}

/// Pure projection: resolve the cursor in `params` against a cached
/// [`ProjectEntry`] to the entity (decl or resolved reference) under it.
/// Lives outside `Backend` so the unit tests can exercise it without
/// spawning the full LSP server.
///
/// Returns `EntityAtResponse { entity: None }` for any position that is
/// not a resolved person/marriage id — that is a successful "nothing
/// selected" answer, not an error. Gating on `target.is_some()` keeps an
/// unresolved reference (which has a name but no live declaration) from
/// reporting a phantom highlight.
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

    /// Resolve `(id, kind)` for the cursor at byte `offset` in a single-file
    /// fixture. `None` means `entity: null`.
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
        assert!(entity_at_offset(src, 0).is_none()); // `person` keyword
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

    /// Cross-file resolution: a cursor on a reference whose declaration
    /// lives in a sibling `.kul` file resolves to that declaration's id.
    /// This is the project-wide namespace payoff (ADR-0015) — the cursor
    /// can sit in one file while the preview is opened on another.
    #[test]
    fn cross_file_resolved_reference() {
        let alice_src = "person alice name:\"Alice\" gender:female\n";
        let marriage_src = "person bob name:\"Bob\" gender:male\nmarriage m alice bob start:2010\n";
        let entry = test_project_entry(&[("alice.kul", alice_src), ("marriage.kul", marriage_src)]);
        let marriage_url = Url::parse("file:///marriage.kul").unwrap();
        // Cursor on `alice` inside `marriage m alice bob`, declared in alice.kul.
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
