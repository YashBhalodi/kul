//! Multi-file project tests (per ADR-0015).
//!
//! Fixtures under `tests/fixtures/multi-file/<scenario>/` drive
//! `kul_core::check` and snapshot rendered diagnostics so regressions to
//! cross-file R01 / R02 / R13 or to KUL-M06 surface as snapshot diffs.

use std::path::{Path, PathBuf};

use kul_core::CheckResult;
use kul_core::ast::InputFile;
use kul_core::diagnostic::Diagnostic;
use kul_core::semantic::EntityKind;
use kul_core::span::FileId;

fn fixture_dir(scenario: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("multi-file")
        .join(scenario)
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

/// Load a multi-file project fixture. Files are sorted lexicographically so
/// snapshot output is deterministic.
fn load_project(scenario: &str) -> (String, Vec<InputFile>) {
    let dir = fixture_dir(scenario);
    let manifest = read(&dir.join("kul.yml"));
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("read fixture {}: {err}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("kul"))
        .collect();
    entries.sort();
    let inputs = entries
        .iter()
        .map(|p| {
            InputFile::new(
                p.file_name().unwrap().to_string_lossy().into_owned(),
                read(p),
            )
        })
        .collect();
    (manifest, inputs)
}

fn check_project(scenario: &str) -> (Vec<InputFile>, CheckResult) {
    let (manifest, inputs) = load_project(scenario);
    let result = kul_core::check("kul.yml", &manifest, &inputs);
    (inputs, result)
}

/// Render a `FileId` as its source filename. Used in snapshots so reordering
/// fixture inputs produces a reviewer-readable diff rather than a raw id swap.
fn file_label(file: FileId, inputs: &[InputFile]) -> String {
    if file == FileId::MANIFEST {
        return "kul.yml".to_string();
    }
    let idx = file.as_u32() as usize;
    if idx == 0 || idx > inputs.len() {
        return format!("<file:{}>", file.as_u32());
    }
    inputs[idx - 1].name.clone()
}

fn render_diagnostics(result: &CheckResult, inputs: &[InputFile]) -> String {
    result
        .diagnostics
        .iter()
        .map(|d| render_one(d, inputs))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_one(d: &Diagnostic, inputs: &[InputFile]) -> String {
    let primary = match d.primary {
        Some(p) => format!(
            "{} [{}..{}]",
            file_label(p.file, inputs),
            p.span.start,
            p.span.end
        ),
        None => "<unanchored>".to_string(),
    };
    let mut s = format!("{} {}: {}", d.code, primary, d.message);
    for r in &d.related {
        s.push_str(&format!(
            "\n  related {} [{}..{}]: {}",
            file_label(r.span.file, inputs),
            r.span.span.start,
            r.span.span.end,
            r.label
        ));
    }
    s
}

#[test]
fn cross_file_resolution_is_quiet() {
    let (inputs, result) = check_project("cross-file-resolution");
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got:\n{}",
        render_diagnostics(&result, &inputs)
    );

    let resolved = result.resolved();
    assert!(resolved.person("alice").is_some());
    assert!(resolved.person("bob").is_some());
    assert!(resolved.person("carol").is_some());
    assert!(resolved.marriage("m_alice_bob").is_some());
}

#[test]
fn project_wide_iteration_walks_every_file() {
    let (inputs, result) = check_project("cross-file-resolution");
    let resolved = result.resolved();

    let all_persons: Vec<&str> = resolved.persons().map(|p| p.id.name.as_str()).collect();
    assert_eq!(all_persons, ["alice", "bob", "carol"]);

    let a_kul_idx = inputs.iter().position(|i| i.name == "a.kul").unwrap();
    let b_kul_idx = inputs.iter().position(|i| i.name == "b.kul").unwrap();
    let a_kul = FileId::from_raw((a_kul_idx + 1) as u32);
    let b_kul = FileId::from_raw((b_kul_idx + 1) as u32);

    let in_a: Vec<&str> = resolved
        .persons_in(a_kul)
        .map(|p| p.id.name.as_str())
        .collect();
    let in_b: Vec<&str> = resolved
        .persons_in(b_kul)
        .map(|p| p.id.name.as_str())
        .collect();
    assert_eq!(in_a, ["alice", "bob"]);
    assert_eq!(in_b, ["carol"]);

    let m = resolved.marriage("m_alice_bob").expect("marriage resolves");
    let spouses: Vec<&str> = resolved.spouses_of(m).map(|p| p.id.name.as_str()).collect();
    assert_eq!(spouses, ["alice", "bob"]);

    // `entity()` reports the declaring file so cross-file consumers can
    // route to the right URI.
    let alice_entity = resolved.entity("alice").expect("alice resolved");
    assert_eq!(alice_entity.file, a_kul);
    assert_eq!(alice_entity.kind, EntityKind::Person);
    let carol_entity = resolved.entity("carol").expect("carol resolved");
    assert_eq!(carol_entity.file, b_kul);
}

#[test]
fn cross_file_duplicate_id_fires_r01_with_primary_on_second() {
    let (inputs, result) = check_project("cross-file-duplicate");

    let r01: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "KUL-R01")
        .collect();
    assert_eq!(r01.len(), 1, "expected one R01 across files");
    let d = r01[0];

    let primary = d.primary.expect("R01 must anchor");
    let related = d.related.first().expect("R01 carries related-span");

    // Primary anchors on the second discovery, related-span on the first.
    let first_idx = inputs.iter().position(|i| i.name == "first.kul").unwrap();
    let second_idx = inputs.iter().position(|i| i.name == "second.kul").unwrap();
    assert!(
        first_idx < second_idx,
        "fixture relies on lexicographic ordering"
    );
    let first_id = FileId::from_raw((first_idx + 1) as u32);
    let second_id = FileId::from_raw((second_idx + 1) as u32);
    assert_eq!(primary.file, second_id, "primary points to second decl");
    assert_eq!(related.span.file, first_id, "related points to first decl");

    insta::assert_snapshot!(render_diagnostics(&result, &inputs));
}

#[test]
fn empty_project_fires_kul_m06() {
    let (inputs, result) = check_project("empty-project");
    assert!(inputs.is_empty(), "fixture has no .kul files");
    let m06: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "KUL-M06")
        .collect();
    assert_eq!(m06.len(), 1, "expected exactly one KUL-M06");
    let primary = m06[0].primary.expect("M06 anchors at manifest");
    assert_eq!(primary.file, FileId::MANIFEST);

    insta::assert_snapshot!(render_diagnostics(&result, &inputs));
}

#[test]
fn r09_cross_file_related_span_points_to_spouse_file() {
    // b.kul declares a marriage (start:1940) whose spouse `alice` is
    // declared in a.kul (born:1950). R09 fires: the primary anchors on the
    // marriage's `start` in b.kul, but the related "born here" span belongs
    // to alice's declaration in a.kul — not the iterating (marriage) file.
    let (inputs, result) = check_project("cross-file-temporal");
    let r09: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "KUL-R09")
        .collect();
    assert_eq!(r09.len(), 1, "expected exactly one R09 across files");
    let d = r09[0];

    let resolved = result.resolved();
    let marriage_file = resolved
        .entity("m_alice_bob")
        .expect("marriage resolves")
        .file;
    let spouse_file = resolved.entity("alice").expect("alice resolves").file;
    assert_ne!(
        marriage_file, spouse_file,
        "fixture must place the marriage and the spouse in different files"
    );

    let primary = d.primary.expect("R09 must anchor");
    assert_eq!(
        primary.file, marriage_file,
        "primary anchors in the marriage's file"
    );
    let related = d.related.first().expect("R09 carries a related-span");
    assert_eq!(
        related.span.file, spouse_file,
        "related span must resolve to the spouse's declaring file, not the iterating file"
    );

    insta::assert_snapshot!(render_diagnostics(&result, &inputs));
}

#[test]
fn r13_parenthood_cycle_spans_two_files() {
    // a.kul and b.kul each declare a marriage whose only child is adopted
    // or born into the other file's marriage — a's parent ancestry runs
    // through b's marriage and vice versa. R13 detects the single cycle.
    let (inputs, result) = check_project("cross-file-cycle");
    let r13: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "KUL-R13")
        .collect();
    assert!(!r13.is_empty(), "expected R13 to fire across files");

    insta::assert_snapshot!(render_diagnostics(&result, &inputs));
}
