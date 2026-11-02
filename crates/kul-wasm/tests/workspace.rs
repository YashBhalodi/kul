//! The `Workspace` lifecycle and its wiring to the language service and the
//! renderer. Feature behaviour is proven once, in `kul-language-service`;
//! these tests prove the workspace holds the right files, re-checks when
//! they change, and routes each call. Offsets here are ASCII, so bytes and
//! UTF-16 units coincide; `utf16.rs` covers the conversion.

mod common;

use common::{example_files, example_projects, example_workspace, workspace};
use kul_core::ast::InputFile;
use kul_core::query::EntityDetail;
use kul_language_service::{EntityKind, Location, Span, TokenKind};
use kul_visual::{ThemeConfig, render_from_check};
use kul_wasm::{FileVersion, RenameResult, SvgTheme, Workspace};

const FOUNDERS: &str = "person alice name:\"Alice\" gender:female\n\
                        person bob name:\"Bob\" gender:male\n\
                        marriage m alice bob start:1972\n";
const CHILDREN: &str = "person carol name:\"Carol\" gender:female\n  birth m\n";

fn codes(ws: &Workspace) -> Vec<String> {
    ws.analyze()
        .diagnostics
        .into_iter()
        .map(|d| d.code)
        .collect()
}

#[test]
fn every_example_analyzes_clean_and_renders() {
    for project in example_projects() {
        let analysis = example_workspace(&project).analyze();
        assert!(
            analysis.diagnostics.is_empty(),
            "{project}: {:?}",
            analysis.diagnostics
        );
        let svg = analysis.svg.unwrap_or_else(|| panic!("{project}: no svg"));
        assert!(svg.starts_with("<svg") && svg.contains("kul-card"));
    }
}

/// The workspace renders exactly what the shared render facade renders for
/// the same files (ADR-0031), in both themes.
#[test]
fn svgs_match_the_render_facade_for_every_example() {
    for project in example_projects() {
        let inputs: Vec<InputFile> = example_files(&project)
            .into_iter()
            .map(|(name, text)| InputFile::new(name, text))
            .collect();
        let check = kul_core::check(&inputs);
        let ws = example_workspace(&project);
        for (theme, config) in [
            (SvgTheme::Agnostic, ThemeConfig::default()),
            (SvgTheme::FileExport, ThemeConfig::for_file_export()),
        ] {
            let expected = render_from_check(&check, &config).ok();
            assert_eq!(ws.render_svg(theme), expected, "{project} {theme:?}");
        }
        assert_eq!(ws.analyze().svg, ws.render_svg(SvgTheme::Agnostic));
    }
}

#[test]
fn snapshot_example_02_svg() {
    let svg = example_workspace("02-three-generations")
        .analyze()
        .svg
        .expect("renders");
    insta::assert_snapshot!(svg);
}

#[test]
fn file_export_svg_is_self_contained_with_a_legend() {
    let svg = workspace(&[("a.kul", FOUNDERS)])
        .render_svg(SvgTheme::FileExport)
        .expect("renders");
    assert!(svg.contains("<style>"));
    assert!(svg.contains("kul-legend"));
    let agnostic = workspace(&[("a.kul", FOUNDERS)])
        .render_svg(SvgTheme::Agnostic)
        .expect("renders");
    assert!(!agnostic.contains("<style>"));
}

#[test]
fn a_project_with_errors_has_diagnostics_and_no_svg() {
    let ws = workspace(&[("a.kul", "person alice gender:female\n")]);
    let analysis = ws.analyze();
    assert!(analysis.svg.is_none());
    assert_eq!(codes(&ws), vec!["KUL-R03".to_owned()]);
    assert!(ws.render_svg(SvgTheme::FileExport).is_none());
}

fn deep_lineage_source(depth: usize) -> String {
    let mut s = String::from(
        "person c0 name:\"C0\" gender:male\nperson s0 name:\"S0\" gender:female\nmarriage m0 c0 s0 start:1000\n",
    );
    for i in 1..depth {
        s.push_str(&format!(
            "person c{i} name:\"C{i}\" gender:male\n  birth m{prev}\nperson s{i} name:\"S{i}\" gender:female\nmarriage m{i} c{i} s{i} start:{year}\n",
            prev = i - 1,
            year = 1000 + i,
        ));
    }
    s
}

/// A lineage past the depth cap passes its checks but cannot be drawn
/// (ADR-0032): the render failure joins the project's diagnostics instead of
/// trapping across the boundary.
#[test]
fn a_lineage_too_deep_to_draw_reports_kul_v01() {
    let ws = workspace(&[("deep.kul", &deep_lineage_source(3_000))]);
    let analysis = ws.analyze();
    assert!(analysis.svg.is_none());
    let v01 = analysis
        .diagnostics
        .iter()
        .find(|d| d.code == "KUL-V01")
        .expect("a KUL-V01 diagnostic");
    assert!(v01.location.is_none(), "V01 concerns the whole project");
}

#[test]
fn analysis_reports_every_file_and_version_in_file_order() {
    let mut ws = Workspace::new();
    ws.set_file("b.kul".into(), CHILDREN.into(), 4);
    ws.set_file("a.kul".into(), FOUNDERS.into(), 7);
    ws.set_file("B.kul".into(), String::new(), 1);
    assert_eq!(
        ws.analyze().files,
        vec![
            FileVersion {
                name: "B.kul".into(),
                version: 1
            },
            FileVersion {
                name: "a.kul".into(),
                version: 7
            },
            FileVersion {
                name: "b.kul".into(),
                version: 4
            },
        ]
    );
}

#[test]
fn an_edit_is_seen_by_the_next_analysis() {
    let mut ws = workspace(&[("a.kul", FOUNDERS), ("b.kul", CHILDREN)]);
    assert!(codes(&ws).is_empty());
    ws.set_file("a.kul".into(), "person alice gender:female\n".into(), 2);
    assert_eq!(
        codes(&ws),
        vec!["KUL-R03", "KUL-R02"],
        "missing name, and carol's birth now names no marriage"
    );
    assert_eq!(ws.analyze().files[0].version, 2);
    ws.set_file("a.kul".into(), FOUNDERS.into(), 3);
    assert!(codes(&ws).is_empty());
}

#[test]
fn an_edit_to_one_file_leaves_its_sibling_alone() {
    let mut ws = workspace(&[("a.kul", FOUNDERS), ("b.kul", CHILDREN)]);
    ws.set_file("a.kul".into(), format!("{FOUNDERS}# note\n"), 2);
    assert_eq!(
        ws.locate("carol"),
        Some(Location {
            file: "b.kul".into(),
            span: Span { start: 7, end: 12 }
        })
    );
}

#[test]
fn renaming_a_file_moves_its_declarations() {
    let mut ws = workspace(&[("a.kul", FOUNDERS), ("b.kul", CHILDREN)]);
    assert!(ws.rename_file("b.kul", "0-children.kul".into()));
    assert_eq!(ws.locate("carol").unwrap().file, "0-children.kul");
    let names: Vec<String> = ws.analyze().files.into_iter().map(|f| f.name).collect();
    assert_eq!(names, vec!["0-children.kul", "a.kul"]);
    assert!(codes(&ws).is_empty());
}

#[test]
fn renaming_onto_an_existing_file_or_from_a_missing_one_changes_nothing() {
    let mut ws = workspace(&[("a.kul", FOUNDERS), ("b.kul", CHILDREN)]);
    assert!(!ws.rename_file("b.kul", "a.kul".into()));
    assert!(!ws.rename_file("nope.kul", "c.kul".into()));
    assert_eq!(ws.analyze().files.len(), 2);
    assert_eq!(ws.locate("carol").unwrap().file, "b.kul");
}

#[test]
fn deleting_a_file_drops_its_declarations() {
    let mut ws = workspace(&[("a.kul", FOUNDERS), ("b.kul", CHILDREN)]);
    assert!(ws.delete_file("a.kul"));
    assert!(!ws.delete_file("a.kul"));
    assert!(ws.locate("alice").is_none());
    assert_eq!(codes(&ws), vec!["KUL-R02"]);
}

#[test]
fn an_empty_workspace_is_a_valid_empty_family() {
    let analysis = Workspace::new().analyze();
    assert!(analysis.files.is_empty());
    assert!(analysis.diagnostics.is_empty());
    assert!(analysis.svg.is_some());
}

#[test]
fn features_on_an_unknown_file_answer_nothing() {
    let ws = workspace(&[("a.kul", FOUNDERS)]);
    assert!(ws.hover("nope.kul", 0).is_none());
    assert!(ws.complete("nope.kul", 0).is_empty());
    assert!(ws.definition("nope.kul", 0).is_none());
    assert!(ws.references("nope.kul", 0, true).is_none());
    assert!(ws.prepare_rename("nope.kul", 0).is_none());
    assert!(ws.symbols("nope.kul").is_empty());
    assert!(ws.quick_fixes("nope.kul", 0, 1).is_empty());
    assert!(ws.semantic_tokens("nope.kul").is_empty());
    assert!(ws.lexical_tokens("nope.kul").is_empty());
    assert!(ws.format("nope.kul").is_none());
    assert!(ws.entity_at("nope.kul", 0).is_none());
    assert!(matches!(
        ws.rename("nope.kul", 0, "x"),
        RenameResult::Refused { .. }
    ));
}

#[test]
fn routes_every_cursor_feature() {
    let ws = workspace(&[("a.kul", FOUNDERS), ("b.kul", CHILDREN)]);
    let m_ref = (CHILDREN.find("birth m").unwrap() + "birth ".len()) as u32;
    let m_decl = Location {
        file: "a.kul".into(),
        span: Span {
            start: FOUNDERS.find("m alice").unwrap(),
            end: FOUNDERS.find("m alice").unwrap() + 1,
        },
    };

    assert!(
        ws.hover("b.kul", m_ref)
            .unwrap()
            .markdown
            .contains("marriage m")
    );
    assert_eq!(ws.definition("b.kul", m_ref), Some(m_decl.clone()));
    assert_eq!(
        ws.references("b.kul", m_ref, true).unwrap(),
        vec![
            m_decl,
            Location {
                file: "b.kul".into(),
                span: Span {
                    start: m_ref as usize,
                    end: m_ref as usize + 1
                }
            }
        ]
    );
    assert_eq!(
        ws.prepare_rename("b.kul", m_ref),
        Some(Span {
            start: m_ref as usize,
            end: m_ref as usize + 1
        })
    );
    let entity = ws.entity_at("b.kul", m_ref).unwrap();
    assert_eq!(
        (entity.id.as_str(), entity.kind),
        ("m", EntityKind::Marriage)
    );

    let gender = (CHILDREN.find("gender:").unwrap() + "gender:".len()) as u32;
    let labels: Vec<String> = ws
        .complete("b.kul", gender)
        .into_iter()
        .map(|c| c.label)
        .collect();
    assert_eq!(labels, vec!["male", "female", "other"]);
}

/// The rename the editor applies to two files at once.
#[test]
fn rename_returns_edits_for_every_file() {
    let ws = workspace(&[("a.kul", FOUNDERS), ("b.kul", CHILDREN)]);
    let RenameResult::Renamed { ok, edits } = ws.rename("a.kul", 7, "alicia") else {
        panic!("rename refused");
    };
    assert!(ok);
    let files: Vec<(&str, usize)> = edits
        .iter()
        .map(|e| (e.file.as_str(), e.edits.len()))
        .collect();
    assert_eq!(files, vec![("a.kul", 2)]);

    let RenameResult::Renamed { edits, .. } = ws.rename(
        "b.kul",
        CHILDREN.find("birth m").unwrap() as u32 + 6,
        "m_alice_bob",
    ) else {
        panic!("rename refused");
    };
    let files: Vec<&str> = edits.iter().map(|e| e.file.as_str()).collect();
    assert_eq!(files, vec!["a.kul", "b.kul"]);
}

#[test]
fn a_refused_rename_says_why() {
    let ws = workspace(&[("a.kul", FOUNDERS)]);
    let RenameResult::Refused { ok, message } = ws.rename("a.kul", 7, "bob") else {
        panic!("rename to an existing id must be refused");
    };
    assert!(!ok);
    assert!(message.contains("already used"));
}

#[test]
fn routes_every_file_feature() {
    let dirty = "person alice name:\"A\"\n";
    let ws = workspace(&[("a.kul", dirty)]);
    let symbols = ws.symbols("a.kul");
    assert_eq!(symbols.len(), 1);
    let fixes = ws.quick_fixes("a.kul", 0, dirty.len() as u32);
    assert_eq!(fixes.len(), 3);
    assert_eq!(fixes[0].diagnostic.code, "KUL-R03");
    let edits = ws.format("a.kul").unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].text, "person alice  name:\"A\"\n");

    let kinds: Vec<TokenKind> = ws.semantic_tokens("a.kul").iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Keyword,
            TokenKind::Declaration,
            TokenKind::FieldName,
            TokenKind::String
        ]
    );
    assert_eq!(ws.lexical_tokens("a.kul").len(), 4);
}

#[test]
fn lexical_tokens_need_no_analysis_and_follow_the_latest_text() {
    let mut ws = workspace(&[("a.kul", "person")]);
    ws.set_file("a.kul".into(), "# only a comment".into(), 2);
    let tokens = ws.lexical_tokens("a.kul");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Comment);
}

#[test]
fn details_answer_for_a_person_and_a_marriage() {
    let ws = workspace(&[("a.kul", FOUNDERS), ("b.kul", CHILDREN)]);
    let Some(EntityDetail::Person {
        children,
        marriages,
        ..
    }) = ws.details("alice")
    else {
        panic!("alice has person details");
    };
    assert_eq!(marriages.len(), 1);
    assert_eq!(children[0].person.id, "carol");
    assert!(matches!(
        ws.details("m"),
        Some(EntityDetail::Marriage { .. })
    ));
    assert!(ws.details("nobody").is_none());
}
