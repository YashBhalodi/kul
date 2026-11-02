//! Every offset crossing the `Workspace` boundary is a UTF-16 code-unit
//! offset. The fixtures put Gujarati, Devanagari and emoji (one UTF-16 unit
//! for three UTF-8 bytes, two units for four bytes) before the positions each
//! feature reads or reports, on the same line and on earlier lines, and every
//! test checks the offset against what JavaScript would compute: the UTF-16
//! length of the text before it.

mod common;

use common::workspace;
use kul_language_service::{Location, Span, TokenKind};
use kul_wasm::{RenameResult, Workspace};

/// Ids are ASCII (the grammar says so); names and comments carry the scripts.
const FAMILY: &str = "# પરિવાર 👨‍👩‍👧 — परिवार\n\
person bhavna name:\"ભાવના પટેલ\" gender:female born:~1950\n\
person ravi name:\"રવિ 🎉\" gender:male\n\
marriage m_ravi_bhavna ravi bhavna start:1972 # लग्न 💍\n";

const CHILDREN: &str = concat!(
    "# 🎉 બાળકો — बच्चे\n",
    "person kiran name:\"કિરણ\" gender:other\n",
    "  birth m_ravi_bhavna\n",
    "person dev name:\"देव 🎉\"\n",
);

/// The UTF-16 offset JavaScript sees for byte offset `byte` of `text`.
fn u16_at(text: &str, byte: usize) -> usize {
    text[..byte].encode_utf16().count()
}

/// The UTF-16 offset of the first `pat` in `text`, plus `skip` bytes.
fn u16_of(text: &str, pat: &str, skip: usize) -> usize {
    u16_at(text, text.find(pat).expect("pattern in fixture") + skip)
}

/// `text[span]` the way JavaScript slices it.
fn js_slice(text: &str, span: Span) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    String::from_utf16(&units[span.start..span.end]).expect("span on char boundaries")
}

fn source(file: &str) -> &'static str {
    match file {
        "a.kul" => FAMILY,
        "b.kul" => CHILDREN,
        _ => panic!("unknown fixture file {file}"),
    }
}

fn family() -> Workspace {
    workspace(&[("a.kul", FAMILY), ("b.kul", CHILDREN)])
}

/// Where `location` points, read the way JavaScript reads it.
fn text_at(location: &Location) -> String {
    js_slice(source(&location.file), location.span)
}

#[test]
fn hover_reads_and_reports_utf16_offsets() {
    let ws = family();
    let at = u16_of(CHILDREN, "birth m_", 6) as u32;
    let hover = ws.hover("b.kul", at).expect("hover on the birth reference");
    assert_eq!(js_slice(CHILDREN, hover.span), "m_ravi_bhavna");
    assert!(hover.markdown.contains("ભાવના પટેલ"));

    let born = u16_of(FAMILY, "~1950", 0) as u32;
    let hover = ws
        .hover("a.kul", born)
        .expect("hover on a date after Gujarati");
    assert_eq!(js_slice(FAMILY, hover.span), "~1950");
}

#[test]
fn definition_references_and_locate_report_utf16_spans() {
    let ws = family();
    let at = u16_of(CHILDREN, "birth m_", 6) as u32;
    let definition = ws.definition("b.kul", at).expect("definition");
    assert_eq!(definition.file, "a.kul");
    assert_eq!(text_at(&definition), "m_ravi_bhavna");
    assert_eq!(definition.span.start, u16_of(FAMILY, "m_ravi_bhavna", 0));

    let references = ws.references("b.kul", at, true).expect("references");
    let found: Vec<(String, String)> = references
        .iter()
        .map(|l| (l.file.clone(), text_at(l)))
        .collect();
    assert_eq!(
        found,
        vec![
            ("a.kul".to_owned(), "m_ravi_bhavna".to_owned()),
            ("b.kul".to_owned(), "m_ravi_bhavna".to_owned()),
        ]
    );

    let bhavna = ws.locate("bhavna").expect("locate");
    assert_eq!(text_at(&bhavna), "bhavna");
    assert_eq!(bhavna.span.start, u16_of(FAMILY, "bhavna", 0));
}

#[test]
fn entity_at_reads_a_utf16_offset() {
    let ws = family();
    let spouse = u16_of(FAMILY, " ravi bhavna", 1) as u32;
    assert_eq!(ws.entity_at("a.kul", spouse).unwrap().id, "ravi");
    let spouse = u16_of(FAMILY, " bhavna start", 1) as u32;
    assert_eq!(ws.entity_at("a.kul", spouse).unwrap().id, "bhavna");
}

#[test]
fn completion_reads_a_utf16_offset() {
    let ws = family();
    let after = u16_of(CHILDREN, "birth m_", 6) as u32;
    let labels: Vec<String> = ws
        .complete("b.kul", after)
        .into_iter()
        .map(|c| c.label)
        .collect();
    assert_eq!(labels, vec!["m_ravi_bhavna"]);
    let gender = u16_of(FAMILY, "gender:male", "gender:".len()) as u32;
    let labels: Vec<String> = ws
        .complete("a.kul", gender)
        .into_iter()
        .map(|c| c.label)
        .collect();
    assert_eq!(labels, vec!["male", "female", "other"]);
}

#[test]
fn rename_reads_and_reports_utf16_offsets_in_every_file() {
    let ws = family();
    let at = u16_of(FAMILY, "m_ravi_bhavna", 0) as u32;
    let span = ws.prepare_rename("a.kul", at).expect("renameable");
    assert_eq!(js_slice(FAMILY, span), "m_ravi_bhavna");

    let RenameResult::Renamed { edits, .. } = ws.rename("a.kul", at, "m_rb") else {
        panic!("rename refused");
    };
    let touched: Vec<(String, String)> = edits
        .iter()
        .flat_map(|f| {
            f.edits
                .iter()
                .map(|e| (f.file.clone(), js_slice(source(&f.file), e.span)))
        })
        .collect();
    assert_eq!(
        touched,
        vec![
            ("a.kul".to_owned(), "m_ravi_bhavna".to_owned()),
            ("b.kul".to_owned(), "m_ravi_bhavna".to_owned()),
        ]
    );
}

#[test]
fn symbols_report_utf16_spans() {
    let ws = family();
    let symbols = ws.symbols("a.kul");
    let names: Vec<(&str, String)> = symbols
        .iter()
        .map(|s| (s.name.as_str(), js_slice(FAMILY, s.selection)))
        .collect();
    assert_eq!(
        names,
        vec![
            ("ભાવના પટેલ", "bhavna".to_owned()),
            ("રવિ 🎉", "ravi".to_owned()),
            ("રવિ 🎉 & ભાવના પટેલ", "m_ravi_bhavna".to_owned()),
        ]
    );
    assert!(js_slice(FAMILY, symbols[0].span).starts_with("person bhavna"));
    let kiran = &ws.symbols("b.kul")[0];
    assert_eq!(
        js_slice(CHILDREN, kiran.children[0].span),
        "birth m_ravi_bhavna"
    );
}

#[test]
fn semantic_and_lexical_tokens_report_utf16_spans() {
    let ws = family();
    for tokens in [ws.semantic_tokens("a.kul"), ws.lexical_tokens("a.kul")] {
        let texts: Vec<(String, TokenKind)> = tokens
            .iter()
            .map(|t| (js_slice(FAMILY, t.span), t.kind))
            .collect();
        for expected in [
            ("\"ભાવના પટેલ\"", TokenKind::String),
            ("~1950", TokenKind::Date),
            ("\"રવિ 🎉\"", TokenKind::String),
            ("m_ravi_bhavna", TokenKind::Declaration),
            ("1972", TokenKind::Date),
        ] {
            assert!(
                texts.contains(&(expected.0.to_owned(), expected.1)),
                "missing {expected:?} in {texts:?}"
            );
        }
    }
    let comments: Vec<String> = ws
        .lexical_tokens("a.kul")
        .into_iter()
        .filter(|t| t.kind == TokenKind::Comment)
        .map(|t| js_slice(FAMILY, t.span))
        .collect();
    assert_eq!(comments, vec!["# પરિવાર 👨‍👩‍👧 — परिवार", "# लग्न 💍"]);
}

#[test]
fn diagnostics_and_related_spans_are_utf16() {
    let duplicate = "# 🎉 again\nperson ravi name:\"रवि\" gender:male\n";
    let ws = workspace(&[("a.kul", FAMILY), ("b.kul", CHILDREN), ("c.kul", duplicate)]);
    let analysis = ws.analyze();

    let r03 = analysis
        .diagnostics
        .iter()
        .find(|d| d.code == "KUL-R03")
        .expect("dev has no gender");
    assert_eq!(text_at(r03.location.as_ref().unwrap()), "dev");

    let r01 = analysis
        .diagnostics
        .iter()
        .find(|d| d.code == "KUL-R01")
        .expect("ravi is declared twice");
    let primary = r01.location.as_ref().unwrap();
    assert_eq!(primary.file, "c.kul");
    assert_eq!(js_slice(duplicate, primary.span), "ravi");
    assert_eq!(primary.span.start, u16_of(duplicate, "ravi", 0));
    assert_eq!(text_at(&r01.related[0].location), "ravi");
}

#[test]
fn quick_fixes_read_and_report_utf16_offsets() {
    let ws = family();
    let start = u16_of(CHILDREN, "person dev", 0) as u32;
    let end = CHILDREN.encode_utf16().count() as u32;
    let fixes = ws.quick_fixes("b.kul", start, end);
    let female = fixes
        .iter()
        .find(|f| f.title == "Add `gender:female`")
        .expect("a gender fix for dev");
    assert_eq!(text_at(female.diagnostic.location.as_ref().unwrap()), "dev");
    let edit = &female.edits[0];
    // Inserted right after `name:"देव 🎉"`, on the line that holds the emoji.
    let after_name = u16_of(CHILDREN, "\"देव 🎉\"", "\"देव 🎉\"".len());
    assert_eq!(
        edit.span,
        Span {
            start: after_name,
            end: after_name
        }
    );

    let units: Vec<u16> = CHILDREN.encode_utf16().collect();
    let mut fixed: Vec<u16> = units[..edit.span.start].to_vec();
    fixed.extend(edit.text.encode_utf16());
    fixed.extend(&units[edit.span.end..]);
    assert!(
        String::from_utf16(&fixed)
            .unwrap()
            .ends_with("name:\"देव 🎉\" gender:female\n")
    );
}

#[test]
fn format_replaces_the_whole_file_in_utf16_units() {
    let messy = "# 🎉\nperson ravi name:\"રવિ\"   gender:male\n";
    let ws = workspace(&[("a.kul", messy)]);
    let edits = ws.format("a.kul").expect("formattable");
    assert_eq!(edits.len(), 1);
    assert_eq!(
        edits[0].span,
        Span {
            start: 0,
            end: messy.encode_utf16().count()
        }
    );
}

#[test]
fn completion_cursor_is_a_utf16_offset_into_its_insertion() {
    let ws = workspace(&[("a.kul", "# 🎉\nperson ravi ")]);
    let at = "# 🎉\nperson ravi ".encode_utf16().count() as u32;
    let name = ws
        .complete("a.kul", at)
        .into_iter()
        .find(|c| c.label == "name:")
        .expect("name: offered");
    let insert = name.insert.expect("name: inserts its quotes");
    assert_eq!((insert.text.as_str(), insert.cursor), ("name:\"\"", 6));
}

#[test]
fn an_offset_inside_a_surrogate_pair_snaps_to_the_character() {
    let ws = workspace(&[("a.kul", "# 🎉\nperson ravi name:\"R\" gender:male\n")]);
    // Unit 3 is the second half of 🎉; it reads as the emoji's start, which
    // is inside the comment, so nothing is there.
    assert!(ws.hover("a.kul", 3).is_none());
    assert!(ws.hover("a.kul", 13).is_some());
}
