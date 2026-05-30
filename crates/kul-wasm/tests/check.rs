//! Snapshot tests for the WASM `check` bridge: failure-shape locks,
//! clean-corpus sweep across `examples/*/`, and a cross-file unresolved
//! reference confirming `primary.file` carries the offending file.
//!
//! Snapshots are pretty-printed JSON of the `CheckEnvelope`.

use std::path::{Path, PathBuf};

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

fn input(name: &str, source: &str) -> WasmInputFile {
    WasmInputFile {
        name: name.into(),
        source: source.into(),
    }
}

fn check_json(source: &str) -> String {
    let files = vec![input("input.kul", source)];
    let envelope = kul_wasm::check(files, kul_core::manifest::Manifest::default());
    serde_json::to_string_pretty(&envelope).expect("serialize envelope")
}

fn check_multi_file_json(files: Vec<WasmInputFile>) -> String {
    let envelope = kul_wasm::check(files, kul_core::manifest::Manifest::default());
    serde_json::to_string_pretty(&envelope).expect("serialize envelope")
}

fn multi_file_inputs(dir: &str) -> Vec<WasmInputFile> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(examples_dir().join(dir))
        .unwrap_or_else(|err| panic!("read {dir} dir: {err}"))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("kul"))
        .collect();
    entries.sort();
    entries
        .iter()
        .map(|p| input(&p.file_name().unwrap().to_string_lossy(), &read(p)))
        .collect()
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

/// `primary.file` must point at `b.kul` (where the bad reference lives),
/// not `a.kul` (where the referenced id was declared).
#[test]
fn failure_envelope_unresolved_reference_across_files() {
    let files = vec![
        input("a.kul", "person alice name:\"Alice\" gender:female\n"),
        input("b.kul", "marriage m alice ghost start:1972\n"),
    ];
    insta::assert_snapshot!(check_multi_file_json(files));
}

macro_rules! clean_example {
    ($name:ident, $dir:literal, $stem:literal) => {
        #[test]
        fn $name() {
            let path = examples_dir().join($dir).join(concat!($stem, ".kul"));
            let files = vec![input(concat!($stem, ".kul"), &read(&path))];
            let envelope = kul_wasm::check(files, kul_core::manifest::Manifest::default());
            assert!(
                envelope.diagnostics.is_empty(),
                "{} produced diagnostics: {:#?}",
                $dir,
                envelope.diagnostics
            );
        }
    };
}

clean_example!(
    example_01_nuclear_family_is_clean,
    "01-nuclear-family",
    "nuclear-family"
);
clean_example!(
    example_02_three_generations_is_clean,
    "02-three-generations",
    "three-generations"
);
clean_example!(
    example_03_divorce_and_remarriage_is_clean,
    "03-divorce-and-remarriage",
    "divorce-and-remarriage"
);
clean_example!(
    example_04_adoption_and_belonging_is_clean,
    "04-adoption-and-belonging",
    "adoption-and-belonging"
);
clean_example!(
    example_05_cousins_and_in_laws_is_clean,
    "05-cousins-and-in-laws",
    "cousins-and-in-laws"
);
clean_example!(
    example_06_polygamous_household_is_clean,
    "06-polygamous-household",
    "polygamous-household"
);
clean_example!(
    example_07_disconnected_lineages_is_clean,
    "07-disconnected-lineages",
    "disconnected-lineages"
);
clean_example!(
    example_09_family_across_a_century_is_clean,
    "09-family-across-a-century",
    "family-across-a-century"
);

#[test]
fn example_08_multi_file_project_is_clean() {
    let files = multi_file_inputs("08-multi-file-project");
    let envelope = kul_wasm::check(files, kul_core::manifest::Manifest::default());
    assert!(
        envelope.diagnostics.is_empty(),
        "08-multi-file-project produced diagnostics: {:#?}",
        envelope.diagnostics
    );
}

#[test]
fn every_example_has_a_dedicated_clean_check_test() {
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
        "an example file was added or removed without updating the wasm check clean-corpus list"
    );
}
