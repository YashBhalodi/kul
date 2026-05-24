//! End-to-end snapshot test: run the full pipeline
//! (`kul_core::check` → `kul_render::compute` → `kul_layout::layout` →
//! `kul_svg::render`) over each example project the layout adapter
//! currently supports, snapshotting the raw SVG string.

use std::path::{Path, PathBuf};

use kul_core::CheckResult;
use kul_core::ast::InputFile;
use kul_core::manifest::Manifest;
use kul_layout::{LayoutConfig, layout};
use kul_render::compute;
use kul_svg::{ThemeConfig, render};

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
    let positioned = layout(&shape, &LayoutConfig::default());
    pretty(&render(&positioned, &ThemeConfig::default()))
}

/// Split the SVG into one element per line at element boundaries
/// (`><`) so diff readers don't have to scroll horizontally. Text
/// content stays attached to its parent tag.
fn pretty(svg: &str) -> String {
    svg.replace("><", ">\n<")
}

#[test]
fn example_02_nuclear_family() {
    let svg = render_example("02-nuclear-family");
    insta::assert_snapshot!(svg);
}

#[test]
fn example_03_three_generations() {
    let svg = render_example("03-three-generations");
    insta::assert_snapshot!(svg);
}

#[test]
fn example_04_polygamous_family() {
    let svg = render_example("04-polygamous-family");
    insta::assert_snapshot!(svg);
}

#[test]
fn example_08_divorce_and_remarriage() {
    let svg = render_example("08-divorce-and-remarriage");
    insta::assert_snapshot!(svg);
}
