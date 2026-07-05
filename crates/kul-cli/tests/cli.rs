//! End-to-end CLI tests. Each test sets `current_dir` on the spawned
//! `kul` process at an example or temp-fixture project root.

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

/// Workspace `target/` scratch directory for a throwaway project.
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

/// Like [`tempdir`] but writes a default `kul.yml` so the directory
/// parses as a Kul project root.
fn project_dir(name: &str) -> PathBuf {
    let dir = tempdir(name);
    std::fs::write(dir.join("kul.yml"), "kul: \"0.1\"\n").unwrap();
    dir
}

#[test]
fn validate_in_single_file_project_root_succeeds() {
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-nuclear-family"))
        .arg("validate")
        .assert()
        .success()
        .stdout(contains("ok"));
}

#[test]
fn validate_in_multi_file_project_root_succeeds() {
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("08-multi-file-project"))
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
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-nuclear-family"))
        .args(["validate", "some-file.kul"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn validate_quiet_suppresses_ok_line() {
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-nuclear-family"))
        .args(["validate", "--quiet"])
        .assert()
        .success()
        .stdout(predicates::str::is_empty());
}

#[test]
fn validate_json_format_emits_jsonl() {
    let dir = project_dir("validate-json");
    std::fs::write(
        dir.join("alice.kul"),
        "person alice gender:female\n", // missing `name:` triggers KUL-R03
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

#[test]
fn cross_file_duplicate_id_surfaces_r01() {
    let dir = project_dir("cross-file-r01");
    std::fs::write(
        dir.join("a.kul"),
        "person alice  name:\"Alice\"  gender:female  born:1950\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("b.kul"),
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
    let dir = project_dir("cross-file-r02");
    std::fs::write(
        dir.join("a.kul"),
        "person alice  name:\"Alice\"  gender:female  born:1950\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("b.kul"),
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
    let dir = project_dir("cross-file-r13");
    // alice's father is bob in b.kul; bob's parent is alice in a.kul — cycle across files.
    std::fs::write(
        dir.join("a.kul"),
        "person alice  name:\"Alice\"  gender:female  born:1950\n  adoption m_bob_self alice\n\
         marriage m_bob_self bob bob_partner  start:1900\n\
         person bob_partner  name:\"Bob Partner\"  gender:female  born:1925\n",
    )
    .unwrap();
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

#[test]
fn format_check_passes_on_every_example_project() {
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
    let dir = project_dir("format-multi-file");
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
    let dir = project_dir("format-check-diff");
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

/// Parse errors surface through the same miette renderer `validate` uses.
#[test]
fn format_with_parse_errors_renders_miette_report() {
    let dir = project_dir("format-parse-error");
    std::fs::write(dir.join("broken.kul"), "person\n").unwrap();
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(&dir)
        .arg("format")
        .assert()
        .failure()
        .code(1)
        .stderr(contains("cannot format project with parse errors"))
        .stderr(contains("KUL-P"));
}

/// Cross-file related-spans surface as a `see also: …` footnote since
/// miette's single-source renderer can't draw them inline.
#[test]
fn validate_cross_file_duplicate_emits_see_also_footnote() {
    let dir = project_dir("validate-cross-file-r01");
    std::fs::write(
        dir.join("a.kul"),
        "person alice name:\"Alice A\" gender:female\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("b.kul"),
        "person alice name:\"Alice B\" gender:male\n",
    )
    .unwrap();
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(&dir)
        .arg("validate")
        .assert()
        .failure()
        .code(1)
        .stderr(contains("KUL-R01"))
        .stderr(contains("see also:"))
        .stderr(contains("a.kul"));
}

#[test]
fn export_single_file_project_emits_success_envelope() {
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-nuclear-family"))
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
        .current_dir(examples_dir().join("08-multi-file-project"))
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
        .current_dir(examples_dir().join("01-nuclear-family"))
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
        .current_dir(examples_dir().join("01-nuclear-family"))
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
        .current_dir(examples_dir().join("01-nuclear-family"))
        .args(["export", "--format", "cytoscape"])
        .output()
        .expect("run kul export --format cytoscape");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let env: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(env["ok"], true);
    let nodes = env["graph"]["nodes"].as_array().expect("nodes array");
    let edges = env["graph"]["edges"].as_array().expect("edges array");
    assert!(nodes.iter().any(|n| n["data"]["id"] == "p:hiroshi"));
    assert!(nodes.iter().any(|n| n["data"]["id"] == "m:m_hiroshi_yuki"));
    assert!(edges.iter().any(|e| e["data"]["type"] == "spouse"));
    assert!(
        edges
            .iter()
            .any(|e| e["data"]["type"] == "biological_child")
    );
}

#[test]
fn export_format_svg_streams_self_contained_svg() {
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("02-three-generations"))
        .args(["export", "--format", "svg"])
        .output()
        .expect("run kul export --format svg");
    assert!(output.status.success(), "expected exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("<svg"), "expected an SVG document");
    assert!(
        stdout.contains("<style>"),
        "expected an inline <style> block"
    );
    assert!(
        !stdout.contains("var(--vscode-"),
        "self-contained SVG must not reference VSCode theme variables",
    );
    insta::assert_snapshot!(stdout);
}

#[test]
fn export_format_svg_with_positions_is_usage_error() {
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("02-three-generations"))
        .args(["export", "--format", "svg", "--with-positions"])
        .assert()
        .failure()
        .code(2)
        .stdout(predicates::str::is_empty())
        .stderr(contains("--with-positions"))
        .stderr(contains("svg"));
}

#[test]
fn export_format_svg_on_error_project_writes_nothing_to_stdout() {
    let dir = project_dir("export-svg-error");
    std::fs::write(dir.join("broken.kul"), "person alice gender:female\n").unwrap();
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(&dir)
        .args(["export", "--format", "svg"])
        .output()
        .expect("run kul export --format svg");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "a blocked render must write nothing to stdout; got {:?}",
        String::from_utf8_lossy(&output.stdout),
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("KUL-R03"),
        "expected the blocking diagnostic on stderr: {stderr}",
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

#[test]
fn query_person_human_snapshot() {
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-nuclear-family"))
        .args(["query", "person", "hiroshi"])
        .output()
        .expect("run kul query person");
    assert!(output.status.success(), "expected exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(stdout);
}

#[test]
fn query_person_json_snapshot() {
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-nuclear-family"))
        .args(["query", "person", "hiroshi", "--format", "json"])
        .output()
        .expect("run kul query person --format json");
    assert!(output.status.success(), "expected exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    // Valid, and the ok envelope carrying the person.
    let env: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid json");
    assert_eq!(env["ok"], true);
    assert_eq!(env["result"]["id"], "hiroshi");
    insta::assert_snapshot!(stdout);
}

#[test]
fn query_marriage_human_renders_recorded_fields() {
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-nuclear-family"))
        .args(["query", "marriage", "m_hiroshi_yuki"])
        .assert()
        .success()
        .stdout(contains("marriage m_hiroshi_yuki"))
        .stdout(contains("hiroshi, yuki"));
}

/// Not-found is honest, not a crash: nonzero exit and a stderr diagnostic
/// naming the id.
#[test]
fn query_unknown_id_exits_nonzero_with_stderr_diagnostic() {
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-nuclear-family"))
        .args(["query", "person", "nobody"])
        .assert()
        .failure()
        .code(1)
        .stderr(contains("no person with id `nobody`"));
}

/// Under `--format json` a not-found still emits the ok envelope with a
/// `null` result on stdout (the contract answer) alongside the stderr
/// diagnostic and nonzero exit.
#[test]
fn query_unknown_id_json_emits_null_result_on_stdout() {
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-nuclear-family"))
        .args(["query", "person", "nobody", "--format", "json"])
        .output()
        .expect("run kul query person --format json");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    let env: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid json");
    assert_eq!(env["ok"], true);
    assert!(env["result"].is_null(), "expected null result: {stdout}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("no person with id `nobody`"), "{stderr}");
}

/// Wrong-kind id (a marriage id asked for as a person) is not-found too.
#[test]
fn query_wrong_kind_id_is_not_found() {
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-nuclear-family"))
        .args(["query", "person", "m_hiroshi_yuki"])
        .assert()
        .failure()
        .code(1)
        .stderr(contains("no person with id `m_hiroshi_yuki`"));
}

/// Load-and-check gate (human): a project that fails its checks blocks the
/// query — diagnostics to stderr, nonzero exit.
#[test]
fn query_failing_project_gate_human() {
    let dir = project_dir("query-failing-human");
    std::fs::write(dir.join("alice.kul"), "person alice gender:female\n").unwrap();
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(&dir)
        .args(["query", "person", "alice"])
        .assert()
        .failure()
        .code(1)
        .stderr(contains("KUL-R03"));
}

/// Load-and-check gate (json): the envelope's error arm is written to
/// stdout with a nonzero exit — never a partial answer.
#[test]
fn query_failing_project_gate_json() {
    let dir = project_dir("query-failing-json");
    std::fs::write(dir.join("alice.kul"), "person alice gender:female\n").unwrap();
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(&dir)
        .args(["query", "person", "alice", "--format", "json"])
        .output()
        .expect("run kul query person --format json");
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    let env: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid json");
    assert_eq!(env["ok"], false);
    assert!(
        env["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "KUL-R03"),
        "expected KUL-R03 in error arm: {stdout}"
    );
}

/// The CLI `--format json` path is the epic's contract-snapshot harness:
/// its bytes must equal the core `query` envelope serialization the WASM
/// surface also returns (both serialize `kul_core::query::person_lookup`).
#[test]
fn query_json_matches_core_envelope_bytes() {
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-nuclear-family"))
        .args(["query", "person", "hiroshi", "--format", "json"])
        .output()
        .expect("run kul query person --format json");
    let cli_json = String::from_utf8(output.stdout).unwrap();

    let source = std::fs::read_to_string(
        examples_dir()
            .join("01-nuclear-family")
            .join("nuclear-family.kul"),
    )
    .unwrap();
    let inputs = vec![kul_core::ast::InputFile::new("nuclear-family.kul", source)];
    let check = kul_core::check_with_manifest(
        "kul.yml",
        "",
        &kul_core::manifest::Manifest::default(),
        &inputs,
    );
    let envelope = kul_core::query::person_lookup(&check, "hiroshi");
    let core_json = serde_json::to_string(&envelope).unwrap();

    assert_eq!(cli_json.trim(), core_json);
}

// ---- Kin-set queries (`kul query kin`) ----

#[test]
fn query_kin_human_snapshot() {
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("02-three-generations"))
        .args(["query", "kin", "chidi", "ancestors"])
        .output()
        .expect("run kul query kin");
    assert!(output.status.success(), "expected exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    insta::assert_snapshot!(stdout);
}

/// Human output stays terminology-neutral — structured descriptor facts,
/// never a kinship word.
#[test]
fn query_kin_human_has_no_kinship_words() {
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("02-three-generations"))
        .args(["query", "kin", "chidi", "ancestors"])
        .output()
        .expect("run kul query kin");
    let stdout = String::from_utf8(output.stdout).unwrap().to_lowercase();
    for word in [
        "grandmother",
        "grandfather",
        "grandparent",
        "mother",
        "father",
        "parent ",
        "ancestor of",
    ] {
        assert!(
            !stdout.contains(word),
            "human output leaked a kinship word `{word}`: {stdout}"
        );
    }
}

#[test]
fn query_kin_json_snapshot() {
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-nuclear-family"))
        .args(["query", "kin", "akiko", "parents", "--format", "json"])
        .output()
        .expect("run kul query kin --format json");
    assert!(output.status.success(), "expected exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let env: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid json");
    assert_eq!(env["ok"], true);
    assert_eq!(env["result"]["members"][0]["personId"], "hiroshi");
    insta::assert_snapshot!(stdout);
}

/// An empty set is a complete answer: exit 0, nothing on stdout.
#[test]
fn query_kin_empty_set_exits_zero() {
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-nuclear-family"))
        .args(["query", "kin", "hiroshi", "ancestors"])
        .output()
        .expect("run kul query kin");
    assert!(output.status.success(), "empty set exits 0");
    assert!(output.stdout.is_empty(), "empty set prints nothing");
}

/// Unknown anchor → diagnostic naming the id + nonzero, never an empty set.
#[test]
fn query_kin_unknown_anchor_nonzero() {
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-nuclear-family"))
        .args(["query", "kin", "nobody", "parents"])
        .assert()
        .failure()
        .code(1)
        .stderr(contains("no person with id `nobody`"));
}

/// The CLI `--format json` kin bytes equal the core `kin_query` envelope the
/// WASM surface also returns (the contract-snapshot harness).
#[test]
fn query_kin_json_matches_core_envelope_bytes() {
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-nuclear-family"))
        .args(["query", "kin", "akiko", "parents", "--format", "json"])
        .output()
        .expect("run kul query kin --format json");
    let cli_json = String::from_utf8(output.stdout).unwrap();

    let source = std::fs::read_to_string(
        examples_dir()
            .join("01-nuclear-family")
            .join("nuclear-family.kul"),
    )
    .unwrap();
    let inputs = vec![kul_core::ast::InputFile::new("nuclear-family.kul", source)];
    let check = kul_core::check_with_manifest(
        "kul.yml",
        "",
        &kul_core::manifest::Manifest::default(),
        &inputs,
    );
    let query =
        kul_core::query::Query::kin_ancestors("akiko", kul_core::query::IntRange::exactly(1), None);
    let core_json = serde_json::to_string(&kul_core::query::kin_query(&check, &query)).unwrap();

    assert_eq!(cli_json.trim(), core_json);
}

/// Collateral human output: structured facts including sharing and apex
/// seniority, still terminology-neutral (no "cousin" / "aunt").
#[test]
fn query_kin_collateral_human_snapshot() {
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("05-cousins-and-in-laws"))
        .args(["query", "kin", "matteo", "aunts-uncles"])
        .output()
        .expect("run kul query kin aunts-uncles");
    assert!(output.status.success(), "expected exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    for word in ["cousin ", "aunt", "uncle", "niece", "nephew", "sibling"] {
        assert!(
            !stdout.to_lowercase().contains(word),
            "human output leaked a kinship word `{word}`: {stdout}"
        );
    }
    insta::assert_snapshot!(stdout);
}

/// `cousins --degree D --removed R` maps onto the collateralByDegree Query.
#[test]
fn query_kin_cousins_json_snapshot() {
    let output = Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("05-cousins-and-in-laws"))
        .args([
            "query", "kin", "matteo", "cousins", "--degree", "1", "--format", "json",
        ])
        .output()
        .expect("run kul query kin cousins --format json");
    assert!(output.status.success(), "expected exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let env: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid json");
    assert_eq!(env["ok"], true);
    assert_eq!(env["result"]["members"][0]["personId"], "giulia");
    insta::assert_snapshot!(stdout);
}

/// `cousins` without `--degree` is a usage error (exit 2), not an empty set.
#[test]
fn query_kin_cousins_requires_degree() {
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("05-cousins-and-in-laws"))
        .args(["query", "kin", "matteo", "cousins"])
        .assert()
        .failure()
        .code(2)
        .stderr(contains("requires --degree"));
}

#[test]
fn query_missing_id_is_usage_error() {
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(examples_dir().join("01-nuclear-family"))
        .args(["query", "person"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn query_outside_project_root_errors() {
    let dir = tempdir("query-no-manifest");
    Command::cargo_bin("kul")
        .unwrap()
        .current_dir(&dir)
        .args(["query", "person", "alice"])
        .assert()
        .failure()
        .code(1)
        .stderr(contains("not a Kul project root"));
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
