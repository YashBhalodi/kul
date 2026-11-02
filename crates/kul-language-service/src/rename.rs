//! Rename an id across the project.
//!
//! Two stages: [`prepare_rename`] reports the editable range, [`rename`]
//! validates the new name and returns one [`FileEdit`] per affected file,
//! covering the declaration and every reference (ADR-0015).

use kul_core::CheckResult;
use kul_core::lexer::{is_identifier, is_reserved_word};
use kul_core::span::FileSpan;

use crate::{FileEdit, Span, TextEdit, entity_under, file_id};

/// Why a rename was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameError {
    /// The cursor is not on a person or marriage id.
    NotRenameable,
    /// The cursor is on a reference that names no declaration.
    UnresolvedReference,
    /// The new name doesn't match `[A-Za-z_][A-Za-z0-9_-]*`.
    InvalidIdentifier {
        /// The refused name.
        proposed: String,
    },
    /// The new name is a Kul keyword.
    ReservedKeyword {
        /// The refused name.
        proposed: String,
    },
    /// The new name is already another id in the project.
    Collision {
        /// The refused name.
        proposed: String,
    },
}

impl RenameError {
    /// The refusal, written for the author.
    #[must_use]
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

/// The range of the id a rename at `offset` in `file` would change, or
/// `None` when nothing there can be renamed (including an unresolved
/// reference, which has no declaration to anchor on).
#[must_use]
pub fn prepare_rename(check: &CheckResult, file: &str, offset: usize) -> Option<Span> {
    let entity = entity_under(check, file_id(check, file)?, offset)?;
    if !entity.is_decl && entity.target.is_none() {
        return None;
    }
    Some(entity.ident_span.span.into())
}

/// Rename the id at `offset` in `file` to `new_name`, project-wide. The
/// edits come one [`FileEdit`] per affected file, in file order; renaming
/// an id to itself yields no edits.
///
/// # Errors
///
/// A [`RenameError`] when the cursor is not on a renameable id, or
/// `new_name` is not a valid, unused, non-keyword id.
pub fn rename(
    check: &CheckResult,
    file: &str,
    offset: usize,
    new_name: &str,
) -> Result<Vec<FileEdit>, RenameError> {
    let file = file_id(check, file).ok_or(RenameError::NotRenameable)?;
    let entity = entity_under(check, file, offset).ok_or(RenameError::NotRenameable)?;
    let decl_span: FileSpan = entity.decl_span().ok_or(RenameError::UnresolvedReference)?;
    let current = entity.name;

    if new_name == current {
        return Ok(Vec::new());
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
    if check.resolved().entity(new_name).is_some() {
        return Err(RenameError::Collision {
            proposed: new_name.to_owned(),
        });
    }

    let mut spans: Vec<FileSpan> = check.resolved().references_to(current, entity.kind);
    spans.push(decl_span);
    spans.sort_by_key(|s| (s.file.as_u32(), s.span.start));
    spans.dedup();

    let document = check.document();
    let mut edits: Vec<FileEdit> = Vec::new();
    for fs in spans {
        let Some(name) = document.name_of(fs.file) else {
            continue;
        };
        let edit = TextEdit {
            span: fs.span.into(),
            text: new_name.to_owned(),
        };
        match edits.last_mut() {
            Some(last) if last.file == name => last.edits.push(edit),
            _ => edits.push(FileEdit {
                file: name.to_owned(),
                edits: vec![edit],
            }),
        }
    }
    Ok(edits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FILE, check_one, check_project, idx};

    fn run_rename(
        source: &str,
        offset: usize,
        new_name: &str,
    ) -> Result<Vec<FileEdit>, RenameError> {
        rename(&check_one(source), FILE, offset, new_name)
    }

    fn run_prepare(source: &str, offset: usize) -> Option<Span> {
        prepare_rename(&check_one(source), FILE, offset)
    }

    #[test]
    fn prepare_rename_on_person_decl_returns_range() {
        let src = "person alice name:\"A\" gender:female\n";
        assert_eq!(
            run_prepare(src, idx(src, "alice")),
            Some(Span { start: 7, end: 12 })
        );
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
        let edits = run_rename(src, idx(src, "alice"), "alicia").unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].file, FILE);
        assert_eq!(edits[0].edits.len(), 2);
        assert!(edits[0].edits.iter().all(|e| e.text == "alicia"));
    }

    #[test]
    fn rename_marriage_updates_decl_and_refs() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b start:1972\n\
                   person kid name:\"K\" gender:other\n  birth m\n";
        let m_decl = idx(src, "marriage m") + "marriage ".len();
        let edits = run_rename(src, m_decl, "m_a_b").unwrap();
        assert_eq!(edits[0].edits.len(), 2);
    }

    #[test]
    fn rename_from_reference_works_same_as_decl() {
        let src = "person alice name:\"A\" gender:female\n\
                   person bob name:\"B\" gender:male\n\
                   marriage m alice bob start:1972\n";
        let from_decl = run_rename(src, idx(src, "alice"), "alicia").unwrap();
        let marriage_line = idx(src, "marriage ");
        let alice_ref = src[marriage_line..]
            .find(" alice ")
            .map(|i| marriage_line + i + 1)
            .unwrap();
        let from_ref = run_rename(src, alice_ref, "alicia").unwrap();
        assert_eq!(from_decl, from_ref);
    }

    #[test]
    fn rename_to_invalid_identifier_returns_error() {
        let src = "person alice name:\"A\" gender:female\n";
        for bad in ["1bad", "has space", "", "weird!"] {
            let err = run_rename(src, idx(src, "alice"), bad).unwrap_err();
            assert!(matches!(err, RenameError::InvalidIdentifier { .. }));
        }
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
    fn rename_no_op_returns_no_edits() {
        let src = "person alice name:\"A\" gender:female\n";
        assert!(
            run_rename(src, idx(src, "alice"), "alice")
                .unwrap()
                .is_empty()
        );
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
    fn snapshot_rename_file_edits() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bob name:\"Bob\" gender:male\n\
                   person carol name:\"Carol\" gender:female\n\
                   marriage m1 alice bob start:1972\n\
                   marriage m2 alice carol start:2000\n";
        let edits = run_rename(src, idx(src, "alice"), "alicia").unwrap();
        insta::assert_json_snapshot!(edits);
    }

    /// Cross-file rename (ADR-0015).
    #[test]
    fn rename_spans_every_project_file() {
        let alice_src = "person alice name:\"Alice\" gender:female\n";
        let marriage_src = "person bob name:\"Bob\" gender:male\nmarriage m alice bob start:2010\n";
        let check = check_project(&[("alice.kul", alice_src), ("marriage.kul", marriage_src)]);
        let edits = rename(
            &check,
            "alice.kul",
            alice_src.find("alice").unwrap(),
            "alicia",
        )
        .unwrap();
        let files: Vec<(&str, usize)> = edits
            .iter()
            .map(|e| (e.file.as_str(), e.edits.len()))
            .collect();
        assert_eq!(files, vec![("alice.kul", 1), ("marriage.kul", 1)]);
    }
}
