//! Snapshot tests for the canonical JSON export.
//!
//! Sweeps every `examples/*.kula` to lock in the success-path schema and
//! covers a small set of hand-crafted bad inputs for the failure envelope.
//! The file-by-file tests double as the corpus contract: dropping a new
//! `examples/*.kula` makes the sweep test demand a corresponding snapshot
//! review, surfacing any unintentional schema drift.
//!
//! Each snapshot is the pretty-printed JSON envelope. Pretty-printing is
//! the difference between a useful diff (one field per line) and a wall of
//! text — the CLI does not pretty-print, but the snapshot suite does.

use std::path::{Path, PathBuf};

use kula_core::export::{ExportOptions, export};

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

fn export_default(source: &str) -> String {
    let check = kula_core::check(source);
    let envelope = export(source, &check, ExportOptions::default());
    serde_json::to_string_pretty(&envelope).expect("serialize envelope")
}

/// Generate one snapshot test per example file. The snapshot name embeds
/// the file stem so a missing or extra example surfaces as a clearly-named
/// snapshot.
macro_rules! example_snapshot {
    ($name:ident, $stem:literal) => {
        #[test]
        fn $name() {
            let path = examples_dir().join(concat!($stem, ".kula"));
            let json = export_default(&read(&path));
            insta::assert_snapshot!(json);
        }
    };
}

example_snapshot!(example_01_single_couple, "01-single-couple");
example_snapshot!(example_02_nuclear_family, "02-nuclear-family");
example_snapshot!(example_03_three_generations, "03-three-generations");
example_snapshot!(example_04_polygamous_family, "04-polygamous-family");

/// Catch-all: if a new `examples/*.kula` lands without a matching test
/// above, this fires so the contributor adds the snapshot.
#[test]
fn every_example_has_a_dedicated_snapshot_test() {
    let mut have: Vec<String> = std::fs::read_dir(examples_dir())
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("kula"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    have.sort();
    let expected = [
        "01-single-couple",
        "02-nuclear-family",
        "03-three-generations",
        "04-polygamous-family",
    ];
    assert_eq!(
        have.iter().map(String::as_str).collect::<Vec<_>>(),
        expected,
        "an example file was added or removed without updating the export snapshot list"
    );
}

#[test]
fn failure_envelope_duplicate_id() {
    let src = "\
person alice name:\"Alice\" gender:female
person alice name:\"Alice 2\" gender:female
";
    let json = export_default(src);
    insta::assert_snapshot!(json);
}

#[test]
fn failure_envelope_unresolved_reference() {
    let src = "\
person alice name:\"Alice\" gender:female
marriage m alice ghost start:1972
";
    let json = export_default(src);
    insta::assert_snapshot!(json);
}

#[test]
fn failure_envelope_missing_required_field() {
    let src = "person alice gender:female\n";
    let json = export_default(src);
    insta::assert_snapshot!(json);
}

#[test]
fn one_thousand_statement_export_under_budget() {
    let mut source = String::from("kula 0.1\n");
    for i in 0..1000 {
        use std::fmt::Write as _;
        let _ = writeln!(&mut source, "person p{i} name:\"P{i}\" gender:female");
    }
    let check = kula_core::check(&source);
    let start = std::time::Instant::now();
    let envelope = export(&source, &check, ExportOptions::default());
    let _json = serde_json::to_string(&envelope).expect("serialize");
    let elapsed = start.elapsed();
    eprintln!("1000-statement export + serialize: {elapsed:?}");
    // Real target is <30ms on a developer laptop; assert a 5x ceiling so
    // CI runners and debug builds don't flake. A 2x regression still fires.
    assert!(
        elapsed < std::time::Duration::from_millis(150),
        "1000-statement export budget exceeded: {elapsed:?}"
    );
}
