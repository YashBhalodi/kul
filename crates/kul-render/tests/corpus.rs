//! End-to-end `compute(&check)` snapshots over the `examples/` corpus.
//! Doubles as the principle-completeness contract via the catch-all
//! `every_example_has_a_snapshot` test.

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

/// Load every `.kul` file from `dir` lexicographically (mirrors
/// `kul-loader::load` on-disk order).
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

example_render_snapshot!(example_01_nuclear_family, "01-nuclear-family");
example_render_snapshot!(example_02_three_generations, "02-three-generations");
example_render_snapshot!(
    example_03_divorce_and_remarriage,
    "03-divorce-and-remarriage"
);
example_render_snapshot!(
    example_04_adoption_and_belonging,
    "04-adoption-and-belonging"
);
example_render_snapshot!(example_05_cousins_and_in_laws, "05-cousins-and-in-laws");
example_render_snapshot!(example_06_polygamous_household, "06-polygamous-household");
example_render_snapshot!(example_07_disconnected_lineages, "07-disconnected-lineages");
example_render_snapshot!(example_08_multi_file_project, "08-multi-file-project");
example_render_snapshot!(
    example_09_family_across_a_century,
    "09-family-across-a-century"
);

/// Catch-all: a new example dir without a snapshot in this file fires here.
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
        "an example was added/removed without updating the render-shape snapshot list"
    );
}
