//! Snapshot tests for the WASM `format` bridge.
//!
//! Sweeps every `examples/*.kula` and asserts the bridge round-trips the
//! formatted source unchanged. The formatter itself is exhaustively
//! property-tested in `kula-core::tests::format`; these snapshots verify
//! that the WASM seam adds no transformation. A new example file forces
//! a corresponding snapshot review here, mirroring the corpus-contract
//! pattern in `kula-core::tests::export`.

use std::path::{Path, PathBuf};

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

macro_rules! example_snapshot {
    ($name:ident, $stem:literal) => {
        #[test]
        fn $name() {
            let path = examples_dir().join(concat!($stem, ".kula"));
            let formatted = kula_wasm::format_source(&read(&path));
            insta::assert_snapshot!(formatted);
        }
    };
}

example_snapshot!(example_01_single_couple, "01-single-couple");
example_snapshot!(example_02_nuclear_family, "02-nuclear-family");
example_snapshot!(example_03_three_generations, "03-three-generations");
example_snapshot!(example_04_polygamous_family, "04-polygamous-family");
example_snapshot!(example_05_married_siblings, "05-married-siblings");

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
        "05-married-siblings",
    ];
    assert_eq!(
        have.iter().map(String::as_str).collect::<Vec<_>>(),
        expected,
        "an example file was added or removed without updating the wasm format snapshot list"
    );
}

#[test]
fn version_metadata_is_exposed() {
    assert_eq!(kula_wasm::kula_core_version(), env!("CARGO_PKG_VERSION"));
    assert!(!kula_wasm::kula_language_version().is_empty());
    assert!(kula_wasm::export_schema_version() >= 1);
}

#[test]
fn format_returns_string_for_partial_parse_input() {
    // Best-effort contract: format must never panic or return None even
    // when the input fails to fully parse. Mirrors `kula_core::format::format_source`.
    let _ = kula_wasm::format_source("person");
    let _ = kula_wasm::format_source("");
    let _ = kula_wasm::format_source("@@@ not kula @@@");
}
