//! Snapshot tests for the WASM `format` bridge.
//!
//! Sweeps every `examples/*/<name>.kul` and asserts the bridge round-trips
//! the formatted source unchanged. The formatter itself is exhaustively
//! property-tested in `kul-core::tests::format`; these snapshots verify
//! that the WASM seam adds no transformation. A new example file forces
//! a corresponding snapshot review here, mirroring the corpus-contract
//! pattern in `kul-core::tests::export`.

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
    ($name:ident, $dir:literal, $stem:literal) => {
        #[test]
        fn $name() {
            let path = examples_dir().join($dir).join(concat!($stem, ".kul"));
            let formatted = kul_wasm::format_source(&read(&path));
            insta::assert_snapshot!(formatted);
        }
    };
}

example_snapshot!(
    example_01_nuclear_family,
    "01-nuclear-family",
    "nuclear-family"
);
example_snapshot!(
    example_02_three_generations,
    "02-three-generations",
    "three-generations"
);
example_snapshot!(
    example_03_divorce_and_remarriage,
    "03-divorce-and-remarriage",
    "divorce-and-remarriage"
);
example_snapshot!(
    example_04_adoption_and_belonging,
    "04-adoption-and-belonging",
    "adoption-and-belonging"
);
example_snapshot!(
    example_05_cousins_and_in_laws,
    "05-cousins-and-in-laws",
    "cousins-and-in-laws"
);
example_snapshot!(
    example_06_polygamous_household,
    "06-polygamous-household",
    "polygamous-household"
);
example_snapshot!(
    example_07_disconnected_lineages,
    "07-disconnected-lineages",
    "disconnected-lineages"
);

// The multi-file project (per ADR-0015) is per-file from `format`'s
// perspective — the formatter takes one source at a time even after
// PRD 0001's signature lift (per ADR-0011's "rule of three" stance and
// PRD 0001's explicit "format stays per-source"). Snapshot each of the
// three files independently.
example_snapshot!(
    example_08_multi_file_project_01_founders,
    "08-multi-file-project",
    "01-founders"
);
example_snapshot!(
    example_08_multi_file_project_02_children,
    "08-multi-file-project",
    "02-children"
);
example_snapshot!(
    example_08_multi_file_project_03_grandchildren,
    "08-multi-file-project",
    "03-grandchildren"
);
example_snapshot!(
    example_09_family_across_a_century,
    "09-family-across-a-century",
    "family-across-a-century"
);

#[test]
fn every_example_has_a_dedicated_snapshot_test() {
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
        "an example file was added or removed without updating the wasm format snapshot list"
    );
}

#[test]
fn version_metadata_is_exposed() {
    assert_eq!(kul_wasm::kul_core_version(), env!("CARGO_PKG_VERSION"));
    assert!(!kul_wasm::kul_language_version().is_empty());
    assert!(kul_wasm::export_schema_version() >= 1);
}

#[test]
fn format_returns_string_for_partial_parse_input() {
    // Best-effort contract: format must never panic or return None even
    // when the input fails to fully parse. Mirrors `kul_core::format::format_source`.
    let _ = kul_wasm::format_source("person");
    let _ = kul_wasm::format_source("");
    let _ = kul_wasm::format_source("@@@ not kul @@@");
}
