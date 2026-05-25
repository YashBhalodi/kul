//! Snapshot + cross-surface tests for the WASM `renderSvg` bridge.
//!
//! Three contracts:
//!
//! - **Per-example snapshot** for `examples/02-three-generations/` locks
//!   the envelope shape (an `ok: true` envelope carrying the canonical
//!   SVG) for a representative example. Follow-up
//!   issues extend the snapshotted set one example at a time.
//! - **Cross-surface bit-identical** asserts that the pretty-printed
//!   JSON of the WASM `renderSvg` envelope equals the pretty-printed
//!   JSON of running the pipeline directly
//!   (`kul_render::compute` → `kul_layout::layout` → `kul_svg::render`,
//!   then constructing the same envelope variants) for every example,
//!   plus a deliberately-broken source to exercise the failure arm.
//!   Mirrors the cross-surface test in `export_graph.rs`.
//! - **Failure round-trip** confirms that a deliberately-broken source
//!   produces an `ok: false` envelope carrying the same
//!   `ExportedDiagnostic` shape `exportGraph`'s failure envelope uses,
//!   so consumers can share diagnostic rendering across the two
//!   operations.
//!
//! See ADR-0011 (amended) — `renderSvg` exists so JS-ecosystem
//! consumers can reach the full canonical-visual pipeline without
//! shelling out to the LSP, with bit-identical JSON on both surfaces.

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

/// Reconstruct the envelope by driving the deep modules
/// (`kul_render::compute` → `kul_layout::layout` → `kul_svg::render`)
/// directly and wrapping the result in the same shape `kul-wasm`
/// returns. This is the cross-surface oracle: the wasm bridge must
/// produce byte-for-byte identical JSON.
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
        RenderShape::Success(_) => {
            let positioned = layout(&shape, &LayoutConfig::default());
            let svg = render(&positioned, &ThemeConfig::default());
            RenderEnvelope::Success(RenderSuccess { ok: true, svg })
        }
    };
    serde_json::to_string_pretty(&envelope).expect("serialize envelope")
}

/// v1 tracer per the PRD: `examples/02-three-generations/` is the only
/// example whose SVG snapshot is committed today. Follow-up issues
/// extend the snapshotted set one pattern-primitive at a time.
#[test]
fn example_02_three_generations() {
    let inputs = project_inputs(&examples_dir().join("02-three-generations"));
    let json = render_svg_json(&inputs);
    insta::assert_snapshot!(json);
}

/// Cross-surface bit-identical: the wasm bridge JSON must equal the
/// JSON produced by driving the deep modules directly, for every
/// example in the corpus. Catches drift between the two construction
/// paths regardless of how the corpus grows.
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

/// The wasm-bridge `renderSvg` is implemented in terms of
/// [`kul_wasm::render_svg_with`], which the snapshots above call
/// directly. This smoke test drives the public wasm-ABI signature
/// (`Vec<WasmInputFile>`) to confirm the `WasmInputFile` → `InputFile`
/// conversion is wired up correctly and produces the same envelope.
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

/// Failure case: a deliberately-broken source produces a
/// `{ ok: false, diagnostics: [...] }` envelope. The diagnostic shape
/// matches what `exportGraph`'s failure envelope produces (same
/// `ExportedDiagnostic` type), and the JSON matches the direct
/// pipeline reconstruction byte-for-byte.
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
