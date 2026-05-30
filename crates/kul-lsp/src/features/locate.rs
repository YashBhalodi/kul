//! `kul/locate` custom LSP request.
//!
//! Resolves a project-wide entity id (a person or a marriage) to the
//! [`Location`] of its declaration's id token, so the VSCode preview
//! panel can jump from a rendered card or marriage bar back to source.
//!
//! Persons and marriages share one project-wide id namespace (ADR-0015),
//! so the request needs only `{ uri, id }` — no kind discriminator.
//! Resolution reuses the same seam goto-definition walks:
//! [`ResolvedDocument::entity`] →
//! [`EntityRef::span`](kul_core::semantic::EntityRef::span) (the id-token
//! [`FileSpan`](kul_core::span::FileSpan)) →
//! [`ProjectEntry::location_for`].
//!
//! An id that resolves to no live declaration is not an error: the
//! response carries `location: null`. Only a URI that is not open in the
//! language server is a request error (mirrors [`crate::features::render`]).

use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::{Location, Url};

use crate::state::ProjectEntry;

/// Request parameters for `kul/locate`. Camel-case to match LSP custom
/// requests, which conventionally mirror the protocol's casing.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocateParams {
    /// Any document in the project to search. Must already be open
    /// (`textDocument/didOpen`). Resolution is project-wide, so the URI
    /// identifies the project, not the file the id lives in.
    pub uri: Url,
    /// The entity id to resolve — a person id (`data-person-id`) or a
    /// marriage id (`data-marriage-id`) from the rendered SVG.
    pub id: String,
}

/// `kul/locate` response. `location` is `null` when `id` resolves to no
/// live declaration in the project (e.g. a stale id after an edit).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocateResponse {
    /// The declaration's id-token [`Location`], or `None` for an
    /// unresolved id.
    pub location: Option<Location>,
}

/// Pure projection: resolve `id` against a cached [`ProjectEntry`] to the
/// declaration's id-token [`Location`]. Lives outside `Backend` so the
/// unit tests can exercise it without spawning the full LSP server.
///
/// Returns `LocateResponse { location: None }` for an id that has no live
/// declaration — that is a successful "nothing to jump to" answer, not an
/// error.
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
        // Range points at `alice` in `person alice ...` (line 0, cols 7..12).
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
        // The marriage id `m` lives on line 2 at byte offset "marriage ".len().
        assert_eq!(loc.range.start.line, 2);
        assert_eq!(loc.range.start.character as usize, "marriage ".len());
    }

    /// Cross-file resolution: an id declared in a sibling `.kul` file
    /// resolves to that file's [`Location`]. This is the project-wide
    /// namespace payoff (ADR-0015) — the preview panel can be opened on
    /// one file and jump into another.
    #[test]
    fn resolves_id_declared_in_sibling_file() {
        let alice_src = "person alice name:\"Alice\" gender:female\n";
        let marriage_src = "person bob name:\"Bob\" gender:male\nmarriage m alice bob start:2010\n";
        let entry = test_project_entry(&[("alice.kul", alice_src), ("marriage.kul", marriage_src)]);
        let marriage_url = Url::parse("file:///marriage.kul").unwrap();
        let alice_url = Url::parse("file:///alice.kul").unwrap();
        // The preview was opened on marriage.kul; the clicked card carries
        // `alice`, whose declaration lives in alice.kul.
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
        // alice's decl id span starts at byte 7 of alice.kul.
        assert_eq!(loc.range.start.line, 0);
        assert_eq!(loc.range.start.character, 7);
    }

    #[test]
    fn unknown_id_resolves_to_null_location() {
        let src = "person alice name:\"A\" gender:female\n";
        assert!(locate_id(src, "nobody").is_none());
    }
}
