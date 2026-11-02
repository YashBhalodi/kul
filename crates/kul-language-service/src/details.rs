//! Details: the read-only facts about one entity, for the canvas details
//! panel.
//!
//! Answered by the detail lookup (`kul_core::query::detail`, ADR-0037) over
//! [`ResolvedDocument`](kul_core::semantic::ResolvedDocument)'s one-hop
//! seams — `parents_of`, `spouses_of`, `children_of_marriage` — never by the
//! kinship query engine. Like the export it is strict on errors (ADR-0009):
//! the facts are export shapes, which only a project that passes its checks
//! can fill, so a failing project has no details.

use kul_core::CheckResult;
use kul_core::query::EntityDetail;

/// The facts about the person or marriage called `id`: a person's fields,
/// parents, marriages with their spouses, and children (adoptions are the
/// adoptive parent rows); a marriage's fields, spouses and children. `None`
/// when no entity has that id, or the project has errors.
#[must_use]
pub fn details(check: &CheckResult, id: &str) -> Option<EntityDetail> {
    if check.has_errors() {
        return None;
    }
    kul_core::query::detail(check.resolved(), id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::check_project;

    const FOUNDERS: &str = "person alice name:\"Alice\" gender:female born:~1950\n\
                            person bob name:\"Bob\" gender:male\n\
                            marriage m alice bob start:1972 end:1990 end_reason:divorce\n";
    const CHILDREN: &str = "person carol name:\"Carol\" gender:female\n  birth m\n\
                            person dan name:\"Dan\" gender:male\n  adoption m start:1980\n";

    #[test]
    fn unknown_id_has_no_details() {
        let check = check_project(&[("a.kul", FOUNDERS)]);
        assert!(details(&check, "nobody").is_none());
    }

    #[test]
    fn a_project_with_errors_has_no_details() {
        let check = check_project(&[("a.kul", "person alice gender:female\n")]);
        assert!(check.has_errors());
        assert!(details(&check, "alice").is_none());
    }

    #[test]
    fn snapshot_person_details_across_files() {
        let check = check_project(&[("a.kul", FOUNDERS), ("b.kul", CHILDREN)]);
        insta::assert_json_snapshot!(details(&check, "alice"));
    }

    #[test]
    fn snapshot_marriage_details_across_files() {
        let check = check_project(&[("a.kul", FOUNDERS), ("b.kul", CHILDREN)]);
        insta::assert_json_snapshot!(details(&check, "m"));
    }

    #[test]
    fn snapshot_adopted_person_details() {
        let check = check_project(&[("a.kul", FOUNDERS), ("b.kul", CHILDREN)]);
        insta::assert_json_snapshot!(details(&check, "dan"));
    }
}
