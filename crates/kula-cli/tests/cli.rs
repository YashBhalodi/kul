//! End-to-end CLI smoke tests.

use std::path::PathBuf;

use assert_cmd::Command;
use predicates::str::contains;

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("kula-core")
        .join("tests")
        .join("corpus")
}

#[test]
fn validate_valid_file_exits_zero() {
    let path = corpus_root().join("valid/01-single-person.kula");
    Command::cargo_bin("kula")
        .unwrap()
        .args(["validate"])
        .arg(&path)
        .assert()
        .success()
        .stdout(contains("ok"));
}

#[test]
fn validate_missing_name_exits_one() {
    let path = corpus_root().join("invalid/rule-03-missing-name.kula");
    Command::cargo_bin("kula")
        .unwrap()
        .args(["validate"])
        .arg(&path)
        .assert()
        .failure()
        .code(1)
        .stderr(contains("KULA-R03"))
        .stderr(contains("missing required field `name`"));
}

#[test]
fn validate_missing_gender_exits_one() {
    let path = corpus_root().join("invalid/rule-03-missing-gender.kula");
    Command::cargo_bin("kula")
        .unwrap()
        .args(["validate"])
        .arg(&path)
        .assert()
        .failure()
        .code(1)
        .stderr(contains("KULA-R03"))
        .stderr(contains("missing required field `gender`"));
}

#[test]
fn version_flag_prints_both_versions() {
    Command::cargo_bin("kula")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains("kula-core"));
}

#[test]
fn validate_couple_is_clean() {
    let path = corpus_root().join("valid/03-couple.kula");
    Command::cargo_bin("kula")
        .unwrap()
        .args(["validate"])
        .arg(&path)
        .assert()
        .success();
}

#[test]
fn validate_duplicate_id_reports_rule_01() {
    let path = corpus_root().join("invalid/rule-01-duplicate-id.kula");
    Command::cargo_bin("kula")
        .unwrap()
        .args(["validate"])
        .arg(&path)
        .assert()
        .failure()
        .code(1)
        .stderr(contains("KULA-R01"))
        .stderr(contains("duplicate id"));
}

#[test]
fn validate_self_marriage_reports_rule_04() {
    let path = corpus_root().join("invalid/rule-04-self-marriage.kula");
    Command::cargo_bin("kula")
        .unwrap()
        .args(["validate"])
        .arg(&path)
        .assert()
        .failure()
        .code(1)
        .stderr(contains("KULA-R04"))
        .stderr(contains("spouses must be distinct"));
}
