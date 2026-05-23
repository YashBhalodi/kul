//! Snapshot tests for the WASM `check` bridge.
//!
//! Three contracts:
//!
//! - **Failure shapes** — three hand-crafted broken sources (duplicate id,
//!   unresolved reference, missing required field) lock the diagnostic
//!   wire shape that downstream JS consumers depend on. Mirrors
//!   `kul-core::tests::export::failure_envelope_*` so any drift between
//!   the CLI export envelope and the WASM `check` projection surfaces
//!   immediately.
//! - **Clean-corpus sweep** — every `examples/*/` directory must produce
//!   an empty `diagnostics` array. Single-file examples pass a one-element
//!   array; the multi-file example
//!   (`07-multi-file-extended-family`) exercises the array-based
//!   signature with every `.kul` file in the directory. Dropping a new
//!   example forces an update here.
//! - **Multi-file failure** — a cross-file `KUL-R02` (`marriage` references
//!   an id declared in a different file) locks the file-aware diagnostic
//!   shape: the `primary.file` field must carry the offending file's
//!   name, not the file that declared the referenced id.
//!
//! Broken inputs live as inline strings; the `examples/*/<name>.kul`
//! corpus stays documentation-grade.
//!
//! Snapshots are pretty-printed JSON of the `CheckEnvelope`.
//!
//! See [ADR-0011](../../../docs/adr/0011-wasm-surface-three-shapes-no-wrappers.md)
//! for why `check` is its own entrypoint with an empty-array discriminator
//! rather than a uniform `{ ok, ... }` envelope.

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

/// A `marriage` declared in `b.kul` references a `person` declared in
/// `a.kul` but typo'd as `ghost`. The diagnostic's `primary.file` must
/// point at `b.kul` (where the bad reference lives), not `a.kul`. Locks
/// the per-file anchoring that the array-based signature unlocks.
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
    example_01_single_couple_is_clean,
    "01-single-couple",
    "single-couple"
);
clean_example!(
    example_02_nuclear_family_is_clean,
    "02-nuclear-family",
    "nuclear-family"
);
clean_example!(
    example_03_three_generations_is_clean,
    "03-three-generations",
    "three-generations"
);
clean_example!(
    example_04_polygamous_family_is_clean,
    "04-polygamous-family",
    "polygamous-family"
);
clean_example!(
    example_05_married_siblings_is_clean,
    "05-married-siblings",
    "married-siblings"
);
clean_example!(
    example_06_three_branch_dynasty_is_clean,
    "06-three-branch-dynasty",
    "three-branch-dynasty"
);
clean_example!(
    example_08_divorce_and_remarriage_is_clean,
    "08-divorce-and-remarriage",
    "divorce-and-remarriage"
);
clean_example!(
    example_09_multi_adoption_is_clean,
    "09-multi-adoption",
    "multi-adoption"
);
clean_example!(
    example_10_disconnected_lineages_and_orphan_is_clean,
    "10-disconnected-lineages-and-orphan",
    "disconnected-lineages-and-orphan"
);
clean_example!(
    example_11_cousin_marriage_is_clean,
    "11-cousin-marriage",
    "cousin-marriage"
);
clean_example!(
    example_12_polygamy_with_birth_family_is_clean,
    "12-polygamy-with-birth-family",
    "polygamy-with-birth-family"
);

#[test]
fn example_07_multi_file_extended_family_is_clean() {
    let files = multi_file_inputs("07-multi-file-extended-family");
    let envelope = kul_wasm::check(files, kul_core::manifest::Manifest::default());
    assert!(
        envelope.diagnostics.is_empty(),
        "07-multi-file-extended-family produced diagnostics: {:#?}",
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
    ];
    assert_eq!(
        have.iter().map(String::as_str).collect::<Vec<_>>(),
        expected,
        "an example file was added or removed without updating the wasm check clean-corpus list"
    );
}
