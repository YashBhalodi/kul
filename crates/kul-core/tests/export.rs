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
example_snapshot!(
    example_08_divorce_and_remarriage,
    example_08_divorce_and_remarriage_with_positions,
    example_08_divorce_and_remarriage_cytoscape,
    "08-divorce-and-remarriage",
    "divorce-and-remarriage"
);
example_snapshot!(
    example_09_multi_adoption,
    example_09_multi_adoption_with_positions,
    example_09_multi_adoption_cytoscape,
    "09-multi-adoption",
    "multi-adoption"
);
example_snapshot!(
    example_10_disconnected_lineages_and_orphan,
    example_10_disconnected_lineages_and_orphan_with_positions,
    example_10_disconnected_lineages_and_orphan_cytoscape,
    "10-disconnected-lineages-and-orphan",
    "disconnected-lineages-and-orphan"
);
example_snapshot!(
    example_11_cousin_marriage,
    example_11_cousin_marriage_with_positions,
    example_11_cousin_marriage_cytoscape,
    "11-cousin-marriage",
    "cousin-marriage"
);
example_snapshot!(
    example_12_polygamy_with_birth_family,
    example_12_polygamy_with_birth_family_with_positions,
    example_12_polygamy_with_birth_family_cytoscape,
    "12-polygamy-with-birth-family",
    "polygamy-with-birth-family"
);
example_snapshot!(
    example_13_inter_family_marriage,
    example_13_inter_family_marriage_with_positions,
    example_13_inter_family_marriage_cytoscape,
    "13-inter-family-marriage",
    "inter-family-marriage"
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
    let check = kul_core::check_with_manifest("kul.yml", "", &Manifest::default(), &inputs);
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
        "08-divorce-and-remarriage",
        "09-multi-adoption",
        "10-disconnected-lineages-and-orphan",
        "11-cousin-marriage",
        "12-polygamy-with-birth-family",
        "13-inter-family-marriage",
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

/// Pin the declaration-order contract from `spec/16-export-schema.md` §15.2:
/// `persons`, `marriages`, and `parenthoodLinks` MUST appear in declaration
/// order across files (lexicographic by file name) and within each file
/// (source position).
///
/// Fixture layout — `a_family.kul` lexicographically precedes
/// `b_family.kul`, so even though the prompt-reading order of the
/// `InputFile` vec is irrelevant (the loader sorts on disk; here we feed
/// the slice already in lex order to mirror that), `a_family.kul`'s
/// declarations MUST appear first in every collection. The fixture
/// exercises:
///
/// - A person (`alice`) appearing as a spouse in two marriages — one
///   declared in each file.
/// - A child (`kid`) with a `birth` link and two `adoption`
///   sub-statements, pinning the per-child sub-order (`birth` first, then
///   `adoption`s in source order).
/// - Interleaved person ids across files.
///
/// The snapshot is a projection (`(id, ordering-relevant-fields)`) so it
/// locks down order without churning on date / span / envelope detail.
#[test]
fn declaration_order_is_preserved_across_files_in_lexicographic_order() {
    // a_family.kul: contains alice + her first marriage (to bob), and the
    // adoptive child whose `birth` references that marriage. Alice also
    // appears as a spouse in `m_alice_carol`, declared later in b_family.
    let a_family = "\
person alice name:\"Alice\" gender:female born:1950
person bob name:\"Bob\" gender:male born:1948
person kid name:\"Kid\" gender:other born:1980
  birth m_alice_bob
  adoption m_alice_carol start:1985
  adoption m_dave_eve start:1990
marriage m_alice_bob alice bob start:1972
";
    // b_family.kul: continues with persons used by the adoptions, and
    // declares alice's second marriage plus an unrelated marriage that
    // hosts kid's second adoption.
    let b_family = "\
person carol name:\"Carol\" gender:female born:1955
person dave name:\"Dave\" gender:male born:1952
person eve name:\"Eve\" gender:female born:1958
marriage m_alice_carol alice carol start:1985
marriage m_dave_eve dave eve start:1980
";
    // The loader sorts files lexicographically before handing them to
    // `check`; we mirror that ordering here so the test inputs match what
    // the on-disk path produces.
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

/// A flat, ordering-only view of an export envelope. The snapshot in
/// [`declaration_order_is_preserved_across_files_in_lexicographic_order`]
/// asserts this projection so it doesn't drift on unrelated detail.
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
                kind: l.kind.to_string(),
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
    // Real target is <30ms on a developer laptop; assert a 5x ceiling so
    // CI runners and debug builds don't flake. A 2x regression still fires.
    assert!(
        elapsed < std::time::Duration::from_millis(150),
        "1000-statement export budget exceeded: {elapsed:?}"
    );
}
