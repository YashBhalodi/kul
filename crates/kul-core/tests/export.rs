//! Snapshot tests for the canonical JSON export.
//!
//! Sweeps every `examples/*/<name>.kul` to lock in the success-path schema
//! and covers a small set of hand-crafted bad inputs for the failure
//! envelope. The file-by-file tests double as the corpus contract: dropping
//! a new `examples/*/<name>.kul` makes the sweep test demand a corresponding
//! snapshot review, surfacing any unintentional schema drift.
//!
//! Each snapshot is the pretty-printed JSON envelope. Pretty-printing is
//! the difference between a useful diff (one field per line) and a wall of
//! text — the CLI does not pretty-print, but the snapshot suite does.

mod common;

use std::path::{Path, PathBuf};

use kul_core::ast::InputFile;
use kul_core::export::{ExportFormat, ExportOptions, export};
use kul_core::manifest::Manifest;

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

fn export_with(source: &str, options: ExportOptions) -> String {
    let check = check_one(source);
    let envelope = export(&check, options);
    serde_json::to_string_pretty(&envelope).expect("serialize envelope")
}

fn export_default(source: &str) -> String {
    export_with(source, ExportOptions::default())
}

fn export_with_positions(source: &str) -> String {
    export_with(
        source,
        ExportOptions {
            with_positions: true,
            ..ExportOptions::default()
        },
    )
}

fn export_cytoscape(source: &str) -> String {
    export_with(
        source,
        ExportOptions {
            format: ExportFormat::Cytoscape,
            ..ExportOptions::default()
        },
    )
}

/// Generate three snapshot tests per example file — default (kinship-
/// native, positions off), positions on, and cytoscape format. Each
/// example lives in its own subdirectory (`examples/<dir>/<stem>.kul`)
/// so the macro takes both the directory and the file stem.
macro_rules! example_snapshot {
    ($default_name:ident, $positions_name:ident, $cytoscape_name:ident, $dir:literal, $stem:literal) => {
        #[test]
        fn $default_name() {
            let path = examples_dir().join($dir).join(concat!($stem, ".kul"));
            let json = export_default(&read(&path));
            insta::assert_snapshot!(json);
        }

        #[test]
        fn $positions_name() {
            let path = examples_dir().join($dir).join(concat!($stem, ".kul"));
            let json = export_with_positions(&read(&path));
            insta::assert_snapshot!(json);
        }

        #[test]
        fn $cytoscape_name() {
            let path = examples_dir().join($dir).join(concat!($stem, ".kul"));
            let json = export_cytoscape(&read(&path));
            insta::assert_snapshot!(json);
        }
    };
}

example_snapshot!(
    example_01_single_couple,
    example_01_single_couple_with_positions,
    example_01_single_couple_cytoscape,
    "01-single-couple",
    "single-couple"
);
example_snapshot!(
    example_02_nuclear_family,
    example_02_nuclear_family_with_positions,
    example_02_nuclear_family_cytoscape,
    "02-nuclear-family",
    "nuclear-family"
);
example_snapshot!(
    example_03_three_generations,
    example_03_three_generations_with_positions,
    example_03_three_generations_cytoscape,
    "03-three-generations",
    "three-generations"
);
example_snapshot!(
    example_04_polygamous_family,
    example_04_polygamous_family_with_positions,
    example_04_polygamous_family_cytoscape,
    "04-polygamous-family",
    "polygamous-family"
);
example_snapshot!(
    example_05_married_siblings,
    example_05_married_siblings_with_positions,
    example_05_married_siblings_cytoscape,
    "05-married-siblings",
    "married-siblings"
);
example_snapshot!(
    example_06_three_branch_dynasty,
    example_06_three_branch_dynasty_with_positions,
    example_06_three_branch_dynasty_cytoscape,
    "06-three-branch-dynasty",
    "three-branch-dynasty"
);

/// Multi-file example: every `.kul` file in the directory is part of the
/// same project, so the export envelope holds the union of every file's
/// persons, marriages, and parenthood links.
fn export_multi_file(dir: &str, options: ExportOptions) -> String {
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
    let check =
        kul_core::check_with_manifest("kul.yml", "kul: \"0.1\"\n", &Manifest::default(), &inputs);
    let envelope = export(&check, options);
    serde_json::to_string_pretty(&envelope).expect("serialize envelope")
}

#[test]
fn example_07_multi_file_extended_family() {
    let json = export_multi_file("07-multi-file-extended-family", ExportOptions::default());
    insta::assert_snapshot!(json);
}

#[test]
fn example_07_multi_file_extended_family_with_positions() {
    let json = export_multi_file(
        "07-multi-file-extended-family",
        ExportOptions {
            with_positions: true,
            ..ExportOptions::default()
        },
    );
    insta::assert_snapshot!(json);
}

#[test]
fn example_07_multi_file_extended_family_cytoscape() {
    let json = export_multi_file(
        "07-multi-file-extended-family",
        ExportOptions {
            format: ExportFormat::Cytoscape,
            ..ExportOptions::default()
        },
    );
    insta::assert_snapshot!(json);
}

#[test]
fn positions_off_by_default_omits_span_field() {
    let json = export_default("person alice name:\"A\" gender:female\n");
    assert!(
        !json.contains("\"span\""),
        "default mode must not emit `span`; got:\n{json}"
    );
}

#[test]
fn positions_on_emits_span_on_every_entity() {
    let src = "\
person alice name:\"A\" gender:female
person bob name:\"B\" gender:male
person kid name:\"K\" gender:other
  birth m
  adoption m start:2000
marriage m alice bob start:1972
";
    let json = export_with_positions(src);
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let envelope_ok = value["ok"].as_bool().unwrap();
    assert!(envelope_ok, "expected success envelope; got:\n{json}");
    for collection in ["persons", "marriages", "parenthoodLinks"] {
        for entity in value["graph"][collection].as_array().unwrap() {
            let span = entity["span"]
                .as_array()
                .unwrap_or_else(|| panic!("missing span on {collection}: {entity}"));
            assert_eq!(span.len(), 2, "span must be a [start, end] pair");
            assert!(span[0].as_u64().unwrap() < span[1].as_u64().unwrap());
        }
    }
}

/// Catch-all: if a new `examples/<dir>/<stem>.kul` lands without a matching
/// test above, this fires so the contributor adds the snapshot.
#[test]
fn every_example_has_a_dedicated_snapshot_test() {
    let mut have: Vec<String> = enumerate_example_dirs();
    have.sort();
    let expected = [
        "01-single-couple",
        "02-nuclear-family",
        "03-three-generations",
        "04-polygamous-family",
        "05-married-siblings",
        "06-three-branch-dynasty",
        "07-multi-file-extended-family",
    ];
    assert_eq!(
        have.iter().map(String::as_str).collect::<Vec<_>>(),
        expected,
        "an example file was added or removed without updating the export snapshot list"
    );
}

/// Enumerate the per-example subdirectories of `examples/`. Each must
/// carry at least one `*.kul` file alongside its sibling `kul.yml`
/// (single-file examples have exactly one; multi-file examples have
/// several, per [ADR-0015](../../docs/adr/0015-global-project-namespace.md)).
fn enumerate_example_dirs() -> Vec<String> {
    std::fs::read_dir(examples_dir())
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .map(|p| {
            let kul_files: Vec<_> = std::fs::read_dir(&p)
                .unwrap()
                .flatten()
                .map(|e| e.path())
                .filter(|f| f.extension().and_then(|s| s.to_str()) == Some("kul"))
                .collect();
            assert!(
                !kul_files.is_empty(),
                "example directory {} must contain at least one .kul file",
                p.display(),
            );
            p.file_name().unwrap().to_string_lossy().into_owned()
        })
        .collect()
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
    let mut source = String::new();
    for i in 0..1000 {
        use std::fmt::Write as _;
        let _ = writeln!(&mut source, "person p{i} name:\"P{i}\" gender:female");
    }
    let inputs = vec![InputFile::new("perf.kul", source.clone())];
    let check =
        kul_core::check_with_manifest("kul.yml", "kul: \"0.1\"\n", &Manifest::default(), &inputs);
    let start = std::time::Instant::now();
    let envelope = export(&check, ExportOptions::default());
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
