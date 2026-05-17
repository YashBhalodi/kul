//! End-to-end CLI tests.
//!
//! Every subcommand operates on the project rooted at CWD (issue #83);
//! each test sets `current_dir` on the spawned `kul` process to point
//! at an example or a temp-fixture project root. Multi-file scenarios
//! exercise the project-wide validator semantics R01 / R02 / R13.

use std::path::PathBuf;

use assert_cmd::Command;
use predicates::str::contains;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
}

/// Workspace `target/` scratch directory for tests that build a
/// throwaway project on disk. Each test calls
/// `tempdir("test-name")` to claim its own subdirectory.
fn tempdir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join("kul-cli-tests")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// === `kul validate` ===

#[test]
fn validate_in_single_file_project_root_succeeds() {
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-single-couple"))
        .arg("validate")
        .assert()
        .success()
        .stdout(contains("ok"));
}

#[test]
fn validate_in_multi_file_project_root_succeeds() {
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("07-multi-file-extended-family"))
        .arg("validate")
        .assert()
        .success()
        .stdout(contains("ok"));
}

#[test]
fn validate_outside_project_root_errors() {
    let dir = tempdir("validate-no-manifest");
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(&dir)
        .arg("validate")
        .assert()
        .failure()
        .code(1)
        .stderr(contains("not a Kul project root"))
        .stderr(contains("no kul.yml in current directory"));
}

#[test]
fn validate_rejects_positional_argument() {
    // Sanity-check that the positional file arg is gone — passing any
    // bare path must trip the clap-level usage error (exit code 2).
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-single-couple"))
        .args(["validate", "some-file.kul"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn validate_quiet_suppresses_ok_line() {
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-single-couple"))
        .args(["validate", "--quiet"])
        .assert()
        .success()
        .stdout(predicates::str::is_empty());
}

#[test]
fn validate_json_format_emits_jsonl() {
    let dir = tempdir("validate-json");
    std::fs::write(dir.join("kul.yml"), "kul: \"0.1\"\n").unwrap();
    std::fs::write(
        dir.join("alice.kul"),
        // Missing `name:` — KUL-R03 anchors at the id.
        "person alice gender:female\n",
    )
    .unwrap();
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(&dir)
        .args(["validate", "--format", "json"])
        .output()
        .expect("run kul");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    let line = stdout.lines().next().expect("at least one diagnostic");
    let value: serde_json::Value = serde_json::from_str(line).expect("valid json");
    assert_eq!(value["code"], "KUL-R03");
    assert_eq!(value["severity"], "error");
    assert_eq!(value["primary"]["file"], "alice.kul");
    assert!(value["primary"]["line"].is_u64());
    assert!(value["primary"]["column"].is_u64());
}

// === Multi-file cross-file diagnostic coverage ===

#[test]
fn cross_file_duplicate_id_surfaces_r01() {
    let dir = tempdir("cross-file-r01");
    std::fs::write(dir.join("kul.yml"), "kul: \"0.1\"\n").unwrap();
    std::fs::write(
        dir.join("a.kul"),
        "person alice  name:\"Alice\"  gender:female  born:1950\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("b.kul"),
        // Same id `alice` declared in a sibling file — R01 cross-file.
        "person alice  name:\"Alice Duplicate\"  gender:female  born:1951\n",
    )
    .unwrap();
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(&dir)
        .arg("validate")
        .assert()
        .failure()
        .code(1)
        .stderr(contains("KUL-R01"));
}

#[test]
fn cross_file_unresolved_reference_surfaces_r02() {
    let dir = tempdir("cross-file-r02");
    std::fs::write(dir.join("kul.yml"), "kul: \"0.1\"\n").unwrap();
    std::fs::write(
        dir.join("a.kul"),
        "person alice  name:\"Alice\"  gender:female  born:1950\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("b.kul"),
        // `m_alice_ghost` is declared nowhere in the project — R02.
        "person carol  name:\"Carol\"  gender:female  born:1975\n  birth m_alice_ghost\n",
    )
    .unwrap();
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(&dir)
        .arg("validate")
        .assert()
        .failure()
        .code(1)
        .stderr(contains("KUL-R02"));
}

#[test]
fn cross_file_parent_cycle_surfaces_r13() {
    let dir = tempdir("cross-file-r13");
    std::fs::write(dir.join("kul.yml"), "kul: \"0.1\"\n").unwrap();
    // alice's father is bob (declared in b.kul).
    std::fs::write(
        dir.join("a.kul"),
        "person alice  name:\"Alice\"  gender:female  born:1950\n  adoption m_bob_self alice\n\
         marriage m_bob_self bob bob_partner  start:1900\n\
         person bob_partner  name:\"Bob Partner\"  gender:female  born:1925\n",
    )
    .unwrap();
    // bob's parent is alice — closes the cycle across files.
    std::fs::write(
        dir.join("b.kul"),
        "person bob  name:\"Bob\"  gender:male  born:1948\n  adoption m_alice_self bob\n\
         marriage m_alice_self alice alice_partner  start:1900\n\
         person alice_partner  name:\"Alice Partner\"  gender:male  born:1925\n",
    )
    .unwrap();
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(&dir)
        .arg("validate")
        .assert()
        .failure()
        .code(1)
        .stderr(contains("KUL-R13"));
}

// === `kul format` ===

#[test]
fn format_check_passes_on_every_example_project() {
    // Every example in the workspace must be canonical at HEAD. Each
    // example is its own project root (a directory with a `kul.yml`),
    // so we run `kul format --check` once per project directory.
    let mut project_roots: Vec<PathBuf> = std::fs::read_dir(examples_dir())
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.join("kul.yml").exists())
        .collect();
    project_roots.sort();
    assert!(
        !project_roots.is_empty(),
        "examples/ has no project subdirectories"
    );
    for root in &project_roots {
        Command::cargo_bin("kul")
            .unwrap()
            .current_dir(root)
            .args(["format", "--check"])
            .assert()
            .success();
    }
}

#[test]
fn format_rewrites_every_kul_file_in_project() {
    let dir = tempdir("format-multi-file");
    std::fs::write(dir.join("kul.yml"), "kul: \"0.1\"\n").unwrap();
    // Two files, both dirty — fields out of canonical order.
    let dirty_a = "person alice  born:1950  name:\"Alice\"  gender:female\n";
    let dirty_b = "person bob    born:1948  name:\"Bob\"    gender:male\n";
    std::fs::write(dir.join("a.kul"), dirty_a).unwrap();
    std::fs::write(dir.join("b.kul"), dirty_b).unwrap();
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(&dir)
        .arg("format")
        .assert()
        .success();
    let after_a = std::fs::read_to_string(dir.join("a.kul")).unwrap();
    let after_b = std::fs::read_to_string(dir.join("b.kul")).unwrap();
    assert_ne!(after_a, dirty_a, "a.kul should have been rewritten");
    assert_ne!(after_b, dirty_b, "b.kul should have been rewritten");
    assert!(after_a.contains("name:\"Alice\""));
    assert!(after_b.contains("name:\"Bob\""));
}

#[test]
fn format_check_reports_diff_without_writing() {
    let dir = tempdir("format-check-diff");
    std::fs::write(dir.join("kul.yml"), "kul: \"0.1\"\n").unwrap();
    let dirty = "person alice  born:1950  name:\"Alice\"  gender:female\n";
    std::fs::write(dir.join("alice.kul"), dirty).unwrap();
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(&dir)
        .args(["format", "--check"])
        .assert()
        .failure()
        .code(1)
        .stderr(contains("alice.kul"))
        .stderr(contains("not formatted"));
    // File must not have been touched.
    assert_eq!(
        std::fs::read_to_string(dir.join("alice.kul")).unwrap(),
        dirty
    );
}

#[test]
fn format_outside_project_root_errors() {
    let dir = tempdir("format-no-manifest");
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(&dir)
        .arg("format")
        .assert()
        .failure()
        .code(1)
        .stderr(contains("not a Kul project root"));
}

// === `kul export` ===

#[test]
fn export_single_file_project_emits_success_envelope() {
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-single-couple"))
        .arg("export")
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
fn export_multi_file_project_emits_one_envelope_with_unioned_graph() {
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("07-multi-file-extended-family"))
        .arg("export")
        .output()
        .expect("run kul export");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        1,
        "multi-file project must emit exactly one envelope; got {lines:?}",
    );
    let env: serde_json::Value = serde_json::from_str(lines[0]).expect("valid json");
    assert_eq!(env["ok"], true);
    let persons = env["graph"]["persons"].as_array().expect("persons array");
    let marriages = env["graph"]["marriages"]
        .as_array()
        .expect("marriages array");
    // The fixture has founders + parents + grandchildren spread
    // across three files — assert the export contains more than one
    // person and at least one marriage to confirm the union actually
    // crosses file boundaries.
    assert!(
        persons.len() > 2,
        "expected unioned persons across files; got {}",
        persons.len(),
    );
    assert!(
        !marriages.is_empty(),
        "expected at least one marriage across files",
    );
}

#[test]
fn export_with_positions_attaches_span_to_every_entity() {
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("02-nuclear-family"))
        .args(["export", "--with-positions"])
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
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("02-nuclear-family"))
        .arg("export")
        .output()
        .expect("run kul export");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        !stdout.contains("\"span\""),
        "default mode must not emit `span`; got:\n{stdout}",
    );
}

#[test]
fn export_format_cytoscape_emits_nodes_and_edges() {
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("02-nuclear-family"))
        .args(["export", "--format", "cytoscape"])
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
fn export_outside_project_root_errors() {
    let dir = tempdir("export-no-manifest");
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(&dir)
        .arg("export")
        .assert()
        .failure()
        .code(1)
        .stderr(contains("not a Kul project root"));
}

// === Misc ===

#[test]
fn version_flag_prints_both_versions() {
    Command::cargo_bin("kul")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains("kul-core"));
}
