//! Snapshot tests for the `query` seam's **batched detail lookup**.
//!
//! Each snapshot is one `details` answer over the example corpus, serialized
//! as the canvas details panel receives it through the language service's
//! `details`. The cases are the ones a details panel has to answer: a marriage whose children mix `biological` and `adoptive`
//! links, a person whose parent set runs from empty to six via a birth plus
//! two adoptions, a person in several marriages (concurrent and
//! divorced-then-remarried), and an adoption link addressed by its
//! `(childId, marriageId)` pair.
//!
//! The batch cases pin what makes the operation *batched*: several targets,
//! mixed kinds, answered in the order asked, with `null` in the position of a
//! target that names no entity.

mod common;

use std::path::{Path, PathBuf};

use kul_core::CheckResult;
use kul_core::query::{DetailTarget, EntityDetail, details};

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

fn detail_json(check: &CheckResult, targets: &[DetailTarget]) -> String {
    serde_json::to_string_pretty(&details(check.resolved(), targets)).expect("serialize details")
}

fn adoption_project() -> CheckResult {
    check_example("04-adoption-and-belonging", "adoption-and-belonging")
}

// ---- A marriage whose children mix biological and adoptive links ----

#[test]
fn marriage_with_children_of_mixed_link_kinds() {
    // The Mendozas: Mateo by birth, Bayani and Dalisay by adoption.
    let check = adoption_project();
    insta::assert_snapshot!(detail_json(
        &check,
        &[DetailTarget::marriage("m_carlos_rosa")]
    ));
}

// ---- Parent sets from zero to six ----

#[test]
fn person_with_no_parents() {
    // Carlos declares no `birth` and no `adoption` — an empty parent list is
    // the honest answer, not an absent field.
    let check = adoption_project();
    insta::assert_snapshot!(detail_json(&check, &[DetailTarget::person("carlos")]));
}

#[test]
fn person_with_two_parents_by_birth_alone() {
    let check = adoption_project();
    insta::assert_snapshot!(detail_json(&check, &[DetailTarget::person("mateo")]));
}

#[test]
fn person_with_two_parents_by_adoption_alone() {
    // Bayani has no birth family on record; only the adoption reaches parents.
    let check = adoption_project();
    insta::assert_snapshot!(detail_json(&check, &[DetailTarget::person("bayani")]));
}

#[test]
fn person_with_six_parents_by_birth_plus_two_adoptions() {
    // Dalisay: born to Eduardo & Luz, adopted by Tomas & Elena (ended), then
    // by Carlos & Rosa — six parent rows, one per link, in declaration order.
    let check = adoption_project();
    insta::assert_snapshot!(detail_json(&check, &[DetailTarget::person("dalisay")]));
}

// ---- A person in several marriages ----

#[test]
fn person_with_three_concurrent_marriages() {
    // Khalid hosts three marriages; his children hang off all three.
    let check = check_example("06-polygamous-household", "polygamous-household");
    insta::assert_snapshot!(detail_json(&check, &[DetailTarget::person("khalid")]));
}

#[test]
fn person_with_an_ended_marriage_and_a_later_one() {
    // Erik: one divorced marriage (with `end` and `endReason`) and one
    // ongoing — each its own row, which is what explains two cards.
    let check = check_example("03-divorce-and-remarriage", "divorce-and-remarriage");
    insta::assert_snapshot!(detail_json(&check, &[DetailTarget::person("erik")]));
}

// ---- An adoption entity, addressed by (childId, marriageId) ----

#[test]
fn adoption_with_a_start_and_an_end() {
    let check = adoption_project();
    insta::assert_snapshot!(detail_json(
        &check,
        &[DetailTarget::adoption("dalisay", "m_tomas_elena")]
    ));
}

#[test]
fn adoption_with_a_start_only() {
    let check = adoption_project();
    insta::assert_snapshot!(detail_json(
        &check,
        &[DetailTarget::adoption("bayani", "m_carlos_rosa")]
    ));
}

// ---- The batch itself ----

#[test]
fn one_batch_answers_mixed_entity_kinds_in_order() {
    let check = adoption_project();
    insta::assert_snapshot!(detail_json(
        &check,
        &[
            DetailTarget::person("rosa"),
            DetailTarget::marriage("m_eduardo_luz"),
            DetailTarget::adoption("dalisay", "m_carlos_rosa"),
        ]
    ));
}

#[test]
fn batch_of_person_targets_carries_each_display_name() {
    // One call, N person ids, `person.name` on each.
    let check = check_example("06-polygamous-household", "polygamous-household");
    let targets: Vec<DetailTarget> = ["yusuf", "zahra", "hassan", "noor"]
        .into_iter()
        .map(DetailTarget::person)
        .collect();
    let names: Vec<String> = details(check.resolved(), &targets)
        .into_iter()
        .map(|entry| match entry.expect("every id is a known person") {
            EntityDetail::Person { person, .. } => person.name,
            other => panic!("expected a person detail, got {other:?}"),
        })
        .collect();
    assert_eq!(
        names,
        [
            "Yusuf Al-Rashid",
            "Zahra Al-Rashid",
            "Hassan Al-Rashid",
            "Noor Al-Rashid"
        ]
    );
}

// ---- Absence is the answer ----

#[test]
fn unknown_and_wrong_kind_targets_are_null_in_position() {
    // An unknown id, a person id asked for as a marriage, and an adoption
    // pair naming a marriage the child was never adopted into: each `null`,
    // each in its own position, while the rest of the batch answers.
    let check = adoption_project();
    insta::assert_snapshot!(detail_json(
        &check,
        &[
            DetailTarget::person("nobody"),
            DetailTarget::marriage("carlos"),
            DetailTarget::adoption("mateo", "m_carlos_rosa"),
            DetailTarget::person("mateo"),
        ]
    ));
}

#[test]
fn empty_batch_is_an_empty_answer() {
    let check = adoption_project();
    insta::assert_snapshot!(detail_json(&check, &[]));
}
