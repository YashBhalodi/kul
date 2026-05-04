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

#[test]
fn validate_quiet_suppresses_ok_line() {
    let path = corpus_root().join("valid/01-single-person.kula");
    Command::cargo_bin("kula")
        .unwrap()
        .args(["validate", "--quiet"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicates::str::is_empty());
}

#[test]
fn validate_stdin_reads_dash() {
    Command::cargo_bin("kula")
        .unwrap()
        .args(["validate", "-"])
        .write_stdin("person alice name:\"Alice\" gender:female\n")
        .assert()
        .success()
        .stdout(contains("<stdin>: ok"));
}

#[test]
fn validate_json_format_emits_jsonl() {
    let path = corpus_root().join("invalid/rule-03-missing-name.kula");
    let output = Command::cargo_bin("kula")
        .unwrap()
        .args(["validate", "--format", "json"])
        .arg(&path)
        .output()
        .expect("run kula");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    let line = stdout.lines().next().expect("at least one diagnostic");
    let value: serde_json::Value = serde_json::from_str(line).expect("valid json");
    assert_eq!(value["code"], "KULA-R03");
    assert_eq!(value["severity"], "error");
    assert!(value["primary"]["line"].is_u64());
    assert!(value["primary"]["column"].is_u64());
}

#[test]
fn validate_multiple_files_exits_one_if_any_fail() {
    let valid = corpus_root().join("valid/01-single-person.kula");
    let invalid = corpus_root().join("invalid/rule-03-missing-name.kula");
    Command::cargo_bin("kula")
        .unwrap()
        .args(["validate"])
        .arg(&valid)
        .arg(&invalid)
        .assert()
        .failure()
        .code(1);
}
