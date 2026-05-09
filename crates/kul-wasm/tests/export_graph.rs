//! Snapshot + cross-surface tests for the WASM `exportGraph` bridge.
//!
//! Two contracts:
//!
//! - **Per-example snapshots** lock the envelope shape per option combo
//!   (default, `withPositions: true`, `format: "cytoscape"`). Mirrors the
//!   matrix in `kul-core::tests::export` so any drift between the CLI
//!   export envelope and the WASM bridge surfaces immediately.
//! - **Cross-surface bit-identical** asserts that the pretty-printed JSON
//!   from the WASM bridge equals the pretty-printed JSON from a direct
//!   `kul_core::export::export` call for every example × every option
//!   combo. WASM `check` has no CLI counterpart, but `exportGraph` does,
//!   and the two surfaces must speak the same JSON.
//!
//! See [ADR-0011](../../../docs/adr/0011-wasm-surface-three-shapes-no-wrappers.md)
//! — `exportGraph` exists so JS-ecosystem consumers can reach
//! `kul_core::export::export` without shelling out to the CLI, with the
//! same envelope shape on both surfaces.

use std::path::{Path, PathBuf};

use kul_core::export::{ExportFormat, ExportOptions};

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

fn export_graph_json(source: &str, options: ExportOptions) -> String {
    let manifest = kul_core::manifest::Manifest::default();
    let envelope = kul_wasm::export_with(source, &manifest, options);
    serde_json::to_string_pretty(&envelope).expect("serialize envelope")
}

fn core_export_json(source: &str, options: ExportOptions) -> String {
    let check = kul_core::check(source, &kul_core::manifest::Manifest::default());
    let envelope = kul_core::export::export(source, &check, options);
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
    ($default_name:ident, $positions_name:ident, $cytoscape_name:ident, $stem:literal) => {
        #[test]
        fn $default_name() {
            let path = examples_dir().join(concat!($stem, ".kul"));
            let json = export_graph_json(&read(&path), options_default());
            insta::assert_snapshot!(json);
        }

        #[test]
        fn $positions_name() {
            let path = examples_dir().join(concat!($stem, ".kul"));
            let json = export_graph_json(&read(&path), options_with_positions());
            insta::assert_snapshot!(json);
        }

        #[test]
        fn $cytoscape_name() {
            let path = examples_dir().join(concat!($stem, ".kul"));
            let json = export_graph_json(&read(&path), options_cytoscape());
            insta::assert_snapshot!(json);
        }
    };
}

example_snapshot!(
    example_01_single_couple,
    example_01_single_couple_with_positions,
    example_01_single_couple_cytoscape,
    "01-single-couple"
);
example_snapshot!(
    example_02_nuclear_family,
    example_02_nuclear_family_with_positions,
    example_02_nuclear_family_cytoscape,
    "02-nuclear-family"
);
example_snapshot!(
    example_03_three_generations,
    example_03_three_generations_with_positions,
    example_03_three_generations_cytoscape,
    "03-three-generations"
);
example_snapshot!(
    example_04_polygamous_family,
    example_04_polygamous_family_with_positions,
    example_04_polygamous_family_cytoscape,
    "04-polygamous-family"
);
example_snapshot!(
    example_05_married_siblings,
    example_05_married_siblings_with_positions,
    example_05_married_siblings_cytoscape,
    "05-married-siblings"
);
example_snapshot!(
    example_06_three_branch_dynasty,
    example_06_three_branch_dynasty_with_positions,
    example_06_three_branch_dynasty_cytoscape,
    "06-three-branch-dynasty"
);

#[test]
fn every_example_has_a_dedicated_export_graph_test() {
    let mut have: Vec<String> = std::fs::read_dir(examples_dir())
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("kul"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    have.sort();
    let expected = [
        "01-single-couple",
        "02-nuclear-family",
        "03-three-generations",
        "04-polygamous-family",
        "05-married-siblings",
        "06-three-branch-dynasty",
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
    for entry in std::fs::read_dir(examples_dir()).unwrap().flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("kul") {
            continue;
        }
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        let source = read(&path);
        for (combo_name, opts_fn) in combos {
            let opts = opts_fn();
            let wasm_json = export_graph_json(&source, opts);
            let core_json = core_export_json(&source, opts);
            assert_eq!(
                wasm_json, core_json,
                "wasm exportGraph and kul_core::export diverged for example {stem} with options {combo_name}"
            );
        }
    }
}

#[test]
fn failure_envelope_for_broken_source_is_bit_identical() {
    // Sanity: the strict-on-errors path also round-trips byte-for-byte.
    let src = "person alice gender:female\n";
    let wasm_json = export_graph_json(src, options_default());
    let core_json = core_export_json(src, options_default());
    assert_eq!(wasm_json, core_json);
    assert!(
        wasm_json.contains("\"ok\": false"),
        "expected failure envelope; got:\n{wasm_json}"
    );
}
