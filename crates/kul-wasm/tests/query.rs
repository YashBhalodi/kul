//! Wiring + cross-surface tests for the WASM query surface.
//!
//! The query envelope's *contract serialization* is pinned by the core
//! `query` snapshots; here we assert only that the WASM bridge is wired to
//! that core path (bit-identical JSON), that the ABI signature round-trips,
//! and that the ok / null-payload / error arms surface correctly. Kinship
//! correctness is not re-tested at the adapter.

use std::path::{Path, PathBuf};

use kul_core::ast::InputFile;
use kul_core::manifest::Manifest;
use kul_core::query::{marriage_lookup, person_lookup};
use kul_wasm::{WasmInputFile, query_marriage_with, query_person, query_person_with};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn nuclear_inputs() -> Vec<InputFile> {
    let path = workspace_root()
        .join("examples")
        .join("01-nuclear-family")
        .join("nuclear-family.kul");
    let source = std::fs::read_to_string(&path).expect("read nuclear-family.kul");
    vec![InputFile::new("nuclear-family.kul", source)]
}

/// The bridge's person lookup is byte-identical to the core `person_lookup`
/// it wraps (the CLI relies on the same equality for `--format json`).
#[test]
fn person_lookup_json_matches_core() {
    let inputs = nuclear_inputs();
    let manifest = Manifest::default();
    let via_wasm = query_person_with(&inputs, &manifest, "hiroshi");
    let check = kul_core::check_with_manifest("kul.yml", "", &manifest, &inputs);
    let via_core = person_lookup(&check, "hiroshi");
    assert_eq!(
        serde_json::to_string_pretty(&via_wasm).unwrap(),
        serde_json::to_string_pretty(&via_core).unwrap(),
    );
}

#[test]
fn marriage_lookup_json_matches_core() {
    let inputs = nuclear_inputs();
    let manifest = Manifest::default();
    let via_wasm = query_marriage_with(&inputs, &manifest, "m_hiroshi_yuki");
    let check = kul_core::check_with_manifest("kul.yml", "", &manifest, &inputs);
    let via_core = marriage_lookup(&check, "m_hiroshi_yuki");
    assert_eq!(
        serde_json::to_string_pretty(&via_wasm).unwrap(),
        serde_json::to_string_pretty(&via_core).unwrap(),
    );
}

/// Drives the public wasm-ABI signature (`Vec<WasmInputFile>` + `String`
/// id) to confirm the conversion is wired up, and that a known id lands in
/// the ok arm carrying the person payload.
#[test]
fn wasm_abi_signature_returns_ok_arm_for_known_id() {
    let inputs = nuclear_inputs();
    let files = vec![WasmInputFile {
        name: "nuclear-family.kul".into(),
        source: inputs[0].source.clone(),
    }];
    let envelope = query_person(files, Manifest::default(), "hiroshi".to_string());
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["result"]["id"], "hiroshi");
}

/// A clean project with an unknown id yields the ok arm with a `null`
/// result — absence is the answer, not an error.
#[test]
fn unknown_id_yields_ok_arm_with_null_result() {
    let inputs = nuclear_inputs();
    let envelope = query_person_with(&inputs, &Manifest::default(), "nobody");
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["ok"], true);
    assert!(json["result"].is_null(), "expected null result: {json}");
}

/// A project that fails its checks yields the error arm carrying the
/// diagnostics — never a partial answer.
#[test]
fn failing_project_yields_error_arm() {
    let inputs = vec![InputFile::new("input.kul", "person alice gender:female\n")];
    let envelope = query_person_with(&inputs, &Manifest::default(), "alice");
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["ok"], false);
    assert!(
        json["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "KUL-R03"),
        "expected KUL-R03 in error arm: {json}"
    );
}
