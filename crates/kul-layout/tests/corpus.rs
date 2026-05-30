//! End-to-end snapshot test: full pipeline (`kul_core::check` →
//! `kul_render::compute` → `kul_layout::layout`) per example, with a
//! local YAML mirror because `PositionedShape` is not `Serialize`
//! (ADR-0016).

use std::path::{Path, PathBuf};

use kul_core::CheckResult;
use kul_core::ast::InputFile;
use kul_core::manifest::Manifest;
use kul_layout::{
    EdgeKind, LayoutConfig, PositionedCard, PositionedEdge, PositionedShape, SlotKind, layout,
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
    generation: u32,
    gender: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    given: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    born: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    died: Option<String>,
}

#[derive(Serialize)]
struct EdgeDump {
    link_kind: String,
    marriage_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    child_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_past: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    host_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    joining_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    adoption_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    adoption_end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_ended: Option<bool>,
    points: Vec<(f64, f64)>,
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
            generation: c.generation,
            gender: c.gender.to_owned(),
            family: c.family.clone(),
            given: c.given.clone(),
            born: c.born.clone(),
            died: c.died.clone(),
        }
    }
}

impl From<&PositionedEdge> for EdgeDump {
    fn from(e: &PositionedEdge) -> Self {
        let mut dump = Self {
            link_kind: String::new(),
            marriage_id: e.marriage_id.clone(),
            child_id: None,
            is_past: None,
            host_id: None,
            joining_id: None,
            start: None,
            end: None,
            end_reason: None,
            adoption_start: None,
            adoption_end: None,
            is_ended: None,
            points: e.points.clone(),
        };
        match &e.kind {
            EdgeKind::Birth { child_id, is_past } => {
                dump.link_kind = "birth".to_owned();
                dump.child_id = Some(child_id.clone());
                dump.is_past = Some(*is_past);
            }
            EdgeKind::Adoption {
                child_id,
                is_past,
                start,
                end,
            } => {
                dump.link_kind = "adoption".to_owned();
                dump.child_id = Some(child_id.clone());
                dump.is_past = Some(*is_past);
                dump.adoption_start = start.clone();
                dump.adoption_end = end.clone();
            }
            EdgeKind::Marriage {
                host_id,
                joining_id,
                start,
                end,
                end_reason,
                is_ended,
            } => {
                dump.link_kind = "marriage".to_owned();
                dump.host_id = Some(host_id.clone());
                dump.joining_id = Some(joining_id.clone());
                dump.start = Some(start.clone());
                dump.end = end.clone();
                dump.end_reason = end_reason.clone();
                dump.is_ended = Some(*is_ended);
            }
        }
        dump
    }
}
