//! End-to-end CLI smoke tests.

use std::path::PathBuf;

use assert_cmd::Command;
use predicates::str::contains;

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("kul-core")
        .join("tests")
        .join("corpus")
}

#[test]
fn validate_valid_file_exits_zero() {
    let path = corpus_root().join("valid/01-single-person.kul");
    Command::cargo_bin("kul")
        .unwrap()
        .args(["validate"])
        .arg(&path)
        .assert()
        .success()
        .stdout(contains("ok"));
}

#[test]
fn validate_missing_name_exits_one() {
    let path = corpus_root().join("invalid/rule-03-missing-name.kul");
    Command::cargo_bin("kul")
        .unwrap()
        .args(["validate"])
        .arg(&path)
        .assert()
        .failure()
        .code(1)
        .stderr(contains("KUL-R03"))
        .stderr(contains("needs a `name:` field"));
}

#[test]
fn validate_missing_gender_exits_one() {
    let path = corpus_root().join("invalid/rule-03-missing-gender.kul");
    Command::cargo_bin("kul")
        .unwrap()
        .args(["validate"])
        .arg(&path)
        .assert()
        .failure()
        .code(1)
        .stderr(contains("KUL-R03"))
        .stderr(contains("needs a `gender:` field"));
}

#[test]
fn version_flag_prints_both_versions() {
    Command::cargo_bin("kul")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains("kul-core"));
}

#[test]
fn validate_couple_is_clean() {
    let path = corpus_root().join("valid/03-couple.kul");
    Command::cargo_bin("kul")
        .unwrap()
        .args(["validate"])
        .arg(&path)
        .assert()
        .success();
}

#[test]
fn validate_duplicate_id_reports_rule_01() {
    let path = corpus_root().join("invalid/rule-01-duplicate-id.kul");
    Command::cargo_bin("kul")
        .unwrap()
        .args(["validate"])
        .arg(&path)
        .assert()
        .failure()
        .code(1)
        .stderr(contains("KUL-R01"))
        .stderr(contains("is already used"));
}

#[test]
fn validate_self_marriage_reports_rule_04() {
    let path = corpus_root().join("invalid/rule-04-self-marriage.kul");
    Command::cargo_bin("kul")
        .unwrap()
        .args(["validate"])
        .arg(&path)
        .assert()
        .failure()
        .code(1)
        .stderr(contains("KUL-R04"))
        .stderr(contains("spouses must be distinct"));
}

#[test]
fn validate_quiet_suppresses_ok_line() {
    let path = corpus_root().join("valid/01-single-person.kul");
    Command::cargo_bin("kul")
        .unwrap()
        .args(["validate", "--quiet"])
        .arg(&path)
        .assert()
        .success()
        .stdout(predicates::str::is_empty());
}

#[test]
fn validate_missing_manifest_alongside_file_errors() {
    let dir = tempfile_dir().join("validate-missing-manifest");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("alice.kul");
    std::fs::write(&path, "person alice name:\"Alice\" gender:female\n").unwrap();
    let manifest = dir.join("kul.yml");
    let _ = std::fs::remove_file(&manifest);
    Command::cargo_bin("kul")
        .unwrap()
        .args(["validate"])
        .arg(&path)
        .assert()
        .failure()
        .code(1)
        .stderr(contains("missing project manifest"));
}

#[test]
fn validate_malformed_manifest_errors() {
    let dir = tempfile_dir().join("validate-malformed-manifest");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("alice.kul");
    std::fs::write(&path, "person alice name:\"Alice\" gender:female\n").unwrap();
    std::fs::write(dir.join("kul.yml"), "kul: [not-a-string]\n").unwrap();
    Command::cargo_bin("kul")
        .unwrap()
        .args(["validate"])
        .arg(&path)
        .assert()
        .failure()
        .code(1)
        .stderr(contains("parse"));
}

#[test]
fn validate_json_format_emits_jsonl() {
    let path = corpus_root().join("invalid/rule-03-missing-name.kul");
    let output = Command::cargo_bin("kul")
        .unwrap()
        .args(["validate", "--format", "json"])
        .arg(&path)
        .output()
        .expect("run kul");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    let line = stdout.lines().next().expect("at least one diagnostic");
    let value: serde_json::Value = serde_json::from_str(line).expect("valid json");
    assert_eq!(value["code"], "KUL-R03");
    assert_eq!(value["severity"], "error");
    assert!(value["primary"]["line"].is_u64());
    assert!(value["primary"]["column"].is_u64());
}

#[test]
fn validate_multiple_files_exits_one_if_any_fail() {
    let valid = corpus_root().join("valid/01-single-person.kul");
    let invalid = corpus_root().join("invalid/rule-03-missing-name.kul");
    Command::cargo_bin("kul")
        .unwrap()
        .args(["validate"])
        .arg(&valid)
        .arg(&invalid)
        .assert()
        .failure()
        .code(1);
}

// === `kul format` ===

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
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("kul"))
        .collect();
    entries.sort();
    let mut cmd = Command::cargo_bin("kul").unwrap();
    cmd.args(["format", "--check"]);
    for p in &entries {
        cmd.arg(p);
    }
    cmd.assert().success();
}

#[test]
fn format_rewrites_file_in_place() {
    let dir = tempfile_dir();
    let path = dir.join("alice.kul");
    let dirty = "person alice born:1950 name:\"Alice\" gender:female\n";
    std::fs::write(&path, dirty).unwrap();
    std::fs::write(dir.join("kul.yml"), "kul: \"0.1\"\n").unwrap();
    Command::cargo_bin("kul")
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

// === `kul export` ===

#[test]
fn export_clean_file_emits_success_envelope_and_exits_zero() {
    let path = examples_dir().join("01-single-couple.kul");
    let output = Command::cargo_bin("kul")
        .unwrap()
        .args(["export"])
        .arg(&path)
        .output()
        .expect("run kul export");
    assert!(output.status.success(), "expected exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let env: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid json");
    assert_eq!(env["ok"], true);
    assert_eq!(env["schema"], 1);
    assert_eq!(env["kul"], "0.1");
    assert!(env["graph"]["persons"].is_array());
    assert!(env["graph"]["marriages"].is_array());
    assert!(env["graph"]["parenthoodLinks"].is_array());
}

#[test]
fn export_multiple_files_emits_one_envelope_per_line() {
    let p1 = examples_dir().join("01-single-couple.kul");
    let p2 = examples_dir().join("02-nuclear-family.kul");
    let output = Command::cargo_bin("kul")
        .unwrap()
        .args(["export"])
        .arg(&p1)
        .arg(&p2)
        .output()
        .expect("run kul export");
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
    let valid = examples_dir().join("01-single-couple.kul");
    let invalid_path = corpus_root().join("invalid/rule-03-missing-name.kul");
    let output = Command::cargo_bin("kul")
        .unwrap()
        .args(["export"])
        .arg(&valid)
        .arg(&invalid_path)
        .output()
        .expect("run kul export");
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
fn export_with_positions_attaches_span_to_every_entity() {
    let path = examples_dir().join("02-nuclear-family.kul");
    let output = Command::cargo_bin("kul")
        .unwrap()
        .args(["export", "--with-positions"])
        .arg(&path)
        .output()
        .expect("run kul export --with-positions");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let env: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    for collection in ["persons", "marriages", "parenthoodLinks"] {
        for entity in env["graph"][collection].as_array().unwrap() {
            let span = entity["span"]
                .as_array()
                .unwrap_or_else(|| panic!("missing span on {collection}: {entity}"));
            assert_eq!(span.len(), 2);
            assert!(span[0].as_u64().unwrap() < span[1].as_u64().unwrap());
        }
    }
}

#[test]
fn export_default_omits_span_field() {
    let path = examples_dir().join("02-nuclear-family.kul");
    let output = Command::cargo_bin("kul")
        .unwrap()
        .args(["export"])
        .arg(&path)
        .output()
        .expect("run kul export");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        !stdout.contains("\"span\""),
        "default mode must not emit `span`; got:\n{stdout}"
    );
}

#[test]
fn export_format_cytoscape_emits_nodes_and_edges() {
    let path = examples_dir().join("02-nuclear-family.kul");
    let output = Command::cargo_bin("kul")
        .unwrap()
        .args(["export", "--format", "cytoscape"])
        .arg(&path)
        .output()
        .expect("run kul export --format cytoscape");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let env: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(env["ok"], true);
    let nodes = env["graph"]["nodes"].as_array().expect("nodes array");
    let edges = env["graph"]["edges"].as_array().expect("edges array");
    assert!(nodes.iter().any(|n| n["data"]["id"] == "p:alice"));
    assert!(nodes.iter().any(|n| n["data"]["id"] == "m:m_alice_bob"));
    assert!(edges.iter().any(|e| e["data"]["type"] == "spouse"));
    assert!(
        edges
            .iter()
            .any(|e| e["data"]["type"] == "biological_child")
    );
}

#[test]
fn export_format_json_is_default_and_explicit_flag_works() {
    let path = examples_dir().join("01-single-couple.kul");
    let with_flag = Command::cargo_bin("kul")
        .unwrap()
        .args(["export", "--format", "json"])
        .arg(&path)
        .output()
        .expect("run kul export");
    let without_flag = Command::cargo_bin("kul")
        .unwrap()
        .args(["export"])
        .arg(&path)
        .output()
        .expect("run kul export");
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
        .join("kul-cli-format-tests");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
