//! `kul/locate` custom LSP request.
//!
//! Resolves a project-wide entity id (ADR-0015) to the declaration's
//! id-token [`Location`] so the preview panel can click-to-source.
//! Persons and marriages share one id namespace, so `{ uri, id }` suffices.

use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::{Location, Url};

use crate::state::ProjectEntry;

/// Request parameters for `kul/locate`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocateParams {
    /// Any open document in the project — identifies the project, not the
    /// file the id lives in.
    pub uri: Url,
    /// The entity id from the rendered SVG (`data-person-id` / `data-marriage-id`).
    pub id: String,
}

/// `kul/locate` response. `location` is `null` for an id with no live decl.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocateResponse {
    pub location: Option<Location>,
}

/// Resolve `id` to the declaration's id-token [`Location`]. A stale id
/// yields `location: None` — a successful "nothing to jump to", not an error.
pub fn locate(entry: &ProjectEntry, params: &LocateParams) -> LocateResponse {
    let location = entry
        .check
        .resolved()
        .entity(&params.id)
        .and_then(|e| entry.location_for(e.span()));
    LocateResponse { location }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{test_open_file, test_project_entry, test_url as url};

    fn locate_id(source: &str, id: &str) -> Option<Location> {
        let doc = test_open_file(source);
        locate(
            &doc,
            &LocateParams {
                uri: url(),
                id: id.to_owned(),
            },
        )
        .location
    }

    #[test]
    fn person_id_resolves_to_decl_id_token() {
        let src = "person alice name:\"A\" gender:female\n";
        let loc = locate_id(src, "alice").expect("location");
        assert_eq!(loc.uri, url());
        assert_eq!(loc.range.start.line, 0);
        assert_eq!(loc.range.start.character, 7);
        assert_eq!(loc.range.end.line, 0);
        assert_eq!(loc.range.end.character, 12);
    }

    #[test]
    fn marriage_id_resolves_to_decl_id_token() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1980\n";
        let loc = locate_id(src, "m").expect("location");
        assert_eq!(loc.range.start.line, 2);
        assert_eq!(loc.range.start.character as usize, "marriage ".len());
    }

    /// Cross-file resolution (ADR-0015): id declared in a sibling file.
    #[test]
    fn resolves_id_declared_in_sibling_file() {
        let alice_src = "person alice name:\"Alice\" gender:female\n";
        let marriage_src = "person bob name:\"Bob\" gender:male\nmarriage m alice bob start:2010\n";
        let entry = test_project_entry(&[("alice.kul", alice_src), ("marriage.kul", marriage_src)]);
        let marriage_url = Url::parse("file:///marriage.kul").unwrap();
        let alice_url = Url::parse("file:///alice.kul").unwrap();
        let loc = locate(
            &entry,
            &LocateParams {
                uri: marriage_url,
                id: "alice".to_owned(),
            },
        )
        .location
        .expect("location");
        assert_eq!(loc.uri, alice_url);
        assert_eq!(loc.range.start.line, 0);
        assert_eq!(loc.range.start.character, 7);
    }

    #[test]
    fn unknown_id_resolves_to_null_location() {
        let src = "person alice name:\"A\" gender:female\n";
        assert!(locate_id(src, "nobody").is_none());
    }
}
