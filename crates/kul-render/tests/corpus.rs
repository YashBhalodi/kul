//! End-to-end snapshot tests: `compute(&check)` over every Kul example
//! project in the workspace's `examples/` corpus.
//!
//! The corpus doubles as the principle-completeness contract: every
//! canonical UI pattern principle must be exercised by at
//! least one example, and the catch-all `every_example_has_a_snapshot`
//! test fires when a new example lands without a matching snapshot.

use std::path::{Path, PathBuf};

use kul_core::CheckResult;
use kul_core::ast::InputFile;
use kul_core::manifest::Manifest;

use kul_render::compute;

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

/// Load every `.kul` file from `dir` (lexicographic order, matching
/// what `kul-loader::load` does on disk) into a `CheckResult`.
fn check_example(dir: &Path) -> CheckResult {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("read_dir {}: {err}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("kul"))
        .collect();
    entries.sort();
    let inputs: Vec<InputFile> = entries
        .iter()
        .map(|p| {
            InputFile::new(
                p.file_name().unwrap().to_string_lossy().into_owned(),
                read(p),
            )
        })
        .collect();
    kul_core::check_with_manifest("kul.yml", "", &Manifest::default(), &inputs)
}

fn render_example(dir: &str) -> String {
    let check = check_example(&examples_dir().join(dir));
    let shape = compute(&check);
    serde_json::to_string_pretty(&shape).expect("serialize render shape")
}

macro_rules! example_render_snapshot {
    ($name:ident, $dir:literal) => {
        #[test]
        fn $name() {
            let json = render_example($dir);
            insta::assert_snapshot!(json);
        }
    };
}

example_render_snapshot!(example_01_single_couple, "01-single-couple");
example_render_snapshot!(example_02_nuclear_family, "02-nuclear-family");
example_render_snapshot!(example_03_three_generations, "03-three-generations");
example_render_snapshot!(example_04_polygamous_family, "04-polygamous-family");
example_render_snapshot!(example_05_married_siblings, "05-married-siblings");
example_render_snapshot!(example_06_three_branch_dynasty, "06-three-branch-dynasty");
example_render_snapshot!(
    example_07_multi_file_extended_family,
    "07-multi-file-extended-family"
);
example_render_snapshot!(
    example_08_divorce_and_remarriage,
    "08-divorce-and-remarriage"
);
example_render_snapshot!(example_09_multi_adoption, "09-multi-adoption");
example_render_snapshot!(
    example_10_disconnected_lineages_and_orphan,
    "10-disconnected-lineages-and-orphan"
);
example_render_snapshot!(example_11_cousin_marriage, "11-cousin-marriage");
example_render_snapshot!(
    example_12_polygamy_with_birth_family,
    "12-polygamy-with-birth-family"
);
example_render_snapshot!(example_13_inter_family_marriage, "13-inter-family-marriage");
example_render_snapshot!(
    example_14_grand_nested_inter_family,
    "14-grand-nested-inter-family"
);
example_render_snapshot!(
    example_15_polygamy_with_three_wives,
    "15-polygamy-with-three-wives"
);

/// Catch-all: a new `examples/<dir>/<stem>.kul` landing without a
/// matching snapshot in this file fires here. Mirrors the same
/// discipline `kul-core`'s `tests/export.rs` enforces.
#[test]
fn every_example_has_a_render_snapshot() {
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
        "14-grand-nested-inter-family",
        "15-polygamy-with-three-wives",
    ];
    assert_eq!(
        have.iter().map(String::as_str).collect::<Vec<_>>(),
        expected,
        "an example was added/removed without updating the render-shape snapshot list"
    );
}
