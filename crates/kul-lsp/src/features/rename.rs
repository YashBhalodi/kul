//! Rename for `textDocument/prepareRename` and `textDocument/rename`.
//!
//! Two stages: [`prepare_rename`] advertises the editable range,
//! [`rename`] validates the new name and returns a project-wide
//! [`WorkspaceEdit`] covering decl + every reference (ADR-0015).

use std::collections::HashMap;

use kul_core::lexer::{is_identifier, is_reserved_word};
use kul_core::semantic::ResolvedDocument;
use kul_core::span::{FileId, FileSpan};
use tower_lsp::lsp_types::{Position, PrepareRenameResponse, TextEdit, Url, WorkspaceEdit};

use crate::convert::LineIndex;
use crate::state::ProjectEntry;

/// What went wrong with a rename request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameError {
    NotRenameable,
    /// Cursor on an unresolved reference — no decl to anchor on.
    UnresolvedReference,
    /// Doesn't match `[A-Za-z_][A-Za-z0-9_-]*`.
    InvalidIdentifier {
        proposed: String,
    },
    ReservedKeyword {
        proposed: String,
    },
    /// Collides with another id in the project.
    Collision {
        proposed: String,
    },
}

impl RenameError {
    pub fn message(&self) -> String {
        match self {
            RenameError::NotRenameable => {
                "the cursor isn't on a person id or a marriage id — rename only applies to declared ids and their references".to_owned()
            }
            RenameError::UnresolvedReference => {
                "this reference doesn't resolve to a declaration — fix the spelling first, then rename the declaration".to_owned()
            }
            RenameError::InvalidIdentifier { proposed } => format!(
                "`{proposed}` isn't a valid id — ids must start with a letter or underscore and contain only letters, digits, `_`, or `-`"
            ),
            RenameError::ReservedKeyword { proposed } => format!(
                "`{proposed}` is a reserved keyword in Kul — pick a different id"
            ),
            RenameError::Collision { proposed } => format!(
                "`{proposed}` is already used by another person or marriage in this project — every id must be unique"
            ),
        }
    }
}

/// Indicate whether a rename is possible at the cursor, and the editable
/// range the client should show in its rename popover.
pub fn prepare_rename(
    file: FileId,
    resolved: &ResolvedDocument,
    line_index: &LineIndex,
    byte_offset: usize,
) -> Option<PrepareRenameResponse> {
    let entity = resolved
        .node_at(file, byte_offset)?
        .entity_reference(file)?;
    // Don't advertise rename for an unresolved reference.
    if !entity.is_decl && entity.target.is_none() {
        return None;
    }
    Some(PrepareRenameResponse::Range(
        line_index.range(entity.ident_span.span),
    ))
}

/// Rename the id under the cursor to `new_name`, project-wide.
pub fn rename(
    entry: &ProjectEntry,
    uri: &Url,
    position: Position,
    new_name: &str,
) -> Result<WorkspaceEdit, RenameError> {
    let c = entry
        .cursor_for_uri(uri, position)
        .ok_or(RenameError::NotRenameable)?;
    let entity = c.entity().ok_or(RenameError::NotRenameable)?;
    let decl_span: FileSpan = entity.decl_span().ok_or(RenameError::UnresolvedReference)?;
    let current = entity.name;
    let kind = entity.kind;

    // No-op rename: empty workspace edit so clients don't loop on it.
    if new_name == current {
        return Ok(WorkspaceEdit::default());
    }

    if !is_identifier(new_name) {
        return Err(RenameError::InvalidIdentifier {
            proposed: new_name.to_owned(),
        });
    }
    if is_reserved_word(new_name) {
        return Err(RenameError::ReservedKeyword {
            proposed: new_name.to_owned(),
        });
    }
    if c.resolved.entity(new_name).is_some() {
        return Err(RenameError::Collision {
            proposed: new_name.to_owned(),
        });
    }

    // Group spans by file so each file gets one Vec<TextEdit> against its own LineIndex.
    let mut spans: Vec<FileSpan> = c.resolved.references_to(current, kind);
    spans.push(decl_span);
    spans.sort_by_key(|s| (s.file.as_u32(), s.span.start));
    spans.dedup();

    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
    for fs in spans {
        let Some(url) = entry.url_for(fs.file) else {
            continue;
        };
        let Some(line_index) = entry.line_index_for(fs.file) else {
            continue;
        };
        changes.entry(url.clone()).or_default().push(TextEdit {
            range: line_index.range(fs.span),
            new_text: new_name.to_owned(),
        });
    }

    Ok(WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{idx, position_for, test_open_file, test_project_entry, test_url as url};

    fn run_rename(
        source: &str,
        offset: usize,
        new_name: &str,
    ) -> Result<WorkspaceEdit, RenameError> {
        let doc = test_open_file(source);
        rename(&doc, &url(), position_for(source, offset), new_name)
    }

    fn run_prepare(source: &str, offset: usize) -> Option<PrepareRenameResponse> {
        let doc = test_open_file(source);
        let v = doc.view();
        prepare_rename(v.file, v.resolved, v.line_index, offset)
    }

    #[test]
    fn prepare_rename_on_person_decl_returns_range() {
        let src = "person alice name:\"A\" gender:female\n";
        let resp = run_prepare(src, idx(src, "alice")).unwrap();
        match resp {
            PrepareRenameResponse::Range(r) => {
                assert_eq!(r.start.character, 7);
                assert_eq!(r.end.character, 12);
            }
            _ => panic!("expected Range"),
        }
    }

    #[test]
    fn prepare_rename_on_keyword_returns_none() {
        let src = "person alice name:\"A\" gender:female\n";
        assert!(run_prepare(src, 0).is_none());
    }

    #[test]
    fn prepare_rename_on_unresolved_ref_returns_none() {
        let src = "marriage m ghost b start:1972\nperson b name:\"B\" gender:male\n";
        let marriage_line = idx(src, "marriage ");
        let ghost = src[marriage_line..]
            .find("ghost")
            .map(|i| marriage_line + i)
            .unwrap();
        assert!(run_prepare(src, ghost).is_none());
    }

    #[test]
    fn rename_person_updates_decl_and_refs() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:1972\n";
        let we = run_rename(src, idx(src, "alice"), "alicia").unwrap();
        let edits = &we.changes.unwrap()[&url()];
        assert_eq!(edits.len(), 2);
        assert!(edits.iter().all(|e| e.new_text == "alicia"));
    }

    #[test]
    fn rename_marriage_updates_decl_and_refs() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1972\n\
                   person kid name:\"K\" gender:other\n  birth m\n";
        let m_decl = idx(src, "marriage m") + "marriage ".len();
        let we = run_rename(src, m_decl, "m_a_b").unwrap();
        let edits = &we.changes.unwrap()[&url()];
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn rename_from_reference_works_same_as_decl() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:1972\n";
        let from_decl = run_rename(src, idx(src, "alice"), "alicia")
            .unwrap()
            .changes
            .unwrap();
        let marriage_line = idx(src, "marriage ");
        let alice_ref = src[marriage_line..]
            .find(" alice ")
            .map(|i| marriage_line + i + 1)
            .unwrap();
        let from_ref = run_rename(src, alice_ref, "alicia")
            .unwrap()
            .changes
            .unwrap();
        assert_eq!(from_decl, from_ref);
    }

    #[test]
    fn rename_to_invalid_identifier_returns_error() {
        let src = "person alice name:\"A\" gender:female\n";
        let err = run_rename(src, idx(src, "alice"), "1bad").unwrap_err();
        assert!(matches!(err, RenameError::InvalidIdentifier { .. }));
        let err = run_rename(src, idx(src, "alice"), "has space").unwrap_err();
        assert!(matches!(err, RenameError::InvalidIdentifier { .. }));
        let err = run_rename(src, idx(src, "alice"), "").unwrap_err();
        assert!(matches!(err, RenameError::InvalidIdentifier { .. }));
        let err = run_rename(src, idx(src, "alice"), "weird!").unwrap_err();
        assert!(matches!(err, RenameError::InvalidIdentifier { .. }));
    }

    #[test]
    fn rename_to_reserved_keyword_returns_error() {
        let src = "person alice name:\"A\" gender:female\n";
        for kw in [
            "person",
            "marriage",
            "birth",
            "adoption",
            "name",
            "gender",
            "born",
            "start",
            "end",
            "end_reason",
            "divorce",
            "male",
            "female",
            "other",
        ] {
            let err = run_rename(src, idx(src, "alice"), kw).unwrap_err();
            assert!(
                matches!(err, RenameError::ReservedKeyword { .. }),
                "rename to `{kw}` should fail with ReservedKeyword, got {err:?}",
            );
        }
    }

    #[test]
    fn rename_to_existing_id_returns_error() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n";
        let err = run_rename(src, idx(src, "alice"), "bob").unwrap_err();
        assert!(matches!(err, RenameError::Collision { .. }));
    }

    #[test]
    fn rename_to_existing_marriage_id_returns_error() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:1972\n";
        let err = run_rename(src, idx(src, "alice"), "m").unwrap_err();
        assert!(matches!(err, RenameError::Collision { .. }));
    }

    #[test]
    fn rename_no_op_returns_empty_edit() {
        let src = "person alice name:\"A\" gender:female\n";
        let we = run_rename(src, idx(src, "alice"), "alice").unwrap();
        assert!(we.changes.is_none() || we.changes.as_ref().unwrap().is_empty());
    }

    #[test]
    fn rename_on_unresolved_reference_returns_error() {
        let src = "marriage m ghost b start:1972\nperson b name:\"B\" gender:male\n";
        let marriage_line = idx(src, "marriage ");
        let ghost = src[marriage_line..]
            .find("ghost")
            .map(|i| marriage_line + i)
            .unwrap();
        let err = run_rename(src, ghost, "spirit").unwrap_err();
        assert!(matches!(err, RenameError::UnresolvedReference));
    }

    #[test]
    fn rename_on_keyword_returns_error() {
        let src = "person alice name:\"A\" gender:female\n";
        let err = run_rename(src, 0, "x").unwrap_err();
        assert!(matches!(err, RenameError::NotRenameable));
    }

    #[test]
    fn rename_error_messages_are_actionable() {
        let invalid = RenameError::InvalidIdentifier {
            proposed: "1bad".into(),
        };
        assert!(invalid.message().contains("1bad"));
        let reserved = RenameError::ReservedKeyword {
            proposed: "person".into(),
        };
        assert!(reserved.message().contains("person"));
        let collision = RenameError::Collision {
            proposed: "bob".into(),
        };
        assert!(collision.message().contains("bob"));
    }

    #[test]
    fn snapshot_rename_workspace_edit() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bob name:\"Bob\" gender:male\n\
                   person carol name:\"Carol\" gender:female\n\
                   marriage m1 alice bob start:1972\n\
                   marriage m2 alice carol start:2000\n";
        let we = run_rename(src, idx(src, "alice"), "alicia").unwrap();
        insta::assert_json_snapshot!(we);
    }

    /// Cross-file rename (ADR-0015).
    #[test]
    fn rename_spans_every_project_file() {
        let alice_src = "person alice name:\"Alice\" gender:female\n";
        let marriage_src = "person bob name:\"Bob\" gender:male\nmarriage m alice bob start:2010\n";
        let entry = test_project_entry(&[("alice.kul", alice_src), ("marriage.kul", marriage_src)]);
        let alice_url = Url::parse("file:///alice.kul").unwrap();
        let marriage_url = Url::parse("file:///marriage.kul").unwrap();
        let we = rename(
            &entry,
            &alice_url,
            position_for(alice_src, alice_src.find("alice").unwrap()),
            "alicia",
        )
        .unwrap();
        let changes = we.changes.unwrap();
        assert!(changes.contains_key(&alice_url));
        assert!(changes.contains_key(&marriage_url));
        assert_eq!(changes[&alice_url].len(), 1);
        assert_eq!(changes[&marriage_url].len(), 1);
    }
}
