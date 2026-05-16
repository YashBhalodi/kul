//! Multi-file project tests (per ADR-0015).
//!
//! Fixtures live under `tests/fixtures/multi-file/<scenario>/` and each
//! carries a `kul.yml` plus one or more `.kul` files. The tests drive
//! `kul_core::check` with the project as it would arrive at the
//! toolchain edge (manifest YAML + a vector of `InputFile`s), and
//! snapshot the rendered diagnostic list so regressions to R01 / R02 /
//! R13 cross-file semantics or to KUL-M06 surface as snapshot diffs.

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

/// Load a multi-file project fixture. Returns `(manifest_yaml, inputs)`
/// in stable lexicographic order over the directory's `.kul` files so
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

/// Resolve a `FileId` into the input-order index it points at (1-based,
/// per the multi-file `Document` convention where `FileId(0)` is the
/// manifest). Snapshot output uses this rather than the raw `FileId` so
/// reordering the fixture's input list flips snapshots in a way a
/// reviewer can read.
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
    // Two files: a.kul declares alice + bob; b.kul declares marriage
    // m_alice_bob (referencing alice and bob across the file boundary)
    // and a child carol whose `birth m_alice_bob` resolves across-file.
    // Under project-wide resolution every reference resolves; no R02.
    let (inputs, result) = check_project("cross-file-resolution");
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got:\n{}",
        render_diagnostics(&result, &inputs)
    );

    // Sanity: every id is reachable via the project-wide lookups.
    let resolved = result.resolved();
    assert!(resolved.person("alice").is_some());
    assert!(resolved.person("bob").is_some());
    assert!(resolved.person("carol").is_some());
    assert!(resolved.marriage("m_alice_bob").is_some());
}

#[test]
fn project_wide_iteration_walks_every_file() {
    // Project-wide `persons()` returns every declared person regardless
    // of which file owns it; the per-file helpers restrict to one file.
    let (inputs, result) = check_project("cross-file-resolution");
    let resolved = result.resolved();

    let all_persons: Vec<&str> = resolved.persons().map(|p| p.id.name.as_str()).collect();
    assert_eq!(all_persons, ["alice", "bob", "carol"]);

    // First file declares alice + bob; second declares carol. The
    // input order determines `FileId` assignment.
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

    // The marriage and its spouses cross the file boundary.
    let m = resolved.marriage("m_alice_bob").expect("marriage resolves");
    let spouses: Vec<&str> = resolved.spouses_of(m).map(|p| p.id.name.as_str()).collect();
    assert_eq!(spouses, ["alice", "bob"]);

    // `entity()` reports the declaring file, so cross-file consumers
    // (LSP, future renames) can route to the right URI.
    let alice_entity = resolved.entity("alice").expect("alice resolved");
    assert_eq!(alice_entity.file, a_kul);
    assert_eq!(alice_entity.kind, EntityKind::Person);
    let carol_entity = resolved.entity("carol").expect("carol resolved");
    assert_eq!(carol_entity.file, b_kul);
}

#[test]
fn cross_file_duplicate_id_fires_r01_with_primary_on_second() {
    // first.kul and second.kul both declare `person alice`. R01 fires
    // with the primary on the second-discovered declaration and a
    // related-span on the first.
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

    // first.kul appears earlier in `inputs` (alphabetic order); its file
    // id is the smaller of the two. The primary anchors at the *second*
    // discovery, the related-span at the *first*.
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
    // A project directory with `kul.yml` but zero `.kul` files emits
    // KUL-M06 anchored at the manifest start. Severity error.
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
fn r13_parenthood_cycle_spans_two_files() {
    // Cross-file parenthood cycle:
    // file a.kul: person `a` is adopted into `m_branch_b` (declared in b.kul);
    //             marriage `m_branch_a` spouses are `a` and `partner_a`.
    // file b.kul: person `b` is bio child of `m_branch_a` (declared in a.kul);
    //             marriage `m_branch_b` spouses are `b` and `partner_b`.
    // Cycle: a's parent set includes b (via m_branch_b) → b's parent set includes a
    // (via m_branch_a) → a. R13 detects it as one cycle.
    let (inputs, result) = check_project("cross-file-cycle");
    let r13: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == "KUL-R13")
        .collect();
    assert!(!r13.is_empty(), "expected R13 to fire across files");

    insta::assert_snapshot!(render_diagnostics(&result, &inputs));
}
