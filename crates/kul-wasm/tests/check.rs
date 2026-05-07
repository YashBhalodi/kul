//! Snapshot tests for the WASM `check` bridge.
//!
//! Two contracts:
//!
//! - **Failure shapes** — three hand-crafted broken sources (duplicate id,
//!   unresolved reference, missing required field) lock the diagnostic
//!   wire shape that downstream JS consumers depend on. Mirrors
//!   `kul-core::tests::export::failure_envelope_*` so any drift between
//!   the CLI export envelope and the WASM `check` projection surfaces
//!   immediately.
//! - **Clean-corpus sweep** — every `examples/*.kul` must produce an
//!   empty `diagnostics` array. The corpus-contract guard mirrors the
//!   pattern in `kul-core::tests::export` and `format.rs`: dropping a
//!   new example forces a snapshot review here.
//!
//! Broken inputs live as inline strings; the `examples/*.kul` corpus
//! stays documentation-grade.
//!
//! Snapshots are pretty-printed JSON of the `CheckEnvelope`.
//!
//! See [ADR-0011](../../../docs/adr/0011-wasm-surface-three-shapes-no-wrappers.md)
//! for why `check` is its own entrypoint with an empty-array discriminator
//! rather than a uniform `{ ok, ... }` envelope.

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

fn check_json(source: &str) -> String {
    let envelope = kul_wasm::check(source);
    serde_json::to_string_pretty(&envelope).expect("serialize envelope")
}

#[test]
fn failure_envelope_duplicate_id() {
    let src = "\
person alice name:\"Alice\" gender:female
person alice name:\"Alice 2\" gender:female
";
    insta::assert_snapshot!(check_json(src));
}

#[test]
fn failure_envelope_unresolved_reference() {
    let src = "\
person alice name:\"Alice\" gender:female
marriage m alice ghost start:1972
";
    insta::assert_snapshot!(check_json(src));
}

#[test]
fn failure_envelope_missing_required_field() {
    let src = "person alice gender:female\n";
    insta::assert_snapshot!(check_json(src));
}

macro_rules! clean_example {
    ($name:ident, $stem:literal) => {
        #[test]
        fn $name() {
            let path = examples_dir().join(concat!($stem, ".kul"));
            let envelope = kul_wasm::check(&read(&path));
            assert!(
                envelope.diagnostics.is_empty(),
                "{} produced diagnostics: {:#?}",
                $stem,
                envelope.diagnostics
            );
        }
    };
}

clean_example!(example_01_single_couple_is_clean, "01-single-couple");
clean_example!(example_02_nuclear_family_is_clean, "02-nuclear-family");
clean_example!(
    example_03_three_generations_is_clean,
    "03-three-generations"
);
clean_example!(
    example_04_polygamous_family_is_clean,
    "04-polygamous-family"
);
clean_example!(example_05_married_siblings_is_clean, "05-married-siblings");

#[test]
fn every_example_has_a_dedicated_clean_check_test() {
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
    ];
    assert_eq!(
        have.iter().map(String::as_str).collect::<Vec<_>>(),
        expected,
        "an example file was added or removed without updating the wasm check clean-corpus list"
    );
}
