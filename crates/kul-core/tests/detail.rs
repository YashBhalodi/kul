//! Snapshot tests for the `query` seam's **detail lookup**.
//!
//! Each snapshot is one `detail` answer over the example corpus, serialized
//! as the canvas details panel receives it through the language service's
//! `details`. The cases are the ones a details panel has to answer: a
//! marriage whose children mix `biological` and `adoptive` links, a person
//! whose parent set runs from empty to six via a birth plus two adoptions
//! (one of them ended), and a person in several marriages (concurrent, and
//! divorced then remarried).

mod common;

use std::path::{Path, PathBuf};

use kul_core::CheckResult;
use kul_core::query::detail;

use crate::common::check_one;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn check_example(dir: &str, stem: &str) -> CheckResult {
    let path = workspace_root()
        .join("examples")
        .join(dir)
        .join(format!("{stem}.kul"));
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    check_one(&source)
}

fn detail_json(check: &CheckResult, id: &str) -> String {
    serde_json::to_string_pretty(&detail(check.resolved(), id)).expect("serialize the detail")
}

fn adoption_project() -> CheckResult {
    check_example("04-adoption-and-belonging", "adoption-and-belonging")
}

// ---- A marriage whose children mix biological and adoptive links ----

#[test]
fn marriage_with_children_of_mixed_link_kinds() {
    // The Mendozas: Mateo by birth, Bayani and Dalisay by adoption.
    let check = adoption_project();
    insta::assert_snapshot!(detail_json(&check, "m_carlos_rosa"));
}

// ---- Parent sets from zero to six ----

#[test]
fn person_with_no_parents() {
    // Carlos declares no `birth` and no `adoption` — an empty parent list is
    // the honest answer, not an absent field.
    let check = adoption_project();
    insta::assert_snapshot!(detail_json(&check, "carlos"));
}

#[test]
fn person_with_two_parents_by_birth_alone() {
    let check = adoption_project();
    insta::assert_snapshot!(detail_json(&check, "mateo"));
}

#[test]
fn person_with_two_parents_by_adoption_alone() {
    // Bayani has no birth family on record; only the adoption reaches parents.
    let check = adoption_project();
    insta::assert_snapshot!(detail_json(&check, "bayani"));
}

#[test]
fn person_with_six_parents_by_birth_plus_two_adoptions() {
    // Dalisay: born to Eduardo & Luz, adopted by Tomas & Elena (ended), then
    // by Carlos & Rosa — six parent rows, one per link, in declaration order.
    let check = adoption_project();
    insta::assert_snapshot!(detail_json(&check, "dalisay"));
}

// ---- A person in several marriages ----

#[test]
fn person_with_three_concurrent_marriages() {
    // Khalid hosts three marriages; his children hang off all three.
    let check = check_example("06-polygamous-household", "polygamous-household");
    insta::assert_snapshot!(detail_json(&check, "khalid"));
}

#[test]
fn person_with_an_ended_marriage_and_a_later_one() {
    // Erik: one divorced marriage (with `end` and `endReason`) and one
    // ongoing — each its own row, which is what explains two cards.
    let check = check_example("03-divorce-and-remarriage", "divorce-and-remarriage");
    insta::assert_snapshot!(detail_json(&check, "erik"));
}

// ---- Absence is the answer ----

#[test]
fn an_id_naming_no_entity_has_no_detail() {
    let check = adoption_project();
    assert!(detail(check.resolved(), "nobody").is_none());
}
