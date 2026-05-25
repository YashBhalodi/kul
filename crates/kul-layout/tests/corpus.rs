//! End-to-end snapshot test: run the full pipeline
//! (`kul_core::check` → `kul_render::compute` → `kul_layout::layout`)
//! over each example project the layout adapter currently supports.
//!
//! Every example project in the corpus is covered here, one test
//! per example.
//!
//! The snapshot serialises [`PositionedShape`] to YAML for diff
//! readability — `PositionedShape` is deliberately not `Serialize`
//! (ADR-0016), so this test harness defines a local serialisable
//! mirror.

use std::path::{Path, PathBuf};

use kul_core::CheckResult;
use kul_core::ast::InputFile;
use kul_core::manifest::Manifest;
use kul_layout::{
    EdgeRouting, LayoutConfig, PositionedCard, PositionedEdge, PositionedShape, SlotKind, layout,
};
use kul_render::{GhostReason, compute};
use serde::Serialize;

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

fn layout_example(dir: &str) -> String {
    let check = check_example(&examples_dir().join(dir));
    let shape = compute(&check);
    let positioned = layout(&shape, &LayoutConfig::default());
    let dump: PositionedDump = (&positioned).into();
    serde_yaml::to_string(&dump).expect("serialize positioned shape")
}

#[test]
fn example_01_nuclear_family() {
    let yaml = layout_example("01-nuclear-family");
    insta::assert_snapshot!(yaml);
}

#[test]
fn example_02_three_generations() {
    let yaml = layout_example("02-three-generations");
    insta::assert_snapshot!(yaml);
}

#[test]
fn example_03_divorce_and_remarriage() {
    let yaml = layout_example("03-divorce-and-remarriage");
    insta::assert_snapshot!(yaml);
}

#[test]
fn example_04_adoption_and_belonging() {
    let yaml = layout_example("04-adoption-and-belonging");
    insta::assert_snapshot!(yaml);
}

#[test]
fn example_05_cousins_and_in_laws() {
    let yaml = layout_example("05-cousins-and-in-laws");
    insta::assert_snapshot!(yaml);
}

#[test]
fn example_06_polygamous_household() {
    let yaml = layout_example("06-polygamous-household");
    insta::assert_snapshot!(yaml);
}

#[test]
fn example_07_disconnected_lineages() {
    let yaml = layout_example("07-disconnected-lineages");
    insta::assert_snapshot!(yaml);
}

#[test]
fn example_08_multi_file_project() {
    let yaml = layout_example("08-multi-file-project");
    insta::assert_snapshot!(yaml);
}

#[test]
fn example_09_family_across_a_century() {
    let yaml = layout_example("09-family-across-a-century");
    insta::assert_snapshot!(yaml);
}

// ---- Serialisable mirror ------------------------------------------------
//
// `PositionedShape` is intentionally not `Serialize` (ADR-0016). The
// snapshot test only needs a readable diff format; this module defines
// the local mirror so the production crate does not gain a serde
// dependency on its public types.

#[derive(Serialize)]
struct PositionedDump {
    width: f64,
    height: f64,
    cards: Vec<CardDump>,
    edges: Vec<EdgeDump>,
}

#[derive(Serialize)]
struct CardDump {
    person_id: String,
    kind: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    name: String,
}

#[derive(Serialize)]
struct EdgeDump {
    kind: String,
    routing: String,
    child_id: String,
    marriage_id: String,
    points: Vec<(f64, f64)>,
    ended: bool,
}

impl From<&PositionedShape> for PositionedDump {
    fn from(s: &PositionedShape) -> Self {
        Self {
            width: s.width,
            height: s.height,
            cards: s.cards.iter().map(CardDump::from).collect(),
            edges: s.edges.iter().map(EdgeDump::from).collect(),
        }
    }
}

impl From<&PositionedCard> for CardDump {
    fn from(c: &PositionedCard) -> Self {
        let kind = match c.kind {
            SlotKind::Canonical => "canonical".to_owned(),
            SlotKind::Ghost {
                reason: GhostReason::PastMarriage,
            } => "ghost:past_marriage".to_owned(),
            SlotKind::Ghost {
                reason: GhostReason::PastAdoption,
            } => "ghost:past_adoption".to_owned(),
            SlotKind::Ghost {
                reason: GhostReason::PastBirth,
            } => "ghost:past_birth".to_owned(),
        };
        Self {
            person_id: c.person_id.clone(),
            kind,
            x: c.x,
            y: c.y,
            width: c.width,
            height: c.height,
            name: c.name.clone(),
        }
    }
}

impl From<&PositionedEdge> for EdgeDump {
    fn from(e: &PositionedEdge) -> Self {
        let kind = match e.kind {
            kul_layout::EdgeKind::Birth => "birth".to_owned(),
            kul_layout::EdgeKind::Adoption => "adoption".to_owned(),
            kul_layout::EdgeKind::Marriage => "marriage".to_owned(),
        };
        let routing = match e.routing {
            EdgeRouting::InTree => "in_tree".to_owned(),
            EdgeRouting::CrossTree => "cross_tree".to_owned(),
        };
        Self {
            kind,
            routing,
            child_id: e.child_id.clone(),
            marriage_id: e.marriage_id.clone(),
            points: e.points.clone(),
            ended: e.ended,
        }
    }
}
