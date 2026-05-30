//! Snapshot + cross-surface tests for the WASM `exportGraph` bridge.
//!
//! Per-example snapshots (default, `withPositions`, cytoscape) lock the
//! envelope shape; a cross-surface test asserts byte-identical JSON
//! against `kul_core::export::export` for every example × option combo;
//! a failure round-trip covers strict-on-errors.

use std::path::{Path, PathBuf};

use kul_core::ast::InputFile;
use kul_core::export::{ExportFormat, ExportOptions};
use kul_wasm::WasmInputFile;

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

fn project_inputs(dir: &Path) -> Vec<InputFile> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("read {}: {err}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("kul"))
        .collect();
    entries.sort();
    entries
        .iter()
        .map(|p| {
            InputFile::new(
                p.file_name().unwrap().to_string_lossy().into_owned(),
                read(p),
            )
        })
        .collect()
}

fn export_graph_json(inputs: &[InputFile], options: ExportOptions) -> String {
    let manifest = kul_core::manifest::Manifest::default();
    let envelope = kul_wasm::export_with(inputs, &manifest, options);
    serde_json::to_string_pretty(&envelope).expect("serialize envelope")
}

fn core_export_json(inputs: &[InputFile], options: ExportOptions) -> String {
    let check = kul_core::check_with_manifest(
        "kul.yml",
        "",
        &kul_core::manifest::Manifest::default(),
        inputs,
    );
    let envelope = kul_core::export::export(&check, options);
    serde_json::to_string_pretty(&envelope).expect("serialize envelope")
}

fn options_default() -> ExportOptions {
    ExportOptions::default()
}

fn options_with_positions() -> ExportOptions {
    ExportOptions {
        with_positions: true,
        ..ExportOptions::default()
    }
}

fn options_cytoscape() -> ExportOptions {
    ExportOptions {
        format: ExportFormat::Cytoscape,
        ..ExportOptions::default()
    }
}

macro_rules! example_snapshot {
    ($default_name:ident, $positions_name:ident, $cytoscape_name:ident, $dir:literal, $stem:literal) => {
        #[test]
        fn $default_name() {
            let path = examples_dir().join($dir).join(concat!($stem, ".kul"));
            let inputs = vec![InputFile::new(concat!($stem, ".kul"), read(&path))];
            let json = export_graph_json(&inputs, options_default());
            insta::assert_snapshot!(json);
        }

        #[test]
        fn $positions_name() {
            let path = examples_dir().join($dir).join(concat!($stem, ".kul"));
            let inputs = vec![InputFile::new(concat!($stem, ".kul"), read(&path))];
            let json = export_graph_json(&inputs, options_with_positions());
            insta::assert_snapshot!(json);
        }

        #[test]
        fn $cytoscape_name() {
            let path = examples_dir().join($dir).join(concat!($stem, ".kul"));
            let inputs = vec![InputFile::new(concat!($stem, ".kul"), read(&path))];
            let json = export_graph_json(&inputs, options_cytoscape());
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

#[test]
fn example_08_multi_file_project() {
    let inputs = project_inputs(&examples_dir().join("08-multi-file-project"));
    let json = export_graph_json(&inputs, options_default());
    insta::assert_snapshot!(json);
}

#[test]
fn example_08_multi_file_project_with_positions() {
    let inputs = project_inputs(&examples_dir().join("08-multi-file-project"));
    let json = export_graph_json(&inputs, options_with_positions());
    insta::assert_snapshot!(json);
}

#[test]
fn example_08_multi_file_project_cytoscape() {
    let inputs = project_inputs(&examples_dir().join("08-multi-file-project"));
    let json = export_graph_json(&inputs, options_cytoscape());
    insta::assert_snapshot!(json);
}

#[test]
fn every_example_has_a_dedicated_export_graph_test() {
    let mut have: Vec<String> = std::fs::read_dir(examples_dir())
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
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
        "an example file was added or removed without updating the wasm exportGraph snapshot list"
    );
}

type OptionsCombo = (&'static str, fn() -> ExportOptions);

#[test]
fn cross_surface_json_is_bit_identical_for_every_example_and_options_combo() {
    let combos: &[OptionsCombo] = &[
        ("default", options_default),
        ("with_positions", options_with_positions),
        ("cytoscape", options_cytoscape),
    ];
    for dir_entry in std::fs::read_dir(examples_dir()).unwrap().flatten() {
        let dir = dir_entry.path();
        if !dir.is_dir() {
            continue;
        }
        let inputs = project_inputs(&dir);
        let project_label = dir.file_name().unwrap().to_string_lossy().into_owned();
        for (combo_name, opts_fn) in combos {
            let opts = opts_fn();
            let wasm_json = export_graph_json(&inputs, opts);
            let core_json = core_export_json(&inputs, opts);
            assert_eq!(
                wasm_json, core_json,
                "wasm exportGraph and kul_core::export diverged for project {project_label} with options {combo_name}"
            );
        }
    }
}

/// Drives the public wasm-ABI signature (`Vec<WasmInputFile>`) to
/// confirm the `WasmInputFile` → `InputFile` conversion is wired up.
#[test]
fn wasm_abi_signature_round_trips_to_export_with() {
    let path = examples_dir()
        .join("01-nuclear-family")
        .join("nuclear-family.kul");
    let source = read(&path);
    let manifest = kul_core::manifest::Manifest::default();
    let files = vec![WasmInputFile {
        name: "nuclear-family.kul".into(),
        source: source.clone(),
    }];
    let via_abi = kul_wasm::export_graph(files, manifest.clone(), None);
    let via_native = kul_wasm::export_with(
        &[InputFile::new("nuclear-family.kul", source)],
        &manifest,
        ExportOptions::default(),
    );
    let abi_json = serde_json::to_string_pretty(&via_abi).expect("abi");
    let native_json = serde_json::to_string_pretty(&via_native).expect("native");
    assert_eq!(abi_json, native_json);
}

#[test]
fn failure_envelope_for_broken_source_is_bit_identical() {
    let inputs = vec![InputFile::new("input.kul", "person alice gender:female\n")];
    let wasm_json = export_graph_json(&inputs, options_default());
    let core_json = core_export_json(&inputs, options_default());
    assert_eq!(wasm_json, core_json);
    assert!(
        wasm_json.contains("\"ok\": false"),
        "expected failure envelope; got:\n{wasm_json}"
    );
}
