//! Snapshot + cross-surface tests for the WASM `renderSvg` bridge:
//! per-example snapshot, byte-identical JSON against the direct
//! `compute → layout → render` pipeline, and a failure round-trip.

use std::path::{Path, PathBuf};

use kul_core::ast::InputFile;
use kul_layout::{LayoutConfig, layout};
use kul_render::{RenderShape, compute};
use kul_svg::{ThemeConfig, render};
use kul_wasm::{RenderEnvelope, RenderFailure, RenderSuccess, WasmInputFile};

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

fn render_svg_json(inputs: &[InputFile]) -> String {
    let manifest = kul_core::manifest::Manifest::default();
    let envelope = kul_wasm::render_svg_with(inputs, &manifest);
    serde_json::to_string_pretty(&envelope).expect("serialize envelope")
}

/// Cross-surface oracle: reconstruct the envelope by driving the deep
/// modules directly. The wasm bridge must produce byte-identical JSON.
fn direct_pipeline_json(inputs: &[InputFile]) -> String {
    let check = kul_core::check_with_manifest(
        "kul.yml",
        "",
        &kul_core::manifest::Manifest::default(),
        inputs,
    );
    let shape = compute(&check);
    let envelope = match shape {
        RenderShape::Failure(f) => RenderEnvelope::Failure(RenderFailure {
            ok: false,
            diagnostics: f.diagnostics,
        }),
        RenderShape::Success(s) => {
            let positioned = layout(&s, &LayoutConfig::default());
            let svg = render(&positioned, &ThemeConfig::default());
            RenderEnvelope::Success(RenderSuccess { ok: true, svg })
        }
    };
    serde_json::to_string_pretty(&envelope).expect("serialize envelope")
}

#[test]
fn example_02_three_generations() {
    let inputs = project_inputs(&examples_dir().join("02-three-generations"));
    let json = render_svg_json(&inputs);
    insta::assert_snapshot!(json);
}

#[test]
fn cross_surface_json_is_bit_identical_for_every_example() {
    for dir_entry in std::fs::read_dir(examples_dir()).unwrap().flatten() {
        let dir = dir_entry.path();
        if !dir.is_dir() {
            continue;
        }
        let inputs = project_inputs(&dir);
        let project_label = dir.file_name().unwrap().to_string_lossy().into_owned();
        let wasm_json = render_svg_json(&inputs);
        let direct_json = direct_pipeline_json(&inputs);
        assert_eq!(
            wasm_json, direct_json,
            "wasm renderSvg and direct pipeline diverged for project {project_label}"
        );
    }
}

/// Drives the public wasm-ABI signature (`Vec<WasmInputFile>`) to
/// confirm the `WasmInputFile` → `InputFile` conversion is wired up.
#[test]
fn wasm_abi_signature_round_trips_to_render_svg_with() {
    let path = examples_dir()
        .join("01-nuclear-family")
        .join("nuclear-family.kul");
    let source = read(&path);
    let manifest = kul_core::manifest::Manifest::default();
    let files = vec![WasmInputFile {
        name: "nuclear-family.kul".into(),
        source: source.clone(),
    }];
    let via_abi = kul_wasm::render_svg(files, manifest.clone());
    let via_native =
        kul_wasm::render_svg_with(&[InputFile::new("nuclear-family.kul", source)], &manifest);
    let abi_json = serde_json::to_string_pretty(&via_abi).expect("abi");
    let native_json = serde_json::to_string_pretty(&via_native).expect("native");
    assert_eq!(abi_json, native_json);
}

#[test]
fn failure_envelope_for_broken_source_is_bit_identical() {
    let inputs = vec![InputFile::new("input.kul", "person alice gender:female\n")];
    let wasm_json = render_svg_json(&inputs);
    let direct_json = direct_pipeline_json(&inputs);
    assert_eq!(wasm_json, direct_json);
    assert!(
        wasm_json.contains("\"ok\": false"),
        "expected failure envelope; got:\n{wasm_json}"
    );
    assert!(
        wasm_json.contains("\"diagnostics\""),
        "expected diagnostics in failure envelope; got:\n{wasm_json}"
    );
}
