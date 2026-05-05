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
        .stderr(contains("needs a `name:` field"));
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
        .stderr(contains("needs a `gender:` field"));
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
        .stderr(contains("is already used"));
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

// === `kula format` ===

#[test]
fn format_stdin_writes_canonical_form_to_stdout() {
    let out = Command::cargo_bin("kula")
        .unwrap()
        .args(["format", "-"])
        .write_stdin("person alice  born:1950 name:\"Alice\" gender:female\n")
        .output()
        .expect("run kula format");
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        stdout,
        "person alice  name:\"Alice\"  gender:female  born:1950\n"
    );
}

#[test]
fn format_check_passes_on_canonical_input() {
    Command::cargo_bin("kula")
        .unwrap()
        .args(["format", "--check", "-"])
        .write_stdin("person alice  name:\"Alice\"  gender:female\n")
        .assert()
        .success();
}

#[test]
fn format_check_fails_on_non_canonical_input() {
    Command::cargo_bin("kula")
        .unwrap()
        .args(["format", "--check", "-"])
        .write_stdin("person alice name:\"Alice\" gender:female\n")
        .assert()
        .failure()
        .code(1)
        .stderr(contains("<stdin>: not formatted"));
}

#[test]
fn format_check_passes_on_corpus_examples() {
    // Every example in the workspace must be canonical at HEAD.
    let examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples");
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&examples_dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("kula"))
        .collect();
    entries.sort();
    let mut cmd = Command::cargo_bin("kula").unwrap();
    cmd.args(["format", "--check"]);
    for p in &entries {
        cmd.arg(p);
    }
    cmd.assert().success();
}

#[test]
fn format_rewrites_file_in_place() {
    let dir = tempfile_dir();
    let path = dir.join("alice.kula");
    let dirty = "person alice born:1950 name:\"Alice\" gender:female\n";
    std::fs::write(&path, dirty).unwrap();
    Command::cargo_bin("kula")
        .unwrap()
        .args(["format"])
        .arg(&path)
        .assert()
        .success();
    let after = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        after,
        "person alice  name:\"Alice\"  gender:female  born:1950\n"
    );
}

#[test]
fn format_refuses_input_with_parse_errors() {
    Command::cargo_bin("kula")
        .unwrap()
        .args(["format", "-"])
        .write_stdin("person\n")
        .assert()
        .failure()
        .code(1)
        .stderr(contains("cannot format input with parse errors"));
}

// === `kula export` ===

#[test]
fn export_clean_file_emits_success_envelope_and_exits_zero() {
    let path = examples_dir().join("01-single-couple.kula");
    let output = Command::cargo_bin("kula")
        .unwrap()
        .args(["export"])
        .arg(&path)
        .output()
        .expect("run kula export");
    assert!(output.status.success(), "expected exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let env: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid json");
    assert_eq!(env["ok"], true);
    assert_eq!(env["schema"], 1);
    assert_eq!(env["kula"], "0.1");
    assert!(env["graph"]["persons"].is_array());
    assert!(env["graph"]["marriages"].is_array());
    assert!(env["graph"]["parenthood_links"].is_array());
}

#[test]
fn export_dirty_file_emits_failure_envelope_and_exits_one() {
    let output = Command::cargo_bin("kula")
        .unwrap()
        .args(["export", "-"])
        .write_stdin("person alice gender:female\n")
        .output()
        .expect("run kula export");
    assert_eq!(output.status.code(), Some(1), "expected exit 1");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let env: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid json");
    assert_eq!(env["ok"], false);
    let diags = env["diagnostics"].as_array().expect("diagnostics array");
    assert!(diags.iter().any(|d| d["code"] == "KULA-R03"));
}

#[test]
fn export_stdin_succeeds_on_clean_input() {
    let output = Command::cargo_bin("kula")
        .unwrap()
        .args(["export", "-"])
        .write_stdin("person alice name:\"Alice\" gender:female\n")
        .output()
        .expect("run kula export");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let env: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(env["ok"], true);
    assert_eq!(env["graph"]["persons"][0]["id"], "alice");
}

#[test]
fn export_multiple_files_emits_one_envelope_per_line() {
    let p1 = examples_dir().join("01-single-couple.kula");
    let p2 = examples_dir().join("02-nuclear-family.kula");
    let output = Command::cargo_bin("kula")
        .unwrap()
        .args(["export"])
        .arg(&p1)
        .arg(&p2)
        .output()
        .expect("run kula export");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "expected one envelope per file");
    for line in lines {
        let env: serde_json::Value = serde_json::from_str(line).expect("valid json");
        assert_eq!(env["ok"], true);
    }
}

#[test]
fn export_multiple_files_exits_one_if_any_fail() {
    let valid = examples_dir().join("01-single-couple.kula");
    let invalid_path = corpus_root().join("invalid/rule-03-missing-name.kula");
    let output = Command::cargo_bin("kula")
        .unwrap()
        .args(["export"])
        .arg(&valid)
        .arg(&invalid_path)
        .output()
        .expect("run kula export");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2);
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(first["ok"], true);
    assert_eq!(second["ok"], false);
}

#[test]
fn export_format_json_is_default_and_explicit_flag_works() {
    let path = examples_dir().join("01-single-couple.kula");
    let with_flag = Command::cargo_bin("kula")
        .unwrap()
        .args(["export", "--format", "json"])
        .arg(&path)
        .output()
        .expect("run kula export");
    let without_flag = Command::cargo_bin("kula")
        .unwrap()
        .args(["export"])
        .arg(&path)
        .output()
        .expect("run kula export");
    assert!(with_flag.status.success());
    assert!(without_flag.status.success());
    assert_eq!(with_flag.stdout, without_flag.stdout);
}

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
}

fn tempfile_dir() -> PathBuf {
    // Use the test binary's target directory to avoid colliding with global
    // tempdir. The directory is created on demand and reused across runs.
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join("kula-cli-format-tests");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
