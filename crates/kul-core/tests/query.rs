//! Snapshot tests for the `query` seam's id → detail lookups.
//!
//! Looks up known person and marriage ids in three structurally different
//! examples (nuclear, polygamous-household, multi-file) and snapshots the
//! serialized envelope shapes, plus the unknown-id and wrong-kind-id
//! `null`-result cases. The serialization pins the *contract* the WASM and
//! CLI surfaces mirror; kinship correctness is not at stake here.

mod common;

use std::path::{Path, PathBuf};

use kul_core::CheckResult;
use kul_core::ast::InputFile;
use kul_core::manifest::Manifest;
use kul_core::query::{marriage_lookup, person_lookup};

use crate::common::check_one;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn examples_dir() -> PathBuf {
    workspace_root().join("examples")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn check_example(dir: &str, stem: &str) -> CheckResult {
    let path = examples_dir().join(dir).join(format!("{stem}.kul"));
    check_one(&read(&path))
}

fn check_multi_file(dir: &str) -> CheckResult {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(examples_dir().join(dir))
        .expect("read multi-file example directory")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("kul"))
        .collect();
    entries.sort();
    let inputs: Vec<InputFile> = entries
        .iter()
        .map(|p| {
            InputFile::new(
                p.file_name().unwrap().to_string_lossy().into_owned(),
                read(p),
            )
        })
        .collect();
    kul_core::check_with_manifest("kul.yml", "", &Manifest::default(), &inputs)
}

fn person_json(check: &CheckResult, id: &str) -> String {
    serde_json::to_string_pretty(&person_lookup(check, id)).expect("serialize person envelope")
}

fn marriage_json(check: &CheckResult, id: &str) -> String {
    serde_json::to_string_pretty(&marriage_lookup(check, id)).expect("serialize marriage envelope")
}

// ---- Nuclear family (single file, simplest topology) ----

#[test]
fn nuclear_person_lookup() {
    let check = check_example("01-nuclear-family", "nuclear-family");
    insta::assert_snapshot!(person_json(&check, "hiroshi"));
}

#[test]
fn nuclear_marriage_lookup() {
    let check = check_example("01-nuclear-family", "nuclear-family");
    insta::assert_snapshot!(marriage_json(&check, "m_hiroshi_yuki"));
}

// ---- Polygamous household (a person in multiple marriages) ----

#[test]
fn polygamous_person_lookup() {
    let check = check_example("06-polygamous-household", "polygamous-household");
    insta::assert_snapshot!(person_json(&check, "khalid"));
}

#[test]
fn polygamous_marriage_lookup() {
    let check = check_example("06-polygamous-household", "polygamous-household");
    insta::assert_snapshot!(marriage_json(&check, "m_khalid_aisha"));
}

// ---- Multi-file project (cross-file id resolution) ----

#[test]
fn multi_file_person_lookup() {
    let check = check_multi_file("08-multi-file-project");
    insta::assert_snapshot!(person_json(&check, "diego"));
}

#[test]
fn multi_file_marriage_lookup() {
    let check = check_multi_file("08-multi-file-project");
    insta::assert_snapshot!(marriage_json(&check, "m_diego_carmen"));
}

// ---- Absence-is-the-answer: unknown id and wrong-kind id ----

#[test]
fn unknown_person_id_yields_null_result() {
    let check = check_example("01-nuclear-family", "nuclear-family");
    insta::assert_snapshot!(person_json(&check, "nobody"));
}

#[test]
fn unknown_marriage_id_yields_null_result() {
    let check = check_example("01-nuclear-family", "nuclear-family");
    insta::assert_snapshot!(marriage_json(&check, "nobody"));
}

#[test]
fn wrong_kind_person_id_yields_null_result() {
    // `m_hiroshi_yuki` names a marriage, not a person → null.
    let check = check_example("01-nuclear-family", "nuclear-family");
    insta::assert_snapshot!(person_json(&check, "m_hiroshi_yuki"));
}

#[test]
fn wrong_kind_marriage_id_yields_null_result() {
    // `hiroshi` names a person, not a marriage → null.
    let check = check_example("01-nuclear-family", "nuclear-family");
    insta::assert_snapshot!(marriage_json(&check, "hiroshi"));
}

// ---- Failing project: error envelope, never a partial answer ----

#[test]
fn failing_project_yields_error_envelope() {
    let check = check_one("person alice gender:female\n"); // missing name → KUL-R03
    insta::assert_snapshot!(person_json(&check, "alice"));
}
