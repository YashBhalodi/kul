//! Snapshot tests for the canonical JSON export.
//!
//! Sweeps every `examples/*/<name>.kul` for the success envelope and
//! covers a few hand-crafted bad inputs for the failure envelope.
//! Snapshots are pretty-printed for diff-friendly review.

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

/// Generate three snapshot tests per example: default, positions on, cytoscape.
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
    example_01_nuclear_family,
    example_01_nuclear_family_with_positions,
    example_01_nuclear_family_cytoscape,
    "01-nuclear-family",
    "nuclear-family"
);
example_snapshot!(
    example_02_three_generations,
    example_02_three_generations_with_positions,
    example_02_three_generations_cytoscape,
    "02-three-generations",
    "three-generations"
);
example_snapshot!(
    example_03_divorce_and_remarriage,
    example_03_divorce_and_remarriage_with_positions,
    example_03_divorce_and_remarriage_cytoscape,
    "03-divorce-and-remarriage",
    "divorce-and-remarriage"
);
example_snapshot!(
    example_04_adoption_and_belonging,
    example_04_adoption_and_belonging_with_positions,
    example_04_adoption_and_belonging_cytoscape,
    "04-adoption-and-belonging",
    "adoption-and-belonging"
);
example_snapshot!(
    example_05_cousins_and_in_laws,
    example_05_cousins_and_in_laws_with_positions,
    example_05_cousins_and_in_laws_cytoscape,
    "05-cousins-and-in-laws",
    "cousins-and-in-laws"
);
example_snapshot!(
    example_06_polygamous_household,
    example_06_polygamous_household_with_positions,
    example_06_polygamous_household_cytoscape,
    "06-polygamous-household",
    "polygamous-household"
);
example_snapshot!(
    example_07_disconnected_lineages,
    example_07_disconnected_lineages_with_positions,
    example_07_disconnected_lineages_cytoscape,
    "07-disconnected-lineages",
    "disconnected-lineages"
);
example_snapshot!(
    example_09_family_across_a_century,
    example_09_family_across_a_century_with_positions,
    example_09_family_across_a_century_cytoscape,
    "09-family-across-a-century",
    "family-across-a-century"
);

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
    let check = kul_core::check_with_manifest("kul.yml", "", &Manifest::default(), &inputs);
    let envelope = export(&check, options);
    serde_json::to_string_pretty(&envelope).expect("serialize envelope")
}

#[test]
fn example_08_multi_file_project() {
    let json = export_multi_file("08-multi-file-project", ExportOptions::default());
    insta::assert_snapshot!(json);
}

#[test]
fn example_08_multi_file_project_with_positions() {
    let json = export_multi_file(
        "08-multi-file-project",
        ExportOptions {
            with_positions: true,
            ..ExportOptions::default()
        },
    );
    insta::assert_snapshot!(json);
}

#[test]
fn example_08_multi_file_project_cytoscape() {
    let json = export_multi_file(
        "08-multi-file-project",
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

#[test]
fn every_example_has_a_dedicated_snapshot_test() {
    let mut have: Vec<String> = enumerate_example_dirs();
    have.sort();
    let expected = [
        "01-nuclear-family",
        "02-three-generations",
        "03-divorce-and-remarriage",
        "04-adoption-and-belonging",
        "05-cousins-and-in-laws",
        "06-polygamous-household",
        "07-disconnected-lineages",
        "08-multi-file-project",
        "09-family-across-a-century",
    ];
    assert_eq!(
        have.iter().map(String::as_str).collect::<Vec<_>>(),
        expected,
        "an example file was added or removed without updating the export snapshot list"
    );
}

/// Enumerate per-example subdirectories of `examples/`. Each must carry at
/// least one `*.kul` file (per ADR-0015).
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

/// Pin the declaration-order contract from `spec/16-export-schema.md` §15.2:
/// declarations appear in source order across files (lexicographic by file
/// name) and within each file. The snapshot projects only ordering-relevant
/// fields so it does not churn on date / span / envelope detail.
#[test]
fn declaration_order_is_preserved_across_files_in_lexicographic_order() {
    let a_family = "\
person alice name:\"Alice\" gender:female born:1950
person bob name:\"Bob\" gender:male born:1948
person kid name:\"Kid\" gender:other born:1980
  birth m_alice_bob
  adoption m_alice_carol start:1985
  adoption m_dave_eve start:1990
marriage m_alice_bob alice bob start:1972
";
    let b_family = "\
person carol name:\"Carol\" gender:female born:1955
person dave name:\"Dave\" gender:male born:1952
person eve name:\"Eve\" gender:female born:1958
marriage m_alice_carol alice carol start:1985
marriage m_dave_eve dave eve start:1980
";
    // Mirror the loader's lexicographic file ordering.
    let inputs = vec![
        InputFile::new("a_family.kul", a_family),
        InputFile::new("b_family.kul", b_family),
    ];
    let check = kul_core::check_with_manifest("kul.yml", "", &Manifest::default(), &inputs);
    let envelope = export(&check, ExportOptions::default());
    let projection = project_ordering(&envelope);
    let json = serde_json::to_string_pretty(&projection).expect("serialize projection");
    insta::assert_snapshot!(json);
}

/// Ordering-only projection of an export envelope.
#[derive(serde::Serialize)]
struct OrderingProjection {
    person_ids: Vec<String>,
    marriages: Vec<MarriageOrdering>,
    parenthood_links: Vec<ParenthoodOrdering>,
}

#[derive(serde::Serialize)]
struct MarriageOrdering {
    id: String,
    spouses: [String; 2],
}

#[derive(serde::Serialize)]
struct ParenthoodOrdering {
    child_id: String,
    marriage_id: String,
    kind: String,
}

fn project_ordering(envelope: &kul_core::export::ExportEnvelope) -> OrderingProjection {
    let kul_core::export::ExportEnvelope::Success(success) = envelope else {
        panic!("expected success envelope; got failure");
    };
    let graph = success
        .graph
        .as_native()
        .expect("expected native graph payload");
    OrderingProjection {
        person_ids: graph.persons.iter().map(|p| p.id.clone()).collect(),
        marriages: graph
            .marriages
            .iter()
            .map(|m| MarriageOrdering {
                id: m.id.clone(),
                spouses: m.spouses.clone(),
            })
            .collect(),
        parenthood_links: graph
            .parenthood_links
            .iter()
            .map(|l| ParenthoodOrdering {
                child_id: l.child_id.clone(),
                marriage_id: l.marriage_id.clone(),
                kind: serde_json::to_value(l.kind)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .expect("ParenthoodLinkKind serializes to a string"),
            })
            .collect(),
    }
}

#[test]
fn one_thousand_statement_export_under_budget() {
    let mut source = String::new();
    for i in 0..1000 {
        use std::fmt::Write as _;
        let _ = writeln!(&mut source, "person p{i} name:\"P{i}\" gender:female");
    }
    let inputs = vec![InputFile::new("perf.kul", source.clone())];
    let check = kul_core::check_with_manifest("kul.yml", "", &Manifest::default(), &inputs);
    let start = std::time::Instant::now();
    let envelope = export(&check, ExportOptions::default());
    let _json = serde_json::to_string(&envelope).expect("serialize");
    let elapsed = start.elapsed();
    eprintln!("1000-statement export + serialize: {elapsed:?}");
    // Real target ~30ms; 5x ceiling absorbs CI/debug-build noise while
    // still catching a 2x regression.
    assert!(
        elapsed < std::time::Duration::from_millis(150),
        "1000-statement export budget exceeded: {elapsed:?}"
    );
}
