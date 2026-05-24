//! Snapshot + cross-surface tests for the WASM `exportGraph` bridge.
//!
//! Three contracts:
//!
//! - **Per-example snapshots** lock the envelope shape per option combo
//!   (default, `withPositions: true`, `format: "cytoscape"`). Mirrors the
//!   matrix in `kul-core::tests::export` so any drift between the CLI
//!   export envelope and the WASM bridge surfaces immediately. The
//!   multi-file example (`07-multi-file-extended-family`) is included to
//!   exercise the array-based signature end-to-end.
//! - **Cross-surface bit-identical** asserts that the pretty-printed JSON
//!   from the WASM bridge equals the pretty-printed JSON from a direct
//!   `kul_core::export::export` call for every example × every option
//!   combo, including the multi-file case. WASM `check` has no CLI
//!   counterpart, but `exportGraph` does, and the two surfaces must speak
//!   the same JSON regardless of how many files the project holds.
//! - **Failure round-trip** confirms strict-on-errors produces a byte-for-
//!   byte identical failure envelope across the two surfaces.
//!
//! See [ADR-0011](../../../docs/adr/0011-wasm-surface-three-shapes-no-wrappers.md)
//! — `exportGraph` exists so JS-ecosystem consumers can reach
//! `kul_core::export::export` without shelling out to the CLI, with the
//! same envelope shape on both surfaces.

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

/// Multi-file example: snapshots assert the WASM bridge unions persons,
/// marriages, and parenthood links across every `.kul` file in the
/// project, with the same envelope shape as the single-file path.
#[test]
fn example_07_multi_file_extended_family() {
    let inputs = project_inputs(&examples_dir().join("07-multi-file-extended-family"));
    let json = export_graph_json(&inputs, options_default());
    insta::assert_snapshot!(json);
}

#[test]
fn example_07_multi_file_extended_family_with_positions() {
    let inputs = project_inputs(&examples_dir().join("07-multi-file-extended-family"));
    let json = export_graph_json(&inputs, options_with_positions());
    insta::assert_snapshot!(json);
}

#[test]
fn example_07_multi_file_extended_family_cytoscape() {
    let inputs = project_inputs(&examples_dir().join("07-multi-file-extended-family"));
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

/// The wasm-bridge `exportGraph` is implemented in terms of
/// [`kul_wasm::export_with`], which the Rust-side snapshots above call
/// directly. This smoke test instead drives the public wasm-ABI signature
/// (`Vec<WasmInputFile>`) to confirm the `WasmInputFile` → `InputFile`
/// conversion is wired up correctly and produces the same envelope.
#[test]
fn wasm_abi_signature_round_trips_to_export_with() {
    let path = examples_dir()
        .join("01-single-couple")
        .join("single-couple.kul");
    let source = read(&path);
    let manifest = kul_core::manifest::Manifest::default();
    let files = vec![WasmInputFile {
        name: "single-couple.kul".into(),
        source: source.clone(),
    }];
    let via_abi = kul_wasm::export_graph(files, manifest.clone(), None);
    let via_native = kul_wasm::export_with(
        &[InputFile::new("single-couple.kul", source)],
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
